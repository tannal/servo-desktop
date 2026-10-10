use embedder_traits::EmbedderMethods;
use servo::{Servo, ServoBuilder};
use std::sync::Arc;
use tao::window::Window as TaoWindow;

pub struct ServoDesktopWindow {
    pub tao_window: Arc<TaoWindow>,
    pub servo: Servo,
}

impl ServoDesktopWindow {
    pub fn new(
        tao_window: Arc<TaoWindow>,
        embedder: Box<dyn EmbedderMethods>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let size = tao_window.inner_size();
        let scale = tao_window.scale_factor();

        let servo = ServoBuilder::new(embedder)
            .window_size(size.width, size.height)
            .device_pixel_ratio(scale as f32)
            .build();

        Ok(Self { tao_window, servo })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.servo.handle_resize(width, height);
    }

    pub fn present(&mut self) {
        self.servo.present();
    }
}
