//! LLM Ant — The Brain
//!
//! Subscribes to stt_text (what the swarm heard)
//! Thinks via configurable LLM provider
//! Publishes response to tts_text (for TTS ant to speak)
//!
//! Config: reads llm-ant.json for provider, model, system prompt
//! Providers: ollama (local), anthropic (claude), google (gemini)
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
}

fn d_provider() -> String { "ollama".into() }
fn d_model() -> String { "gemma4".into() }
fn d_url() -> String { "http://localhost:11434/api/chat".into() }
fn d_key_env() -> String { "".into() }
fn d_prompt() -> String {
    "You are Jarvina, a voice assistant for Sparked Matter LLC. Keep responses SHORT — 1-3 sentences. No markdown. Be warm and helpful.".into()
}
fn d_max_tokens() -> u32 { 200 }

impl Default for LlmConfig {
    fn default() -> Self {
        Self { provider: d_provider(), model: d_model(), url: d_url(),
               api_key_env: d_key_env(), system_prompt: d_prompt(), max_tokens: d_max_tokens() }
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
        .timeout(std::time::Duration::from_secs(30)).build().unwrap();

    let resp = client.post(&cfg.url)
        .json(&body).send()
        .map_err(|e| format!("Ollama: {}", e))?;

    let json: serde_json::Value = resp.json().map_err(|e| format!("Parse: {}", e))?;
    Ok(json["message"]["content"].as_str().unwrap_or("...").to_string())
}

fn call_anthropic(cfg: &LlmConfig, text: &str, history: &[(String, String)]) -> Result<String, String> {
    let api_key = std::env::var(&cfg.api_key_env).unwrap_or_default();
    if api_key.is_empty() { return Err(format!("{} not set", cfg.api_key_env)); }

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
        .timeout(std::time::Duration::from_secs(30)).build().unwrap();

    let resp = client.post(&cfg.url)
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body).send()
        .map_err(|e| format!("Anthropic: {}", e))?;

    let json: serde_json::Value = resp.json().map_err(|e| format!("Parse: {}", e))?;
    Ok(json["content"][0]["text"].as_str().unwrap_or("...").to_string())
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

    let mut iox = Config::default();
    iox.global.set_root_path(&Path::new(b"/tmp/iceoryx2/").unwrap());
    let node = NodeBuilder::new().config(&iox).create::<ipc::Service>()?;

    let text_in = node.service_builder(&"stt_text".try_into()?)
        .publish_subscribe::<[u8]>()
        .open_or_create()?;
    let sub = text_in.subscriber_builder().create()?;

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

            eprintln!("[LLM] Heard: \"{}\"", &text[..text.len().min(60)]);

            match think(&cfg, &text, &history) {
                Ok(reply) => {
                    eprintln!("[LLM] Reply: \"{}\"", &reply[..reply.len().min(60)]);

                    // Publish to TTS
                    let bytes = reply.as_bytes();
                    let s = pub_.loan_slice_uninit(bytes.len())?;
                    s.write_from_slice(bytes).send()?;

                    // Update history
                    history.push((text, reply));
                    if history.len() > 10 { history.remove(0); }
                }
                Err(e) => eprintln!("[LLM] Error: {}", e),
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
