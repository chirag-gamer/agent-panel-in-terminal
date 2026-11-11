# Project: agent-panel-in-terminal

`agent-panel-in-terminal` is a native, ultra-lightweight Windows Terminal extension that surfaces AI coding agents directly inside the Windows Terminal new tab (`+`) dropdown menu.

## 1. Architectural Model

- **Native Windows Terminal Integration**: Integrates directly into Windows Terminal's native `settings.json` (`newTabMenu` feature).
- **No Background Footprint**: Runs zero background daemons, zero system tray icons, zero global hotkeys, zero external overlay windows, and zero console injection hacks.
- **Dropdown Layout**:
  - Displays 2–4 user-chosen agent entries directly in the `+` dropdown.
  - Each agent entry is a submenu (`folder`) containing:
    - **"Open here"**: Injects and runs the agent in the currently focused tab via an `action` entry (`sendInput`).
    - **"Open in new tab"**: Spawns a fresh tab running the agent via a `profile` entry (`commandline`).
  - **"Customize…" Entry**: Launches a lightweight, standalone settings GUI to choose which agents appear in the dropdown, select models, or manually configure custom commands. Spawns detached with `closeOnExit: "always"` to leave no lingering tab.

## 2. Zero Hardcoding Policy

This tool is designed as a public, modular product:
- **No Hardcoded Agents**: Agents are detected from the user's installed CLIs or added manually.
- **No Hardcoded Models**: Models are discovered dynamically from the user's active environment or entered manually in the Customize GUI.
- **Modular Schema**: Configuration stores user-defined or discovered agents and compiles them surgically into the user's `settings.json`.

## 3. Configuration & Isolation

- **Isolated Settings Mutation**: The tool modifies only its own tagged entries in `settings.json`:
  - Tagged `actions` (`AgentPanel.*`)
  - Tagged hidden `profiles` (`AgentPanel.*`)
  - Tagged `newTabMenu` folder entries
- **Automatic Live Reload**: Windows Terminal automatically detects file modifications and updates the dropdown menu in real time without restart.
- **Safety First**: Prior to any write, `settings.json` is automatically backed up with a timestamp.

## 4. Repository & Version Control Rules

- **Markdown Policy**: Only `AGENTS.md` is committed and tracked in the Git repository.
- **Local-Only Documents**: All internal planning, decision, and specification markdown files (`decisions.md`, `spec.md`, `plan.md`) are strictly local and ignored by Git via `.gitignore`.
