# Agent Panel In Terminal

Adds AI coding-agent launchers to the Windows Terminal new tab (`+`) dropdown menu.
A settings window lets the user choose which agents appear; entries are compiled
surgically into Windows Terminal's `settings.json` with a timestamped backup and
SHA-256 evidence before every write.

## Stack
- Rust backend: JSONC-preserving settings.json editor, CLI/model detection,
  config store (`%APPDATA%\agent-panel\config.json`).
- Tauri 2 + WebView2 UI: Vite, vanilla TypeScript/HTML/CSS.

## Build & test
- `cargo test` — Rust engine unit tests (run in `src-tauri/`).
- `npm install`, then `npm run tauri dev` — run the settings window in dev mode.
- `npm run tauri build` — release build and installer.

## Rules for agents working on this repo
- Never hardcode agent or model lists; agents and models come from detection
  on the user's machine or manual entry only.
- Never write settings.json without a timestamped backup and its SHA-256 hash;
  modify only AgentPanel-tagged blocks.
- Entries are identified by deterministic GUIDs and the `AgentPanel.` action-id prefix.
- No Tauri plugins; expose only our own commands to the front end; keep the strict CSP.
