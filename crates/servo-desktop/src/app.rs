use std::sync::Arc;
use tao::event_loop::{ControlFlow, EventLoop, EventLoopBuilder};
use tao::window::WindowBuilder;

use crate::embedder::{MinimalServoEmbedder, ServoEvent};
use crate::window::ServoDesktopWindow;

pub struct ServoApp {
    pub event_loop: EventLoop<ServoEvent>,
}

impl ServoApp {
    pub fn new() -> Self {
        let event_loop = EventLoopBuilder::<ServoEvent>::with_user_event().build();
        Self { event_loop }
    }

    pub fn run<F>(self, title: &str, mut on_init: F)
    where
        F: FnMut(&tao::window::Window) + 'static,
    {
        let proxy = self.event_loop.create_proxy();
        let tao_window = Arc::new(
            WindowBuilder::new()
                .with_title(title)
                .build(&self.event_loop)
                .expect("Failed to create TAO window"),
        );

        on_init(&tao_window);

        let embedder = Box::new(MinimalServoEmbedder::new(proxy));
        let mut servo_window = ServoDesktopWindow::new(tao_window.clone(), embedder)
            .expect("Failed to initialize Servo window");

        self.event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                tao::event::Event::WindowEvent { event, .. } => match event {
                    tao::event::WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    tao::event::WindowEvent::Resized(size) => {
                        servo_window.resize(size.width, size.height);
                    }
                    _ => {}
                },
                tao::event::Event::UserEvent(ServoEvent::RedrawRequested) => {
                    servo_window.present();
                }
                _ => {}
            }
        });
    }
}
