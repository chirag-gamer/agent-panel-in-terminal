//! Headless CLI (`agent-panel-cli.exe`): --check, --apply, --strip.
//! Console-subsystem binary so output, blocking and exit codes work normally
//! (D-026). The GUI exe is GUI-subsystem and never attaches a console.

use agent_panel_lib::{config::PanelConfig, detect, settings, store};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("--check") => headless_check(),
        Some("--apply") => headless_apply(),
        Some("--strip") => headless_strip(),
        _ => {
            eprintln!("Usage: agent-panel-cli [--check|--apply|--strip]");
            std::process::exit(2);
        }
    }
}

fn headless_check() {
    let detections = detect::detect_all();
    println!("Installed agent CLIs:");
    for d in &detections {
        if d.installed {
            println!("  {:12} {} ({})", d.adapter.id, d.version, d.adapter.name);
        }
    }
    let missing: Vec<&str> = detections
        .iter()
        .filter(|d| !d.installed)
        .map(|d| d.adapter.id)
        .collect();
    println!("Not installed: {}", missing.join(", "));
    println!("\nDiscovered models (tier-1/tier-2 sources):");
    for d in detections.iter().filter(|d| d.installed) {
        let models = detect::discover_models(d.adapter);
        if models.is_empty() {
            println!("  {:12} (manual entry)", d.adapter.id);
        } else {
            println!("  {:12} {} model(s)", d.adapter.id, models.len());
            for m in models.iter().take(5) {
                println!("    - {m}");
            }
            if models.len() > 5 {
                println!("    … {} more", models.len() - 5);
            }
        }
    }
    let cfg = store::load_config();
    println!(
        "\nConfig: {} agent(s) at {}",
        cfg.agents.len(),
        store::config_path().display()
    );
}

fn headless_apply() {
    let cfg: PanelConfig = store::load_config();
    let active = cfg.agents.iter().filter(|a| a.enabled).count();
    if !(2..=4).contains(&active) {
        eprintln!("ERROR: 2–4 active agents required (currently {active}). Configure via the GUI first.");
        std::process::exit(1);
    }
    match store::compile_and_write(&cfg) {
        Ok((backup, backup_hash, new_hash)) => {
            println!("Applied.");
            println!("  backup: {}", backup.display());
            println!("  backup SHA-256: {backup_hash}");
            println!("  new SHA-256:    {new_hash}");
        }
        Err(e) => {
            eprintln!("ERROR: {e}");
            std::process::exit(1);
        }
    }
}

fn headless_strip() {
    let path = store::wt_settings_path();
    let src = std::fs::read_to_string(&path).expect("settings.json readable");
    let out = settings::strip(&src).expect("strip");
    let (backup, _) = store::backup_settings().expect("backup");
    std::fs::write(&path, out).expect("write");
    println!("Stripped all AgentPanel blocks. Backup: {}", backup.display());
}
