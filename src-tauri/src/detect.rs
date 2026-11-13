//! Built-in CLI adapters, installation detection, and dynamic model discovery.
//!
//! Zero hardcoding policy: adapters contain only flag SYNTAX and discovery
//! mechanisms — never agent lists or model lists. All models come from the
//! user's environment (subcommand / config file) or manual GUI entry.

use crate::config::AgentEntry;
use serde::Serialize;

/// How the panel discovers the models available to a CLI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ModelSource {
    /// Run `<exe> <args>` and parse plain-text lines / JSON.
    Command(&'static str),
    /// Read a JSON config file and extract model ids by structure.
    JsonFile { path: &'static str, extract: JsonExtract },
    /// Read a TOML config file; collect values under the given key names
    /// (e.g. `model` in codex config.toml, plus profile models).
    TomlFile { path: &'static str },
    /// No non-interactive source: manual entry only.
    Manual,
}

/// JSON extraction strategies (all string-walk, defensive against drift).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum JsonExtract {
    /// Single string value at `key` (e.g. Claude `{"model": "..."}`).
    Str(&'static str),
    /// Array at `key` of strings or objects with `model`/`id` members
    /// (e.g. droid `customModels`).
    List(&'static str),
    /// Collect the KEYS of every object member named `models` reachable
    /// under the given root keys (opencode/qwen/copilot provider tables:
    /// `providers.<id>.models.<modelId>`).
    ProviderModels(&'static [&'static str]),
    /// Collect every string under the given key path segments, at any
    /// object depth (defensive catch-all for catalog-shaped files).
    DeepStrings(&'static [&'static str]),
}

/// How reasoning effort is selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum EffortStyle {
    /// `<flag> <level>` with these levels.
    Flag(&'static str, &'static [&'static str]),
    /// Append `#<level>` to the model id (opencode variants).
    VariantSuffix(&'static [&'static str]),
    /// Not supported at launch; omit.
    None,
}

#[derive(Debug, Clone, Serialize)]
pub struct Adapter {
    /// Stable agent id used in `AgentPanel.<id>.*` tags.
    pub id: &'static str,
    /// Display name.
    pub name: &'static str,
    /// Executable to look up on PATH.
    pub exe: &'static str,
    /// Arguments that confirm identity + version (default `--version`).
    pub version_args: &'static str,
    /// Substring that must appear in version output (disambiguates generic
    /// binary names like Cursor's `agent`).
    pub version_hint: &'static str,
    /// Interactive launch command (may differ from bare exe).
    pub launch: &'static str,
    /// Model flag syntax, e.g. `--model` / `-m`; None = no launch flag.
    pub model_flag: Option<&'static str>,
    pub effort: EffortStyle,
    pub models: &'static [ModelSource],
}

/// Host model-discovery commands are plain-text where documented:
/// opencode: `opencode models` (lines `provider/model`) — host-verified.
/// crush:    `crush models` (lines `provider/id`).
/// cursor:   `agent models` (line list).
/// aider:    `aider --models .` (line list).
/// codex:    `codex debug models` (JSON catalog; parse defensively).
pub const ADAPTERS: &[Adapter] = &[
    Adapter {
        id: "claude-code",
        name: "Claude Code",
        exe: "claude",
        version_args: "--version",
        version_hint: "Claude Code",
        launch: "claude",
        model_flag: Some("--model"),
        effort: EffortStyle::Flag("--effort", &["low", "medium", "high", "xhigh", "max"]),
        models: &[ModelSource::JsonFile { path: "%USERPROFILE%\\.claude\\settings.json", extract: JsonExtract::Str("model") }],
    },
    Adapter {
        id: "codex",
        name: "Codex CLI",
        exe: "codex",
        version_args: "--version",
        version_hint: "",
        launch: "codex",
        model_flag: Some("--model"),
        effort: EffortStyle::None, // config-only (model_reasoning_effort)
        models: &[ModelSource::Command("debug models"), ModelSource::TomlFile { path: "%USERPROFILE%\\.codex\\config.toml" }],
    },
    Adapter {
        id: "gemini",
        name: "Gemini CLI",
        exe: "gemini",
        version_args: "--version",
        version_hint: "",
        launch: "gemini",
        model_flag: Some("-m"),
        effort: EffortStyle::None,
        models: &[ModelSource::JsonFile { path: "%USERPROFILE%\\.gemini\\settings.json", extract: JsonExtract::DeepStrings(&["model"]) }],
    },
    Adapter {
        id: "copilot",
        name: "Copilot CLI",
        exe: "copilot",
        version_args: "version",
        version_hint: "",
        launch: "copilot",
        model_flag: Some("--model"),
        effort: EffortStyle::Flag("--reasoning-effort", &["none", "minimal", "low", "medium", "high", "xhigh", "max"]),
        models: &[
            ModelSource::JsonFile { path: "%USERPROFILE%\\.copilot\\providers.json", extract: JsonExtract::ProviderModels(&["providers"]) },
            ModelSource::JsonFile { path: "%USERPROFILE%\\.copilot\\providers.json", extract: JsonExtract::ProviderModels(&["models"]) },
        ],
    },
    Adapter {
        id: "cursor",
        name: "Cursor Agent",
        exe: "agent",
        version_args: "--version",
        version_hint: "Cursor",
        launch: "agent",
        model_flag: Some("--model"),
        effort: EffortStyle::None, // effort encoded in model id
        models: &[ModelSource::Command("models"), ModelSource::JsonFile { path: "%USERPROFILE%\\.cursor\\cli-config.json", extract: JsonExtract::DeepStrings(&["model"]) }],
    },
    Adapter {
        id: "aider",
        name: "Aider",
        exe: "aider",
        version_args: "--version",
        version_hint: "",
        launch: "aider",
        model_flag: Some("--model"),
        effort: EffortStyle::Flag("--reasoning-effort", &["low", "medium", "high"]),
        models: &[ModelSource::Command("--models .")],
    },
    Adapter {
        id: "opencode",
        name: "OpenCode",
        exe: "opencode",
        version_args: "--version",
        version_hint: "",
        launch: "opencode",
        model_flag: Some("-m"),
        effort: EffortStyle::VariantSuffix(&["low", "medium", "high", "max"]),
        models: &[ModelSource::Command("models"), ModelSource::JsonFile { path: "%USERPROFILE%\\.config\\opencode\\opencode.json", extract: JsonExtract::ProviderModels(&["providers"]) }],
    },
    Adapter {
        id: "crush",
        name: "Crush",
        exe: "crush",
        version_args: "--version",
        version_hint: "",
        launch: "crush",
        model_flag: None, // config slots / TUI picker
        effort: EffortStyle::None,
        models: &[ModelSource::Command("models"), ModelSource::JsonFile { path: "%USERPROFILE%\\.config\\crush\\crush.json", extract: JsonExtract::DeepStrings(&["models"]) }],
    },
    Adapter {
        id: "qwen",
        name: "Qwen Code",
        exe: "qwen",
        version_args: "--version",
        version_hint: "",
        launch: "qwen",
        model_flag: Some("--model"),
        effort: EffortStyle::None, // in-session /effort
        models: &[ModelSource::JsonFile { path: "%USERPROFILE%\\.qwen\\settings.json", extract: JsonExtract::DeepStrings(&["model", "modelProviders"]) }],
    },
    Adapter {
        id: "goose",
        name: "Goose",
        exe: "goose",
        version_args: "--version",
        version_hint: "",
        launch: "goose session", // bare `goose` TUI is UNVERIFIED
        model_flag: Some("--model"), // documented on `goose run`
        effort: EffortStyle::None,
        models: &[ModelSource::Manual], // live fetch is inside interactive configure
    },
    Adapter {
        id: "droid",
        name: "Droid",
        exe: "droid",
        version_args: "--version",
        version_hint: "",
        launch: "droid",
        model_flag: Some("-m"),
        effort: EffortStyle::Flag("--reasoning-effort", &["low", "medium", "high", "xhigh", "max"]),
        models: &[ModelSource::JsonFile { path: "%USERPROFILE%\\.factory\\settings.json", extract: JsonExtract::List("customModels") }],
    },
    // --- Phase 4 additions (official-source verified, 2026-11-13) ---
    Adapter {
        id: "kiro",
        name: "Kiro CLI (AWS Q)",
        exe: "kiro-cli",
        version_args: "--version",
        version_hint: "",
        launch: "kiro-cli",
        model_flag: None, // settings cmd / TUI /model only
        effort: EffortStyle::None,
        models: &[ModelSource::JsonFile { path: "%USERPROFILE%\\.kiro\\settings\\cli.json", extract: JsonExtract::DeepStrings(&["model", "defaultModel"]) }],
    },
    Adapter {
        id: "qoder",
        name: "Qoder CLI",
        exe: "qoder",
        version_args: "--version",
        version_hint: "",
        launch: "qoder",
        model_flag: Some("--model"),
        effort: EffortStyle::Flag("--reasoning-effort", &["low", "medium", "high"]),
        models: &[ModelSource::Command("--list-models")],
    },
    Adapter {
        id: "trae",
        name: "Trae CLI",
        exe: "traecli",
        version_args: "--version",
        version_hint: "",
        launch: "traecli",
        model_flag: Some("--model"),
        effort: EffortStyle::None, // TUI /model picks model + strength
        models: &[ModelSource::TomlFile { path: "%USERPROFILE%\\.trae\\traecli.toml" }],
    },
    Adapter {
        id: "codebuddy",
        name: "CodeBuddy Code",
        exe: "codebuddy",
        version_args: "--version",
        version_hint: "",
        launch: "codebuddy",
        model_flag: Some("--model"),
        effort: EffortStyle::None,
        models: &[ModelSource::JsonFile { path: "%USERPROFILE%\\.codebuddy\\settings.json", extract: JsonExtract::Str("model") }],
    },
    Adapter {
        id: "comate",
        name: "Comate CLI (Baidu)",
        exe: "comatecli",
        version_args: "--version",
        version_hint: "",
        launch: "comatecli",
        model_flag: Some("--model"),
        effort: EffortStyle::None,
        models: &[ModelSource::Command("model list")],
    },
    Adapter {
        id: "vibe",
        name: "Mistral Vibe",
        exe: "vibe",
        version_args: "--version",
        version_hint: "",
        launch: "vibe",
        model_flag: Some("--model"),
        effort: EffortStyle::None, // per-model thinking key in config
        models: &[ModelSource::TomlFile { path: "%USERPROFILE%\\.vibe\\config.toml" }],
    },
    Adapter {
        id: "antigravity",
        name: "Antigravity CLI",
        exe: "agy",
        version_args: "--version",
        version_hint: "",
        launch: "agy",
        model_flag: Some("--model"),
        effort: EffortStyle::Flag("--effort", &["low", "medium", "high", "max"]),
        models: &[
            ModelSource::Command("models"),
            ModelSource::JsonFile { path: "%USERPROFILE%\\.gemini\\antigravity-cli\\settings.json", extract: JsonExtract::Str("model") },
        ],
    },
    Adapter {
        id: "minimax",
        name: "MiniMax Code",
        exe: "mcode",
        version_args: "--version",
        version_hint: "",
        launch: "mcode",
        model_flag: Some("--model"),
        effort: EffortStyle::None,
        models: &[ModelSource::Manual], // provider list --json shape UNVERIFIED
    },
    Adapter {
        id: "costrict",
        name: "Costrict",
        exe: "cs",
        version_args: "--version",
        version_hint: "",
        launch: "cs",
        model_flag: Some("--model"),
        effort: EffortStyle::None, // ctrl+t cycles variants
        models: &[ModelSource::Manual], // global costrict.json path UNVERIFIED
    },
    Adapter {
        id: "warp",
        name: "Warp Agent CLI",
        exe: "warp",
        version_args: "--version",
        version_hint: "",
        launch: "warp",
        model_flag: None, // /model picker persists to profile
        effort: EffortStyle::None,
        models: &[ModelSource::Manual],
    },
    Adapter {
        id: "devin",
        name: "Devin CLI",
        exe: "devin",
        version_args: "version",
        version_hint: "",
        launch: "devin",
        model_flag: Some("--model"),
        effort: EffortStyle::None, // in-session Alt+T
        models: &[
            ModelSource::Command("models list"),
            ModelSource::JsonFile { path: "%APPDATA%\\devin\\config.json", extract: JsonExtract::DeepStrings(&["model", "pinned_models"]) },
        ],
    },
    Adapter {
        id: "cline",
        name: "Cline",
        exe: "cline",
        version_args: "--version",
        version_hint: "",
        launch: "cline",
        model_flag: Some("--model"),
        effort: EffortStyle::Flag("--thinking", &["none", "low", "medium", "high", "xhigh"]),
        models: &[ModelSource::Manual], // providers.json layout UNVERIFIED
    },
    Adapter {
        id: "kilo",
        name: "Kilo Code",
        exe: "kilo",
        version_args: "--version",
        version_hint: "",
        launch: "kilo",
        model_flag: Some("--model"),
        effort: EffortStyle::None, // --variant is run-only
        models: &[
            ModelSource::Command("models"),
            ModelSource::JsonFile { path: "%USERPROFILE%\\.config\\kilo\\kilo.jsonc", extract: JsonExtract::DeepStrings(&["model"]) },
        ],
    },
    Adapter {
        id: "auggie",
        name: "Augment (auggie)",
        exe: "auggie",
        version_args: "--version",
        version_hint: "",
        launch: "auggie",
        model_flag: Some("--model"),
        effort: EffortStyle::Flag("--thinking", &["off", "minimal", "low", "medium", "high", "xhigh"]),
        models: &[
            ModelSource::Command("model list"),
            ModelSource::JsonFile { path: "%USERPROFILE%\\.augment\\settings.json", extract: JsonExtract::Str("defaultModel") },
        ],
    },
    Adapter {
        id: "neovate",
        name: "Neovate Code",
        exe: "neovate",
        version_args: "--version",
        version_hint: "",
        launch: "neovate",
        model_flag: Some("-m"),
        effort: EffortStyle::None, // config thinkingLevel
        models: &[ModelSource::JsonFile { path: "%USERPROFILE%\\.neovate\\config.json", extract: JsonExtract::DeepStrings(&["model"]) }],
    },
    Adapter {
        id: "open-interpreter",
        name: "Open Interpreter",
        exe: "interpreter",
        version_args: "--version",
        version_hint: "",
        launch: "interpreter",
        model_flag: Some("--model"),
        effort: EffortStyle::None, // config model_reasoning_effort
        models: &[ModelSource::TomlFile { path: "%USERPROFILE%\\.openinterpreter\\config.toml" }],
    },
    Adapter {
        id: "aichat",
        name: "AIChat",
        exe: "aichat",
        version_args: "-V",
        version_hint: "",
        launch: "aichat",
        model_flag: Some("--model"),
        effort: EffortStyle::None,
        models: &[ModelSource::Command("--list-models")],
    },
    Adapter {
        id: "forge",
        name: "Forge Code",
        exe: "forge",
        version_args: "--version",
        version_hint: "",
        launch: "forge",
        model_flag: None, // model via TUI :model / forge.yaml
        effort: EffortStyle::None,
        models: &[ModelSource::Command("list model")],
    },
    Adapter {
        id: "codebuff",
        name: "Codebuff",
        exe: "codebuff",
        version_args: "-v",
        version_hint: "",
        launch: "codebuff",
        model_flag: None, // modes only; backend-chosen models
        effort: EffortStyle::None,
        models: &[ModelSource::Manual],
    },
    Adapter {
        id: "gitlab-duo",
        name: "GitLab Duo CLI",
        exe: "duo",
        version_args: "--version",
        version_hint: "",
        launch: "duo",
        model_flag: Some("--model"),
        effort: EffortStyle::None,
        models: &[ModelSource::Manual],
    },
    Adapter {
        id: "continue",
        name: "Continue",
        exe: "cn",
        version_args: "--version",
        version_hint: "",
        launch: "cn",
        model_flag: Some("--model"),
        effort: EffortStyle::None,
        models: &[ModelSource::Manual],
    },
    Adapter {
        id: "rovodev",
        name: "Rovo Dev CLI",
        exe: "acli",
        version_args: "--version",
        version_hint: "",
        launch: "acli rovodev run",
        model_flag: None, // /models in TUI; config.yml agent.modelId
        effort: EffortStyle::None,
        models: &[ModelSource::Manual],
    },
];

/// Result of probing one adapter on this machine.
#[derive(Debug, Clone, Serialize)]
pub struct Detection {
    pub adapter: &'static Adapter,
    pub installed: bool,
    pub version: String,
}

/// Suppress console flash on Windows when spawning helper processes.
fn spawn_quiet(program: &str, args: &[&str]) -> Option<std::process::Output> {
    let mut cmd = std::process::Command::new(program);
    cmd.args(args);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd.output().ok()
}

/// Run `where <exe>`; returns first path found on PATH.
pub fn find_on_path(exe: &str) -> Option<String> {
    let out = spawn_quiet("where", &[exe])?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines().next().map(|l| l.trim().to_string()).filter(|l| !l.is_empty())
}

/// Probe one adapter: PATH lookup + version output (identity hint enforced).
pub fn detect(adapter: &'static Adapter) -> Detection {
    let mut d = Detection { adapter, installed: false, version: String::new() };
    let Some(path) = find_on_path(adapter.exe) else { return d };
    if let Some(out) = spawn_quiet(&path, &[adapter.version_args]) {
        let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let text = if text.is_empty() { String::from_utf8_lossy(&out.stderr).trim().to_string() } else { text };
        if !adapter.version_hint.is_empty() && !text.to_lowercase().contains(&adapter.version_hint.to_lowercase()) {
            return d; // wrong binary with this name
        }
        d.installed = true;
        d.version = text;
    }
    d
}

/// Detect all built-in adapters.
pub fn detect_all() -> Vec<Detection> {
    ADAPTERS.iter().map(detect).collect()
}

/// Expand `%VAR%` environment references in a config path.
fn expand_path(p: &str) -> Option<std::path::PathBuf> {
    if !p.contains('%') {
        return Some(std::path::PathBuf::from(p));
    }
    // Alternate literal / variable segments; unresolved variables fail lookup
    // and are treated as literals, so a missing env var yields None below.
    let mut out = String::new();
    for (i, part) in p.split('%').enumerate() {
        if i % 2 == 1 {
            if let Ok(v) = std::env::var(part) {
                out.push_str(&v);
                continue;
            }
        }
        out.push_str(part);
    }
    if out.contains('%') {
        None
    } else {
        Some(std::path::PathBuf::from(out))
    }
}

/// Discover available model ids for an adapter across ALL its sources
/// (subcommand output + config files), merged, sorted, deduped.
pub fn discover_models(adapter: &Adapter) -> Vec<String> {
    let mut models: Vec<String> = Vec::new();
    for source in adapter.models {
        match source {
            ModelSource::Command(args) => {
                let arglist: Vec<&str> = args.split_whitespace().collect();
                // Resolve like detection does (PATH may need explicit .exe resolution).
                let program = find_on_path(adapter.exe).unwrap_or_else(|| adapter.exe.to_string());
                let Some(out) = spawn_quiet(&program, &arglist) else { continue };
                if !out.status.success() {
                    continue;
                }
                let text = String::from_utf8_lossy(&out.stdout);
                // Multi-line JSON (e.g. `codex debug models` catalog): try
                // parsing the whole output first.
                let trimmed = text.trim();
                if trimmed.starts_with('{') || trimmed.starts_with('[') {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
                        collect_json_model_ids(&v, &mut models);
                        continue;
                    }
                }
                for line in text.lines() {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                        collect_json_model_ids(&v, &mut models);
                    } else if looks_like_model_id(line) {
                        models.push(line.to_string());
                    }
                }
            }
            ModelSource::JsonFile { path, extract } => {
                let Some(p) = expand_path(path) else { continue };
                let Ok(text) = std::fs::read_to_string(p) else { continue };
                // JSONC-tolerant: strip comments/trailing commas before parse.
                let Ok(v) = serde_json::from_str::<serde_json::Value>(&jsonc_to_json(&text))
                else {
                    continue;
                };
                extract_json_models(&v, extract, &mut models);
            }
            ModelSource::TomlFile { path } => {
                let Some(p) = expand_path(path) else { continue };
                let Ok(text) = std::fs::read_to_string(p) else { continue };
                let Ok(v) = toml::from_str::<serde_json::Value>(&text) else { continue };
                collect_toml_models(&v, &mut models);
            }
            ModelSource::Manual => {}
        }
    }
    models.sort();
    models.dedup();
    models
}

/// Minimal JSONC → JSON: strip `//` and `/* */` comments (string-aware) and
/// trailing commas so user config files parse with serde_json.
fn jsonc_to_json(src: &str) -> String {
    let b = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0usize;
    let mut in_string = false;
    while i < b.len() {
        let c = b[i];
        if in_string {
            if c == b'\\' {
                let end = (i + 2).min(b.len());
                out.push_str(&src[i..end]);
                i = end;
                continue;
            }
            if c == b'"' {
                in_string = false;
            }
            out.push(c as char);
            i += 1;
            continue;
        }
        match c {
            b'"' => {
                in_string = true;
                out.push('"');
                i += 1;
            }
            b'/' if i + 1 < b.len() && b[i + 1] == b'/' => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if i + 1 < b.len() && b[i + 1] == b'*' => {
                i += 2;
                while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                    i += 1;
                }
                i += 2;
            }
            _ => {
                out.push(c as char);
                i += 1;
            }
        }
    }
    // Drop trailing commas.
    let mut clean = String::with_capacity(out.len());
    let mut j = 0usize;
    while j < out.len() {
        let c = out.as_bytes()[j];
        if c == b',' {
            let mut k = j + 1;
            while k < out.len() && matches!(out.as_bytes()[k], b' ' | b'\t' | b'\r' | b'\n') {
                k += 1;
            }
            if k < out.len() && matches!(out.as_bytes()[k], b'}' | b']') {
                j += 1;
                continue;
            }
        }
        clean.push(c as char);
        j += 1;
    }
    clean
}

fn extract_json_models(v: &serde_json::Value, extract: &JsonExtract, out: &mut Vec<String>) {
    match extract {
        JsonExtract::Str(key) => {
            if let Some(s) = v.get(key).and_then(|x| x.as_str()) {
                if looks_like_model_id(s) {
                    out.push(s.to_string());
                }
            }
        }
        JsonExtract::List(key) => {
            if let Some(arr) = v.get(key).and_then(|x| x.as_array()) {
                for item in arr {
                    match item {
                        serde_json::Value::String(s) => out.push(s.clone()),
                        other => collect_json_model_ids(other, out),
                    }
                }
            }
        }
        JsonExtract::ProviderModels(roots) => {
            // Walk to each root key, then collect keys of any nested
            // "models" object plus string values inside it.
            let mut frontier: Vec<&serde_json::Value> = vec![v];
            let mut matched_root = roots.is_empty();
            for key in *roots {
                let mut next = Vec::new();
                for node in &frontier {
                    if let Some(child) = node.get(*key) {
                        next.push(child);
                    }
                }
                if !next.is_empty() {
                    matched_root = true;
                }
                frontier = next;
            }
            if !matched_root {
                return;
            }
            for node in frontier {
                collect_models_object_keys(node, out);
            }
        }
        JsonExtract::DeepStrings(keys) => {
            for key in *keys {
                collect_strings_under_key(v, key, out);
            }
        }
    }
}

/// Recursively collect object KEYS of members named `models` (provider model
/// tables) and any string values inside them.
fn collect_models_object_keys(v: &serde_json::Value, out: &mut Vec<String>) {
    match v {
        serde_json::Value::Object(o) => {
            for (k, val) in o {
                if k == "models" {
                    if let serde_json::Value::Object(models) = val {
                        for id in models.keys() {
                            out.push(id.clone());
                        }
                    } else if let serde_json::Value::Array(arr) = val {
                        for item in arr {
                            collect_json_model_ids(item, out);
                        }
                    }
                } else {
                    collect_models_object_keys(val, out);
                }
            }
        }
        serde_json::Value::Array(a) => a.iter().for_each(|x| collect_models_object_keys(x, out)),
        _ => {}
    }
}

/// Recursively collect string values under any object key in `keys`.
fn collect_strings_under_key(v: &serde_json::Value, key: &str, out: &mut Vec<String>) {
    match v {
        serde_json::Value::Object(o) => {
            for (k, val) in o {
                if k == key {
                    collect_json_model_ids(val, out);
                } else {
                    collect_strings_under_key(val, key, out);
                }
            }
        }
        serde_json::Value::Array(a) => a.iter().for_each(|x| collect_strings_under_key(x, key, out)),
        _ => {}
    }
}

/// TOML: collect every string value under a key named `model` (covers
/// `model = "..."` and `[profiles.x] model = "..."`).
fn collect_toml_models(v: &serde_json::Value, out: &mut Vec<String>) {
    match v {
        serde_json::Value::Object(o) => {
            for (k, val) in o {
                if k == "model" {
                    if let Some(s) = val.as_str() {
                        if looks_like_model_id(s) {
                            out.push(s.to_string());
                        }
                    }
                } else {
                    collect_toml_models(val, out);
                }
            }
        }
        serde_json::Value::Array(a) => a.iter().for_each(|x| collect_toml_models(x, out)),
        _ => {}
    }
}

fn looks_like_model_id(line: &str) -> bool {
    // Filter out table headers/help noise: no spaces, no trailing colon.
    !line.contains(' ') && line.len() > 2 && !line.ends_with(':') && !line.contains(':')
}

fn collect_json_model_ids(v: &serde_json::Value, out: &mut Vec<String>) {
    match v {
        serde_json::Value::String(s) => {
            if looks_like_model_id(s) {
                out.push(s.clone());
            }
        }
        serde_json::Value::Array(a) => a.iter().for_each(|x| collect_json_model_ids(x, out)),
        serde_json::Value::Object(o) => {
            for (k, val) in o {
                if k == "id" || k == "model" || k == "name" || k == "modelId" || k == "defaultModel" {
                    collect_json_model_ids(val, out);
                } else if val.is_object() || val.is_array() {
                    collect_json_model_ids(val, out);
                }
            }
        }
        _ => {}
    }
}

/// Compose the final commandline from adapter flag syntax + user choices.
pub fn compose_commandline(adapter: &Adapter, model: Option<&str>, effort: Option<&str>) -> String {
    let mut parts: Vec<String> = vec![adapter.launch.to_string()];
    if let Some(m) = model.filter(|s| !s.trim().is_empty()) {
        let mut m = m.trim().to_string();
        if let EffortStyle::VariantSuffix(levels) = &adapter.effort {
            if let Some(e) = effort.filter(|s| !s.trim().is_empty()) {
                if levels.contains(&e.trim()) {
                    m = format!("{m}#{}", e.trim());
                }
            }
        } else if let (EffortStyle::Flag(flag, _), Some(e)) = (&adapter.effort, effort) {
            if !e.trim().is_empty() {
                parts.push(flag.to_string());
                parts.push(e.trim().to_string());
            }
        }
        if let Some(flag) = adapter.model_flag {
            parts.push(flag.to_string());
            parts.push(m);
        }
    } else if let (EffortStyle::Flag(flag, _), Some(e)) = (&adapter.effort, effort) {
        if !e.trim().is_empty() {
            parts.push(flag.to_string());
            parts.push(e.trim().to_string());
        }
    }
    parts.join(" ")
}

/// Build a user config entry from an adapter + selections.
pub fn entry_from(adapter: &Adapter, model: Option<&str>, effort: Option<&str>) -> AgentEntry {
    AgentEntry {
        id: adapter.id.to_string(),
        name: adapter.name.to_string(),
        commandline: compose_commandline(adapter, model, effort),
        open_here_input: format!("{}\r", compose_commandline(adapter, model, effort)),
        model: model.map(|s| s.to_string()),
        entry_order: 0,
        enabled: true,
        manual: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adapter(id: &str) -> &'static Adapter {
        ADAPTERS.iter().find(|a| a.id == id).unwrap()
    }

    #[test]
    fn compose_claude_with_model_and_effort() {
        let a = adapter("claude-code");
        assert_eq!(compose_commandline(a, Some("claude-sonnet-5"), Some("high")), "claude --effort high --model claude-sonnet-5");
        assert_eq!(compose_commandline(a, None, None), "claude");
    }

    #[test]
    fn compose_opencode_variant_suffix() {
        let a = adapter("opencode");
        assert_eq!(
            compose_commandline(a, Some("openai/gpt-6.1-sol"), Some("high")),
            "opencode -m openai/gpt-6.1-sol#high"
        );
    }

    #[test]
    fn compose_crush_has_no_model_flag() {
        let a = adapter("crush");
        assert_eq!(compose_commandline(a, Some("x/y"), None), "crush");
    }

    #[test]
    fn compose_effort_without_model() {
        let a = adapter("claude-code");
        assert_eq!(compose_commandline(a, None, Some("low")), "claude --effort low");
    }

    #[test]
    fn entry_from_ends_with_cr() {
        let e = entry_from(adapter("claude-code"), None, None);
        assert!(e.open_here_input.ends_with('\r'));
    }

    #[test]
    fn expand_path_resolves_env() {
        // SAFETY: test-only, single-threaded, unique variable name.
        unsafe { std::env::set_var("AP_TEST_VAR", "hello") };
        assert_eq!(expand_path("%AP_TEST_VAR%\\x.json"), Some(std::path::PathBuf::from("hello\\x.json")));
        assert_eq!(expand_path("C:\\plain.json"), Some(std::path::PathBuf::from("C:\\plain.json")));
    }

    #[test]
    fn looks_like_model_id_filters_noise() {
        assert!(looks_like_model_id("openai/gpt-6.1-sol"));
        assert!(looks_like_model_id("claude-sonnet-5"));
        assert!(!looks_like_model_id("Usage: opencode models"));
        assert!(!looks_like_model_id(""));
    }

    #[test]
    fn jsonc_to_json_strips_comments_and_trailing_commas() {
        let src = "{\n  // comment\n  \"model\": \"x-1\", // trailing comment\n  \"list\": [\"a\", \"b\",],\n  /* block\n     comment */\n}";
        let v: serde_json::Value = serde_json::from_str(&jsonc_to_json(src)).unwrap();
        assert_eq!(v["model"], "x-1");
        assert_eq!(v["list"][1], "b");
    }

    #[test]
    fn extract_str_takes_single_value() {
        let v = serde_json::json!({"model": "claude-sonnet-5", "other": "nope nope"});
        let mut out = Vec::new();
        extract_json_models(&v, &JsonExtract::Str("model"), &mut out);
        assert_eq!(out, vec!["claude-sonnet-5"]);
    }

    #[test]
    fn extract_list_handles_strings_and_objects() {
        let v = serde_json::json!({"customModels": ["a-1", {"model": "b-2"}, {"id": "c-3"}]});
        let mut out = Vec::new();
        extract_json_models(&v, &JsonExtract::List("customModels"), &mut out);
        assert!(out.contains(&"a-1".to_string()));
        assert!(out.contains(&"b-2".to_string()));
        assert!(out.contains(&"c-3".to_string()));
    }

    #[test]
    fn extract_provider_models_collects_keys() {
        let v = serde_json::json!({
            "providers": {
                "openai": {"models": {"gpt-6.1-sol": {}, "gpt-5": {}}},
                "anthropic": {"models": ["claude-sonnet-5"]}
            }
        });
        let mut out = Vec::new();
        extract_json_models(&v, &JsonExtract::ProviderModels(&["providers"]), &mut out);
        assert!(out.contains(&"gpt-6.1-sol".to_string()));
        assert!(out.contains(&"claude-sonnet-5".to_string()));
    }

    #[test]
    fn extract_deep_strings_walks_nested_keys() {
        let v = serde_json::json!({"model": {"name": "gemini-3-pro"}, "x": {"model": "alias-pro"}});
        let mut out = Vec::new();
        extract_json_models(&v, &JsonExtract::DeepStrings(&["model"]), &mut out);
        assert!(out.contains(&"gemini-3-pro".to_string()));
        assert!(out.contains(&"alias-pro".to_string()));
    }

    #[test]
    fn toml_models_collected() {
        let src = "model = \"gpt-6.1-sol\"\n\n[profiles.fast]\nmodel = \"gpt-5-mini\"\n";
        let v: serde_json::Value = toml::from_str(src).unwrap();
        let mut out = Vec::new();
        collect_toml_models(&v, &mut out);
        assert!(out.contains(&"gpt-6.1-sol".to_string()));
        assert!(out.contains(&"gpt-5-mini".to_string()));
    }
}
