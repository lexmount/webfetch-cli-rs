use std::{fs, path::PathBuf, process::ExitCode, time::Duration};

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
    Skill {
        #[command(subcommand)]
        command: SkillCommand,
    },
}

#[derive(Subcommand)]
enum AuthCommand {
    Login {
        #[arg(long)]
        open: bool,
        #[arg(long)]
        connect_base_url: Option<String>,
        #[arg(long, default_value = "Agent")]
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

#[derive(Subcommand)]
enum SkillCommand {
    Status {
        #[arg(long)]
        dest: Option<PathBuf>,
    },
    Install {
        #[arg(long)]
        dest: Option<PathBuf>,
        #[arg(long)]
        force: bool,
    },
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
        Command::Skill { command } => run_skill(command)?,
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
    let stored = auth::load_credentials(Some(&path))?;
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
            json!({"authenticated":true,"credentials_file":path,"sources":sources,"project_id":client.project_id(),"api_base_url":client.base_url(),"has_api_key":true,"stored":stored.as_ref().map(|v|json!({"project_id":v.project_id,"api_base_url":v.api_base_url,"scope":v.scope,"has_api_key":!v.api_key.is_empty()}))}),
        ),
        Err(error) => Ok(
            json!({"authenticated":false,"credentials_file":path,"sources":sources,"error":error.to_string(),"login_command":format!("webfetch-cli auth login --open --connect-base-url {} --client-name Agent",auth::DEFAULT_CONNECT_BASE_URL),"next_step":"Run login_command, then rerun webfetch-cli auth status."}),
        ),
    }
}

fn default_skill_destination() -> Result<PathBuf> {
    let root = std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|v| v.join(".codex")))
        .ok_or_else(|| Error::Config("home directory is unavailable".into()))?;
    Ok(root.join("skills/lexmount-webfetch"))
}
fn skill_files() -> [(&'static str, &'static str); 4] {
    [
        (
            "SKILL.md",
            include_str!("../skills/lexmount-webfetch/SKILL.md"),
        ),
        (
            "references/authentication.md",
            include_str!("../skills/lexmount-webfetch/references/authentication.md"),
        ),
        (
            "references/commands.md",
            include_str!("../skills/lexmount-webfetch/references/commands.md"),
        ),
        (
            "references/troubleshooting.md",
            include_str!("../skills/lexmount-webfetch/references/troubleshooting.md"),
        ),
    ]
}
fn run_skill(command: SkillCommand) -> Result<()> {
    match command {
        SkillCommand::Status { dest } => {
            let dest = dest.map(Ok).unwrap_or_else(default_skill_destination)?;
            emit_json(
                &json!({"installed":dest.join("SKILL.md").exists(),"destination":dest,"skill_file":dest.join("SKILL.md")}),
            )
        }
        SkillCommand::Install { dest, force } => {
            let dest = dest.map(Ok).unwrap_or_else(default_skill_destination)?;
            if dest.exists() {
                if !force {
                    return Err(Error::Config(format!(
                        "Skill destination already exists: {}. Use --force.",
                        dest.display()
                    )));
                }
                fs::remove_dir_all(&dest)?;
            }
            for (rel, content) in skill_files() {
                let path = dest.join(rel);
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(path, content)?;
            }
            emit_json(&json!({"ok":true,"installed":true,"destination":dest}))
        }
    }
}

fn doctor() -> Result<Value> {
    let status = auth_status()?;
    let destination = default_skill_destination()?;
    let credentials_ok = status["authenticated"] == true;
    let workbuddy_skill = std::env::var_os("CODEBUDDY_SKILL_DIR")
        .map(PathBuf::from)
        .is_some_and(|path| path.join("SKILL.md").exists());
    let skill_ok = destination.join("SKILL.md").exists() || workbuddy_skill;
    Ok(
        json!({"ok":credentials_ok&&skill_ok,"status":if credentials_ok&&skill_ok{"pass"}else{"fail"},"checks":[
            {"name":"cli_version","status":"pass","version":env!("CARGO_PKG_VERSION")},
            if credentials_ok {json!({"name":"credentials","status":"pass","project_id":status["project_id"],"api_base_url":status["api_base_url"],"has_api_key":true})} else {json!({"name":"credentials","status":"fail","message":status["error"],"repair_command":status["login_command"]})},
            {"name":"codex_skill","status":if skill_ok{"pass"}else{"warn"},"destination":destination,"repair_command":"webfetch-cli skill install --force"}
        ]}),
    )
}

fn capabilities() -> Value {
    json!({"name":"webfetch-cli","version":env!("CARGO_PKG_VERSION"),"default_format":"md","formats":["md","text","json","json-full"],"commands":{"extract":{"inputs":["url","dom_id"],"options":["timeout_ms","format","include_trace","include_raw_dom"],"default_output":"agent_readable_markdown","debug_output":"json-full"},"dump-dom":{"inputs":["url"],"options":["timeout_ms","format","engine","filter_scripts_styles"],"default_output":"agent_readable_markdown","debug_output":"json-full"}},"exit_codes":{"0":"success","1":"runtime or API error","2":"invalid CLI usage"}})
}
