use notify_rust::Notification;
use servo_desktop::ServoApp;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Send native desktop notification
    let _ = Notification::new()
        .summary("Servo Desktop")
        .body("Loading in-engine Tailwind + Dioxus Wasm UI...")
        .show();

    // 2. Point Servo to your test_wasm.html file
    let html_path = format!(
        "file://{}/tannalwork/projects/servo/test_wasm.html",
        std::env::var("HOME").unwrap()
    );

    let app = ServoApp::new();
    app.run("Dioxus + Tailwind Native App", &html_path)?;

    Ok(())
}
