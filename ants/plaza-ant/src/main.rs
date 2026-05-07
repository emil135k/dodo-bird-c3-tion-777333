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
    queue: VecDeque<(usize, PlazaEvent)>,
    active_reviewer: Option<String>,
    subject_frame: Option<u64>,
    plaza_secret: String,
}

type SharedState = Arc<RwLock<PlazaState>>;

#[derive(Deserialize, Debug, Clone)]
struct PlazaEvent {
    speaker: String,
    frame: u64,
    topic: String,
    #[allow(dead_code)]
    channel: String,
    /// Base64-encoded full content from the blessings entry file
    #[serde(default)]
    content_b64: String,
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

/// Decode base64 string
fn base64_decode(input: &str) -> Option<Vec<u8>> {

    let mut output = Vec::new();
    // Simple base64 decode without external crate
    let chars: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    let lookup = |c: u8| -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            b'=' => Some(0),
            _ => None,
        }
    };
    for chunk in chars.chunks(4) {
        if chunk.len() < 2 { break; }
        let a = lookup(chunk[0])?;
        let b = lookup(chunk[1])?;
        output.push((a << 2) | (b >> 4));
        if chunk.len() > 2 && chunk[2] != b'=' {
            let c = lookup(chunk[2])?;
            output.push((b << 4) | (c >> 2));
            if chunk.len() > 3 && chunk[3] != b'=' {
                let d = lookup(chunk[3])?;
                output.push((c << 6) | d);
            }
        }
    }
    Some(output)
}

/// Sanitize a string for safe use in tmux send-keys
fn shell_safe(s: &str) -> String {
    s.replace('\'', "'\\''")
        .replace('`', "")
        .replace('$', "")
        .replace(';', "")
        .replace('|', "")
        .replace('&', "")
        .replace('\n', " ")
        .replace('\r', " ")
}

#[tokio::main]
async fn main() {
    // All reviewers start online
    let mut status = HashMap::new();
    for r in REVIEWERS {
        status.insert(r.display_name.to_string(), true);
    }
    let plaza_secret = env::var("PLAZA_SECRET").unwrap_or_default();
    if plaza_secret.is_empty() {
        println!("[plaza-ant] FATAL: PLAZA_SECRET not set");
        std::process::exit(1);
    }

    let state: SharedState = Arc::new(RwLock::new(PlazaState {
        status,
        queue: VecDeque::new(),
        active_reviewer: None,
        subject_frame: None,
        plaza_secret,
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
    let expected = { state.read().await.plaza_secret.clone() };
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
        // Guard: reject new Cody frames while a review cycle is active
        {
            let plaza = state.read().await;
            if plaza.active_reviewer.is_some() || !plaza.queue.is_empty() {
                println!(
                    "[plaza-ant] REJECT: review cycle in progress (active: {:?}, queue: {}). FRAME #{} deferred.",
                    plaza.active_reviewer, plaza.queue.len(), event.frame
                );
                return (StatusCode::OK, "busy");
            }
        }

        // Build the queue of online reviewers
        let mut plaza = state.write().await;
        plaza.queue.clear();
        plaza.subject_frame = Some(event.frame);
        plaza.active_reviewer = None;
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

        // Spawn dispatch as background task — don't block the HTTP response
        let s = state.clone();
        tokio::spawn(async move { dispatch_next(s).await; });
    } else {
        // A reviewer posted — validate before advancing queue
        let should_advance = {
            let plaza = state.read().await;

            match &plaza.active_reviewer {
                Some(active) => {
                    if event.speaker != *active {
                        println!("[plaza-ant] IGNORE: {} posted but active is {}", event.speaker, active);
                        false
                    } else if let Some(sf) = plaza.subject_frame {
                        if event.frame > 0 && event.frame < sf {
                            println!("[plaza-ant] IGNORE: stale FRAME #{} < subject #{}", event.frame, sf);
                            false
                        } else {
                            true
                        }
                    } else {
                        true
                    }
                }
                None => {
                    // No active reviewer — accept if queue has items (scrape callback)
                    if !plaza.queue.is_empty() {
                        println!("[plaza-ant] Scrape callback from {} — advancing queue", event.speaker);
                        true
                    } else {
                        println!("[plaza-ant] IGNORE: {} posted but no active cycle", event.speaker);
                        false
                    }
                }
            }
        };

        if should_advance {
            // Clear active_reviewer synchronously BEFORE spawning dispatch
            // Prevents duplicate callbacks from advancing twice
            {
                let mut plaza = state.write().await;
                plaza.active_reviewer = None;
            }

            let speaker = event.speaker.clone();
            let frame = event.frame;
            let topic = event.topic.clone();
            let s = state.clone();
            tokio::spawn(async move {
                notify_cody(&format!(
                    "{} posted FRAME #{} — {}. Check the tape.",
                    speaker, frame, topic
                ))
                .await;
                dispatch_next(s).await;
            });
        }
    }

    (StatusCode::OK, "ok")
}

/// Pop the next reviewer from the queue and dispatch — sets active_reviewer
async fn dispatch_next(state: SharedState) {
    let next = {
        let mut plaza = state.write().await;
        plaza.active_reviewer = None;
        plaza.queue.pop_front()
    };

    if let Some((idx, event)) = next {
        let reviewer = &REVIEWERS[idx];
        {
            let mut plaza = state.write().await;
            plaza.active_reviewer = Some(reviewer.display_name.to_string());
            println!(
                "[plaza-ant] Dispatching next in queue: {} ({} remaining)",
                reviewer.display_name, plaza.queue.len()
            );
        }
        dispatch_reviewer(reviewer, &event, state.clone()).await;
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
    let expected = { state.read().await.plaza_secret.clone() };
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

async fn dispatch_reviewer(reviewer: &ReviewerConfig, event: &PlazaEvent, state: SharedState) {
    println!(
        "[plaza-ant] Dispatching to {} for FRAME #{}",
        reviewer.display_name, event.frame
    );

    // Decode full content from base64, fall back to topic
    let full_content = if !event.content_b64.is_empty() {
    
        let decoded = base64_decode(&event.content_b64).unwrap_or_default();
        String::from_utf8(decoded).unwrap_or_else(|_| event.topic.clone())
    } else {
        event.topic.clone()
    };

    // ONE message for ALL reviewers — cookie cutter broadcast
    let message = format!(
        "You are {name}. {content} Write a brief review as your reply. Keep it concise and actionable.",
        name = reviewer.display_name,
        content = full_content,
    );

    match &reviewer.dispatch {
        DispatchMethod::Tmux { session } => {
            // CLI reviewers must commit and push themselves
            // They advance the queue via filmstrip callback
            let cli_message = format!(
                "{} After writing your review to {}, run: git add {} && git commit -m 'FRAME #{} review' && git push",
                message, reviewer.entry_file, reviewer.entry_file, event.frame
            );
            dispatch_tmux(session, reviewer, &cli_message).await;
        }
        DispatchMethod::Cdp { tab_match, scrape } => {
            if *scrape {
                // Scrape reviewers: plaza-ant handles the push
                dispatch_cdp(tab_match, true, reviewer, event, &message).await;
                // Scrape done — clear active_reviewer so filmstrip callback can advance queue
                {
                    let mut plaza = state.write().await;
                    plaza.active_reviewer = None;
                    println!("[plaza-ant] Scrape complete for {} — ready for callback", reviewer.display_name);
                }
            } else {
                // Self-push reviewers: they advance via filmstrip callback
                let push_message = format!(
                    "{} Save your review to {} in repo emil135k/dodo-bird-c3-tion-777333 on main branch. Commit and push the file.",
                    message, reviewer.entry_file
                );
                dispatch_cdp(tab_match, false, reviewer, event, &push_message).await;
            }
        }
    }
}

// ── tmux dispatch (CLI reviewers) ──────────────────────────────────────

async fn dispatch_tmux(session: &str, reviewer: &ReviewerConfig, message: &str) {
    let check = Command::new("tmux")
        .args(["has-session", "-t", session])
        .output()
        .await;

    match check {
        Ok(output) if output.status.success() => {}
        _ => {
            println!(
                "[plaza-ant] WARNING: tmux session '{}' not found — {}",
                session, reviewer.display_name
            );
            return;
        }
    }

    // Text and Enter must be separate calls — CLI TUIs need the text to land first
    let safe_msg = shell_safe(message);
    let _ = Command::new("tmux")
        .args(["send-keys", "-t", session, &safe_msg])
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

async fn dispatch_cdp(tab_match: &str, scrape: bool, reviewer: &ReviewerConfig, event: &PlazaEvent, message: &str) {
    let prompt = message.to_string();

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

    // Clear browser cache before interacting — prevents stale CDP state
    {
        use chromiumoxide::cdp::browser_protocol::network::ClearBrowserCacheParams;
        let _ = browser.execute(ClearBrowserCacheParams::default()).await;
        println!("[plaza-ant] Browser cache cleared for {}", reviewer.display_name);
    }

    // Send the prompt (drops chromiumoxide connection internally, uses raw websocket for input)
    let result = cdp_send_and_click(&mut browser, tab_match, &prompt, reviewer, event, handler_task).await;

    if let Err(msg) = result {
        println!("[plaza-ant] CDP dispatch failed for {}: {}", reviewer.display_name, msg);
        return;
    }

    if scrape {
        scrape_and_push(tab_match, reviewer).await;
    }
}

/// Inner CDP work — returns Ok/Err so the caller always cleans up
async fn cdp_send_and_click(
    browser: &mut Browser,
    tab_match: &str,
    prompt: &str,
    reviewer: &ReviewerConfig,
    event: &PlazaEvent,
    handler_task: tokio::task::JoinHandle<()>,
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
            || document.querySelector('.ql-editor.ql-blank')
            || document.querySelector('.ql-editor')
            || document.querySelector('.tiptap.ProseMirror')
            || document.querySelector('[contenteditable="true"]');
        if (!el) return 'no input';
        el.focus();
        el.textContent = '';
        return 'focused: ' + el.className.substring(0,30);
    })()"#;
    let result = page.evaluate_expression(focus_js).await
        .map_err(|e| format!("focus: {}", e))?;
    let val = result.into_value::<String>().unwrap_or_default();
    if !val.starts_with("focused") {
        return Err(format!("could not focus input: {}", val));
    }
    println!("[plaza-ant] {}", val);
    // duplicate removed — handled above

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Drop chromiumoxide connection before raw websocket injection
    // chromiumoxide's handler loop interferes with Input.insertText on some platforms (Gemini)
    drop(page);
    drop(browser);
    handler_task.abort();
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // Find the tab's websocket URL for raw CDP
    let ws_url = find_tab_ws(tab_match).await
        .ok_or_else(|| format!("lost tab for {}", reviewer.display_name))?;

    // Insert text via raw websocket (works reliably on all platforms)
    raw_cdp_send(&ws_url, "Input.insertText", serde_json::json!({"text": prompt})).await
        .map_err(|e| format!("insertText: {}", e))?;

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // Press Enter via raw websocket
    raw_cdp_send(&ws_url, "Input.dispatchKeyEvent", serde_json::json!({
        "type": "keyDown", "key": "Enter", "code": "Enter",
        "windowsVirtualKeyCode": 13, "nativeVirtualKeyCode": 13
    })).await.map_err(|e| format!("Enter keydown: {}", e))?;

    raw_cdp_send(&ws_url, "Input.dispatchKeyEvent", serde_json::json!({
        "type": "keyUp", "key": "Enter", "code": "Enter",
        "windowsVirtualKeyCode": 13, "nativeVirtualKeyCode": 13
    })).await.map_err(|e| format!("Enter keyup: {}", e))?;

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

    // Validate scraped response
    if response_text.is_empty() {
        println!("[plaza-ant] ERROR: empty scrape for {}", reviewer.display_name);
        return;
    }
    if response_text.len() < 20 {
        println!("[plaza-ant] ERROR: scrape too short ({} chars) for {}", response_text.len(), reviewer.display_name);
        return;
    }
    if response_text.len() > 50_000 {
        println!("[plaza-ant] ERROR: scrape too large ({} chars) for {}", response_text.len(), reviewer.display_name);
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

    // git pull, add, commit, push — structured commands, no shell interpolation
    let dodo_dir = format!("{}/dodo-bird-c3-tion-777333", env::var("HOME").unwrap_or_default());
    let commit_msg = format!("{} review via plaza-ant scrape", reviewer.display_name);

    // Retry push up to 3 times
    for attempt in 1..=3 {
        // git pull
        let pull = Command::new("git")
            .args(["pull", "--no-rebase"])
            .current_dir(&dodo_dir)
            .output()
            .await;
        if let Err(e) = pull {
            println!("[plaza-ant] git pull failed: {}", e);
            continue;
        }

        // git add
        let add = Command::new("git")
            .args(["add", reviewer.entry_file])
            .current_dir(&dodo_dir)
            .output()
            .await;
        if let Err(e) = add {
            println!("[plaza-ant] git add failed: {}", e);
            continue;
        }

        // git commit
        let _ = Command::new("git")
            .args(["commit", "-m", &commit_msg])
            .current_dir(&dodo_dir)
            .output()
            .await;

        // git push
        let push = Command::new("git")
            .args(["push"])
            .current_dir(&dodo_dir)
            .output()
            .await;

        match push {
            Ok(out) if out.status.success() => {
                println!("[plaza-ant] Pushed {} review for {}", reviewer.entry_file, reviewer.display_name);
                return;
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                println!("[plaza-ant] Push retry {} for {}: {}", attempt, reviewer.display_name, stderr.chars().take(100).collect::<String>());
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
            Err(e) => {
                println!("[plaza-ant] git push error: {}", e);
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    }
    println!("[plaza-ant] ERROR: git push failed after 3 retries for {}", reviewer.display_name);
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

/// Raw CDP send — connect, send a CDP command, read response, disconnect
async fn raw_cdp_send(ws_url: &str, method: &str, params: serde_json::Value) -> Result<String, String> {
    use tokio_tungstenite::connect_async;

    let (mut ws, _) = connect_async(ws_url).await.map_err(|e| format!("ws connect: {}", e))?;

    let msg = serde_json::json!({ "id": 1, "method": method, "params": params });

    use futures::SinkExt;
    ws.send(tokio_tungstenite::tungstenite::Message::Text(msg.to_string()))
        .await
        .map_err(|e| format!("ws send: {}", e))?;

    for _ in 0..20 {
        if let Some(Ok(frame)) = ws.next().await {
            if let tokio_tungstenite::tungstenite::Message::Text(text) = frame {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                    if parsed.get("id").and_then(|v| v.as_u64()) == Some(1) {
                        let _ = ws.close(None).await;
                        return Ok(text);
                    }
                }
            }
        }
    }

    let _ = ws.close(None).await;
    Err("no response".to_string())
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

// ── Cody notification ──────────────────────────────────────────────────

async fn notify_cody(message: &str) {
    let safe_msg = shell_safe(message);
    println!("[plaza-ant] Notifying Cody: {safe_msg}");
    let _ = Command::new("tmux")
        .args(["send-keys", "-t", "cody", &safe_msg])
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
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(msg): Json<AiryMessage>,
) -> (StatusCode, &'static str) {
    let expected = { state.read().await.plaza_secret.clone() };
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

    let safe_cmd = shell_safe(&msg.command);
    println!("[Airy→Cody] {}", safe_cmd.chars().take(80).collect::<String>());

    let _ = Command::new("tmux")
        .args(["send-keys", "-t", "cody", &safe_cmd])
        .output()
        .await;

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let _ = Command::new("tmux")
        .args(["send-keys", "-t", "cody", "Enter"])
        .output()
        .await;

    (StatusCode::OK, "{\"status\":\"sent\"}")
}
