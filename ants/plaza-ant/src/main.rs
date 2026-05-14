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

const CONFIG_PATH: &str = "/Users/rocketman/crystalballmini/hypAiAssist/config/plaza-ant.json";

// ── JSON Config ──────────────────────────────────────────────────────────
// ALL behavior is driven by this config. No hardcoded values.
// Change the JSON, restart plaza-ant, behavior changes. No recompilation.

#[derive(Deserialize, Debug, Clone)]
struct PlazaConfig {
    #[serde(default = "default_port")]
    port: u16,
    #[serde(default = "default_cdp_url")]
    cdp_url: String,
    #[serde(default = "default_cody_session")]
    cody_tmux_session: String,
    target: TargetConfig,
    #[serde(default)]
    frame_counter_start: u64,
    reviewers: Vec<ReviewerJsonConfig>,
    prompt_template: PromptTemplate,
}

fn default_port() -> u16 { 3005 }
fn default_cdp_url() -> String { "http://localhost:9222".to_string() }
fn default_cody_session() -> String { "cody".to_string() }

#[derive(Deserialize, Debug, Clone)]
struct TargetConfig {
    local: LocalTarget,
    github: GithubTarget,
    branch: String,
}

#[derive(Deserialize, Debug, Clone)]
struct LocalTarget {
    repo_path: String,
    tape_file: String,
    blessings_dir: String,
    #[serde(default)]
    frame_counter_file: String,
}

#[derive(Deserialize, Debug, Clone)]
struct GithubTarget {
    url: String,
    tape_url: String,
    blessings_url: String,
}

#[derive(Deserialize, Debug, Clone)]
struct ReviewerJsonConfig {
    name: String,
    display_name: String,
    #[serde(rename = "type")]
    reviewer_type: String,  // "cli", "browser", "self_push"
    dispatch: String,       // "tmux" or "cdp"
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default)]
    tmux_session: String,
    #[serde(default)]
    tab_match: String,
    #[serde(default)]
    scrape: bool,
    entry_file: String,
}

fn default_true() -> bool { true }

#[derive(Deserialize, Debug, Clone)]
struct PromptTemplate {
    cli: String,
    browser: String,
    self_push: String,
}

impl PlazaConfig {
    fn load() -> Self {
        match std::fs::read_to_string(CONFIG_PATH) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                eprintln!("[plaza-ant] Config parse error: {} — FATAL", e);
                std::process::exit(1);
            }),
            Err(e) => {
                eprintln!("[plaza-ant] Config not found at {}: {} — FATAL", CONFIG_PATH, e);
                std::process::exit(1);
            }
        }
    }

    /// Create blessings dir and empty entry files for each reviewer
    fn init_blessings(&self) {
        let blessings_path = format!("{}/{}", self.target.local.repo_path, self.target.local.blessings_dir);
        let _ = std::fs::create_dir_all(&blessings_path);

        // Frame counter
        let counter_path = format!("{}/frame_counter.txt", blessings_path);
        if !std::path::Path::new(&counter_path).exists() {
            let _ = std::fs::write(&counter_path, format!("{}", self.frame_counter_start));
        }

        // Entry files for each reviewer
        for r in &self.reviewers {
            let entry_path = format!("{}/{}", blessings_path, r.entry_file);
            if !std::path::Path::new(&entry_path).exists() {
                let _ = std::fs::write(&entry_path, "");
            }
        }
        println!("[plaza-ant] Blessings initialized at {}", blessings_path);
    }

    /// Build the dispatch prompt for a reviewer
    fn build_prompt(&self, reviewer: &ReviewerJsonConfig, content: &str) -> String {
        let template = match reviewer.reviewer_type.as_str() {
            "cli" => &self.prompt_template.cli,
            "browser" => &self.prompt_template.browser,
            "self_push" => &self.prompt_template.self_push,
            _ => &self.prompt_template.browser,
        };

        let prompt = template
            .replace("{local_repo}", &self.target.local.repo_path)
            .replace("{local_tape}", &self.target.local.tape_file)
            .replace("{local_blessings}", &self.target.local.blessings_dir)
            .replace("{github_tape_url}", &self.target.github.tape_url)
            .replace("{github_url}", &self.target.github.url)
            .replace("{branch}", &self.target.branch)
            .replace("{entry_file}", &reviewer.entry_file)
            .replace("{name}", &reviewer.name);

        format!("You are {}. {} {}", reviewer.display_name, content, prompt)
    }
}

// ── Runtime State ────────────────────────────────────────────────────────

/// Shared state: reviewer online/offline status + dispatch queue
struct PlazaState {
    status: HashMap<String, bool>,
    queue: VecDeque<(usize, PlazaEvent)>,
    active_reviewer: Option<String>,
    subject_frame: Option<u64>,
    plaza_secret: String,
    config: PlazaConfig,
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

// Reviewer config is now loaded from plaza-ant.json — no hardcoded list

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
    // Load JSON config
    let config = PlazaConfig::load();
    println!("[plaza-ant] Config loaded: {} reviewers, target: {}", config.reviewers.len(), config.target.local.repo_path);
    config.init_blessings();

    // Set env vars for functions that run without state access
    env::set_var("PLAZA_REPO_PATH", &config.target.local.repo_path);
    env::set_var("PLAZA_BRANCH", &config.target.branch);
    env::set_var("PLAZA_BLESSINGS_DIR", &config.target.local.blessings_dir);
    env::set_var("PLAZA_CDP_URL", &config.cdp_url);
    env::set_var("PLAZA_CODY_SESSION", &config.cody_tmux_session);

    // Only enabled reviewers are online
    let mut status = HashMap::new();
    let enabled_count = config.reviewers.iter().filter(|r| r.enabled).count();
    let disabled: Vec<_> = config.reviewers.iter().filter(|r| !r.enabled).map(|r| r.display_name.as_str()).collect();
    for r in &config.reviewers {
        status.insert(r.name.clone(), r.enabled);
    }
    if !disabled.is_empty() {
        println!("[plaza-ant] Disabled: {}", disabled.join(", "));
    }
    println!("[plaza-ant] Enabled: {} of {} reviewers", enabled_count, config.reviewers.len());
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
        config: config.clone(),
    }));

    let app = Router::new()
        .route("/", post(handle_plaza.clone()))
        .route("/plaza", post(handle_plaza))
        .route("/admin", post(handle_admin.clone()))
        .route("/plaza/admin", post(handle_admin))
        .route("/airy-to-cody", post(handle_airy))
        .with_state(state.clone());

    let port = config.port;
    let cdp_url = config.cdp_url.clone();
    let addr = format!("0.0.0.0:{port}");
    println!("[plaza-ant] Listening on {addr}");
    println!("[plaza-ant] Routes: /plaza, /plaza/admin, /airy-to-cody");
    println!("[plaza-ant] CLI: tmux send-keys | Web: chromiumoxide CDP on {cdp_url}");

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
        // Clone reviewers to avoid borrow conflict
        let reviewers = plaza.config.reviewers.clone();
        for (i, reviewer) in reviewers.iter().enumerate() {
            let is_online = plaza.status.get(&reviewer.name).copied().unwrap_or(true);
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

/// Pop the next reviewer from the queue and dispatch — loops for scrape reviewers
async fn dispatch_next(state: SharedState) {
    loop {
        let next = {
            let mut plaza = state.write().await;
            plaza.active_reviewer = None;
            plaza.queue.pop_front()
        };

        if let Some((idx, event)) = next {
            let reviewer = {
                let plaza = state.read().await;
                plaza.config.reviewers[idx].clone()
            };
            {
                let mut plaza = state.write().await;
                plaza.active_reviewer = Some(reviewer.name.clone());
                println!(
                    "[plaza-ant] Dispatching next in queue: {} ({} remaining)",
                    reviewer.display_name, plaza.queue.len()
                );
            }
            let needs_next = dispatch_reviewer(&reviewer, &event, state.clone()).await;
            if needs_next {
                // Scrape reviewer finished — loop to dispatch next immediately
                continue;
            }
            // Self-push/tmux reviewer — wait for filmstrip callback
            break;
        } else {
            println!("[plaza-ant] Queue empty — all reviewers dispatched");
            break;
        }
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

/// Returns true if the caller should call dispatch_next (scrape completed)
async fn dispatch_reviewer(reviewer: &ReviewerJsonConfig, event: &PlazaEvent, state: SharedState) -> bool {
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

    // Build prompt from config template — includes full paths per reviewer type
    let config = { state.read().await.config.clone() };
    let message = config.build_prompt(reviewer, &full_content);

    match reviewer.dispatch.as_str() {
        "tmux" => {
            dispatch_tmux(&reviewer.tmux_session, reviewer, &message).await;
        }
        "cdp" => {
            if reviewer.scrape {
                // Scrape reviewers: plaza-ant handles the push
                dispatch_cdp(&reviewer.tab_match, true, reviewer, event, &message).await;
                notify_cody(&format!(
                    "{} review scraped and pushed. Check the tape.",
                    reviewer.display_name
                )).await;
                {
                    let mut plaza = state.write().await;
                    plaza.active_reviewer = None;
                }
                return true;
            } else {
                // Self-push reviewers: filmstrip callback advances queue
                dispatch_cdp(&reviewer.tab_match, false, reviewer, event, &message).await;
            }
        }
        _ => {
            println!("[plaza-ant] Unknown dispatch method: {}", reviewer.dispatch);
        }
    }
    false
}

// ── tmux dispatch (CLI reviewers) ──────────────────────────────────────

async fn dispatch_tmux(session: &str, reviewer: &ReviewerJsonConfig, message: &str) {
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

async fn dispatch_cdp(tab_match: &str, scrape: bool, reviewer: &ReviewerJsonConfig, event: &PlazaEvent, message: &str) {
    let prompt = message.to_string();

    // Connect to existing Chrome
    let (mut browser, handler) = match Browser::connect(&env::var("PLAZA_CDP_URL").unwrap_or_else(|_| "http://localhost:9222".to_string())).await {
        Ok(conn) => conn,
        Err(e) => {
            println!(
                "[plaza-ant] ERROR: cannot connect to Chrome at {} — {}: {}",
                &env::var("PLAZA_CDP_URL").unwrap_or_else(|_| "http://localhost:9222".to_string()), reviewer.display_name, e
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
    reviewer: &ReviewerJsonConfig,
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
async fn scrape_and_push(tab_match: &str, reviewer: &ReviewerJsonConfig) {
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

    let check_and_scrape_js = r#"(function(){var stop=document.querySelector('button[aria-label=\"Stop generating\"]')||document.querySelector('button[aria-label=\"Stop Response\"]');if(stop)return 'streaming';var streaming=document.querySelector('.result-streaming');if(streaming)return 'streaming';var gemLoading=document.querySelector('.loading-indicator,.response-loading,.thinking-indicator');if(gemLoading)return 'streaming';var msgs=document.querySelectorAll('[data-message-author-role=\"assistant\"]');if(msgs.length>0){var last=msgs[msgs.length-1];var md=last.querySelector('.markdown');return 'SCRAPED:'+(md||last).textContent.trim();}var gemMsgs=document.querySelectorAll('.model-response-text');if(gemMsgs.length>0){return 'SCRAPED:'+gemMsgs[gemMsgs.length-1].textContent.trim();}var grokMsgs=document.querySelectorAll('.response-content-markdown');if(grokMsgs.length>0){return 'SCRAPED:'+grokMsgs[grokMsgs.length-1].textContent.trim();}return 'empty';})()"#;

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

    // Write to blessings file and push — uses config paths
    let repo_path = env::var("PLAZA_REPO_PATH")
        .unwrap_or_else(|_| format!("{}/dodo-bird-c3-tion-777333", env::var("HOME").unwrap_or_default()));
    let branch = env::var("PLAZA_BRANCH").unwrap_or_else(|_| "main".to_string());
    let blessings_dir = env::var("PLAZA_BLESSINGS_DIR").unwrap_or_else(|_| "blessings".to_string());

    let entry_path = format!("{}/{}/{}", repo_path, blessings_dir, reviewer.entry_file);

    if let Err(e) = tokio::fs::write(&entry_path, &response_text).await {
        println!("[plaza-ant] ERROR: could not write {}: {}", entry_path, e);
        return;
    }

    let dodo_dir = repo_path.clone();
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
        let add_path = format!("{}/{}", blessings_dir, reviewer.entry_file);
        let add = Command::new("git")
            .args(["add", &add_path])
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

        // git push to target branch
        let push = Command::new("git")
            .args(["push", "origin", &branch])
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
    let resp = reqwest::get(&format!("{}/json/list", &env::var("PLAZA_CDP_URL").unwrap_or_else(|_| "http://localhost:9222".to_string()))).await.ok()?;
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
        .args(["send-keys", "-t", &env::var("PLAZA_CODY_SESSION").unwrap_or_else(|_| "cody".to_string()), &safe_msg])
        .output()
        .await;

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let _ = Command::new("tmux")
        .args(["send-keys", "-t", &env::var("PLAZA_CODY_SESSION").unwrap_or_else(|_| "cody".to_string()), "Enter"])
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
        .args(["send-keys", "-t", &env::var("PLAZA_CODY_SESSION").unwrap_or_else(|_| "cody".to_string()), &safe_cmd])
        .output()
        .await;

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let _ = Command::new("tmux")
        .args(["send-keys", "-t", &env::var("PLAZA_CODY_SESSION").unwrap_or_else(|_| "cody".to_string()), "Enter"])
        .output()
        .await;

    (StatusCode::OK, "{\"status\":\"sent\"}")
}
