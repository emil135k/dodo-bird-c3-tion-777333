//! LLM Ant — The Brain
//!
//! Subscribes to stt_text (what the swarm heard)
//! Thinks via configurable LLM provider
//! Publishes response to tts_text (for TTS ant to speak)
//!
//! Config: reads llm-ant.json for provider, model, system prompt
//! Providers: ollama (local), anthropic (claude)
//! Future: google (gemini) — not implemented yet
//!
//! Contract: stt_text contains recognized speech text (UTF-8).
//! This ant is a text-to-text gateway — no audio processing.
//!
//! Data flow: STT → [stt_text] → LLM → [tts_text] → TTS

use iceoryx2::prelude::*;
use iceoryx2_bb_system_types::path::Path;
use iceoryx2_bb_container::semantic_string::SemanticString;
use serde::Deserialize;
use serde_json::json;

const CONFIG_PATH: &str = "/Users/rocketman/crystalballmini/hypAiAssist/config/llm-ant.json";

#[derive(Deserialize, Debug)]
struct LlmConfig {
    #[serde(default = "d_provider")]
    provider: String,
    #[serde(default = "d_model")]
    model: String,
    #[serde(default = "d_url")]
    url: String,
    #[serde(default = "d_key_env")]
    api_key_env: String,
    #[serde(default = "d_prompt")]
    system_prompt: String,
    #[serde(default = "d_max_tokens")]
    max_tokens: u32,
    #[serde(default = "d_timeout_secs")]
    timeout_secs: u64,
}

fn d_provider() -> String { "ollama".into() }
fn d_model() -> String { "gemma4".into() }
fn d_url() -> String { "http://localhost:11434/api/chat".into() }
fn d_key_env() -> String { "".into() }
fn d_prompt() -> String {
    "You are Jarvina, a concise voice assistant. One sentence replies ONLY. Never more than 15 words. No markdown.".into()
}
fn d_max_tokens() -> u32 { 50 }
fn d_timeout_secs() -> u64 { 30 }

impl Default for LlmConfig {
    fn default() -> Self {
        Self { provider: d_provider(), model: d_model(), url: d_url(),
               api_key_env: d_key_env(), system_prompt: d_prompt(),
               max_tokens: d_max_tokens(), timeout_secs: d_timeout_secs() }
    }
}

impl LlmConfig {
    fn load() -> Self {
        match std::fs::read_to_string(CONFIG_PATH) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                eprintln!("[LLM] Config parse error: {} — defaults", e);
                Self::default()
            }),
            Err(_) => { eprintln!("[LLM] No config file — defaults (ollama/gemma4)"); Self::default() }
        }
    }
}

fn call_ollama(cfg: &LlmConfig, text: &str, history: &[(String, String)]) -> Result<String, String> {
    let mut messages = vec![json!({"role": "system", "content": &cfg.system_prompt})];
    for (u, a) in history {
        messages.push(json!({"role": "user", "content": u}));
        messages.push(json!({"role": "assistant", "content": a}));
    }
    messages.push(json!({"role": "user", "content": text}));

    let body = json!({
        "model": &cfg.model,
        "messages": messages,
        "stream": false
    });

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(cfg.timeout_secs))
        .build().map_err(|e| format!("Client build: {}", e))?;

    let resp = client.post(&cfg.url)
        .json(&body).send()
        .map_err(|e| format!("Ollama request: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Ollama HTTP {}", resp.status()));
    }

    let json: serde_json::Value = resp.json().map_err(|e| format!("Parse: {}", e))?;
    json["message"]["content"].as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Ollama: no message.content in response".to_string())
}

fn call_anthropic(cfg: &LlmConfig, text: &str, history: &[(String, String)]) -> Result<String, String> {
    let api_key = std::env::var(&cfg.api_key_env).unwrap_or_default();
    if api_key.is_empty() {
        return Err(format!("API key env var '{}' not set — check Keychain + .bashrc", cfg.api_key_env));
    }

    let mut messages = vec![];
    for (u, a) in history {
        messages.push(json!({"role": "user", "content": u}));
        messages.push(json!({"role": "assistant", "content": a}));
    }
    messages.push(json!({"role": "user", "content": text}));

    let body = json!({
        "model": &cfg.model,
        "max_tokens": cfg.max_tokens,
        "system": &cfg.system_prompt,
        "messages": messages
    });

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(cfg.timeout_secs))
        .build().map_err(|e| format!("Client build: {}", e))?;

    let resp = client.post(&cfg.url)
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body).send()
        .map_err(|e| format!("Anthropic request: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().unwrap_or_default();
        let preview: String = body_text.chars().take(200).collect();
        return Err(format!("Anthropic HTTP {} — {}", status, preview));
    }

    let json: serde_json::Value = resp.json().map_err(|e| format!("Parse: {}", e))?;
    json["content"][0]["text"].as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Anthropic: no content[0].text in response".to_string())
}

fn think(cfg: &LlmConfig, text: &str, history: &[(String, String)]) -> Result<String, String> {
    match cfg.provider.as_str() {
        "ollama" => call_ollama(cfg, text, history),
        "anthropic" => call_anthropic(cfg, text, history),
        other => Err(format!("Unknown provider: {}", other)),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[LLM] Starting Brain Ant...");
    let cfg = LlmConfig::load();
    eprintln!("[LLM] Provider: {} / {}", cfg.provider, cfg.model);
    eprintln!("[LLM] Timeout: {}s, Max tokens: {}", cfg.timeout_secs, cfg.max_tokens);

    // Verify API key is available if using anthropic
    if cfg.provider == "anthropic" {
        let key = std::env::var(&cfg.api_key_env).unwrap_or_default();
        if key.is_empty() {
            eprintln!("[LLM] WARN: {} not set — Anthropic calls will fail", cfg.api_key_env);
        } else {
            eprintln!("[LLM] API key: loaded from Keychain ({})", cfg.api_key_env);
        }
    }

    let mut iox = Config::default();
    iox.global.set_root_path(&Path::new(b"/tmp/iceoryx2/").unwrap());
    let node = NodeBuilder::new().config(&iox).create::<ipc::Service>()?;

    let text_in = node.service_builder(&"stt_text".try_into()?)
        .publish_subscribe::<[u8]>()
        .open_or_create()?;
    let sub = text_in.subscriber_builder().create()?;

    // Contract: tts_text contains ONLY successful assistant replies (UTF-8).
    // API errors, timeouts, and parse failures are logged but NEVER published.
    // Downstream ants must not assume 1:1 correspondence with stt_text inputs.
    // Conversation history is only updated on successful replies.
    let text_out = node.service_builder(&"tts_text".try_into()?)
        .publish_subscribe::<[u8]>()
        .open_or_create()?;
    let pub_ = text_out.publisher_builder()
        .initial_max_slice_len(8192)
        .create()?;

    eprintln!("[LLM] Bus: sub='stt_text' pub='tts_text' — READY");

    let mut history: Vec<(String, String)> = Vec::new();

    loop {
        while let Some(sample) = sub.receive()? {
            let text = std::str::from_utf8(sample.payload())
                .unwrap_or("").trim().to_string();

            if text.is_empty() { continue; }

            let heard_preview: String = text.chars().take(60).collect();
            eprintln!("[LLM] Heard: \"{}\"", heard_preview);

            let t0 = std::time::Instant::now();
            match think(&cfg, &text, &history) {
                Ok(reply) => {
                    let latency_ms = t0.elapsed().as_millis();
                    let reply_preview: String = reply.chars().take(60).collect();
                    eprintln!("[LLM] Reply ({}ms): \"{}\"", latency_ms, reply_preview);

                    let bytes = reply.as_bytes();
                    let s = pub_.loan_slice_uninit(bytes.len())?;
                    s.write_from_slice(bytes).send()?;

                    history.push((text, reply));
                    if history.len() > 10 { history.remove(0); }
                }
                Err(e) => {
                    let latency_ms = t0.elapsed().as_millis();
                    eprintln!("[LLM] Error ({}ms): {}", latency_ms, e);
                    // Do not publish errors to tts_text — LLM errors are log-only
                    // TTS should not speak error messages to the caller
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
