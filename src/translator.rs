//! Calls the user's chosen AI provider. Anthropic uses the Messages API; OpenAI, OpenRouter and
//! Custom share the OpenAI chat-completions format, so they differ only by base URL.

use std::fmt;
use std::net::IpAddr;
use std::time::Duration;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ureq::http::Uri;

pub const ANTHROPIC_VERSION: &str = "2026-10-21";
pub const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";
pub const TIMEOUT_SECS: u64 = 60;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Provider {
    Anthropic,
    OpenAI,
    OpenRouter,
    Custom,
}

impl Provider {
    pub const ALL: [Provider; 4] = [
        Provider::Anthropic,
        Provider::OpenAI,
        Provider::OpenRouter,
        Provider::Custom,
    ];

    /// Serde and keyring id.
    pub fn name(self) -> &'static str {
        match self {
            Provider::Anthropic => "Anthropic",
            Provider::OpenAI => "OpenAI",
            Provider::OpenRouter => "OpenRouter",
            Provider::Custom => "Custom",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Provider::Custom => "Custom (OpenAI-compatible)",
            other => other.name(),
        }
    }

    pub fn default_model(self) -> &'static str {
        match self {
            Provider::Anthropic => "claude-opus-5-5",
            Provider::OpenAI => "gpt-5-mini",
            Provider::OpenRouter => "openai/gpt-5-mini",
            Provider::Custom => "",
        }
    }

    pub fn default_base_url(self) -> &'static str {
        match self {
            Provider::Anthropic => "https://api.anthropic.com/v1",
            Provider::OpenAI => "https://api.openai.com/v1",
            Provider::OpenRouter => "https://openrouter.ai/api/v1",
            Provider::Custom => "http://localhost:11434/v1",
        }
    }
}

/// What to translate. Exactly one of image or text, by type.
pub enum Input {
    Png(Vec<u8>),
    Text(String),
}

pub struct TranslateJob {
    pub provider: Provider,
    pub base_url: String,
    pub model: String,
    pub api_key: String,
    pub target_language: String,
    pub input: Input,
}

// Hand-written: a derived Debug would print the API key and the image.
impl fmt::Debug for TranslateJob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TranslateJob {{ provider: {:?}, model: {} }}",
            self.provider, self.model
        )
    }
}

pub struct HttpRequest {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Value,
}

/// API keys only travel over HTTPS. Plain HTTP is allowed for a server on this machine.
pub fn is_allowed_base_url(url: &str) -> bool {
    let Ok(uri) = url.parse::<Uri>() else {
        return false;
    };
    let Some(host) = uri.host() else {
        return false;
    };
    match uri.scheme_str() {
        Some(s) if s.eq_ignore_ascii_case("https") => true,
        Some(s) if s.eq_ignore_ascii_case("http") => {
            host.eq_ignore_ascii_case("localhost")
                || host
                    .trim_start_matches('[')
                    .trim_end_matches(']')
                    .parse::<IpAddr>()
                    .is_ok_and(|ip| ip.is_loopback())
        }
        _ => false,
    }
}

fn supports_fallback(model: &str) -> bool {
    matches!(
        model,
        "claude-opus-5-5" | "claude-opus-5" | "claude-fable-5-1" | "claude-sonnet-5-5"
    )
}

/// `^claude-(opus|sonnet|haiku|fable|mythos)-5(-|$)`
fn is_claude5(model: &str) -> bool {
    let Some(rest) = model.strip_prefix("claude-") else {
        return false;
    };
    ["opus", "sonnet", "haiku", "fable", "mythos"]
        .iter()
        .filter_map(|family| rest.strip_prefix(family))
        .filter_map(|r| r.strip_prefix("-5"))
        .any(|tail| tail.is_empty() || tail.starts_with('-'))
}

fn instructions(lang: &str) -> String {
    format!(
        "You translate text captured from a screenshot. Reply with only the translation into {lang}: \
         no notes, no quotes, no preamble. Keep the original line breaks. \
         If the text is already in {lang}, reply with it unchanged. \
         If there is no readable text, reply with exactly: (no text found)"
    )
}

pub fn build_request(job: &TranslateJob) -> HttpRequest {
    let base = job.base_url.trim_end_matches('/');
    let mut headers = Vec::new();
    let url = if job.provider == Provider::Anthropic {
        headers.push(("x-api-key".to_string(), job.api_key.clone()));
        headers.push((
            "anthropic-version".to_string(),
            ANTHROPIC_VERSION.to_string(),
        ));
        if supports_fallback(&job.model) {
            headers.push(("anthropic-beta".to_string(), FALLBACK_BETA.to_string()));
        }
        format!("{base}/messages")
    } else {
        if !job.api_key.is_empty() {
            headers.push((
                "Authorization".to_string(),
                format!("Bearer {}", job.api_key),
            ));
        }
        format!("{base}/chat/completions")
    };
    headers.push(("content-type".to_string(), "application/json".to_string()));
    HttpRequest {
        url,
        headers,
        body: build_body(job),
    }
}

pub fn build_body(job: &TranslateJob) -> Value {
    let lang = &job.target_language;
    let instructions = instructions(lang);
    let prompt = match &job.input {
        Input::Png(_) => format!("Translate the text in this image into {lang}."),
        Input::Text(text) => format!("Translate this text into {lang}:\n\n{text}"),
    };

    if job.provider == Provider::Anthropic {
        let mut content = Vec::new();
        if let Input::Png(png) = &job.input {
            content.push(json!({
                "type": "image",
                "source": { "type": "base64", "media_type": "image/png", "data": STANDARD.encode(png) },
            }));
        }
        content.push(json!({ "type": "text", "text": prompt }));
        let mut body = json!({
            "model": job.model,
            "max_tokens": 8192,
            "system": instructions,
            "messages": [{ "role": "user", "content": content }],
        });
        // Translation is a simple task: low effort keeps the popup fast. Older models reject the field.
        if is_claude5(&job.model) {
            body["output_config"] = json!({ "effort": "low" });
        }
        if supports_fallback(&job.model) {
            body["fallbacks"] = json!("default");
        }
        return body;
    }

    let user_content = match &job.input {
        Input::Text(_) => json!(prompt),
        Input::Png(png) => json!([
            { "type": "text", "text": prompt },
            { "type": "image_url", "image_url": { "url": format!("data:image/png;base64,{}", STANDARD.encode(png)) } },
        ]),
    };
    json!({
        "model": job.model,
        "messages": [
            { "role": "system", "content": instructions },
            { "role": "user", "content": user_content },
        ],
    })
}

const UNREADABLE: &str = "The provider sent a reply Snipslate couldn't read.";

pub fn parse_response(provider: Provider, status: u16, body: &str) -> Result<String, String> {
    let ok = (200..300).contains(&status);
    let root = match serde_json::from_str::<Value>(body) {
        Ok(v) => v,
        Err(_) if ok => return Err(UNREADABLE.to_string()),
        Err(_) => Value::Null,
    };
    if !ok {
        return Err(describe_error(status, error_message(&root).as_deref()));
    }
    if !root.is_object() {
        return Err(UNREADABLE.to_string());
    }
    // OpenRouter can report an upstream failure inside an HTTP 200.
    if let Some(error) = error_message(&root) {
        return Err(format!(
            "The provider returned an error.\n\nProvider said: {error}"
        ));
    }
    if provider == Provider::Anthropic {
        read_anthropic(&root)
    } else {
        read_openai(&root)
    }
}

pub fn describe_error(status: u16, detail: Option<&str>) -> String {
    let summary = match status {
        401 | 403 => "The API key was rejected. Check it in Settings.".to_string(),
        404 => "Not found. Check the model name and base URL in Settings.".to_string(),
        429 => "Rate limited or out of credit. Wait a moment, or check your account balance."
            .to_string(),
        500.. => "The provider had a server error. Try again in a moment.".to_string(),
        _ => format!("The provider refused the request (HTTP {status})."),
    };
    match detail.filter(|d| !d.trim().is_empty()) {
        Some(d) => format!("{summary}\n\nProvider said: {d}"),
        None => summary,
    }
}

fn read_anthropic(root: &Value) -> Result<String, String> {
    if root["stop_reason"].as_str() == Some("refusal") {
        return Err("The model declined to translate this snip.".to_string());
    }
    let text: String = root["content"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|b| b["type"].as_str() == Some("text"))
        .filter_map(|b| b["text"].as_str())
        .collect();
    finish(&text)
}

fn read_openai(root: &Value) -> Result<String, String> {
    let message = &root["choices"][0]["message"];
    if message.is_object() {
        if let Some(content) = message["content"].as_str() {
            return finish(content);
        }
        if let Some(refusal) = message["refusal"].as_str() {
            return Err(format!(
                "The model declined to translate this snip.\n\nIt said: {refusal}"
            ));
        }
    }
    Err("The provider's reply had no translation in it.".to_string())
}

fn error_message(root: &Value) -> Option<String> {
    match root.as_object()?.get("error")? {
        Value::String(s) => Some(s.clone()),
        Value::Object(o) => o.get("message")?.as_str().map(str::to_string),
        _ => None,
    }
}

fn finish(text: &str) -> Result<String, String> {
    let text = text.trim();
    if text.is_empty() {
        Err("The model returned an empty reply.".to_string())
    } else {
        Ok(text.to_string())
    }
}

/// Blocking; call from a worker thread.
pub fn translate(job: &TranslateJob) -> Result<String, String> {
    // Checked here, not only in Settings, because settings.json can be edited by hand.
    if !is_allowed_base_url(&job.base_url) {
        return Err(format!(
            "The base URL \"{}\" isn't allowed. It must start with https:// (http:// only for localhost).",
            job.base_url
        ));
    }
    let request = build_request(job);
    let mut builder = agent(TIMEOUT_SECS).post(&request.url);
    for (name, value) in &request.headers {
        builder = builder.header(name.as_str(), value.as_str());
    }
    builder
        .send(request.body.to_string())
        .and_then(|mut response| {
            let status = response.status().as_u16();
            let body = response.body_mut().read_to_string()?;
            Ok((status, body))
        })
        .map_err(|err| transport_error(err, &request.url, TIMEOUT_SECS))
        .and_then(|(status, body)| parse_response(job.provider, status, &body))
}

fn agent(timeout_secs: u64) -> ureq::Agent {
    let config = ureq::Agent::config_builder()
        .http_status_as_error(false)
        // A redirect must never carry the key to another origin.
        .max_redirects(0)
        .timeout_global(Some(Duration::from_secs(timeout_secs)))
        .build();
    ureq::Agent::new_with_config(config)
}

fn transport_error(err: ureq::Error, url: &str, timeout_secs: u64) -> String {
    if matches!(err, ureq::Error::Timeout(_)) {
        return format!(
            "The provider took longer than {timeout_secs} seconds to answer. Try again."
        );
    }
    let uri = url.parse::<Uri>().ok();
    let host = uri.as_ref().and_then(Uri::host).unwrap_or("the provider");
    format!("Couldn't reach {host}. Check your internet connection and the base URL.\n\n{err}")
}

pub const LIST_TIMEOUT_SECS: u64 = 20;
const MAX_MODEL_PAGES: usize = 10;
const UNREADABLE_MODELS: &str = "The provider sent a model list Snipslate couldn't read.";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelInfo {
    pub id: String,
    /// Display name, or the id when the provider has none.
    pub label: String,
    /// Known image support; `None` when the provider doesn't say.
    pub vision: Option<bool>,
}

/// GET request for one page of the model list. `after_id` is Anthropic's cursor.
fn list_request(
    provider: Provider,
    base_url: &str,
    api_key: &str,
    after_id: Option<&str>,
) -> HttpRequest {
    let base = base_url.trim_end_matches('/');
    let mut headers = Vec::new();
    let url = if provider == Provider::Anthropic {
        headers.push(("x-api-key".to_string(), api_key.to_string()));
        headers.push((
            "anthropic-version".to_string(),
            ANTHROPIC_VERSION.to_string(),
        ));
        match after_id {
            Some(id) => format!("{base}/models?limit=1000&after_id={id}"),
            None => format!("{base}/models?limit=1000"),
        }
    } else {
        if !api_key.is_empty() {
            headers.push(("Authorization".to_string(), format!("Bearer {api_key}")));
        }
        format!("{base}/models")
    };
    HttpRequest {
        url,
        headers,
        body: Value::Null,
    }
}

// ponytail: substring heuristic on OpenAI ids; /models has no capability field, so a new
// non-chat family slips through (or a chat model with "audio"/"image" in its name is hidden).
// Upgrade: an allowlist of chat families if it gets noisy.
fn is_chat_model(id: &str) -> bool {
    const NOT_CHAT: [&str; 12] = [
        "embedding",
        "tts",
        "whisper",
        "dall-e",
        "moderation",
        "transcribe",
        "realtime",
        "audio",
        "image",
        "search",
        "babbage",
        "davinci",
    ];
    !NOT_CHAT.iter().any(|s| id.contains(s))
}

/// Returns the page's models and, for Anthropic with more pages, the next `after_id`.
fn parse_models(
    provider: Provider,
    status: u16,
    body: &str,
) -> Result<(Vec<ModelInfo>, Option<String>), String> {
    let ok = (200..300).contains(&status);
    let root = match serde_json::from_str::<Value>(body) {
        Ok(v) => v,
        Err(_) if ok => return Err(UNREADABLE_MODELS.to_string()),
        Err(_) => Value::Null,
    };
    if !ok {
        return Err(describe_error(status, error_message(&root).as_deref()));
    }
    let Some(items) = root["data"].as_array() else {
        return Err(UNREADABLE_MODELS.to_string());
    };
    let mut models: Vec<ModelInfo> = items
        .iter()
        .filter_map(|m| {
            let id = m["id"].as_str().filter(|id| !id.is_empty())?;
            let name = m["name"].as_str();
            let display = m["display_name"].as_str();
            let label = match provider {
                Provider::Anthropic => display,
                Provider::OpenRouter => name,
                _ => None,
            };
            let vision = (provider == Provider::OpenRouter)
                .then(|| m["architecture"]["input_modalities"].as_array())
                .flatten()
                .map(|mods| mods.iter().any(|x| x.as_str() == Some("image")));
            Some(ModelInfo {
                id: id.to_string(),
                label: label.filter(|l| !l.is_empty()).unwrap_or(id).to_string(),
                vision,
            })
        })
        .collect();
    let mut next = None;
    match provider {
        Provider::Anthropic => {
            if root["has_more"].as_bool() == Some(true) {
                next = root["last_id"].as_str().map(str::to_string);
            }
        }
        _ => {
            if provider == Provider::OpenAI {
                models.retain(|m| is_chat_model(&m.id));
            }
            models.sort_by(|a, b| a.id.cmp(&b.id));
            models.dedup_by(|a, b| a.id == b.id);
        }
    }
    Ok((models, next))
}

/// Models this key can use. Blocking; call from a worker thread.
pub fn list_models(
    provider: Provider,
    base_url: &str,
    api_key: &str,
) -> Result<Vec<ModelInfo>, String> {
    if !is_allowed_base_url(base_url) {
        return Err(format!(
            "The base URL \"{base_url}\" isn't allowed. It must start with https:// (http:// only for localhost)."
        ));
    }
    let agent = agent(LIST_TIMEOUT_SECS);
    let mut all = Vec::new();
    let mut after: Option<String> = None;
    for _ in 0..MAX_MODEL_PAGES {
        let request = list_request(provider, base_url, api_key, after.as_deref());
        let mut builder = agent.get(&request.url);
        for (name, value) in &request.headers {
            builder = builder.header(name.as_str(), value.as_str());
        }
        let (status, body) = builder
            .call()
            .and_then(|mut response| {
                let status = response.status().as_u16();
                let body = response.body_mut().read_to_string()?;
                Ok((status, body))
            })
            .map_err(|err| transport_error(err, &request.url, LIST_TIMEOUT_SECS))?;
        let (mut page, next) = parse_models(provider, status, &body)?;
        all.append(&mut page);
        match next {
            Some(cursor) => after = Some(cursor),
            None => break,
        }
    }
    Ok(all)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG: [u8; 3] = [1, 2, 3];

    fn job(provider: Provider, model: Option<&str>, input: Input, key: &str) -> TranslateJob {
        TranslateJob {
            provider,
            base_url: provider.default_base_url().to_string(),
            model: model.unwrap_or(provider.default_model()).to_string(),
            api_key: key.to_string(),
            target_language: "English".to_string(),
            input,
        }
    }

    fn png_job(provider: Provider, model: Option<&str>) -> TranslateJob {
        job(provider, model, Input::Png(PNG.to_vec()), "secret")
    }

    fn header<'a>(req: &'a HttpRequest, name: &str) -> Option<&'a str> {
        req.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    #[test]
    fn anthropic_request_sends_image_with_version_and_fallback() {
        let j = png_job(Provider::Anthropic, None);
        let req = build_request(&j);
        assert_eq!(req.url, "https://api.anthropic.com/v1/messages");
        assert_eq!(header(&req, "x-api-key"), Some("secret"));
        assert_eq!(header(&req, "anthropic-version"), Some(ANTHROPIC_VERSION));
        assert_eq!(header(&req, "anthropic-beta"), Some(FALLBACK_BETA));
        assert_eq!(header(&req, "authorization"), None);

        let body = build_body(&j);
        assert_eq!(body["model"], "claude-opus-5-5");
        assert_eq!(body["fallbacks"], "default");
        assert_eq!(body["output_config"]["effort"], "low");
        assert!(body.get("temperature").is_none());
        let content = &body["messages"][0]["content"];
        assert_eq!(content[0]["type"], "image");
        assert_eq!(content[0]["source"]["media_type"], "image/png");
        assert_eq!(content[0]["source"]["data"], "AQID");
        assert_eq!(content[1]["type"], "text");
    }

    #[test]
    fn older_claude_models_get_no_effort_or_fallback() {
        let j = png_job(Provider::Anthropic, Some("claude-haiku-4-5"));
        assert_eq!(header(&build_request(&j), "anthropic-beta"), None);
        let body = build_body(&j);
        assert!(body.get("output_config").is_none());
        assert!(body.get("fallbacks").is_none());
    }

    #[test]
    fn haiku_five_gets_effort_but_no_fallback() {
        let body = build_body(&png_job(Provider::Anthropic, Some("claude-haiku-5-5")));
        assert_eq!(body["output_config"]["effort"], "low");
        assert!(body.get("fallbacks").is_none());
    }

    #[test]
    fn openai_compatible_request_sends_a_data_url() {
        for (provider, url) in [
            (
                Provider::OpenAI,
                "https://api.openai.com/v1/chat/completions",
            ),
            (
                Provider::OpenRouter,
                "https://openrouter.ai/api/v1/chat/completions",
            ),
        ] {
            let j = png_job(provider, None);
            let req = build_request(&j);
            assert_eq!(req.url, url);
            assert_eq!(header(&req, "authorization"), Some("Bearer secret"));
            let messages = &build_body(&j)["messages"];
            assert_eq!(messages[0]["role"], "system");
            let content = &messages[1]["content"];
            assert_eq!(content[1]["type"], "image_url");
            assert_eq!(content[1]["image_url"]["url"], "data:image/png;base64,AQID");
        }
    }

    #[test]
    fn text_only_sends_plain_string_content() {
        let body = build_body(&job(
            Provider::OpenAI,
            None,
            Input::Text("Hola".to_string()),
            "secret",
        ));
        let content = &body["messages"][1]["content"];
        assert!(content.as_str().is_some_and(|s| s.contains("Hola")));
    }

    #[test]
    fn custom_without_key_sends_no_authorization() {
        let mut j = job(
            Provider::Custom,
            Some("llama3.2-vision"),
            Input::Png(PNG.to_vec()),
            "",
        );
        j.base_url = "http://localhost:11434/v1/".to_string();
        let req = build_request(&j);
        assert_eq!(req.url, "http://localhost:11434/v1/chat/completions");
        assert_eq!(header(&req, "authorization"), None);
    }

    #[test]
    fn base_url_must_be_https_or_local() {
        for (url, allowed) in [
            ("https://example.com/v1", true),
            ("http://localhost:1234/v1", true),
            ("http://127.0.0.1:8080", true),
            ("http://example.com/v1", false),
            ("ftp://example.com", false),
            ("not a url", false),
            ("", false),
        ] {
            assert_eq!(is_allowed_base_url(url), allowed, "{url:?}");
        }
    }

    #[test]
    fn refuses_to_send_a_key_over_plain_http() {
        let mut j = job(
            Provider::Custom,
            Some("m"),
            Input::Text("Hola".to_string()),
            "secret",
        );
        j.base_url = "http://example.com/v1".to_string();
        let err = translate(&j).expect_err("must refuse");
        assert!(err.contains("https://"));
    }

    #[test]
    fn job_debug_hides_the_key() {
        let text = format!(
            "{:?}",
            job(
                Provider::OpenAI,
                None,
                Input::Text("x".to_string()),
                "secret"
            )
        );
        assert!(!text.contains("secret"));
    }

    #[test]
    fn reads_anthropic_text_blocks() {
        let body = r#"{"stop_reason":"end_turn","content":[{"type":"thinking","thinking":""},{"type":"text","text":" Hello "},{"type":"text","text":"world"}]}"#;
        let text = parse_response(Provider::Anthropic, 200, body).expect("text");
        assert_eq!(text, "Hello world");
    }

    #[test]
    fn anthropic_refusal_is_an_error() {
        let err = parse_response(
            Provider::Anthropic,
            200,
            r#"{"stop_reason":"refusal","content":[]}"#,
        )
        .expect_err("refusal");
        assert!(err.contains("declined"));
    }

    #[test]
    fn reads_openai_message_content() {
        let body = r#"{"choices":[{"message":{"role":"assistant","content":"Hello world\n"}}]}"#;
        let text = parse_response(Provider::OpenAI, 200, body).expect("text");
        assert_eq!(text, "Hello world");
    }

    #[test]
    fn error_inside_http_200_is_reported() {
        let body = r#"{"error":{"message":"Provider returned error","code":502}}"#;
        let err = parse_response(Provider::OpenRouter, 200, body).expect_err("error");
        assert!(err.contains("Provider returned error"));
    }

    #[test]
    fn rejected_key_names_the_key() {
        let body = r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#;
        let err = parse_response(Provider::Anthropic, 401, body).expect_err("401");
        assert!(err.contains("API key"));
        assert!(err.contains("invalid x-api-key"));
    }

    #[test]
    fn non_json_server_error_explains_itself() {
        let err =
            parse_response(Provider::OpenAI, 502, "<html>bad gateway</html>").expect_err("502");
        assert!(err.contains("server error"));
    }

    #[test]
    fn empty_reply_is_an_error() {
        let body = r#"{"choices":[{"message":{"content":"  "}}]}"#;
        assert!(parse_response(Provider::OpenAI, 200, body).is_err());
    }

    #[test]
    fn anthropic_text_only_has_one_text_block() {
        let body = build_body(&job(
            Provider::Anthropic,
            None,
            Input::Text("Hola".to_string()),
            "secret",
        ));
        let content = body["messages"][0]["content"]
            .as_array()
            .expect("content array");
        assert_eq!(content.len(), 1);
        assert_eq!(content[0]["type"], "text");
    }

    fn ids(models: &[ModelInfo]) -> Vec<&str> {
        models.iter().map(|m| m.id.as_str()).collect()
    }

    #[test]
    fn anthropic_models_page_follows_cursor() {
        let more = r#"{"data":[{"id":"claude-opus-5-5","display_name":"Claude Opus 5.5"},{"id":"claude-sonnet-5-5","display_name":"Claude Sonnet 5.5"}],"has_more":true,"last_id":"claude-sonnet-5-5"}"#;
        let (models, next) = parse_models(Provider::Anthropic, 200, more).expect("page");
        assert_eq!(ids(&models), ["claude-opus-5-5", "claude-sonnet-5-5"]);
        assert_eq!(models[0].label, "Claude Opus 5.5");
        assert_eq!(models[0].vision, None);
        assert_eq!(next.as_deref(), Some("claude-sonnet-5-5"));

        let last = r#"{"data":[{"id":"claude-haiku-4-5","display_name":"Claude Haiku 4.5"}],"has_more":false,"last_id":"claude-haiku-4-5"}"#;
        let (_, next) = parse_models(Provider::Anthropic, 200, last).expect("page");
        assert_eq!(next, None);
    }

    #[test]
    fn anthropic_list_url_has_limit_and_cursor() {
        let first = list_request(
            Provider::Anthropic,
            "https://api.anthropic.com/v1/",
            "secret",
            None,
        );
        assert_eq!(first.url, "https://api.anthropic.com/v1/models?limit=1000");
        assert_eq!(header(&first, "x-api-key"), Some("secret"));
        assert_eq!(header(&first, "anthropic-version"), Some(ANTHROPIC_VERSION));
        let next = list_request(
            Provider::Anthropic,
            "https://api.anthropic.com/v1",
            "secret",
            Some("m1"),
        );
        assert_eq!(
            next.url,
            "https://api.anthropic.com/v1/models?limit=1000&after_id=m1"
        );
        assert!(!next.url.contains("secret"));
    }

    #[test]
    fn openai_list_drops_non_chat_models_and_sorts() {
        let body = r#"{"data":[{"id":"gpt-5-mini"},{"id":"text-embedding-3-small"},{"id":"tts-1"},{"id":"whisper-1"},{"id":"gpt-4o"},{"id":"gpt-4o"}]}"#;
        let (models, next) = parse_models(Provider::OpenAI, 200, body).expect("list");
        assert_eq!(ids(&models), ["gpt-4o", "gpt-5-mini"]);
        assert_eq!(models[0].label, "gpt-4o");
        assert_eq!(next, None);
    }

    #[test]
    fn custom_list_keeps_everything() {
        let body = r#"{"data":[{"id":"whisper-local"},{"id":"llama3.2"}]}"#;
        let (models, _) = parse_models(Provider::Custom, 200, body).expect("list");
        assert_eq!(ids(&models), ["llama3.2", "whisper-local"]);
    }

    #[test]
    fn openrouter_list_reads_name_and_vision() {
        let body = r#"{"data":[
            {"id":"b/text","name":"Text Only","architecture":{"input_modalities":["text"]}},
            {"id":"a/vis","name":"Seer","architecture":{"input_modalities":["text","image"]}},
            {"id":"c/none"}]}"#;
        let (models, _) = parse_models(Provider::OpenRouter, 200, body).expect("list");
        assert_eq!(ids(&models), ["a/vis", "b/text", "c/none"]);
        assert_eq!(models[0].label, "Seer");
        assert_eq!(models[0].vision, Some(true));
        assert_eq!(models[1].vision, Some(false));
        assert_eq!(models[2].label, "c/none");
        assert_eq!(models[2].vision, None);
    }

    #[test]
    fn custom_list_without_key_sends_no_authorization() {
        let req = list_request(Provider::Custom, "http://localhost:11434/v1/", "", None);
        assert_eq!(req.url, "http://localhost:11434/v1/models");
        assert_eq!(header(&req, "authorization"), None);
        let keyed = list_request(
            Provider::OpenAI,
            "https://api.openai.com/v1",
            "secret",
            None,
        );
        assert_eq!(header(&keyed, "authorization"), Some("Bearer secret"));
    }

    #[test]
    fn list_errors_are_described() {
        let body = r#"{"error":{"message":"bad key"}}"#;
        let err = parse_models(Provider::OpenAI, 401, body).expect_err("401");
        assert!(err.contains("API key"));
        assert!(err.contains("bad key"));
        assert_eq!(
            parse_models(Provider::OpenAI, 200, "<html>").expect_err("junk"),
            UNREADABLE_MODELS
        );
        assert_eq!(
            parse_models(Provider::OpenAI, 200, r#"{"ok":1}"#).expect_err("no data"),
            UNREADABLE_MODELS
        );
        let (empty, _) = parse_models(Provider::OpenAI, 200, r#"{"data":[]}"#).expect("empty");
        assert!(empty.is_empty());
    }

    #[test]
    fn list_refuses_plain_http_without_leaking_the_key() {
        let err = list_models(Provider::Custom, "http://example.com/v1", "secret")
            .expect_err("must refuse");
        assert!(err.contains("https://"));
        assert!(!err.contains("secret"));
    }

    #[test]
    fn describe_error_maps_429_and_404() {
        assert!(describe_error(429, None).starts_with("Rate limited"));
        assert!(describe_error(404, Some("  ")).starts_with("Not found"));
        assert!(
            describe_error(404, Some("no such model")).ends_with("Provider said: no such model")
        );
    }
}
