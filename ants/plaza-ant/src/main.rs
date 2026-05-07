//! Plaza Ant — Village Square Dispatcher
//!
//! Listens for filmstrip Action notifications on port 3005.
//! Dispatches review prompts to AI reviewers:
//!   - CLI reviewers (Codex, Gemini): tmux send-keys into persistent sessions
//!   - Web reviewers (ChatGPT Vale): chromiumoxide CDP into Chrome
//! When a reviewer posts: notifies Cody via tmux send-keys.
//! Also serves as Airy's relay (/airy-to-cody).
//!
//! Identity: the plaza-ant stamps each reviewer's identity via the entry filename.
//! chromiumoxide provides native Rust CDP — no Node.js driver, no Python scripts.

use axum::{extract::{Json, State}, http::{HeaderMap, StatusCode}, routing::post, Router};
use chromiumoxide::Browser;
use futures::StreamExt;
use serde::Deserialize;
use std::collections::{HashMap, VecDeque};
use std::env;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::RwLock;

const PORT: u16 = 3005;
const CDP_URL: &str = "http://localhost:9222";

/// Shared state: reviewer online/offline status + dispatch queue
struct PlazaState {
    status: HashMap<String, bool>,
    queue: VecDeque<(usize, PlazaEvent)>, // (reviewer index, original event)
}

type SharedState = Arc<RwLock<PlazaState>>;

#[derive(Deserialize, Debug, Clone)]
struct PlazaEvent {
    speaker: String,
    frame: u64,
    topic: String,
    #[allow(dead_code)]
    channel: String,
}

#[derive(Deserialize, Debug)]
struct AiryMessage {
    command: String,
}

#[derive(Deserialize, Debug)]
struct AdminRequest {
    action: String,
    reviewer: String,
}

enum DispatchMethod {
    Tmux { session: &'static str },
    /// self_push: reviewer pushes via their own GitHub connector (Ara, Airy)
    /// scrape: plaza-ant scrapes response and pushes on their behalf (ChatGPT, Gemini Chat)
    Cdp { tab_match: &'static str, scrape: bool },
}

struct ReviewerConfig {
    entry_file: &'static str,
    display_name: &'static str,
    dispatch: DispatchMethod,
}

const REVIEWERS: &[ReviewerConfig] = &[
    ReviewerConfig {
        entry_file: "blessings/codex_vale.md",
        display_name: "codex_vale",
        dispatch: DispatchMethod::Tmux { session: "codex-vale" },
    },
    ReviewerConfig {
        entry_file: "blessings/gemini_lyra.md",
        display_name: "gemini_lyra",
        dispatch: DispatchMethod::Tmux { session: "gemini-cli-lyra" },
    },
    ReviewerConfig {
        entry_file: "blessings/gemini_lyra_chat.md",
        display_name: "gemini_lyra_chat",
        dispatch: DispatchMethod::Cdp { tab_match: "gemini.google", scrape: true },
    },
    ReviewerConfig {
        entry_file: "blessings/ara.md",
        display_name: "ara",
        dispatch: DispatchMethod::Cdp { tab_match: "grok", scrape: false },
    },
    ReviewerConfig {
        entry_file: "blessings/chatgpt_vale.md",
        display_name: "chatgpt_vale",
        dispatch: DispatchMethod::Cdp { tab_match: "chatgpt", scrape: true },
    },
    ReviewerConfig {
        entry_file: "blessings/airy.md",
        display_name: "airy",
        dispatch: DispatchMethod::Cdp { tab_match: "claude.ai", scrape: false },
    },
];

/// Sanitize a string for safe use inside shell single-quotes
fn shell_safe(s: &str) -> String {
    s.replace('\'', "'\\''")
        .replace('`', "")
        .replace('$', "")
}

#[tokio::main]
async fn main() {
    // All reviewers start online
    let mut status = HashMap::new();
    for r in REVIEWERS {
        status.insert(r.display_name.to_string(), true);
    }
    let state: SharedState = Arc::new(RwLock::new(PlazaState {
        status,
        queue: VecDeque::new(),
    }));

    let app = Router::new()
        .route("/", post(handle_plaza.clone()))
        .route("/plaza", post(handle_plaza))
        .route("/admin", post(handle_admin.clone()))
        .route("/plaza/admin", post(handle_admin))
        .route("/airy-to-cody", post(handle_airy))
        .with_state(state.clone());

    let addr = format!("0.0.0.0:{PORT}");
    println!("[plaza-ant] Listening on {addr}");
    println!("[plaza-ant] Routes: /plaza, /plaza/admin, /airy-to-cody");
    println!("[plaza-ant] CLI: tmux send-keys | Web: chromiumoxide CDP on {CDP_URL}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind port");
    axum::serve(listener, app)
        .await
        .expect("Server error");
}

async fn handle_plaza(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(event): Json<PlazaEvent>,
) -> (StatusCode, &'static str) {
    let expected = env::var("PLAZA_SECRET").unwrap_or_default();
    let provided = headers
        .get("x-plaza-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if provided != expected || expected.is_empty() {
        println!("[plaza-ant] REJECTED: invalid token");
        return (StatusCode::UNAUTHORIZED, "unauthorized");
    }

    println!(
        "[plaza-ant] Received: {} posted FRAME #{} — {} ({})",
        event.speaker, event.frame, event.topic, event.channel
    );

    if event.speaker == "cody" {
        // Build the queue of online reviewers
        let mut plaza = state.write().await;
        plaza.queue.clear();
        for (i, reviewer) in REVIEWERS.iter().enumerate() {
            let is_online = plaza.status.get(reviewer.display_name).copied().unwrap_or(true);
            if is_online {
                plaza.queue.push_back((i, event.clone()));
                println!("[plaza-ant] Queued: {} (position {})", reviewer.display_name, plaza.queue.len());
            } else {
                println!("[plaza-ant] SKIP: {} is offline", reviewer.display_name);
            }
        }
        drop(plaza);

        // Dispatch the first one in the queue
        dispatch_next(state.clone()).await;
    } else {
        // A reviewer posted — notify Cody
        notify_cody(&format!(
            "{} posted FRAME #{} — {}. Check the tape.",
            event.speaker, event.frame, event.topic
        ))
        .await;

        // Dispatch next reviewer in queue
        dispatch_next(state.clone()).await;
    }

    (StatusCode::OK, "ok")
}

/// Pop the next reviewer from the queue and dispatch
async fn dispatch_next(state: SharedState) {
    let next = {
        let mut plaza = state.write().await;
        plaza.queue.pop_front()
    };

    if let Some((idx, event)) = next {
        let reviewer = &REVIEWERS[idx];
        println!(
            "[plaza-ant] Dispatching next in queue: {} ({} remaining)",
            reviewer.display_name,
            {
                let plaza = state.read().await;
                plaza.queue.len()
            }
        );
        dispatch_reviewer(reviewer, &event).await;
    } else {
        println!("[plaza-ant] Queue empty — all reviewers dispatched");
    }
}

// ── Admin control ──────────────────────────────────────────────────────

async fn handle_admin(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(req): Json<AdminRequest>,
) -> (StatusCode, String) {
    let expected = env::var("PLAZA_SECRET").unwrap_or_default();
    let provided = headers
        .get("x-plaza-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if provided != expected || expected.is_empty() {
        return (StatusCode::UNAUTHORIZED, "unauthorized".to_string());
    }

    match req.action.as_str() {
        "online" | "offline" => {
            let is_online = req.action == "online";
            let mut plaza = state.write().await;
            if req.reviewer == "all" {
                for (_, v) in plaza.status.iter_mut() {
                    *v = is_online;
                }
                println!("[plaza-ant] ADMIN: all reviewers → {}", req.action);
            } else if plaza.status.contains_key(&req.reviewer) {
                plaza.status.insert(req.reviewer.clone(), is_online);
                println!("[plaza-ant] ADMIN: {} → {}", req.reviewer, req.action);
            } else {
                return (StatusCode::BAD_REQUEST, format!("unknown reviewer: {}", req.reviewer));
            }
            (StatusCode::OK, format!("{{\"{}\":\"{}\"}}", req.reviewer, req.action))
        }
        "status" => {
            let plaza = state.read().await;
            let entries: Vec<String> = plaza.status.iter()
                .map(|(k, v)| format!("\"{}\":\"{}\"", k, if *v { "online" } else { "offline" }))
                .collect();
            let queue_len = plaza.queue.len();
            let json = format!("{{\"reviewers\":{{{}}},\"queue_length\":{}}}", entries.join(","), queue_len);
            println!("[plaza-ant] ADMIN: status query");
            (StatusCode::OK, json)
        }
        _ => (StatusCode::BAD_REQUEST, format!("unknown action: {}", req.action)),
    }
}

async fn dispatch_reviewer(reviewer: &ReviewerConfig, event: &PlazaEvent) {
    println!(
        "[plaza-ant] Dispatching to {} for FRAME #{}",
        reviewer.display_name, event.frame
    );

    match &reviewer.dispatch {
        DispatchMethod::Tmux { session } => {
            dispatch_tmux(session, reviewer, event).await;
        }
        DispatchMethod::Cdp { tab_match, scrape } => {
            dispatch_cdp(tab_match, *scrape, reviewer, event).await;
        }
    }
}

// ── tmux dispatch (CLI reviewers) ──────────────────────────────────────

async fn dispatch_tmux(session: &str, reviewer: &ReviewerConfig, event: &PlazaEvent) {
    let check = Command::new("tmux")
        .args(["has-session", "-t", session])
        .output()
        .await;

    match check {
        Ok(output) if output.status.success() => {}
        _ => {
            println!(
                "[plaza-ant] WARNING: tmux session '{}' not found — {} will miss FRAME #{}",
                session, reviewer.display_name, event.frame
            );
            return;
        }
    }

    let safe_topic = shell_safe(&event.topic);
    let prompt = format!(
        "You are {name}, a peer reviewer in the Village Square. \
         Cody just posted FRAME #{frame} — topic: '{topic}'. \
         Instructions: \
         1. Run: git pull \
         2. Read the latest frame in ants/cody_code_updates_comments.md. \
         3. Review the code or comments Cody posted. \
         4. Write your review to {entry}. \
         5. Run: git add {entry} && git commit -m 'FRAME #{frame} review' && git push. \
         Stay focused on this frame only. Do not modify any other files.",
        name = reviewer.display_name,
        frame = event.frame,
        topic = safe_topic,
        entry = reviewer.entry_file,
    );

    // Text and Enter must be separate calls — CLI TUIs need the text to land first
    let _ = Command::new("tmux")
        .args(["send-keys", "-t", session, &prompt])
        .output()
        .await;

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let _ = Command::new("tmux")
        .args(["send-keys", "-t", session, "Enter"])
        .output()
        .await;

    println!("[plaza-ant] Prompt delivered to {} via tmux", reviewer.display_name);
}

// ── CDP dispatch (web reviewers via chromiumoxide) ─────────────────────

async fn dispatch_cdp(tab_match: &str, scrape: bool, reviewer: &ReviewerConfig, event: &PlazaEvent) {
    let prompt = if scrape {
        // Scrape mode: reviewer just writes the review, we handle the push
        format!(
            "You are {name}, a peer reviewer in the Village Square. \
             Cody just posted FRAME #{frame} — topic: {topic}. \
             Read FRAME #{frame} in the flight recorder at \
             https://github.com/emil135k/dodo-bird-c3-tion-777333/blob/main/ants/cody_code_updates_comments.md \
             and review it. Write a brief review as your reply. \
             Do NOT save to GitHub. Do NOT commit. Just write the review text. Keep it concise.",
            name = reviewer.display_name,
            frame = event.frame,
            topic = event.topic,
        )
    } else {
        // Self-push mode: reviewer saves and commits via their own connector
        format!(
            "You are {name}, a peer reviewer in the Village Square. \
             Cody just posted FRAME #{frame} — topic: {topic}. \
             Read FRAME #{frame} in the flight recorder at \
             https://github.com/emil135k/dodo-bird-c3-tion-777333/blob/main/ants/cody_code_updates_comments.md \
             and review it. Write a brief review and save it to {entry} \
             in repo emil135k/dodo-bird-c3-tion-777333 on main branch. Commit and push the file. Keep it concise.",
            name = reviewer.display_name,
            frame = event.frame,
            topic = event.topic,
            entry = reviewer.entry_file,
        )
    };

    // Connect to existing Chrome
    let (mut browser, handler) = match Browser::connect(CDP_URL).await {
        Ok(conn) => conn,
        Err(e) => {
            println!(
                "[plaza-ant] ERROR: cannot connect to Chrome at {} — {}: {}",
                CDP_URL, reviewer.display_name, e
            );
            return;
        }
    };

    // Spawn the handler loop (required by chromiumoxide)
    let handler_task = tokio::spawn(async move {
        let mut handler = handler;
        while let Some(event) = handler.next().await {
            let _ = event;
        }
    });

    // Send the prompt
    let result = cdp_send_and_click(&mut browser, tab_match, &prompt, reviewer, event).await;

    // Drop original connection FIRST — two CDP connections cause response confusion
    drop(browser);
    handler_task.abort();
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    if result.is_err() {
        println!("[plaza-ant] CDP dispatch failed for {}: {}", reviewer.display_name, result.unwrap_err());
        return;
    }

    if scrape {
        // Scrape mode: wait for response, scrape it, write and push locally
        scrape_and_push(tab_match, reviewer).await;
    }
    // Self-push mode: reviewer handles their own commit — we just wait for the filmstrip callback

    if let Err(msg) = result {
        println!("[plaza-ant] CDP dispatch failed for {}: {}", reviewer.display_name, msg);
    }
}

/// Inner CDP work — returns Ok/Err so the caller always cleans up
async fn cdp_send_and_click(
    browser: &mut Browser,
    tab_match: &str,
    prompt: &str,
    reviewer: &ReviewerConfig,
    event: &PlazaEvent,
) -> Result<(), String> {
    println!("[plaza-ant] Connected to Chrome for {}", reviewer.display_name);

    // Fetch existing tabs
    browser.fetch_targets().await.map_err(|e| format!("fetch_targets: {}", e))?;
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Find the matching tab
    let pages = browser.pages().await.map_err(|e| format!("pages: {}", e))?;
    println!("[plaza-ant] Found {} pages", pages.len());

    let tab_lower = tab_match.to_lowercase();
    let mut found_page = None;
    for p in pages {
        let url = p.url().await.ok().flatten().unwrap_or_default().to_lowercase();
        if url.contains(&tab_lower) && !url.contains("codex/cloud") {
            found_page = Some(p);
            break;
        }
    }

    let page = found_page.ok_or_else(|| {
        format!("no tab matching '{}' — {} will miss FRAME #{}", tab_match, reviewer.display_name, event.frame)
    })?;

    println!("[plaza-ant] Found tab for {}", reviewer.display_name);

    // Focus the input via JS
    let focus_js = r#"(function(){
        var el = document.querySelector('#prompt-textarea')
            || document.querySelector('[contenteditable="true"]')
            || document.querySelector('.ql-editor');
        if (!el) return 'no input';
        el.focus();
        el.textContent = '';
        return 'focused';
    })()"#;
    let result = page.evaluate_expression(focus_js).await
        .map_err(|e| format!("focus: {}", e))?;
    let val = result.into_value::<String>().unwrap_or_default();
    if val != "focused" {
        return Err(format!("could not focus input: {}", val));
    }

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Insert text via CDP Input.insertText (React-compatible, instant)
    use chromiumoxide::cdp::browser_protocol::input::InsertTextParams;
    page.execute(InsertTextParams::new(prompt)).await
        .map_err(|e| format!("insertText: {}", e))?;

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // Press Enter via CDP DispatchKeyEvent
    use chromiumoxide::cdp::browser_protocol::input::{
        DispatchKeyEventParams, DispatchKeyEventType,
    };
    let enter_down = DispatchKeyEventParams::builder()
        .r#type(DispatchKeyEventType::KeyDown)
        .key("Enter")
        .code("Enter")
        .windows_virtual_key_code(13)
        .native_virtual_key_code(13)
        .build()
        .unwrap();
    page.execute(enter_down).await.map_err(|e| format!("Enter keydown: {}", e))?;

    let enter_up = DispatchKeyEventParams::builder()
        .r#type(DispatchKeyEventType::KeyUp)
        .key("Enter")
        .code("Enter")
        .windows_virtual_key_code(13)
        .native_virtual_key_code(13)
        .build()
        .unwrap();
    page.execute(enter_up).await.map_err(|e| format!("Enter keyup: {}", e))?;

    println!("[plaza-ant] Prompt sent to {}", reviewer.display_name);
    Ok(())
}

/// Scrape the last assistant response from the browser, write to blessings file, git push
/// Uses raw tokio-tungstenite websocket — chromiumoxide hangs on repeated connect/disconnect
async fn scrape_and_push(tab_match: &str, reviewer: &ReviewerConfig) {
    println!("[plaza-ant] Scraping response for {}...", reviewer.display_name);

    // Wait for the reviewer to finish responding
    tokio::time::sleep(std::time::Duration::from_secs(20)).await;
    println!("[plaza-ant]   initial wait done, starting scrape polls...");

    // Find the tab's websocket URL
    let ws_url = match find_tab_ws(tab_match).await {
        Some(url) => url,
        None => {
            println!("[plaza-ant] ERROR: no tab for {} to scrape", reviewer.display_name);
            return;
        }
    };

    let check_and_scrape_js = r#"(function(){var stop=document.querySelector('button[aria-label=\"Stop generating\"]')||document.querySelector('button[aria-label=\"Stop Response\"]');if(stop)return 'streaming';var streaming=document.querySelector('.result-streaming');if(streaming)return 'streaming';var gemLoading=document.querySelector('.loading-indicator,.response-loading,.thinking-indicator');if(gemLoading)return 'streaming';var msgs=document.querySelectorAll('[data-message-author-role=\"assistant\"]');if(msgs.length>0){var last=msgs[msgs.length-1];var md=last.querySelector('.markdown');return 'SCRAPED:'+(md||last).textContent.trim();}var gemMsgs=document.querySelectorAll('.model-response-text');if(gemMsgs.length>0){return 'SCRAPED:'+gemMsgs[gemMsgs.length-1].textContent.trim();}return 'empty';})()"#;

    let max_attempts = 24;
    let mut response_text = String::new();

    for attempt in 0..max_attempts {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        println!("[plaza-ant]   scrape attempt {}...", attempt + 1);

        // Raw websocket — connect, send one evaluate, read one response, disconnect
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            raw_cdp_evaluate(&ws_url, check_and_scrape_js),
        ).await;

        match result {
            Ok(Ok(text)) => {
                if text.starts_with("SCRAPED:") {
                    response_text = text[8..].to_string();
                    println!("[plaza-ant]   scraped {} chars", response_text.len());
                    break;
                } else if text == "streaming" {
                    println!("[plaza-ant]   still streaming... ({}s)", (attempt + 1) * 5);
                } else {
                    println!("[plaza-ant]   {} ({}s)", text, (attempt + 1) * 5);
                }
            }
            Ok(Err(e)) => println!("[plaza-ant]   raw cdp error: {} ({}s)", e, (attempt + 1) * 5),
            Err(_) => println!("[plaza-ant]   raw cdp timeout ({}s)", (attempt + 1) * 5),
        }
    }

    if response_text.is_empty() {
        println!("[plaza-ant] ERROR: could not scrape response for {}", reviewer.display_name);
        return;
    }

    println!("[plaza-ant] Scraped {} chars from {}", response_text.len(), reviewer.display_name);

    // Write to blessings file and push
    let dodo_path = format!(
        "{}/dodo-bird-c3-tion-777333/{}",
        env::var("HOME").unwrap_or_default(),
        reviewer.entry_file
    );

    if let Err(e) = tokio::fs::write(&dodo_path, &response_text).await {
        println!("[plaza-ant] ERROR: could not write {}: {}", dodo_path, e);
        return;
    }

    // git add, commit, push
    let dodo_dir = format!("{}/dodo-bird-c3-tion-777333", env::var("HOME").unwrap_or_default());
    let commit_msg = format!("{} review via plaza-ant scrape", reviewer.display_name);

    // Retry push up to 3 times — this is the critical function
    let git_result = Command::new("bash")
        .arg("-c")
        .arg(format!(
            "cd {dir} && \
             for i in 1 2 3; do \
               git pull --no-rebase && \
               git add {file} && \
               git commit -m '{msg}' 2>/dev/null; \
               if git push 2>&1; then exit 0; fi; \
               echo 'Push retry '$i; sleep 2; \
             done; exit 1",
            dir = dodo_dir, file = reviewer.entry_file, msg = commit_msg
        ))
        .output()
        .await;

    match git_result {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if out.status.success() {
                println!("[plaza-ant] Pushed {} review for {}", reviewer.entry_file, reviewer.display_name);
            } else {
                let stderr = String::from_utf8_lossy(&out.stderr);
                println!("[plaza-ant] Git push failed for {}: {} {}", reviewer.display_name, stdout, stderr);
            }
        }
        Err(e) => println!("[plaza-ant] Git command failed: {}", e),
    }
}

/// Find a tab's websocket debugger URL by matching title/URL
async fn find_tab_ws(tab_match: &str) -> Option<String> {
    let resp = reqwest::get(&format!("{}/json/list", CDP_URL)).await.ok()?;
    let tabs: Vec<serde_json::Value> = resp.json().await.ok()?;
    let lower = tab_match.to_lowercase();
    for tab in &tabs {
        let url = tab.get("url")?.as_str()?.to_lowercase();
        let tab_type = tab.get("type")?.as_str()?;
        if tab_type == "page" && url.contains(&lower) && !url.contains("codex/cloud") {
            return tab.get("webSocketDebuggerUrl")?.as_str().map(String::from);
        }
    }
    None
}

/// Raw CDP evaluate — connect via tokio-tungstenite, send one Runtime.evaluate, read response, disconnect
async fn raw_cdp_evaluate(ws_url: &str, expression: &str) -> Result<String, String> {
    use tokio_tungstenite::connect_async;

    let (mut ws, _) = connect_async(ws_url).await.map_err(|e| format!("ws connect: {}", e))?;

    let msg = serde_json::json!({
        "id": 1,
        "method": "Runtime.evaluate",
        "params": {
            "expression": expression,
            "returnByValue": true
        }
    });

    use futures::SinkExt;
    ws.send(tokio_tungstenite::tungstenite::Message::Text(msg.to_string()))
        .await
        .map_err(|e| format!("ws send: {}", e))?;

    // Read messages until we find our response (id=1)
    for _ in 0..20 {
        if let Some(Ok(frame)) = ws.next().await {
            if let tokio_tungstenite::tungstenite::Message::Text(text) = frame {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                    if parsed.get("id").and_then(|v| v.as_u64()) == Some(1) {
                        let val = parsed
                            .pointer("/result/result/value")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let _ = ws.close(None).await;
                        return Ok(val);
                    }
                }
            }
        }
    }

    let _ = ws.close(None).await;
    Err("no response after 20 messages".to_string())
}

/// Poll for Update File button with fresh connections — called AFTER original connection is dropped
async fn poll_update_file_button(tab_match: &str, reviewer: &ReviewerConfig) {
    println!("[plaza-ant] Polling for Update File button (fresh connections)...");
    let max_attempts = 24; // 24 * 5s = 2 minutes
    for attempt in 0..max_attempts {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        match poll_and_click_button(tab_match, reviewer).await {
            Ok(true) => {
                println!(
                    "[plaza-ant] Update File clicked for {} ({}s)",
                    reviewer.display_name,
                    (attempt + 1) * 5
                );
                return;
            }
            Ok(false) => println!("[plaza-ant]   polling... ({}s)", (attempt + 1) * 5),
            Err(e) => println!("[plaza-ant]   poll error: {} ({}s)", e, (attempt + 1) * 5),
        }
    }
    println!("[plaza-ant] TIMEOUT: Update File never appeared for {}", reviewer.display_name);
}

/// Fresh connection per poll — find Update File button and click via JS MouseEvent
async fn poll_and_click_button(tab_match: &str, reviewer: &ReviewerConfig) -> Result<bool, String> {
    let (mut browser, handler) = Browser::connect(CDP_URL).await
        .map_err(|e| format!("connect: {}", e))?;

    let handler_task = tokio::spawn(async move {
        let mut handler = handler;
        while let Some(event) = handler.next().await {
            let _ = event;
        }
    });

    browser.fetch_targets().await.map_err(|e| format!("fetch: {}", e))?;
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let pages = browser.pages().await.map_err(|e| format!("pages: {}", e))?;
    let tab_lower = tab_match.to_lowercase();
    let mut found_page = None;
    for p in pages {
        let url = p.url().await.ok().flatten().unwrap_or_default().to_lowercase();
        if url.contains(&tab_lower) && !url.contains("codex/cloud") {
            found_page = Some(p);
            break;
        }
    }

    let page = found_page.ok_or_else(|| format!("no tab for {}", reviewer.display_name))?;

    let js = r#"
        (function() {
            var buttons = document.querySelectorAll('button');
            for (var b of buttons) {
                if (b.textContent.trim() === 'Update File') {
                    var rect = b.getBoundingClientRect();
                    var x = rect.x + rect.width/2;
                    var y = rect.y + rect.height/2;
                    var opts = {bubbles: true, cancelable: true, clientX: x, clientY: y, button: 0};
                    b.dispatchEvent(new MouseEvent('mousedown', opts));
                    b.dispatchEvent(new MouseEvent('mouseup', opts));
                    b.dispatchEvent(new MouseEvent('click', opts));
                    return 'CLICKED';
                }
            }
            return 'waiting';
        })()
    "#;

    let clicked = if let Ok(val) = page.evaluate_expression(js).await {
        val.into_value::<String>().unwrap_or_default() == "CLICKED"
    } else {
        false
    };

    drop(browser);
    handler_task.abort();
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    Ok(clicked)
}

// ── Cody notification ──────────────────────────────────────────────────

async fn notify_cody(message: &str) {
    println!("[plaza-ant] Notifying Cody: {message}");
    // Send directly to Cody's Claude Code session — same as Airy relay
    let _ = Command::new("tmux")
        .args(["send-keys", "-t", "cody", message])
        .output()
        .await;

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let _ = Command::new("tmux")
        .args(["send-keys", "-t", "cody", "Enter"])
        .output()
        .await;
}

// ── Airy relay ─────────────────────────────────────────────────────────

async fn handle_airy(
    State(_state): State<SharedState>,
    headers: HeaderMap,
    Json(msg): Json<AiryMessage>,
) -> (StatusCode, &'static str) {
    let expected = env::var("PLAZA_SECRET").unwrap_or_default();
    let provided = headers
        .get("x-plaza-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if provided != expected || expected.is_empty() {
        println!("[plaza-ant] REJECTED: invalid token on /airy-to-cody");
        return (StatusCode::UNAUTHORIZED, "unauthorized");
    }

    if msg.command.is_empty() {
        return (StatusCode::OK, "empty");
    }

    println!("[Airy→Cody] {}", &msg.command[..msg.command.len().min(80)]);

    let _ = Command::new("tmux")
        .args(["send-keys", "-t", "cody", &msg.command])
        .output()
        .await;

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let _ = Command::new("tmux")
        .args(["send-keys", "-t", "cody", "Enter"])
        .output()
        .await;

    (StatusCode::OK, "{\"status\":\"sent\"}")
}
