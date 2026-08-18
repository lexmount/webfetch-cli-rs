use std::{path::PathBuf, process::ExitCode, time::Duration};

mod skill;

use clap::{Args, Parser, Subcommand, ValueEnum};
use lexmount_webfetch::{Client, Error, Result, auth, output};
use serde_json::{Value, json};

#[derive(Parser)]
#[command(
    name = "webfetch-cli",
    version,
    about = "Native Lexmount WebFetch client"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Version,
    Doctor {
        #[arg(long)]
        json: bool,
    },
    Capabilities {
        #[arg(long)]
        json: bool,
    },
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    Extract(ExtractArgs),
    DumpDom(DumpDomArgs),
}

#[derive(Subcommand)]
enum AuthCommand {
    Login {
        #[arg(long)]
        open: bool,
        #[arg(long)]
        connect_base_url: Option<String>,
        #[arg(long, default_value = auth::DEFAULT_CLIENT_NAME)]
        client_name: String,
        #[arg(long, default_value_t = 300)]
        timeout_seconds: u64,
    },
    Status,
    ClearCredentials,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Md,
    Text,
    Json,
    JsonFull,
}

#[derive(Args)]
struct ExtractArgs {
    #[arg(long)]
    url: Option<String>,
    #[arg(long)]
    dom_id: Option<String>,
    #[arg(long)]
    timeout_ms: Option<u64>,
    #[arg(long, value_enum, default_value = "md")]
    format: OutputFormat,
    #[arg(long)]
    include_trace: bool,
    #[arg(long)]
    include_raw_dom: bool,
}

#[derive(Args)]
struct DumpDomArgs {
    #[arg(long)]
    url: String,
    #[arg(long)]
    timeout_ms: Option<u64>,
    #[arg(long, value_enum, default_value = "md")]
    format: OutputFormat,
    #[arg(long, value_parser=["auto","http","chrome","chrome_cdp","lightmount_lite","lightmount_dcl","lightmount_domstable"])]
    engine: Option<String>,
    #[arg(long)]
    filter_scripts_styles: bool,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<u8> {
    match cli.command {
        Command::Version => {
            emit_json(
                &json!({"name":"webfetch-cli","version":env!("CARGO_PKG_VERSION"),"api_base_url_env":"LEXMOUNT_WEBFETCH_BASE_URL","credentials_file":auth::credentials_path(None)?}),
            )?;
        }
        Command::Capabilities { .. } => {
            emit_json(&capabilities())?;
        }
        Command::Doctor { .. } => {
            let value = doctor()?;
            let code = if value["ok"] == true { 0 } else { 1 };
            emit_json(&value)?;
            return Ok(code);
        }
        Command::Auth { command } => match command {
            AuthCommand::Login {
                open,
                connect_base_url,
                client_name,
                timeout_seconds,
            } => {
                let base = connect_base_url
                    .or_else(|| std::env::var(auth::CONNECT_BASE_URL_ENV).ok())
                    .unwrap_or_else(|| auth::DEFAULT_CONNECT_BASE_URL.into());
                emit_json(&auth::login(
                    &base,
                    &client_name,
                    Duration::from_secs(timeout_seconds),
                    open,
                    None,
                )?)?;
            }
            AuthCommand::Status => {
                emit_json(&auth_status()?)?;
            }
            AuthCommand::ClearCredentials => {
                let path = auth::credentials_path(None)?;
                emit_json(
                    &json!({"ok":true,"removed":auth::clear_credentials(Some(&path))?,"credentials_file":path}),
                )?;
            }
        },
        Command::Extract(args) => {
            if (args.include_trace || args.include_raw_dom)
                && !matches!(args.format, OutputFormat::JsonFull)
            {
                return Err(Error::Config(
                    "--include-trace and --include-raw-dom require --format json-full.".into(),
                ));
            }
            let mut builder = Client::builder();
            if let Some(ms) = args.timeout_ms {
                builder = builder.timeout(Duration::from_millis(ms.max(1000)));
            }
            let payload = builder.build()?.extract(
                args.url.as_deref(),
                args.dom_id.as_deref(),
                args.include_trace,
                args.include_raw_dom,
            )?;
            emit_formatted(&payload, args.format, true)?;
        }
        Command::DumpDom(args) => {
            let mut builder = Client::builder();
            if let Some(ms) = args.timeout_ms {
                builder = builder.timeout(Duration::from_millis(ms.max(1000)));
            }
            let payload = builder.build()?.dump_dom(
                &args.url,
                args.engine.as_deref(),
                args.timeout_ms,
                args.filter_scripts_styles,
            )?;
            emit_formatted(&payload, args.format, false)?;
        }
    }
    Ok(0)
}

fn emit_json(value: &Value) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
fn emit_formatted(payload: &Value, format: OutputFormat, extract: bool) -> Result<()> {
    match (format, extract) {
        (OutputFormat::JsonFull, _) => emit_json(payload),
        (OutputFormat::Json, true) => emit_json(&output::compact_extract(payload)),
        (OutputFormat::Json, false) => emit_json(&output::compact_dump_dom(payload)),
        (OutputFormat::Text, true) => {
            println!("{}", output::render_extract_text(payload));
            Ok(())
        }
        (OutputFormat::Text, false) => {
            println!("{}", output::render_dump_text(payload));
            Ok(())
        }
        (OutputFormat::Md, true) => {
            println!("{}", output::render_extract_markdown(payload));
            Ok(())
        }
        (OutputFormat::Md, false) => {
            println!("{}", output::render_dump_markdown(payload));
            Ok(())
        }
    }
}

fn auth_status() -> Result<Value> {
    let path = auth::credentials_path(None)?;
    let (stored, credentials_file_error) = match auth::load_credentials(Some(&path)) {
        Ok(stored) => (stored, None),
        Err(error) => (None, Some(error.to_string())),
    };
    let env_project = std::env::var_os("LEXMOUNT_PROJECT_ID").is_some();
    let env_key = std::env::var_os("LEXMOUNT_API_KEY").is_some();
    let env_base = std::env::var_os("LEXMOUNT_WEBFETCH_BASE_URL").is_some();
    let sources = json!({
        "project_id": if env_project { Some("env") } else if stored.is_some() { Some("credentials_file") } else { None },
        "api_key": if env_key { Some("env") } else if stored.is_some() { Some("credentials_file") } else { None },
        "api_base_url": if env_base { Some("env") } else if stored.is_some() { Some("credentials_file") } else { None },
    });
    match Client::from_env() {
        Ok(client) => Ok(
            json!({"authenticated":true,"credentials_file":path,"credentials_file_error":credentials_file_error,"sources":sources,"project_id":client.project_id(),"api_base_url":client.base_url(),"has_api_key":true,"stored":stored.as_ref().map(|v|json!({"project_id":v.project_id,"api_base_url":v.api_base_url,"scope":v.scope,"has_api_key":!v.api_key.is_empty()}))}),
        ),
        Err(error) => {
            let error = credentials_file_error
                .as_ref()
                .map(|file_error| {
                    format!(
                        "Failed to load credentials file {}: {file_error}",
                        path.display()
                    )
                })
                .unwrap_or_else(|| error.to_string());
            Ok(json!({
                "authenticated": false,
                "credentials_file": path,
                "credentials_file_error": credentials_file_error,
                "sources": sources,
                "error": error,
                "login_command": auth_login_invocation(
                    std::env::current_exe().ok(),
                    &std::env::var(auth::CONNECT_BASE_URL_ENV)
                        .unwrap_or_else(|_| auth::DEFAULT_CONNECT_BASE_URL.to_owned())
                ),
                "next_step": "Run login_command.executable with login_command.arguments. Add --client-name with the current Agent's name when available, then rerun the Skill-local CLI's auth status command."
            }))
        }
    }
}

fn auth_login_invocation(executable: Option<PathBuf>, connect_base_url: &str) -> Value {
    json!({
        "executable": executable,
        "arguments": [
            "auth",
            "login",
            "--open",
            "--connect-base-url",
            connect_base_url
        ]
    })
}

fn doctor() -> Result<Value> {
    Ok(doctor_report(
        &auth_status()?,
        skill::discover_skill_roots_from_process(),
    ))
}

fn doctor_report(status: &Value, candidates: Vec<skill::SkillRootCandidate>) -> Value {
    let credentials_ok = status["authenticated"] == true;
    let active = candidates
        .iter()
        .find(|candidate| {
            candidate
                .sources
                .contains(&skill::SkillRootSource::CurrentExecutable)
        })
        .or_else(|| {
            candidates.iter().find(|candidate| {
                candidate.sources.iter().any(|source| {
                    matches!(
                        source,
                        skill::SkillRootSource::ClaudeSkillDir
                            | skill::SkillRootSource::WorkBuddySkillDir
                    )
                })
            })
        });
    let active_root = active.map(|candidate| candidate.root.clone());
    let selected_candidate = match active {
        Some(candidate) => candidate.is_installed().then_some(candidate),
        None => candidates.iter().find(|candidate| candidate.is_installed()),
    };
    let selected = selected_candidate
        .map(|candidate| json!({"root":candidate.root,"sources":candidate.sources}));
    let skill_ok = selected.is_some();
    let candidate_reports = candidates
        .iter()
        .map(|candidate| {
            json!({
                "root": candidate.root,
                "sources": candidate.sources,
                "complete": candidate.manifest.complete,
                "missing_files": candidate.manifest.missing_files,
                "active": active_root.as_ref() == Some(&candidate.root),
            })
        })
        .collect::<Vec<_>>();
    let ok = credentials_ok && skill_ok;

    json!({"ok":ok,"status":if ok{"pass"}else{"fail"},"checks":[
        {"name":"cli_version","status":"pass","version":env!("CARGO_PKG_VERSION")},
        if credentials_ok {json!({"name":"credentials","status":"pass","project_id":status["project_id"],"api_base_url":status["api_base_url"],"has_api_key":true})} else {json!({"name":"credentials","status":"fail","message":status["error"],"repair":status["login_command"]})},
        {"name":"agent_skill","status":if skill_ok{"pass"}else{"fail"},"installed":skill_ok,"active_root":active_root,"selected":selected,"candidates":candidate_reports,"required_files":skill::SKILL_ARCHIVE_FILES,"repair":"Install the complete official Skill ZIP at the current Agent's Skill root, then run the Skill-local bootstrap and doctor scripts."}
    ]})
}

fn capabilities() -> Value {
    json!({"name":"webfetch-cli","version":env!("CARGO_PKG_VERSION"),"default_format":"md","formats":["md","text","json","json-full"],"commands":{"extract":{"inputs":["url","dom_id"],"options":["timeout_ms","format","include_trace","include_raw_dom"],"default_output":"agent_readable_markdown","debug_output":"json-full"},"dump-dom":{"inputs":["url"],"options":["timeout_ms","format","engine","filter_scripts_styles"],"default_output":"agent_readable_markdown","debug_output":"json-full"}},"exit_codes":{"0":"success","1":"runtime or API error","2":"invalid CLI usage"}})
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use tempfile::tempdir;

    fn complete_candidate(
        source: skill::SkillRootSource,
    ) -> (tempfile::TempDir, skill::SkillRootCandidate) {
        let dir = tempdir().unwrap();
        for relative in skill::SKILL_ARCHIVE_FILES {
            let path = dir.path().join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, relative).unwrap();
        }
        let candidate = skill::SkillRootCandidate {
            root: dir.path().to_path_buf(),
            sources: vec![source],
            manifest: skill::validate_skill_root(dir.path()),
        };
        (dir, candidate)
    }

    fn authenticated_status() -> Value {
        json!({
            "authenticated": true,
            "project_id": "project-1",
            "api_base_url": "https://api.example.test"
        })
    }

    #[test]
    fn auth_login_defaults_client_name_to_agent() {
        let cli = Cli::try_parse_from(["webfetch-cli", "auth", "login"]).unwrap();
        let Command::Auth {
            command: AuthCommand::Login { client_name, .. },
        } = cli.command
        else {
            panic!("expected auth login command");
        };

        assert_eq!(client_name, auth::DEFAULT_CLIENT_NAME);
    }

    #[test]
    fn auth_login_parses_custom_client_name() {
        let client_name = "Claude Code 中文";
        let cli = Cli::try_parse_from([
            "webfetch-cli",
            "auth",
            "login",
            "--client-name",
            client_name,
        ])
        .unwrap();
        let Command::Auth {
            command:
                AuthCommand::Login {
                    client_name: parsed,
                    ..
                },
        } = cli.command
        else {
            panic!("expected auth login command");
        };

        assert_eq!(parsed, client_name);
    }

    #[test]
    fn auth_login_repair_uses_an_absolute_executable_and_argument_array() {
        let executable = PathBuf::from("/tmp/Skill Root/bin/webfetch-cli");
        let connect_base_url = "https://browser.example.test";
        let invocation = auth_login_invocation(Some(executable.clone()), connect_base_url);

        assert_eq!(invocation["executable"], json!(executable));
        assert_eq!(
            invocation["arguments"],
            json!([
                "auth",
                "login",
                "--open",
                "--connect-base-url",
                connect_base_url
            ])
        );
    }

    #[test]
    fn doctor_accepts_complete_skills_from_all_three_agents() {
        for source in [
            skill::SkillRootSource::CodexAgentsHome,
            skill::SkillRootSource::ClaudeSkillDir,
            skill::SkillRootSource::WorkBuddySkillDir,
            skill::SkillRootSource::ClaudeConfigDir,
            skill::SkillRootSource::CodexHome,
        ] {
            let (_dir, candidate) = complete_candidate(source);
            let report = doctor_report(&authenticated_status(), vec![candidate]);

            assert_eq!(report["ok"], true);
            assert_eq!(report["status"], "pass");
            assert_eq!(report["checks"][2]["name"], "agent_skill");
            assert_eq!(report["checks"][2]["status"], "pass");
            assert_eq!(report["checks"][2]["selected"]["sources"][0], json!(source));
        }
    }

    #[test]
    fn doctor_fails_when_the_skill_zip_is_incomplete() {
        let (dir, mut candidate) = complete_candidate(skill::SkillRootSource::ClaudeSkillDir);
        fs::remove_file(dir.path().join("scripts/doctor.sh")).unwrap();
        candidate.manifest = skill::validate_skill_root(dir.path());

        let report = doctor_report(&authenticated_status(), vec![candidate]);

        assert_eq!(report["ok"], false);
        assert_eq!(report["checks"][2]["status"], "fail");
        assert_eq!(
            report["checks"][2]["candidates"][0]["missing_files"],
            json!(["scripts/doctor.sh"])
        );
    }

    #[test]
    fn doctor_does_not_hide_an_incomplete_active_skill_with_another_installation() {
        for active_source in [
            skill::SkillRootSource::CurrentExecutable,
            skill::SkillRootSource::ClaudeSkillDir,
            skill::SkillRootSource::WorkBuddySkillDir,
        ] {
            let (_other_dir, other) = complete_candidate(skill::SkillRootSource::CodexAgentsHome);
            let (active_dir, mut active) = complete_candidate(active_source);
            fs::remove_file(active_dir.path().join("scripts/doctor.sh")).unwrap();
            active.manifest = skill::validate_skill_root(active_dir.path());

            let report = doctor_report(&authenticated_status(), vec![other, active]);

            assert_eq!(report["ok"], false);
            assert_eq!(report["checks"][2]["active_root"], json!(active_dir.path()));
            assert_eq!(report["checks"][2]["selected"], Value::Null);
            assert_eq!(report["checks"][2]["candidates"][0]["active"], false);
            assert_eq!(report["checks"][2]["candidates"][1]["active"], true);
        }
    }

    #[test]
    fn doctor_treats_configuration_roots_as_fallback_candidates() {
        for fallback_source in [
            skill::SkillRootSource::ClaudeDefaultHome,
            skill::SkillRootSource::ClaudeConfigDir,
            skill::SkillRootSource::CodexHome,
        ] {
            let (_codex_dir, codex) = complete_candidate(skill::SkillRootSource::CodexAgentsHome);
            let fallback_dir = tempdir().unwrap();
            let fallback = skill::SkillRootCandidate {
                root: fallback_dir.path().to_path_buf(),
                sources: vec![fallback_source],
                manifest: skill::validate_skill_root(fallback_dir.path()),
            };

            let report = doctor_report(&authenticated_status(), vec![codex, fallback]);

            assert_eq!(report["ok"], true);
            assert_eq!(
                report["checks"][2]["selected"]["sources"][0],
                json!(skill::SkillRootSource::CodexAgentsHome)
            );
            assert_eq!(report["checks"][2]["active_root"], Value::Null);
        }
    }

    #[test]
    fn doctor_fails_when_credentials_are_missing() {
        let (_dir, candidate) = complete_candidate(skill::SkillRootSource::CodexAgentsHome);
        let report = doctor_report(
            &json!({
                "authenticated": false,
                "error": "missing credentials",
                "login_command": auth_login_invocation(
                    Some(PathBuf::from("/skill/bin/webfetch-cli")),
                    auth::DEFAULT_CONNECT_BASE_URL
                )
            }),
            vec![candidate],
        );

        assert_eq!(report["ok"], false);
        assert_eq!(report["checks"][1]["status"], "fail");
        assert_eq!(report["checks"][2]["status"], "pass");
    }

    #[test]
    fn legacy_skill_install_commands_are_not_exposed() {
        assert!(Cli::try_parse_from(["webfetch-cli", "skill", "status"]).is_err());
        assert!(Cli::try_parse_from(["webfetch-cli", "skill", "install"]).is_err());
    }
}
