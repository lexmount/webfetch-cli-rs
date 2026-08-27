use std::{fs, path::Path, process::Command};

use serde_json::Value;
use tempfile::tempdir;

const SKILL_ARCHIVE_FILES: [&str; 8] = [
    "SKILL.md",
    "references/authentication.md",
    "references/commands.md",
    "references/troubleshooting.md",
    "scripts/bootstrap.ps1",
    "scripts/bootstrap.sh",
    "scripts/doctor.ps1",
    "scripts/doctor.sh",
];

fn write_complete_skill(root: &Path) {
    for relative in SKILL_ARCHIVE_FILES {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("manifest file has a parent"))
            .expect("create Skill directory");
        fs::write(path, relative).expect("write Skill manifest file");
    }
}

fn isolated_cli(home: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_webfetch-cli"));
    command
        .env("HOME", home)
        .env_remove("CODEX_HOME")
        .env_remove("CLAUDE_SKILL_DIR")
        .env_remove("CLAUDE_CONFIG_DIR")
        .env_remove("CODEBUDDY_SKILL_DIR")
        .env_remove("LEXMOUNT_PROJECT_ID")
        .env_remove("LEXMOUNT_API_KEY")
        .env_remove("LEXMOUNT_WEBFETCH_BASE_URL")
        .env_remove("LEXMOUNT_WEBFETCH_CONNECT_BASE_URL");
    command
}

fn check<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["checks"]
        .as_array()
        .expect("doctor checks array")
        .iter()
        .find(|check| check["name"] == name)
        .unwrap_or_else(|| panic!("missing doctor check {name}"))
}

#[test]
fn doctor_cli_accepts_a_complete_claude_skill_and_environment_credentials() {
    let temp = tempdir().expect("temporary directory");
    let skill_root = temp.path().join("claude-skill");
    write_complete_skill(&skill_root);
    let credentials_path = temp.path().join("missing-credentials.json");

    let output = isolated_cli(temp.path())
        .env("CLAUDE_SKILL_DIR", &skill_root)
        .env("LEXMOUNT_WEBFETCH_CREDENTIALS_FILE", credentials_path)
        .env("LEXMOUNT_PROJECT_ID", "project-integration")
        .env("LEXMOUNT_API_KEY", "secret-integration")
        .env("LEXMOUNT_WEBFETCH_BASE_URL", "https://api.example.test")
        .args(["doctor", "--json"])
        .output()
        .expect("run doctor");

    assert!(
        output.status.success(),
        "doctor stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).expect("doctor JSON");
    assert_eq!(report["ok"], true);
    assert_eq!(check(&report, "credentials")["status"], "pass");
    assert_eq!(check(&report, "agent_skill")["status"], "pass");
    assert_eq!(
        check(&report, "agent_skill")["selected"]["sources"][0],
        "claude_skill_dir"
    );
}

#[test]
fn doctor_cli_reports_corrupt_credentials_as_json_with_a_skill_local_repair() {
    let temp = tempdir().expect("temporary directory");
    let skill_root = temp.path().join("claude-skill");
    write_complete_skill(&skill_root);
    let credentials_path = temp.path().join("credentials.json");
    fs::write(&credentials_path, b"{not-json").expect("write corrupt credentials");
    let connect_base_url = "https://browser.example.test";

    let output = isolated_cli(temp.path())
        .env("CLAUDE_SKILL_DIR", &skill_root)
        .env("LEXMOUNT_WEBFETCH_CREDENTIALS_FILE", &credentials_path)
        .env("LEXMOUNT_WEBFETCH_CONNECT_BASE_URL", connect_base_url)
        .args(["doctor", "--json"])
        .output()
        .expect("run doctor");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let report: Value = serde_json::from_slice(&output.stdout).expect("doctor JSON");
    assert_eq!(report["ok"], false);

    let credentials = check(&report, "credentials");
    assert_eq!(credentials["status"], "fail");
    assert!(
        credentials["message"]
            .as_str()
            .expect("credentials error")
            .contains("Failed to load credentials file")
    );
    let repair = &credentials["repair"];
    assert!(Path::new(repair["executable"].as_str().expect("repair executable")).is_absolute());
    assert_eq!(
        repair["arguments"],
        serde_json::json!([
            "auth",
            "login",
            "--open",
            "--connect-base-url",
            connect_base_url
        ])
    );
    assert_eq!(check(&report, "agent_skill")["status"], "pass");
}
