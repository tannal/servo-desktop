use notify_rust::Notification;
use servo_desktop::ServoApp;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Normalizes a CLI argument or path into a valid URL (http://, https://, or file://)
fn normalize_target(input: &str) -> String {
    if input.starts_with("http://") || input.starts_with("https://") || input.starts_with("file://")
    {
        input.to_string()
    } else {
        // Resolve relative paths into canonical absolute file:// URLs
        match fs::canonicalize(input) {
            Ok(abs_path) => format!("file://{}", abs_path.display()),
            Err(_) => {
                let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                format!("file://{}", current_dir.join(input).display())
            }
        }
    }
}

/// Resolves target URL from CLI arg > ENV var > local file detection > default
fn resolve_target_url() -> String {
    // 1. First CLI argument: e.g. `./desktop-app ./my-page.html` or `./desktop-app https://servo.org`
    if let Some(arg) = env::args().nth(1) {
        return normalize_target(&arg);
    }

    // 2. Environment variable: SERVO_URL=test_wasm.html ./desktop-app
    if let Ok(env_target) = env::var("SERVO_URL") {
        return normalize_target(&env_target);
    }

    // 3. Auto-detect common HTML files in the current working directory
    let auto_detect = ["test_wasm.html", "index.html", "dist/index.html"];
    for file in auto_detect {
        if Path::new(file).exists() {
            return normalize_target(file);
        }
    }

    // 4. Default fallback
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    format!(
        "file://{}/tannalwork/projects/tauri-servo/test_wasm.html",
        home
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let target_url = resolve_target_url();
    println!("[DesktopApp] Loading target URL: {}", target_url);

    // Send native desktop notification
    let _ = Notification::new()
        .summary("Servo Desktop")
        .body(&format!("Loading {}", target_url))
        .show();

    let app = ServoApp::new("Servo Desktop Shell", &target_url);
    app.run()?;

    Ok(())
}
