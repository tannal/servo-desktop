use notify_rust::Notification;
use rfd::FileDialog;
use servo_desktop::ServoApp;
// use tao::menu::{MenuBar, MenuItem};
// use tao::system_tray::SystemTrayBuilder;

fn main() {
    env_logger::init();

    let app = ServoApp::new();

    // 1. Fire a native OS desktop notification (D-Bus on Linux, zero WebKit)
    Notification::new()
        .summary("Servo Desktop App")
        .body("Running with native TAO window and Servo renderer!")
        .show()
        .expect("Failed to send notification");

    // 2. Run the application
    app.run("Servo Native App", |window, event_loop| {
        log::info!("Window successfully initialized with ID: {:?}", window.id());

        // 3. Demonstrate native file dialog (XDG Portal / GTK dialog, zero WebKit)
        std::thread::spawn(|| {
            log::info!("Testing native file dialog in background thread...");
            if let Some(path) = FileDialog::new().pick_file() {
                log::info!("User selected file: {:?}", path);
            }
        });
    });
}
