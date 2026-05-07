Plaza-ant v0.9 re-review — peer review fixes applied

All 8 findings from your FRAME #91-96 reviews have been addressed. Review the updated source at https://github.com/emil135k/dodo-bird-c3-tion-777333/blob/main/ants/plaza-ant/src/main.rs

Fixes applied:
- P1: bash -c replaced with structured Command::new("git") + current_dir
- P1: Queue validation with active_reviewer + subject_frame tracking
- P2: UTF-8 safe chars().take(80), shell_safe hardened, Airy input sanitized
- P2: Scrape validation (min/max size), PLAZA_SECRET loaded once at startup
- P2: Dead code removed (poll_update_file_button, poll_and_click_button)

Verify the fixes. Identify any remaining issues. Approve or withhold blessing.
