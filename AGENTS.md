# Project: agent-panel-in-terminal

`agent-panel-in-terminal` is a native, ultra-lightweight Windows command panel for instantly switching and launching AI coding agents directly from Windows Terminal (`wt.exe`) or Command Prompt (`cmd.exe`).

## 1. Supported Agent Ecosystem

The application supports the following agent CLIs:

| Agent Name | CLI Binary | Default Model List | Effort Flag | Permissions Flag |
|---|---|---|---|---|
| **Antigravity** | `agy` | `gemini-3.8-flash-high`, `gemini-3-flash`, `claude-sonnet-4-6` | `--effort <low\|medium\|high>` | `--dangerously-skip-permissions` |
| **Claude** | `claude` | `claude-opus-4-6-thinking`, `claude-sonnet-4-6`, `claude-haiku-4-5` | `--effort <low\|medium\|high>` | `--dangerously-skip-permissions` |
| **OpenClaude** | `openclaude` | `custom-openai`, `custom-gemini`, `local-ollama` | Config-based | Auto-approved |
| **Codex** | `codex` | `codex-5-pro`, `codex-4.5`, `gpt-5-turbo` | Config-based | `--full-access` |
| **GitHub Copilot** | `gh copilot` | `copilot-chat`, `gpt-4o` | Config-based | Default |
| **OpenCode** | `opencode` | `default`, `custom` | Config-based | Auto-approved |
| **openclaw** | `openclaw` | `claw-v1`, `local` | Config-based | `--sandbox` |

## 2. Configuration Schema (`agents.json`)

All agent definitions are persisted in a JSON file at `%APPDATA%\agent-panel-in-terminal\agents.json`:

```json
{
  "active_agents": [
    "antigravity",
    "claude",
    "openclaude",
    "codex",
    "copilot",
    "opencode",
    "openclaw"
  ],
  "available_agents": [],
  "agents": {
    "antigravity": {
      "display_name": "Antigravity",
      "command": "agy",
      "models": [
        "gemini-3.8-flash-high",
        "gemini-3.5-flash-lite",
        "gemini-3.1-flash-lite",
        "gemini-3.1-flash-image",
        "gemini-3-flash",
        "claude-sonnet-4-6",
        "claude-opus-4-6-thinking"
      ],
      "effort_flag": "--effort",
      "permissions_flag": "--dangerously-skip-permissions"
    },
    "claude": {
      "display_name": "Claude",
      "command": "claude",
      "models": [
        "claude-opus-4-6-thinking",
        "claude-sonnet-4-6",
        "claude-haiku-4-5"
      ],
      "effort_flag": "--effort",
      "permissions_flag": "--dangerously-skip-permissions"
    }
  }
}
```

## 3. UI Controls & Interactions

1. **Left Dropdown**: Displays list of enabled agents. Clicking or hovering an agent opens the right flyout.
2. **Right Flyout**:
   - **Model Search Bar**: Live filter on available models for that agent.
   - **Model Selection**: Radio-style selection with checkmark (optional).
   - **Effort Buttons**: `Low` | `Med` | `High`.
   - **Permissions Selector**: `Full access` | `Default`.
   - **Actions**:
     - `Start here`: Injects into active console/terminal or runs directly in stdio.
     - `Start in new tab`: Opens a new tab in Windows Terminal or launches a new CMD window.
3. **Agent Settings**:
   - Opens the reorder/enablement GUI.
   - Top list shows active panel agents.
   - Bottom list shows available agents.
   - Clicking an available agent moves it up into the active panel list.

## 4. Repository & Version Control Rules

- **Markdown Policy**: Only `AGENTS.md` is committed and tracked in the Git repository.
- **Local-Only Documents**: All internal planning, decision, and specification markdown files (`decisions.md`, `spec.md`, `plan.md`) are strictly local and ignored by Git via `.gitignore`.

