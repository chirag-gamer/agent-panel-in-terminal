//! Tool-owned user configuration schema.
//!
//! Stored at `%APPDATA%\agent-panel\config.json`. Contains no hardcoded agent
//! or model data: built-in CLI adapters provide flag syntax only (see
//! `phase1-research.md`), and all agents/models come from detection or manual
//! user entry.

use serde::{Deserialize, Serialize};

/// One agent entry as configured by the user in the Customize GUI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEntry {
    /// Stable slug, e.g. `claude-code`, or user-chosen for manual entries.
    pub id: String,
    /// Display name shown in the dropdown folder.
    pub name: String,
    /// New-tab launch string (executable + flags), composed by the GUI
    /// from the adapter's flag syntax plus the user's model/effort choices.
    pub commandline: String,
    /// Text injected via `sendInput` for "Open here", MUST end with `\r`.
    pub open_here_input: String,
    /// Optional model id selected by the user (already embedded in
    /// `commandline` by the GUI; kept for display/diff purposes).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// 0-based position in the dropdown (active agents: 0..3).
    #[serde(default)]
    pub entry_order: u32,
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// True for user-typed custom entries (no built-in adapter).
    #[serde(default)]
    pub manual: bool,
}

fn default_true() -> bool {
    true
}

/// Root of the tool-owned config file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PanelConfig {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub agents: Vec<AgentEntry>,
    /// Absolute path of this GUI executable, captured at build time for the
    /// `AgentPanel.Customize` commandline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub customize_commandline: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_minimal() {
        let cfg = PanelConfig {
            version: 1,
            agents: vec![AgentEntry {
                id: "manual-x".into(),
                name: "My Tool".into(),
                commandline: "mytool --flag".into(),
                open_here_input: "mytool --flag\r".into(),
                model: None,
                entry_order: 0,
                enabled: true,
                manual: true,
            }],
            customize_commandline: None,
        };
        let s = serde_json::to_string(&cfg).unwrap();
        let back: PanelConfig = serde_json::from_str(&s).unwrap();
        assert_eq!(back.agents[0].name, "My Tool");
        assert!(back.agents[0].enabled);
        assert!(back.agents[0].manual);
    }
}
