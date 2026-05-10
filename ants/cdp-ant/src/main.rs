//! CDP Ant — Bridges iceoryx2 bus to browser AIs via Chrome DevTools Protocol
//!
//! Subscribes to airy_input (text from router-ant in airy mode)
//! Injects into Airy's browser chat via CDP
//! Scrapes response and publishes to tts_text (for Kokoro to speak)
//!
//! Requires Chrome running with --remote-debugging-port=9222

use iceoryx2::prelude::*;
use iceoryx2_bb_system_types::path::Path;
use iceoryx2_bb_container::semantic_string::SemanticString;
use std::sync::{Arc, Mutex};

const CDP_URL: &str = "http://localhost:9222";

/// Find the WebSocket URL for a tab matching the given string
async fn find_tab(tab_match: &str) -> Option<String> {
    let resp = reqwest::get(format!("{}/json", CDP_URL)).await.ok()?;
    let pages: Vec<serde_json::Value> = resp.json().await.ok()?;
    for page in &pages {
        let url = page["url"].as_str().unwrap_or("");
        let title = page["title"].as_str().unwrap_or("");
        if (url.contains(tab_match) || title.to_lowercase().contains(tab_match))
            && !url.contains("accounts.google")
            && !url.contains("youtube")
            && !url.contains("stripe")
            && !url.contains("blob:")
        {
            return page["webSocketDebuggerUrl"].as_str().map(|s| s.to_string());
        }
    }
    None
}

/// Inject text into Airy's chat and scrape response via CDP
async fn chat_with_airy(text: &str) -> Result<String, String> {
    use tokio_tungstenite::connect_async;
    use futures_util::{SinkExt, StreamExt};

    let ws_url = find_tab("claude.ai").await
        .ok_or("No claude.ai tab found in Chrome")?;

    let (mut ws, _) = connect_async(&ws_url).await
        .map_err(|e| format!("CDP connect: {}", e))?;

    // Focus the input area
    let focus_js = r#"document.querySelector('.ProseMirror, [contenteditable=true], textarea')?.focus()"#;
    let msg = serde_json::json!({"id":1,"method":"Runtime.evaluate","params":{"expression":focus_js}});
    ws.send(tokio_tungstenite::tungstenite::Message::Text(msg.to_string())).await
        .map_err(|e| format!("focus: {}", e))?;
    let _ = ws.next().await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Count existing responses before injection
    let count_js = r#"document.querySelectorAll('[data-message-author-role="assistant"], .font-claude-message').length"#;
    let msg = serde_json::json!({"id":2,"method":"Runtime.evaluate","params":{"expression":count_js,"returnByValue":true}});
    ws.send(tokio_tungstenite::tungstenite::Message::Text(msg.to_string())).await
        .map_err(|e| format!("count: {}", e))?;
    let resp = ws.next().await.ok_or("no response")?.map_err(|e| format!("recv: {}", e))?;
    let resp_json: serde_json::Value = serde_json::from_str(&resp.to_string()).unwrap_or_default();
    let before_count = resp_json["result"]["result"]["value"].as_i64().unwrap_or(0);

    // Insert text
    let msg = serde_json::json!({"id":3,"method":"Input.insertText","params":{"text":text}});
    ws.send(tokio_tungstenite::tungstenite::Message::Text(msg.to_string())).await
        .map_err(|e| format!("insert: {}", e))?;
    let _ = ws.next().await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    // Press Enter
    let msg = serde_json::json!({"id":4,"method":"Input.dispatchKeyEvent","params":{"type":"keyDown","key":"Enter","code":"Enter","windowsVirtualKeyCode":13}});
    ws.send(tokio_tungstenite::tungstenite::Message::Text(msg.to_string())).await
        .map_err(|e| format!("enter down: {}", e))?;
    let _ = ws.next().await;
    let msg = serde_json::json!({"id":5,"method":"Input.dispatchKeyEvent","params":{"type":"keyUp","key":"Enter","code":"Enter","windowsVirtualKeyCode":13}});
    ws.send(tokio_tungstenite::tungstenite::Message::Text(msg.to_string())).await
        .map_err(|e| format!("enter up: {}", e))?;
    let _ = ws.next().await;

    eprintln!("[CDP] Prompt sent to Airy, waiting for response...");

    // Poll for new response (check every 2s, timeout after 60s)
    for attempt in 0..30 {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let scrape_js = format!(
            r#"(() => {{
                const msgs = document.querySelectorAll('[data-message-author-role="assistant"], .font-claude-message');
                if (msgs.length > {}) {{
                    const last = msgs[msgs.length - 1];
                    return last.innerText?.trim() || '';
                }}
                return '';
            }})()"#,
            before_count
        );
        let msg = serde_json::json!({"id":100+attempt,"method":"Runtime.evaluate","params":{"expression":scrape_js,"returnByValue":true}});
        ws.send(tokio_tungstenite::tungstenite::Message::Text(msg.to_string())).await
            .map_err(|e| format!("scrape: {}", e))?;
        let resp = ws.next().await.ok_or("no response")?.map_err(|e| format!("recv: {}", e))?;
        let resp_json: serde_json::Value = serde_json::from_str(&resp.to_string()).unwrap_or_default();
        let text = resp_json["result"]["result"]["value"].as_str().unwrap_or("");

        if !text.is_empty() && text.len() > 5 {
            // Wait a bit more to make sure response is complete
            let prev_len = text.len();
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;

            let msg = serde_json::json!({"id":200+attempt,"method":"Runtime.evaluate","params":{"expression":scrape_js,"returnByValue":true}});
            ws.send(tokio_tungstenite::tungstenite::Message::Text(msg.to_string())).await
                .map_err(|e| format!("scrape2: {}", e))?;
            let resp = ws.next().await.ok_or("no response")?.map_err(|e| format!("recv: {}", e))?;
            let resp_json: serde_json::Value = serde_json::from_str(&resp.to_string()).unwrap_or_default();
            let final_text = resp_json["result"]["result"]["value"].as_str().unwrap_or("").to_string();

            if final_text.len() == prev_len || final_text.len() > prev_len {
                eprintln!("[CDP] Airy responded: {} chars", final_text.len());
                let _ = ws.close(None).await;
                return Ok(final_text);
            }
        }
    }

    let _ = ws.close(None).await;
    Err("Timeout waiting for Airy's response".to_string())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[CDP] Starting CDP Ant...");

    let rt = tokio::runtime::Runtime::new()?;

    let mut iox = Config::default();
    iox.global.set_root_path(&Path::new(b"/tmp/iceoryx2/").unwrap());
    let node = NodeBuilder::new().config(&iox).create::<ipc::Service>()?;

    // Subscribe to airy_input (text from router in airy mode)
    let input_svc = node.service_builder(&"airy_input".try_into()?)
        .publish_subscribe::<[u8]>()
        .open_or_create()?;
    let sub = input_svc.subscriber_builder().create()?;

    // Publish to tts_text (Kokoro speaks the response)
    let output_svc = node.service_builder(&"tts_text".try_into()?)
        .publish_subscribe::<[u8]>()
        .open_or_create()?;
    let pub_ = output_svc.publisher_builder()
        .initial_max_slice_len(32768)
        .create()?;

    eprintln!("[CDP] Bus: sub='airy_input' pub='tts_text' — READY");
    eprintln!("[CDP] Waiting for text to send to Airy...");

    loop {
        while let Some(sample) = sub.receive()? {
            let text = std::str::from_utf8(sample.payload())
                .unwrap_or("").trim().to_string();
            if text.is_empty() { continue; }

            eprintln!("[CDP] Received: \"{}\"", text.chars().take(60).collect::<String>());

            match rt.block_on(chat_with_airy(&text)) {
                Ok(response) => {
                    let preview: String = response.chars().take(60).collect();
                    eprintln!("[CDP] Airy replied: \"{}\"", preview);

                    let bytes = response.as_bytes();
                    if let Ok(loan) = pub_.loan_slice_uninit(bytes.len()) {
                        let _ = loan.write_from_slice(bytes).send();
                        eprintln!("[CDP] Published to tts_text ({} bytes)", bytes.len());
                    }
                }
                Err(e) => {
                    eprintln!("[CDP] Error: {}", e);
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}
