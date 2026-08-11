use std::{env, time::Duration};

use reqwest::blocking::Client as HttpClient;
use serde_json::{Value, json};

use crate::{Error, Result, auth};

pub const DEFAULT_API_BASE_URL: &str = "https://api.lexmount.cn";
pub const BASE_URL_ENV: &str = "LEXMOUNT_WEBFETCH_BASE_URL";
pub const API_KEY_ENV: &str = "LEXMOUNT_API_KEY";
pub const PROJECT_ID_ENV: &str = "LEXMOUNT_PROJECT_ID";

#[derive(Debug, Clone, Default)]
pub struct ClientBuilder {
    api_key: Option<String>,
    project_id: Option<String>,
    base_url: Option<String>,
    timeout: Option<Duration>,
}

impl ClientBuilder {
    pub fn api_key(mut self, value: impl Into<String>) -> Self {
        self.api_key = Some(value.into());
        self
    }
    pub fn project_id(mut self, value: impl Into<String>) -> Self {
        self.project_id = Some(value.into());
        self
    }
    pub fn base_url(mut self, value: impl Into<String>) -> Self {
        self.base_url = Some(value.into());
        self
    }
    pub fn timeout(mut self, value: Duration) -> Self {
        self.timeout = Some(value);
        self
    }

    pub fn build(self) -> Result<Client> {
        let stored = auth::load_credentials(None).ok().flatten();
        let project_id = self
            .project_id
            .or_else(|| env::var(PROJECT_ID_ENV).ok())
            .or_else(|| stored.as_ref().map(|c| c.project_id.clone()))
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                Error::Config(format!(
                    "Missing project id. Run webfetch-cli auth login or set {PROJECT_ID_ENV}."
                ))
            })?;
        let api_key = self
            .api_key
            .or_else(|| env::var(API_KEY_ENV).ok())
            .or_else(|| stored.as_ref().map(|c| c.api_key.clone()))
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                Error::Config(format!(
                    "Missing API key. Run webfetch-cli auth login or set {API_KEY_ENV}."
                ))
            })?;
        let base_url = self
            .base_url
            .or_else(|| env::var(BASE_URL_ENV).ok())
            .or_else(|| stored.as_ref().map(|c| c.api_base_url.clone()))
            .unwrap_or_else(|| DEFAULT_API_BASE_URL.into())
            .trim_end_matches('/')
            .to_owned();
        let http = HttpClient::builder()
            .timeout(self.timeout.unwrap_or(Duration::from_secs(30)))
            .build()?;
        Ok(Client {
            api_key,
            project_id,
            base_url,
            http,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Client {
    api_key: String,
    project_id: String,
    base_url: String,
    http: HttpClient,
}

impl Client {
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }
    pub fn from_env() -> Result<Self> {
        Self::builder().build()
    }
    pub fn project_id(&self) -> &str {
        &self.project_id
    }
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    fn post(&self, path: &str, body: &Value) -> Result<Value> {
        let response = self
            .http
            .post(format!("{}{}", self.base_url, path))
            .header("x-project-id", &self.project_id)
            .header("x-api-key", &self.api_key)
            .header("accept", "application/json")
            .json(body)
            .send()?;
        let status = response.status();
        let bytes = response.bytes()?;
        let payload: Value = if bytes.is_empty() {
            json!({})
        } else {
            serde_json::from_slice(&bytes)?
        };
        if status.is_success() {
            return Ok(payload);
        }
        let message = payload
            .get("message")
            .or_else(|| payload.get("error"))
            .or_else(|| payload.get("details"))
            .map(|v| {
                v.as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| v.to_string())
            })
            .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
        Err(Error::Api {
            status: status.as_u16(),
            message,
            body: Some(payload),
        })
    }

    pub fn extract(
        &self,
        url: Option<&str>,
        dom_id: Option<&str>,
        include_trace: bool,
        include_raw_dom: bool,
    ) -> Result<Value> {
        if url.is_none() && dom_id.is_none() {
            return Err(Error::Config(
                "Either --url or --dom-id is required.".into(),
            ));
        }
        let mut extract = serde_json::Map::new();
        if let Some(url) = url {
            extract.insert("url".into(), json!(url));
        }
        if let Some(dom_id) = dom_id {
            extract.insert("dom_id".into(), json!(dom_id));
        }
        let mut body = json!({"extract": extract});
        if include_trace || include_raw_dom {
            let mut trace = serde_json::Map::new();
            if include_trace {
                trace.insert("include_steps".into(), json!(true));
            }
            if include_raw_dom {
                trace.insert("include_raw_dom".into(), json!(true));
            }
            body["trace"] = Value::Object(trace);
        }
        self.post("/v1/extract", &body)
    }

    pub fn dump_dom(
        &self,
        url: &str,
        engine: Option<&str>,
        timeout_ms: Option<u64>,
        filter_scripts_styles: bool,
    ) -> Result<Value> {
        let mut body = json!({"url": url});
        let mut options = serde_json::Map::new();
        if let Some(engine) = engine {
            options.insert("engine_preference".into(), json!(engine));
        }
        if let Some(timeout_ms) = timeout_ms {
            options.insert("timeout_ms".into(), json!(timeout_ms));
        }
        if filter_scripts_styles {
            options.insert("filter_scripts_styles".into(), json!(true));
        }
        if !options.is_empty() {
            body["options"] = Value::Object(options);
        }
        self.post("/v1/dom/dump", &body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::{Method::POST, MockServer};

    #[test]
    fn extract_sends_python_compatible_request() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/v1/extract")
                .header("x-project-id", "project-1")
                .header("x-api-key", "secret")
                .json_body(json!({"extract":{"url":"https://example.com"}}));
            then.status(200)
                .json_body(json!({"result":{"main_text":"Hello"}}));
        });
        let client = Client::builder()
            .project_id("project-1")
            .api_key("secret")
            .base_url(server.base_url())
            .build()
            .unwrap();
        let value = client
            .extract(Some("https://example.com"), None, false, false)
            .unwrap();
        mock.assert();
        assert_eq!(value["result"]["main_text"], "Hello");
    }

    #[test]
    fn dump_dom_sends_options() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST).path("/v1/dom/dump").json_body(json!({"url":"https://example.com","options":{"engine_preference":"lightmount_dcl","timeout_ms":7000,"filter_scripts_styles":true}}));
            then.status(200).json_body(json!({"html":"<main>Hello</main>"}));
        });
        let client = Client::builder()
            .project_id("p")
            .api_key("k")
            .base_url(server.base_url())
            .build()
            .unwrap();
        client
            .dump_dom(
                "https://example.com",
                Some("lightmount_dcl"),
                Some(7000),
                true,
            )
            .unwrap();
        mock.assert();
    }

    #[test]
    fn extract_debug_options_match_python_contract() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST).path("/v1/extract").json_body(json!({
                "extract":{"dom_id":"dom-1"},
                "trace":{"include_steps":true,"include_raw_dom":true}
            }));
            then.status(200)
                .json_body(json!({"result":{"main_text":"Hello"}}));
        });
        let client = Client::builder()
            .project_id("p")
            .api_key("k")
            .base_url(server.base_url())
            .build()
            .unwrap();
        client.extract(None, Some("dom-1"), true, true).unwrap();
        mock.assert();
    }
}
