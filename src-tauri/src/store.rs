//! Persistence: tool-owned config, settings.json path, timestamped backups
//! with SHA-256 evidence, and the compile step that writes the dropdown.

use crate::config::PanelConfig;
use crate::settings::{self, CompiledAgent};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

pub fn config_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("agent-panel")
}

pub fn config_path() -> PathBuf {
    // Test/portable override, same pattern as AGENT_PANEL_WT_SETTINGS.
    if let Some(p) = std::env::var_os("AGENT_PANEL_CONFIG") {
        return PathBuf::from(p);
    }
    config_dir().join("config.json")
}

pub fn load_config() -> PanelConfig {
    std::fs::read_to_string(config_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_config(cfg: &PanelConfig) -> Result<(), String> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    std::fs::write(config_path(), json).map_err(|e| e.to_string())
}

/// Windows Terminal settings.json location (per-user packaged install).
/// `AGENT_PANEL_WT_SETTINGS` overrides the path (testing / portable setups).
pub fn wt_settings_path() -> PathBuf {
    if let Some(p) = std::env::var_os("AGENT_PANEL_WT_SETTINGS") {
        return PathBuf::from(p);
    }
    PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_default())
        .join("Packages")
        .join("Microsoft.WindowsTerminal_8wekyb3d8bbwe")
        .join("LocalState")
        .join("settings.json")
}

pub fn sha256_hex(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    let digest = h.finalize();
    let mut s = String::with_capacity(64);
    for byte in digest {
        s.push_str(&format!("{byte:02X}"));
    }
    s
}

pub fn backup_settings() -> Result<(PathBuf, String), String> {
    let src = wt_settings_path();
    let data = std::fs::read(&src).map_err(|e| e.to_string())?;
    let hash = sha256_hex(&data);
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let dest = src.with_file_name(format!("settings.json.backup_{ts}"));
    std::fs::write(&dest, &data).map_err(|e| e.to_string())?;
    Ok((dest, hash))
}

/// Restore the most recent backup over settings.json; returns restored hash.
pub fn restore_latest_backup() -> Result<String, String> {
    let dir = wt_settings_path().parent().unwrap().to_path_buf();
    let mut backups: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .map(|n| n.to_string_lossy().starts_with("settings.json.backup_"))
                .unwrap_or(false)
        })
        .collect();
    backups.sort();
    let latest = backups.pop().ok_or("no backup found")?;
    let data = std::fs::read(&latest).map_err(|e| e.to_string())?;
    let hash = sha256_hex(&data);
    std::fs::write(wt_settings_path(), &data).map_err(|e| e.to_string())?;
    Ok(hash)
}

/// Compile the config into WT settings.json (backup → apply → atomic write).
/// Returns (backup path, backup hash, new hash).
pub fn compile_and_write(cfg: &PanelConfig) -> Result<(PathBuf, String, String), String> {
    let src = wt_settings_path();
    let original = std::fs::read_to_string(&src).map_err(|e| e.to_string())?;
    let (backup, backup_hash) = backup_settings()?;

    let agents: Vec<CompiledAgent> = cfg
        .agents
        .iter()
        .filter(|a| a.enabled)
        .map(|a| CompiledAgent {
            id: a.id.clone(),
            name: a.name.clone(),
            commandline: a.commandline.clone(),
            open_here_input: a.open_here_input.clone(),
        })
        .collect();
    let active = agents.len();
    if !(2..=4).contains(&active) {
        return Err(format!("2–4 active agents required, got {active}"));
    }
    let customize = cfg
        .customize_commandline
        .clone()
        .unwrap_or_else(default_customize_commandline);

    let out = settings::apply(&original, &agents, &customize)?;

    // Validate the result parses (strict: host file is plain JSON; comments
    // are preserved verbatim so strict parse is only possible when the user
    // file has none — fall back to no-op validation otherwise).
    let has_comment = original.lines().any(|l| {
        let t = l.trim_start();
        t.starts_with("//") || t.starts_with("/*")
    });
    if !has_comment {
        serde_json::from_str::<serde_json::Value>(&out)
            .map_err(|e| format!("compiled settings.json is not valid JSON: {e}"))?;
    }

    // Atomic write: temp file in same dir, then rename.
    let tmp = src.with_file_name("settings.json.agentpanel_tmp");
    std::fs::write(&tmp, &out).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &src).map_err(|e| e.to_string())?;
    let new_hash = sha256_hex(out.as_bytes());
    Ok((backup, backup_hash, new_hash))
}

fn default_customize_commandline() -> String {
    let exe = std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "agent-panel.exe".to_string());
    format!("cmd.exe /c start \"\" \"{exe}\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_known_vector() {
        // sha256("abc")
        assert_eq!(
            sha256_hex(b"abc"),
            "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD"
        );
    }

    #[test]
    fn config_roundtrip_file() {
        // Never touch the real %APPDATA% config: override to a temp path.
        let tmp = std::env::temp_dir().join("agent-panel-test-config.json");
        // SAFETY: test-only env mutation; no other threads set/removed this var.
        unsafe { std::env::set_var("AGENT_PANEL_CONFIG", &tmp) };
        let cfg = PanelConfig {
            version: 1,
            agents: vec![],
            customize_commandline: Some("x".into()),
        };
        save_config(&cfg).unwrap();
        let loaded = load_config();
        assert_eq!(loaded.customize_commandline.as_deref(), Some("x"));
        // cleanup
        unsafe { std::env::remove_var("AGENT_PANEL_CONFIG") };
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn wt_settings_path_shape() {
        let p = wt_settings_path();
        assert!(p.ends_with("LocalState/settings.json"));
    }
}
