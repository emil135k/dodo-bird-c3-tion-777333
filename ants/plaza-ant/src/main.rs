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
    Cdp { tab_match: &'static str, needs_confirm: bool },
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
        entry_file: "blessings/ara.md",
        display_name: "ara",
        dispatch: DispatchMethod::Cdp { tab_match: "grok", needs_confirm: false },
    },
    ReviewerConfig {
        entry_file: "blessings/chatgpt_vale.md",
        display_name: "chatgpt_vale",
        dispatch: DispatchMethod::Cdp { tab_match: "chatgpt", needs_confirm: true },
    },
    ReviewerConfig {
        entry_file: "blessings/airy.md",
        display_name: "airy",
        dispatch: DispatchMethod::Cdp { tab_match: "claude.ai", needs_confirm: false },
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
        DispatchMethod::Cdp { tab_match, needs_confirm } => {
            dispatch_cdp(tab_match, *needs_confirm, reviewer, event).await;
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

async fn dispatch_cdp(tab_match: &str, needs_confirm: bool, reviewer: &ReviewerConfig, event: &PlazaEvent) {
    let prompt = format!(
        "You are {name}, a peer reviewer in the Village Square. \
         Read the flight recorder at \
         https://github.com/emil135k/dodo-bird-c3-tion-777333/blob/main/ants/cody_code_updates_comments.md \
         and review the latest frame. Write a brief review and save it to {entry} \
         in repo emil135k/dodo-bird-c3-tion-777333 on main branch. Commit and push the file. Keep it concise.",
        name = reviewer.display_name,
        entry = reviewer.entry_file,
    );

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

    // Send the prompt — then drop connection BEFORE polling
    let result = cdp_send_and_click(&mut browser, tab_match, needs_confirm, &prompt, reviewer, event).await;

    // Drop original connection FIRST — two CDP connections cause response confusion
    drop(browser);
    handler_task.abort();
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // If confirm needed and prompt was sent, poll with fresh connections
    if result.is_ok() && needs_confirm {
        poll_update_file_button(tab_match, reviewer).await;
    }

    if let Err(msg) = result {
        println!("[plaza-ant] CDP dispatch failed for {}: {}", reviewer.display_name, msg);
    }
}

/// Inner CDP work — returns Ok/Err so the caller always cleans up
async fn cdp_send_and_click(
    browser: &mut Browser,
    tab_match: &str,
    needs_confirm: bool,
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

    // Set prompt text via JS (instant — type_str is too slow for long prompts)
    let escaped_prompt = prompt.replace('\\', "\\\\").replace('\'', "\\'").replace('\n', "\\n");
    let set_text_js = format!(
        r#"(function(){{
            var el = document.querySelector('#prompt-textarea')
                || document.querySelector('[contenteditable="true"]')
                || document.querySelector('.ql-editor');
            if (!el) return 'no input';
            el.focus();
            el.innerText = '{}';
            el.dispatchEvent(new Event('input', {{bubbles: true}}));
            return 'set';
        }})()"#,
        escaped_prompt
    );

    let result = page.evaluate_expression(&set_text_js).await
        .map_err(|e| format!("set text: {}", e))?;
    let val = result.into_value::<String>().unwrap_or_default();
    if val != "set" {
        return Err(format!("could not set text: {}", val));
    }

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // Press Enter via JS key event
    let enter_js = r#"(function(){
        var el = document.querySelector('#prompt-textarea')
            || document.querySelector('[contenteditable="true"]')
            || document.querySelector('.ql-editor');
        if (!el) return 'no input';
        el.dispatchEvent(new KeyboardEvent('keydown', {key: 'Enter', code: 'Enter', keyCode: 13, bubbles: true}));
        el.dispatchEvent(new KeyboardEvent('keyup', {key: 'Enter', code: 'Enter', keyCode: 13, bubbles: true}));
        return 'enter';
    })()"#;
    page.evaluate_expression(enter_js).await.map_err(|e| format!("press Enter: {}", e))?;

    println!("[plaza-ant] Prompt sent to {}", reviewer.display_name);

    if !needs_confirm {
        println!("[plaza-ant] No confirm button needed for {} — dispatch complete", reviewer.display_name);
        return Ok(());
    }

    // Confirm button polling happens AFTER this function returns
    // and the original connection is dropped — two CDP connections cause hangs
    Ok(())
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
