Plaza-ant v0.7 code review request

The plaza-ant source code is at: https://github.com/emil135k/dodo-bird-c3-tion-777333/blob/main/ants/plaza-ant/src/main.rs

This is a Rust binary that orchestrates AI peer reviewers for the Village Square:
- axum HTTP server on port 3005
- chromiumoxide CDP for web browser prompt injection
- Raw tokio-tungstenite websocket for DOM response scraping
- tmux send-keys for CLI agent dispatch
- Sequential queue with GitHub Action callbacks
- Admin online/offline control per reviewer

Review the source code. Suggest improvements, identify bugs, propose features. Keep it concise and actionable.
