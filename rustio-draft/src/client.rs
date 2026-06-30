//! The one place that talks to an LLM. Calls the Claude Messages API over raw
//! HTTP (there is no official Anthropic Rust SDK), constrains the response to
//! the import contract via structured outputs, and parses it into a [`SchemaDoc`].

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};

use crate::schema::{self, SchemaDoc};

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";

/// Default model. Overridable on the CLI; this is the reference default.
pub const DEFAULT_MODEL: &str = "claude-opus-4-8";

/// System prompt: teach the model the contract and the field-type vocabulary.
const SYSTEM_PROMPT: &str = "\
You are a database schema designer for rustio-admin, a Postgres admin framework. \
Given a short description of an application, design a clean, normalised set of models.

Rules you MUST follow:
- Use ONLY these field types: text, integer, boolean, timestamp.
- Model names are CamelCase singular nouns (Client, Appointment), no spaces.
- Field names are snake_case (full_name, starts_at).
- NEVER add an `id` field or a `created_at` field — the generator emits those implicitly.
- Map money/prices/amounts to `integer` (store minor units, e.g. cents).
- Map dates and date-times to `timestamp`. Map counts/quantities to `integer`.
- Relations are out of scope for now: instead of a foreign-key field, add a plain \
`integer` field named `<thing>_id` (e.g. client_id) and note nothing else.
- Prefer a small, sensible set of fields per model over an exhaustive one.

Return only the schema in the required JSON shape — no prose.";

/// A configured client for one generation run.
pub struct DraftClient {
    api_key: String,
    model: String,
    max_tokens: u32,
    http: reqwest::Client,
}

impl DraftClient {
    /// Build a client. `api_key` is the Anthropic API key; `model` and
    /// `max_tokens` come from the CLI (with [`DEFAULT_MODEL`] as the default).
    pub fn new(api_key: String, model: String, max_tokens: u32) -> Self {
        Self {
            api_key,
            model,
            max_tokens,
            http: reqwest::Client::new(),
        }
    }

    /// Turn a natural-language brief into a [`SchemaDoc`]. The result is the raw
    /// model proposal — callers MUST still run [`schema::validate`] before use.
    pub async fn generate(&self, brief: &str) -> Result<SchemaDoc> {
        let body = json!({
            "model": self.model,
            "max_tokens": self.max_tokens,
            "thinking": { "type": "adaptive" },
            "system": SYSTEM_PROMPT,
            "output_config": {
                "format": { "type": "json_schema", "schema": schema::import_json_schema() }
            },
            "messages": [ { "role": "user", "content": brief } ]
        });

        let resp = self
            .http
            .post(API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .json(&body)
            .send()
            .await
            .context("could not reach the Claude API")?;

        let status = resp.status();
        let v: Value = resp
            .json()
            .await
            .context("Claude API returned a non-JSON response")?;

        if !status.is_success() {
            let msg = v
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("unknown error");
            bail!("Claude API error ({status}): {msg}");
        }

        parse_schema_response(&v)
    }
}

/// Extract and parse the schema from a successful Messages API response.
/// Pulled out of the network path so it can be unit-tested.
fn parse_schema_response(v: &Value) -> Result<SchemaDoc> {
    match v.get("stop_reason").and_then(|s| s.as_str()) {
        Some("refusal") => bail!(
            "the model declined this request (stop_reason: refusal). Try rephrasing the brief."
        ),
        Some("max_tokens") => bail!(
            "the response hit the token limit before the schema was complete; \
             re-run with a larger --max-tokens"
        ),
        _ => {}
    }

    let text = v
        .get("content")
        .and_then(|c| c.as_array())
        .ok_or_else(|| anyhow!("unexpected API response: no content array"))?
        .iter()
        .find(|b| b.get("type").and_then(|t| t.as_str()) == Some("text"))
        .and_then(|b| b.get("text"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| anyhow!("the response contained no text block to parse"))?;

    serde_json::from_str(text).context("the model's output was not a valid schema document")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_structured_text_block() {
        // Mimics an adaptive-thinking response: an (empty) thinking block then text.
        let v = json!({
            "stop_reason": "end_turn",
            "content": [
                { "type": "thinking", "thinking": "" },
                { "type": "text", "text": "{ \"models\": [ { \"name\": \"Client\", \"fields\": [ { \"name\": \"full_name\", \"type\": \"text\" } ] } ] }" }
            ]
        });
        let doc = parse_schema_response(&v).unwrap();
        assert_eq!(doc.models.len(), 1);
        assert_eq!(doc.models[0].name, "Client");
    }

    #[test]
    fn refusal_is_a_clean_error() {
        let v = json!({ "stop_reason": "refusal", "content": [] });
        let err = parse_schema_response(&v).unwrap_err().to_string();
        assert!(err.contains("refusal"), "{err}");
    }

    #[test]
    fn truncation_is_a_clean_error() {
        let v = json!({ "stop_reason": "max_tokens", "content": [] });
        let err = parse_schema_response(&v).unwrap_err().to_string();
        assert!(err.contains("max-tokens"), "{err}");
    }
}
