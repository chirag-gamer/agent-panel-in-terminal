//! JSONC-preserving surgical editor for Windows Terminal `settings.json`.
//!
//! Modifies ONLY `AgentPanel.*`-tagged blocks:
//! - `actions` items with `"id": "AgentPanel.<id>.OpenHere"`
//! - `profiles.list` items named `AgentPanel.*`
//! - `newTabMenu` items referencing those actions/profiles
//!
//! Everything else — including comments and trailing commas — is preserved
//! byte-for-byte. Phase 0 verified: profiles must be `hidden: false` and are
//! then excluded from `remainingProfiles`; Windows Terminal live-reloads the
//! file without restart.

use serde_json::{Value, json};
use uuid::Uuid;

/// Fixed namespace for deterministic AgentPanel GUIDs (v5, SHA-1 based).
const GUID_NAMESPACE: Uuid = Uuid::from_u128(0x6f4c3a2b_9d1e_4f8a_b7c6_5e2d1a0f9e8d);

/// One fully-compiled agent, ready to be written into settings.json.
#[derive(Debug, Clone)]
pub struct CompiledAgent {
    pub id: String,
    pub name: String,
    /// Full new-tab commandline including any model/effort flags.
    pub commandline: String,
    /// `sendInput` text for "Open here"; must end with `\r`.
    pub open_here_input: String,
}

type Edit = (usize, usize, String);

// ---------------------------------------------------------------------------
// JSONC scanning primitives
// ---------------------------------------------------------------------------

#[inline]
fn skip_trivia(b: &[u8], mut i: usize) -> usize {
    while i < b.len() {
        match b[i] {
            b' ' | b'\t' | b'\r' | b'\n' => i += 1,
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
                i = (i + 2).min(b.len());
            }
            _ => break,
        }
    }
    i
}

/// Advance past a JSON string starting at `i` (which must be `"`).
/// Returns position after the closing quote, or `i` on failure.
fn skip_string(b: &[u8], i: usize) -> usize {
    if i >= b.len() || b[i] != b'"' {
        return i;
    }
    let mut j = i + 1;
    while j < b.len() {
        match b[j] {
            b'\\' => j += 2,
            b'"' => return j + 1,
            _ => j += 1,
        }
    }
    i
}

/// Advance past any JSON value starting at `i`. Returns end position.
fn scan_value(b: &[u8], i: usize) -> usize {
    let i = skip_trivia(b, i);
    match b.get(i) {
        Some(b'"') => skip_string(b, i),
        Some(b'{') | Some(b'[') => {
            let (open, close) = if b[i] == b'{' { (b'{', b'}') } else { (b'[', b']') };
            let mut j = i + 1;
            let mut depth = 1usize;
            while j < b.len() && depth > 0 {
                match b[j] {
                    _ if b[j] == open => {
                        depth += 1;
                        j += 1;
                    }
                    _ if b[j] == close => {
                        depth -= 1;
                        j += 1;
                    }
                    b'"' => j = skip_string(b, j),
                    _ => j += 1,
                }
            }
            j
        }
        Some(_) => {
            let mut j = i;
            while j < b.len()
                && !matches!(b[j], b',' | b'}' | b']' | b' ' | b'\t' | b'\r' | b'\n')
            {
                j += 1;
            }
            j
        }
        None => i,
    }
}

/// Span of a contiguous source region (byte offsets).
#[derive(Debug, Clone, Copy)]
struct Span {
    start: usize,
    end: usize,
}

/// Find member `key` inside the object spanning `obj` (value span, braces
/// included). Returns the member's VALUE span.
fn find_member(b: &[u8], obj: Span, key: &str) -> Option<Span> {
    let mut i = skip_trivia(b, obj.start + 1);
    loop {
        if i >= obj.end || b[i] == b'}' {
            return None;
        }
        if b[i] == b'"' {
            let key_end = skip_string(b, i);
            let member_key = &b[i + 1..key_end.saturating_sub(1)];
            let j = skip_trivia(b, key_end);
            if j < b.len() && b[j] == b':' {
                let vstart = skip_trivia(b, j + 1);
                let vend = scan_value(b, vstart);
                if member_key == key.as_bytes() {
                    return Some(Span { start: vstart, end: vend });
                }
                i = skip_trivia(b, vend);
                if i < b.len() && b[i] == b',' {
                    i = skip_trivia(b, i + 1);
                    continue;
                }
                continue;
            }
        }
        // Unexpected structure; bail out safely.
        return None;
    }
}

/// Split an array value span into top-level item spans (commas excluded).
fn array_items(b: &[u8], arr: Span) -> Vec<Span> {
    let mut items = Vec::new();
    let mut i = skip_trivia(b, arr.start + 1);
    loop {
        if i >= arr.end || b[i] == b']' {
            break;
        }
        let start = i;
        let end = scan_value(b, i);
        items.push(Span { start, end });
        i = skip_trivia(b, end);
        if i < b.len() && b[i] == b',' {
            i = skip_trivia(b, i + 1);
        }
    }
    items
}

/// Indentation (as a &str of spaces/tabs) of the line holding `pos`.
fn indent_at(b: &[u8], pos: usize) -> String {
    let start = b[..pos].iter().rposition(|&c| c == b'\n').map_or(0, |p| p + 1);
    let line_ws_end = b[start..pos]
        .iter()
        .take_while(|&&c| c == b' ' || c == b'\t')
        .count();
    String::from_utf8_lossy(&b[start..start + line_ws_end]).into_owned()
}

/// Build ONE edit that replaces the whole array span with `ordered_texts`.
///
/// Kept items are preserved verbatim (internal comments/indentation intact);
/// separators are re-normalized. Comments BETWEEN items inside this array are
/// not preserved (documented tradeoff; our arrays are machine-managed blocks
/// or untouched user arrays with no removals).
fn rebuild_array(b: &[u8], arr: Span, ordered_texts: &[String]) -> Edit {
    let any_multiline = ordered_texts.iter().any(|t| t.contains('\n'));
    let inline = !any_multiline && !b[arr.start..arr.end].contains(&b'\n');
    if ordered_texts.is_empty() {
        return (arr.start, arr.end, "[]".into());
    }
    if inline {
        return (arr.start, arr.end, format!("[{}]", ordered_texts.join(", ")));
    }
    let base = indent_at(b, arr.end - 1);
    let ind = format!("{base}    ");
    let body = ordered_texts.join(&format!(",\n{ind}"));
    (arr.start, arr.end, format!("[\n{ind}{body}\n{base}]"))
}

/// Build the edit that inserts a member `"key": <rendered>` into an object
/// span (braces included). Appends after the last member.
fn insert_into_object(b: &[u8], obj: Span, key: &str, rendered: &str) -> Option<Edit> {
    let close = obj.end - 1; // index of '}'
    let mut j = close;
    loop {
        if j == obj.start {
            break;
        }
        j -= 1;
        match b[j] {
            b' ' | b'\t' | b'\r' | b'\n' => continue,
            _ => {}
        }
        let base = indent_at(b, close);
        let ind = format!("{base}    ");
        let text = if b[j] == b'{' {
            format!("\n{ind}\"{key}\": {rendered}\n{base}")
        } else {
            format!(",\n{ind}\"{key}\": {rendered}")
        };
        return Some((close, close, text));
    }
    None
}

fn apply_edits(source: &str, mut edits: Vec<Edit>) -> String {
    edits.sort_by_key(|(s, e, _)| (*s, std::cmp::Reverse(*e)));
    let mut out = String::with_capacity(source.len());
    let mut last = 0usize;
    for (s, e, text) in edits {
        assert!(s >= last, "overlapping edits");
        out.push_str(&source[last..s]);
        out.push_str(&text);
        last = e;
    }
    out.push_str(&source[last..]);
    out
}

// ---------------------------------------------------------------------------
// Rendering helpers
// ---------------------------------------------------------------------------

/// Render a serde_json value with 4-space indentation (WT style).
fn render(v: &Value, indent: &str) -> String {
    let ind4 = format!("{indent}    ");
    match v {
        Value::Object(map) => {
            if map.is_empty() {
                return "{}".into();
            }
            let parts: Vec<String> = map
                .iter()
                .map(|(k, val)| format!("{ind4}{}: {}", json_string(k), render(val, &ind4)))
                .collect();
            format!("{{\n{}\n{indent}}}", parts.join(",\n"))
        }
        Value::Array(items) => {
            if items.is_empty() {
                return "[]".into();
            }
            let parts: Vec<String> = items.iter().map(|val| format!("{ind4}{}", render(val, &ind4))).collect();
            format!("[\n{}\n{indent}]", parts.join(",\n"))
        }
        _ => serde_json::to_string(v).expect("value serialization"),
    }
}

fn json_string(s: &str) -> String {
    serde_json::to_string(s).expect("string serialization")
}

fn braced_guid(seed: &str) -> String {
    let u = Uuid::new_v5(&GUID_NAMESPACE, seed.as_bytes());
    format!("{{{}}}", u.to_string().to_uppercase())
}

// ---------------------------------------------------------------------------
// Entry builders
// ---------------------------------------------------------------------------

fn open_here_action(agent: &CompiledAgent) -> Value {
    json!({
        "id": format!("AgentPanel.{}.OpenHere", agent.id),
        "name": "Open here",
        "command": {
            "action": "sendInput",
            "input": agent.open_here_input
        }
    })
}

fn new_tab_profile(agent: &CompiledAgent) -> Value {
    json!({
        "guid": braced_guid(&format!("AgentPanel.{}.NewTab", agent.id)),
        "name": "Open in new tab",
        "tabTitle": agent.name,
        "commandline": agent.commandline,
        "hidden": false
    })
}

fn customize_profile(customize_commandline: &str) -> Value {
    json!({
        "guid": braced_guid("AgentPanel.Customize"),
        "name": "Agent settings",
        "commandline": customize_commandline,
        "hidden": false,
        "closeOnExit": "always"
    })
}

fn agent_folder(agent: &CompiledAgent) -> Value {
    json!({
        "type": "folder",
        "name": agent.name,
        "entries": [
            {
                "type": "action",
                "id": format!("AgentPanel.{}.OpenHere", agent.id)
            },
            {
                "type": "profile",
                "profile": braced_guid(&format!("AgentPanel.{}.NewTab", agent.id))
            }
        ]
    })
}

fn customize_menu_entry() -> Value {
    json!({
        "type": "profile",
        "profile": braced_guid("AgentPanel.Customize")
    })
}

fn is_ours(text: &str, guids: &[String]) -> bool {
    text.contains("\"AgentPanel.") || guids.iter().any(|g| text.contains(g))
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Rewrite `source` so the three tagged blocks contain exactly the given
/// agents plus the Customize entry. Preserves all unrelated content,
/// comments, and trailing commas.
pub fn apply(
    source: &str,
    agents: &[CompiledAgent],
    customize_commandline: &str,
) -> Result<String, String> {
    for a in agents {
        if !a.open_here_input.ends_with('\r') {
            return Err(format!("agent {}: open_here_input must end with \\r", a.id));
        }
    }
    let b = source.as_bytes();
    let root = Span { start: 0, end: b.len() };
    let customize_guid = braced_guid("AgentPanel.Customize");
    let agent_guids: Vec<String> = agents
        .iter()
        .map(|a| braced_guid(&format!("AgentPanel.{}.NewTab", a.id)))
        .collect();
    let mut all_guids = agent_guids.clone();
    all_guids.push(customize_guid.clone());

    let mut edits: Vec<Edit> = Vec::new();

    // ---- actions -----------------------------------------------------------
    match find_member(b, root, "actions") {
        Some(span) if b[span.start] == b'[' => {
            let base = indent_at(b, span.end - 1);
            let ind = format!("{base}    ");
            let items = array_items(b, span);
            let mut ordered: Vec<String> = items
                .iter()
                .copied()
                .filter(|s| !is_ours(&source[s.start..s.end], &all_guids))
                .map(|s| source[s.start..s.end].to_string())
                .collect();
            ordered.extend(agents.iter().map(|a| render(&open_here_action(a), &ind)));
            edits.push(rebuild_array(b, span, &ordered));
        }
        _ => {
            let arr = render(&Value::Array(
                agents.iter().map(open_here_action).collect(),
            ), "    ");
            edits.push(
                insert_into_object(b, root, "actions", &arr)
                    .ok_or("cannot insert missing \"actions\" member")?,
            );
        }
    }

    // ---- profiles.list -----------------------------------------------------
    let profiles_span: Option<Span> = match find_member(b, root, "profiles") {
        Some(span) if b[span.start] == b'{' => Some(span),
        _ => {
            // Create profiles object with list containing our items.
            let inner = render(
                &json!({ "list": agents.iter().map(new_tab_profile).chain(std::iter::once(customize_profile(customize_commandline))).collect::<Vec<_>>() }),
                "    ",
            );
            edits.push(
                insert_into_object(b, root, "profiles", &inner)
                    .ok_or("cannot insert missing \"profiles\" member")?,
            );
            None
        }
    };
    if let Some(pspan) = profiles_span {
        match find_member(b, pspan, "list") {
            Some(lspan) if b[lspan.start] == b'[' => {
                let base = indent_at(b, lspan.end - 1);
                let ind = format!("{base}    ");
                let items = array_items(b, lspan);
                let mut ordered: Vec<String> = items
                    .iter()
                    .copied()
                    .filter(|s| !is_ours(&source[s.start..s.end], &all_guids))
                    .map(|s| source[s.start..s.end].to_string())
                    .collect();
                ordered.extend(agents.iter().map(|a| render(&new_tab_profile(a), &ind)));
                ordered.push(render(&customize_profile(customize_commandline), &ind));
                edits.push(rebuild_array(b, lspan, &ordered));
            }
            _ => {
                let arr = render(
                    &Value::Array(
                        agents.iter().map(new_tab_profile).chain(std::iter::once(customize_profile(customize_commandline))).collect(),
                    ),
                    "        ",
                );
                edits.push(
                    insert_into_object(b, pspan, "list", &arr)
                        .ok_or("cannot insert missing \"profiles.list\" member")?,
                );
            }
        }
    }

    // ---- newTabMenu --------------------------------------------------------
    match find_member(b, root, "newTabMenu") {
        Some(span) if b[span.start] == b'[' => {
            let base = indent_at(b, span.end - 1);
            let ind = format!("{base}    ");
            let items = array_items(b, span);
            // Ordered rebuild: keep user entries, drop ours, re-insert our
            // blocks right after remainingProfiles (or at the end).
            let mut ordered: Vec<String> = Vec::new();
            let mut placed = agents.is_empty();
            let mut ours_inserted = agents.is_empty();
            for s in items.iter().copied() {
                if is_ours(&source[s.start..s.end], &all_guids) {
                    continue;
                }
                let text = source[s.start..s.end].to_string();
                let is_rp = !placed && text.contains("remainingProfiles");
                let is_bare_separator = text.contains("\"separator\"");
                if placed && !ours_inserted && !is_bare_separator {
                    // Our block must land before this item; the separator we
                    // would have added is still missing.
                    ordered.push(render(&json!({ "type": "separator" }), &ind));
                    ordered.extend(agents.iter().map(|a| render(&agent_folder(a), &ind)));
                    ordered.push(render(&customize_menu_entry(), &ind));
                    ours_inserted = true;
                }
                ordered.push(text);
                if is_rp {
                    placed = true;
                }
                if placed && !ours_inserted && is_bare_separator {
                    // Reuse the separator directly after remainingProfiles.
                    ordered.extend(agents.iter().map(|a| render(&agent_folder(a), &ind)));
                    ordered.push(render(&customize_menu_entry(), &ind));
                    ours_inserted = true;
                }
            }
            if !ours_inserted {
                let last_is_sep = ordered.last().map(|t| t.contains("\"separator\"")).unwrap_or(false);
                if !last_is_sep {
                    ordered.push(render(&json!({ "type": "separator" }), &ind));
                }
                ordered.extend(agents.iter().map(|a| render(&agent_folder(a), &ind)));
                ordered.push(render(&customize_menu_entry(), &ind));
            }
            edits.push(rebuild_array(b, span, &ordered));
        }
        _ => {
            let mut vals: Vec<Value> = vec![json!({ "type": "remainingProfiles" })];
            vals.push(json!({ "type": "separator" }));
            vals.extend(agents.iter().map(agent_folder));
            vals.push(customize_menu_entry());
            let arr = render(&Value::Array(vals), "    ");
            edits.push(
                insert_into_object(b, root, "newTabMenu", &arr)
                    .ok_or("cannot insert missing \"newTabMenu\" member")?,
            );
        }
    }

    Ok(apply_edits(source, edits))
}

/// Names that identify our generated profiles when GUIDs alone are not
/// enough (orphan cleanup, e.g. after config loss).
const OUR_PROFILE_NAMES: [&str; 2] = ["Open in new tab", "Agent settings"];

/// Remove every `AgentPanel.*` block (used by a future "reset" action).
///
/// Tagging model: GUIDs are deterministic (v5 from our namespace), action
/// ids keep the `AgentPanel.` prefix, and profile NAMES are user-facing
/// ("Open in new tab", "Agent settings"). For orphan cleanup we fall back to
/// exact profile-name matching.
pub fn strip(source: &str) -> Result<String, String> {
    let b = source.as_bytes();
    let root = Span { start: 0, end: b.len() };
    let mut edits: Vec<Edit> = Vec::new();

    // Collect guids of our profiles (by name) so menu items referencing them
    // can be removed even without config knowledge.
    let mut our_guids: Vec<String> = Vec::new();
    if let Some(pspan) = find_member(b, root, "profiles") {
        if let Some(lspan) = find_member(b, pspan, "list") {
            for s in array_items(b, lspan) {
                let text = &source[s.start..s.end];
                let ours_name = OUR_PROFILE_NAMES.iter().any(|n| text.contains(&format!("\"name\": \"{n}\"")));
                let legacy = text.contains("\"AgentPanel.");
                if ours_name || legacy {
                    // Extract guid string if present.
                    if let Some(start) = text.find("\"guid\": \"") {
                        let rest = &text[start + 9..];
                        if let Some(end) = rest.find('"') {
                            our_guids.push(rest[..end].to_string());
                        }
                    }
                }
            }
            let ordered: Vec<String> = array_items(b, lspan)
                .into_iter()
                .filter(|s| !is_ours(&source[s.start..s.end], &our_guids))
                .map(|s| source[s.start..s.end].to_string())
                .collect();
            edits.push(rebuild_array(b, lspan, &ordered));
        }
    }
    let all_ours = |s: &Span| -> bool {
        let text = &source[s.start..s.end];
        text.contains("\"AgentPanel.") || our_guids.iter().any(|g| text.contains(g))
    };
    for key in ["actions", "newTabMenu"] {
        if let Some(span) = find_member(b, root, key) {
            if b[span.start] == b'[' {
                let ordered: Vec<String> = array_items(b, span)
                    .into_iter()
                    .filter(|s| !all_ours(s))
                    .map(|s| source[s.start..s.end].to_string())
                    .collect();
                edits.push(rebuild_array(b, span, &ordered));
            }
        }
    }
    Ok(apply_edits(source, edits))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Strict-parse helper for JSONC: strips comments and trailing commas
    /// (string-aware) so tests can validate structure of preserved output.
    fn parse_jsonc(src: &str) -> Value {
        let b = src.as_bytes();
        let mut out = String::with_capacity(src.len());
        let mut i = 0usize;
        let mut in_string = false;
        while i < b.len() {
            let c = b[i];
            if in_string {
                if c == b'\\' {
                    out.push_str(&src[i..(i + 2).min(b.len())]);
                    i += 2;
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
        // Strip trailing commas: `,` followed only by ws/comments then } or ].
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
                    j += 1; // drop the comma
                    continue;
                }
            }
            clean.push(c as char);
            j += 1;
        }
        serde_json::from_str(&clean).expect("lenient JSONC parse")
    }

    const BASELINE: &str = r#"{
    "$help": "https://aka.ms/terminal-documentation",
    "actions": [],
    "defaultProfile": "{61c54bbd-c2c6-5271-96e7-009a87ff44bf}",
    // a user comment that must survive
    "newTabMenu":
    [
        {
            "type": "remainingProfiles"
        },
    ],
    "profiles":
    {
        "defaults": {},
        "list":
        [
            {
                "commandline": "%SystemRoot%\\System32\\cmd.exe",
                "guid": "{0caa0dad-35be-5f56-a8ff-afceeeaa6101}",
                "hidden": false,
                "name": "Command Prompt"
            }
        ]
    },
    "schemes": []
}"#;

    fn demo_agents() -> Vec<CompiledAgent> {
        vec![CompiledAgent {
            id: "claude-code".into(),
            name: "Claude Code".into(),
            commandline: "claude".into(),
            open_here_input: "claude\r".into(),
        }]
    }

    #[test]
    fn applies_to_baseline() {
        let out = apply(BASELINE, &demo_agents(), "cmd.exe /c start \"\" \"x.exe\"").unwrap();
        assert!(out.contains("\"AgentPanel.claude-code.OpenHere\""));
        assert!(out.contains("\"name\": \"Open in new tab\""));
        assert!(out.contains("\"name\": \"Agent settings\""));
        assert!(out.contains("\"tabTitle\": \"Claude Code\""));
        assert!(out.contains("sendInput"));
        assert!(out.contains("\\r")); // trailing CR escape preserved in output
        // untouched user content survives
        assert!(out.contains("// a user comment that must survive"));
        assert!(out.contains("Command Prompt"));
        assert!(out.contains("remainingProfiles"));
        // valid JSON after applying
        let v: Value = parse_jsonc(&out);
        // 1 user profile + agent new-tab + customize
        assert_eq!(v["profiles"]["list"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn idempotent_double_apply() {
        let once = apply(BASELINE, &demo_agents(), "cmd.exe /c start \"\" \"x.exe\"").unwrap();
        let twice = apply(&once, &demo_agents(), "cmd.exe /c start \"\" \"x.exe\"").unwrap();
        assert_eq!(once, twice, "second apply must produce identical output");
    }

    #[test]
    fn replaces_previous_blocks() {
        let with_agent = apply(BASELINE, &demo_agents(), "cmd.exe /c start \"\" \"x.exe\"").unwrap();
        let mut new_agents = demo_agents();
        new_agents.push(CompiledAgent {
            id: "opencode".into(),
            name: "OpenCode".into(),
            commandline: "opencode".into(),
            open_here_input: "opencode\r".into(),
        });
        let replaced = apply(&with_agent, &new_agents, "cmd.exe /c start \"\" \"x.exe\"").unwrap();
        let v: Value = parse_jsonc(&replaced);
        let actions = v["actions"].as_array().unwrap();
        assert_eq!(actions.len(), 2, "old action must be replaced, not duplicated");
        assert!(replaced.contains("AgentPanel.opencode.OpenHere"));
        assert!(replaced.contains("AgentPanel.claude-code.OpenHere"));
        // 1 user profile + 2 agent new-tab + customize
        assert_eq!(v["profiles"]["list"].as_array().unwrap().len(), 4);
    }

    #[test]
    fn strip_removes_all_agent_blocks() {
        let with_agent = apply(BASELINE, &demo_agents(), "cmd.exe /c start \"\" \"x.exe\"").unwrap();
        let stripped = strip(&with_agent).unwrap();
        let v: Value = parse_jsonc(&stripped);
        assert_eq!(v["actions"].as_array().unwrap().len(), 0);
        assert_eq!(v["profiles"]["list"].as_array().unwrap().len(), 1);
        let menu = v["newTabMenu"].as_array().unwrap();
        assert!(menu.iter().all(|e| e["type"] != "folder"));
        assert!(!stripped.contains("AgentPanel."));
        assert!(stripped.contains("// a user comment that must survive"));
    }

    #[test]
    fn creates_missing_keys() {
        let src = "{\n    \"schemes\": []\n}";
        let out = apply(src, &demo_agents(), "cmd.exe /c start \"\" \"x.exe\"").unwrap();
        let v: Value = parse_jsonc(&out);
        assert!(v["actions"].is_array());
        assert!(v["profiles"]["list"].is_array());
        assert!(v["newTabMenu"].is_array());
        assert_eq!(v["newTabMenu"][0]["type"], "remainingProfiles");
    }

    #[test]
    fn rejects_input_without_trailing_cr() {
        let agents = vec![CompiledAgent {
            id: "x".into(),
            name: "X".into(),
            commandline: "x".into(),
            open_here_input: "x".into(),
        }];
        assert!(apply(BASELINE, &agents, "x").is_err());
    }

    #[test]
    fn migrates_legacy_named_profiles() {
        // Simulate a file deployed by the old version (AgentPanel.* names).
        let legacy = r#"{
    "actions": [],
    "newTabMenu": [ { "type": "remainingProfiles" } ],
    "profiles": {
        "list": [
            {
                "guid": "{11111111-1111-1111-1111-111111111111}",
                "name": "AgentPanel.old.NewTab",
                "commandline": "old",
                "hidden": false
            }
        ]
    }
}"#;
        let out = apply(legacy, &demo_agents(), "cmd.exe /c start \"\" \"x.exe\"").unwrap();
        let v = parse_jsonc(&out);
        let list = v["profiles"]["list"].as_array().unwrap();
        assert!(list.iter().all(|p| p["name"] != "AgentPanel.old.NewTab"), "legacy profile replaced");
        assert!(list.iter().any(|p| p["name"] == "Open in new tab"));
        assert!(list.iter().any(|p| p["name"] == "Agent settings"));
    }

    #[test]
    fn deterministic_guids() {
        assert_eq!(braced_guid("AgentPanel.Customize"), braced_guid("AgentPanel.Customize"));
        assert_ne!(
            braced_guid("AgentPanel.a.NewTab"),
            braced_guid("AgentPanel.b.NewTab")
        );
    }
}
