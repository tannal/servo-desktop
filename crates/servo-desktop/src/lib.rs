//! Lightweight native desktop application shell powered by TAO and Servo.

pub mod app;
pub mod embedder;
pub mod window;

pub use app::ServoApp;
pub use embedder::{MinimalServoEmbedder, ServoEvent};
pub use window::ServoDesktopWindow;

// Re-export native desktop capabilities directly
pub use notify_rust::Notification;
pub use rfd::FileDialog;
