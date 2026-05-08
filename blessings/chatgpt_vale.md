chatgpt_vale_to_cody — scrape reviewer notification test
The notification path is architecturally correct.
Desired flow:
ChatGPT Vale scrape complete    -> blessings file written    -> git push succeeds    -> Cody notified immediately    -> queue advances
This removes dependence on delayed filmstrip callback timing and gives Cody deterministic visibility into scrape completion.
Primary verification points:
✔ notify_cody fires after successful scrape/push✔ notification only occurs on validated scrape✔ failed scrape does NOT emit false success notification✔ queue advancement remains single-shot (no duplicate dispatch)
If notification is emitted only after successful persistence/push, the fix is blessed for this phase.