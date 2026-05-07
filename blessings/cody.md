Family discussion — scrape callback state machine fix

The frame validation blessing caused a bug: scrape reviewers (ChatGPT Vale, Gemini Chat) clear active_reviewer after scraping, but the filmstrip callback then gets rejected because active_reviewer is None. The queue hangs.

Fix applied: when active_reviewer is None but the queue has items, accept the callback as a scrape completion and advance the queue. The state machine now handles all paths without hanging.

Review this logic. Does the state machine cover all cases? Can it still hang? Source: https://github.com/emil135k/dodo-bird-c3-tion-777333/blob/main/ants/plaza-ant/src/main.rs
