use serde_json::{Value, json};

fn text(value: Option<&Value>) -> &str {
    value.and_then(Value::as_str).unwrap_or("")
}
fn length(value: &str) -> usize {
    value.trim().chars().count()
}
fn count(value: Option<&Value>) -> usize {
    value.and_then(Value::as_array).map_or(0, Vec::len)
}
fn present<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    keys.iter()
        .find_map(|key| value.get(key).filter(|v| !v.is_null()))
}

pub fn compact_extract(payload: &Value) -> Value {
    let result = payload
        .get("result")
        .filter(|v| v.is_object())
        .unwrap_or(payload);
    let metadata = payload
        .get("metadata")
        .filter(|v| v.is_object())
        .unwrap_or(&Value::Null);
    let main_text = text(result.get("main_text"));
    let mut warnings = Vec::new();
    if length(main_text) < 200 {
        warnings.push("thin_content");
    }
    if payload.get("error").is_some_and(|v| !v.is_null()) {
        warnings.push("error");
    }
    json!({
        "request_id":payload.get("request_id"), "url":present(result,&["url","source_url"]),
        "final_url":result.get("final_url"), "status_code":result.get("status_code"),
        "title":result.get("title"), "description":result.get("description"), "main_text":main_text,
        "publish_time":result.get("publish_time"), "author":result.get("author"), "language":result.get("language"),
        "engine":present(result,&["engine","engine_name"]), "dom_id":result.get("dom_id").or_else(|| metadata.get("dom_id")),
        "error":payload.get("error"),
        "quality":{"text_length":length(main_text),"links_count":count(result.get("links")),"images_count":count(result.get("images")),"has_title":!text(result.get("title")).is_empty(),"has_description":!text(result.get("description")).is_empty(),"warnings":warnings}
    })
}

pub fn compact_dump_dom(payload: &Value) -> Value {
    let html = text(payload.get("html"));
    let mut warnings = Vec::new();
    if length(html) < 500 {
        warnings.push("thin_html");
    }
    if payload.get("error").is_some_and(|v| !v.is_null()) {
        warnings.push("error");
    }
    json!({"request_id":payload.get("request_id"),"url":payload.get("url"),"final_url":payload.get("final_url"),"status_code":payload.get("status_code"),"fetched_at":payload.get("fetched_at"),"engine":payload.get("engine"),"dom_id":payload.get("dom_id"),"html":html,"error":payload.get("error"),"quality":{"html_length":length(html),"warnings":warnings}})
}

fn scalar(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => "-".into(),
        Some(Value::String(v)) if v.is_empty() => "-".into(),
        Some(Value::String(v)) => v.clone(),
        Some(Value::Bool(true)) => "True".into(),
        Some(Value::Bool(false)) => "False".into(),
        Some(v) => v.to_string(),
    }
}
fn warnings(value: &Value) -> String {
    value
        .as_array()
        .filter(|v| !v.is_empty())
        .map(|v| {
            v.iter()
                .map(|x| format!("- {}", scalar(Some(x))))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_else(|| "- None".into())
}

pub fn render_extract_markdown(payload: &Value) -> String {
    let c = compact_extract(payload);
    let q = &c["quality"];
    let mut out = format!(
        "# WebFetch Extract Result\n\n- **Request ID:** {}\n- **URL:** {}\n- **Final URL:** {}\n- **Status:** {}\n- **Title:** {}\n- **Author:** {}\n- **Publish Time:** {}\n- **Language:** {}\n- **Engine:** {}\n- **DOM ID:** {}\n\n## Extraction Quality\n\n- **Text Length:** {}\n- **Links:** {}\n- **Images:** {}\n- **Has Title:** {}\n- **Has Description:** {}\n\n### Warnings\n\n{}",
        scalar(c.get("request_id")),
        scalar(c.get("url")),
        scalar(c.get("final_url")),
        scalar(c.get("status_code")),
        scalar(c.get("title")),
        scalar(c.get("author")),
        scalar(c.get("publish_time")),
        scalar(c.get("language")),
        scalar(c.get("engine")),
        scalar(c.get("dom_id")),
        scalar(q.get("text_length")),
        scalar(q.get("links_count")),
        scalar(q.get("images_count")),
        scalar(q.get("has_title")),
        scalar(q.get("has_description")),
        warnings(&q["warnings"])
    );
    if !text(c.get("description")).is_empty() {
        out.push_str(&format!(
            "\n\n## Description\n\n{}",
            text(c.get("description"))
        ));
    }
    if c.get("error").is_some_and(|v| !v.is_null()) {
        out.push_str(&format!("\n\n## Error\n\n{}", c["error"]));
    }
    out.push_str(&format!("\n\n## Main Text\n\n{}", text(c.get("main_text"))));
    out
}

pub fn render_extract_text(payload: &Value) -> String {
    let c = compact_extract(payload);
    let mut out = format!(
        "Title: {}\nURL: {}\nStatus: {}\nRequest ID: {}\n\n{}",
        scalar(c.get("title")),
        scalar(
            c.get("final_url")
                .filter(|v| !v.is_null())
                .or_else(|| c.get("url"))
        ),
        scalar(c.get("status_code")),
        scalar(c.get("request_id")),
        text(c.get("main_text"))
    );
    if c.get("error").is_some_and(|v| !v.is_null()) {
        let body = text(c.get("main_text")).to_owned();
        out = format!(
            "Title: {}\nURL: {}\nStatus: {}\nRequest ID: {}\n\nError: {}\n\n{}",
            scalar(c.get("title")),
            scalar(
                c.get("final_url")
                    .filter(|v| !v.is_null())
                    .or_else(|| c.get("url"))
            ),
            scalar(c.get("status_code")),
            scalar(c.get("request_id")),
            c["error"],
            body
        );
    }
    out
}

pub fn render_dump_markdown(payload: &Value) -> String {
    let c = compact_dump_dom(payload);
    let q = &c["quality"];
    let mut out = format!(
        "# WebFetch DOM Dump\n\n- **Request ID:** {}\n- **URL:** {}\n- **Final URL:** {}\n- **Status:** {}\n- **Fetched At:** {}\n- **Engine:** {}\n- **DOM ID:** {}\n\n## Dump Quality\n\n- **HTML Length:** {}\n\n### Warnings\n\n{}\n\n## HTML\n\n```html\n{}\n```",
        scalar(c.get("request_id")),
        scalar(c.get("url")),
        scalar(c.get("final_url")),
        scalar(c.get("status_code")),
        scalar(c.get("fetched_at")),
        scalar(c.get("engine")),
        scalar(c.get("dom_id")),
        scalar(q.get("html_length")),
        warnings(&q["warnings"]),
        text(c.get("html"))
    );
    if c.get("error").is_some_and(|v| !v.is_null()) {
        let marker = "\n\n## HTML";
        out = out.replacen(
            marker,
            &format!("\n\n## Error\n\n{}{}", c["error"], marker),
            1,
        );
    }
    out
}

pub fn render_dump_text(payload: &Value) -> String {
    let c = compact_dump_dom(payload);
    let mut out = format!(
        "URL: {}\nStatus: {}\nEngine: {}\nDOM ID: {}\nRequest ID: {}\n\n{}",
        scalar(
            c.get("final_url")
                .filter(|v| !v.is_null())
                .or_else(|| c.get("url"))
        ),
        scalar(c.get("status_code")),
        scalar(c.get("engine")),
        scalar(c.get("dom_id")),
        scalar(c.get("request_id")),
        text(c.get("html"))
    );
    if c.get("error").is_some_and(|v| !v.is_null()) {
        let body = text(c.get("html")).to_owned();
        out = format!(
            "URL: {}\nStatus: {}\nEngine: {}\nDOM ID: {}\nRequest ID: {}\n\nError: {}\n\n{}",
            scalar(
                c.get("final_url")
                    .filter(|v| !v.is_null())
                    .or_else(|| c.get("url"))
            ),
            scalar(c.get("status_code")),
            scalar(c.get("engine")),
            scalar(c.get("dom_id")),
            scalar(c.get("request_id")),
            c["error"],
            body
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compact_hides_trace_and_flags_thin_content() {
        let c =
            compact_extract(&json!({"result":{"title":"Example","main_text":"Hello"},"trace":[1]}));
        assert!(c.get("trace").is_none());
        assert_eq!(c["quality"]["warnings"][0], "thin_content");
    }
    #[test]
    fn markdown_is_agent_readable() {
        let value = render_extract_markdown(
            &json!({"request_id":"r1","result":{"title":"Example","main_text":"Hello"}}),
        );
        assert!(value.starts_with("# WebFetch Extract Result"));
        assert!(value.contains("## Main Text\n\nHello"));
    }
}
