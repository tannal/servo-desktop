use std::sync::Arc;
use tao::event_loop::{ControlFlow, EventLoop, EventLoopBuilder};
use tao::window::{Window, WindowBuilder};

use crate::waker::{DesktopEvent, TaoEventLoopWaker};

pub struct ServoApp {
    pub event_loop: EventLoop<DesktopEvent>,
}

impl ServoApp {
    pub fn new() -> Self {
        let event_loop = EventLoopBuilder::<DesktopEvent>::with_user_event().build();
        Self { event_loop }
    }

    pub fn run<F>(self, title: &str, mut on_init: F)
    where
        F: FnMut(&Window, &EventLoop<DesktopEvent>) + 'static,
    {
        let proxy = self.event_loop.create_proxy();
        let _waker = Box::new(TaoEventLoopWaker::new(proxy));

        let window = Arc::new(
            WindowBuilder::new()
                .with_title(title)
                .build(&self.event_loop)
                .expect("Failed to create TAO window"),
        );

        // Allows configuring system tray, menus, and window in the app
        on_init(&window, &self.event_loop);

        self.event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                tao::event::Event::WindowEvent {
                    event, window_id, ..
                } if window_id == window.id() => match event {
                    tao::event::WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    tao::event::WindowEvent::Resized(_) => {
                        window.request_redraw();
                    }
                    _ => {}
                },
                tao::event::Event::UserEvent(DesktopEvent::ServoWakeup) => {
                    window.request_redraw();
                }
                tao::event::Event::RedrawRequested(_) => {
                    // Servo render/present call
                }
                _ => {}
            }
        });
    }
}
