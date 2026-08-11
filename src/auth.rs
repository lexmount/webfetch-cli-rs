use std::{
    collections::HashMap,
    fs,
    io::{Read, Write},
    net::TcpListener,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use url::Url;

use crate::{Error, Result, client::DEFAULT_API_BASE_URL};

pub const DEFAULT_CONNECT_BASE_URL: &str = "https://browser.lexmount.cn";
pub const CONNECT_BASE_URL_ENV: &str = "LEXMOUNT_WEBFETCH_CONNECT_BASE_URL";
pub const CREDENTIALS_FILE_ENV: &str = "LEXMOUNT_WEBFETCH_CREDENTIALS_FILE";
pub const DEFAULT_SCOPES: &[&str] = &["browser:read"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub project_id: String,
    pub api_base_url: String,
    pub api_key: String,
    #[serde(default)]
    pub scope: Vec<String>,
    #[serde(default)]
    pub saved_at: Option<u64>,
}

pub fn credentials_path(override_path: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = override_path {
        return Ok(path.to_path_buf());
    }
    if let Some(path) = std::env::var_os(CREDENTIALS_FILE_ENV) {
        return Ok(PathBuf::from(path));
    }
    let root = if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(xdg)
    } else {
        dirs::home_dir()
            .ok_or_else(|| Error::Config("home directory is unavailable".into()))?
            .join(".config")
    };
    Ok(root.join("lexmount/webfetch-cli/credentials.json"))
}

pub fn load_credentials(path: Option<&Path>) -> Result<Option<Credentials>> {
    let path = credentials_path(path)?;
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_slice(&fs::read(path)?)?))
}

pub fn save_credentials(credentials: &Credentials, path: Option<&Path>) -> Result<PathBuf> {
    let path = credentials_path(path)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(credentials)?)?;
    #[cfg(windows)]
    if path.exists() {
        fs::remove_file(&path)?;
    }
    fs::rename(&tmp, &path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(path)
}

pub fn clear_credentials(path: Option<&Path>) -> Result<bool> {
    let path = credentials_path(path)?;
    if !path.exists() {
        return Ok(false);
    }
    fs::remove_file(path)?;
    Ok(true)
}

pub fn login(
    connect_base_url: &str,
    client_name: &str,
    timeout: Duration,
    open_browser: bool,
    path: Option<&Path>,
) -> Result<Value> {
    let connect_base_url = connect_base_url.trim_end_matches('/');
    let listener = TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let redirect_uri = format!(
        "http://127.0.0.1:{}/callback",
        listener.local_addr()?.port()
    );
    let verifier = random_urlsafe(48);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let state = random_urlsafe(24);
    let mut login_url = Url::parse(&format!("{connect_base_url}/connect/codex"))
        .map_err(|e| Error::Config(format!("invalid connect base URL: {e}")))?;
    login_url
        .query_pairs_mut()
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("state", &state)
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("scope", &DEFAULT_SCOPES.join(" "))
        .append_pair("client_name", client_name);
    if open_browser {
        open::that(login_url.as_str()).map_err(|e| Error::Io(std::io::Error::other(e)))?;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &json!({"ok":true,"login_url":login_url.as_str(),"opened_browser":open_browser,"callback_timeout_seconds":timeout.as_secs()})
        )?
    );

    let started = Instant::now();
    let callback = loop {
        match listener.accept() {
            Ok((mut stream, _)) => {
                stream.set_read_timeout(Some(Duration::from_secs(5)))?;
                let mut buffer = [0_u8; 16_384];
                let size = stream.read(&mut buffer)?;
                let request = String::from_utf8_lossy(&buffer[..size]);
                let target = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .ok_or_else(|| Error::Config("invalid OAuth callback request".into()))?;
                let callback = Url::parse(&format!("http://127.0.0.1{target}"))
                    .map_err(|e| Error::Config(format!("invalid OAuth callback: {e}")))?;
                let body = b"Lexmount WebFetch login received. You can close this tab.";
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )?;
                stream.write_all(body)?;
                break callback;
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock && started.elapsed() < timeout => {
                thread::sleep(Duration::from_millis(100))
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                return Err(Error::Timeout(
                    "Timed out waiting for browser authorization callback.".into(),
                ));
            }
            Err(e) => return Err(e.into()),
        }
    };
    let query: HashMap<_, _> = callback.query_pairs().collect();
    if query.get("state").map(|v| v.as_ref()) != Some(state.as_str()) {
        return Err(Error::Authentication("OAuth state mismatch".into()));
    }
    let code = query.get("code").ok_or_else(|| {
        Error::Authentication(
            query
                .get("error")
                .map(ToString::to_string)
                .unwrap_or_else(|| "callback did not include an authorization code".into()),
        )
    })?;
    let response = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?
        .post(format!("{connect_base_url}/api/connect/codex/exchange"))
        .json(&json!({"code":code,"code_verifier":verifier,"redirect_uri":redirect_uri}))
        .send()?;
    let status = response.status();
    let payload: Value = response.json()?;
    if !status.is_success() {
        return Err(Error::Api {
            status: status.as_u16(),
            message: payload
                .get("message")
                .or_else(|| payload.get("error"))
                .and_then(Value::as_str)
                .unwrap_or("credential exchange failed")
                .into(),
            body: Some(payload),
        });
    }
    let credential = payload.get("credential").unwrap_or(&payload);
    let project_id = credential
        .get("project_id")
        .or_else(|| payload.get("project_id"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let api_key = credential
        .get("api_key")
        .or_else(|| payload.get("api_key"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    if project_id.is_empty() || api_key.is_empty() {
        return Err(Error::Authentication(
            "Connect exchange did not return project_id and api_key.".into(),
        ));
    }
    let api_base_url = credential
        .get("api_base_url")
        .or_else(|| payload.get("api_base_url"))
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_API_BASE_URL)
        .trim_end_matches('/');
    if is_internal_api_base_url(api_base_url) {
        return Err(Error::Authentication(
            "credential exchange returned an internal API base URL".into(),
        ));
    }
    let scope = match payload.get("scope").or_else(|| credential.get("scope")) {
        Some(Value::Array(v)) => v
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        Some(Value::String(v)) => v.split_whitespace().map(str::to_owned).collect(),
        _ => DEFAULT_SCOPES.iter().map(|v| (*v).to_owned()).collect(),
    };
    let credentials = Credentials {
        project_id: project_id.into(),
        api_base_url: api_base_url.into(),
        api_key: api_key.into(),
        scope,
        saved_at: Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        ),
    };
    let saved = save_credentials(&credentials, path)?;
    Ok(
        json!({"ok":true,"credentials_saved":true,"credentials_file":saved,"project_id":credentials.project_id,"api_base_url":credentials.api_base_url,"scope":credentials.scope,"api_key_redacted":true}),
    )
}

fn random_urlsafe(size: usize) -> String {
    let mut bytes = vec![0_u8; size];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn is_internal_api_base_url(value: &str) -> bool {
    let host = Url::parse(value)
        .ok()
        .and_then(|url| url.host_str().map(str::to_owned))
        .unwrap_or_else(|| value.split('/').next().unwrap_or(value).to_owned())
        .trim_end_matches('.')
        .to_ascii_lowercase();
    host.contains(".svc.") || host.ends_with(".svc") || host.ends_with(".cluster.local")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn rejects_internal_cluster_api_hosts() {
        assert!(is_internal_api_base_url(
            "http://webfetch.default.svc.cluster.local"
        ));
        assert!(!is_internal_api_base_url("https://api.lexmount.cn"));
    }

    #[test]
    fn credentials_round_trip_without_exposing_secret_in_status() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("credentials.json");
        let credentials = Credentials {
            project_id: "project-1".into(),
            api_base_url: "https://api.example.test".into(),
            api_key: "secret".into(),
            scope: vec!["browser:read".into()],
            saved_at: Some(1),
        };
        save_credentials(&credentials, Some(&path)).unwrap();
        let loaded = load_credentials(Some(&path)).unwrap().unwrap();
        assert_eq!(loaded.project_id, "project-1");
        assert_eq!(loaded.api_key, "secret");
    }
}
