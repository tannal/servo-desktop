use std::cell::{Cell, RefCell};
use std::error::Error;
use std::rc::Rc;
use std::sync::Arc;

use euclid::Scale;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use servo::RenderingContext;
use servo::{
    Cursor as ServoCursor, DevicePoint, InputEvent, MouseButton, MouseButtonAction,
    MouseButtonEvent, MouseMoveEvent, PrefValue, Preferences, Servo, ServoBuilder, WebView,
    WebViewBuilder, WebViewDelegate, WebViewPoint, WindowRenderingContext,
};
use tao::event_loop::{ControlFlow, EventLoop, EventLoopBuilder, EventLoopProxy};
use tao::window::{CursorIcon, Window, WindowBuilder};
use url::Url;

#[derive(Debug, Clone, Copy)]
pub struct WakerEvent;

#[derive(Clone)]
pub struct ServoWaker(pub Arc<EventLoopProxy<WakerEvent>>);

impl embedder_traits::EventLoopWaker for ServoWaker {
    fn clone_box(&self) -> Box<dyn embedder_traits::EventLoopWaker> {
        Box::new(self.clone())
    }

    fn wake(&self) {
        let _ = self.0.send_event(WakerEvent);
    }
}

pub struct AppState {
    pub window: Window,
    pub servo: Servo,
    pub rendering_context: Rc<WindowRenderingContext>,
    pub webview: RefCell<Option<WebView>>,
    pub cursor_pos: Cell<DevicePoint>,
}

impl WebViewDelegate for AppState {
    fn notify_new_frame_ready(&self, _: WebView) {
        self.window.request_redraw();
    }

    fn notify_cursor_changed(&self, _: WebView, cursor: ServoCursor) {
        let tao_cursor = match cursor {
            ServoCursor::Pointer => CursorIcon::Hand,
            ServoCursor::Text => CursorIcon::Text,
            ServoCursor::Crosshair => CursorIcon::Crosshair,
            ServoCursor::Move => CursorIcon::Move,
            ServoCursor::NotAllowed => CursorIcon::NotAllowed,
            ServoCursor::Wait => CursorIcon::Wait,
            _ => CursorIcon::Default,
        };
        self.window.set_cursor_icon(tao_cursor);
    }

    fn notify_page_title_changed(&self, _: WebView, title: Option<String>) {
        if let Some(title) = title {
            self.window.set_title(&title);
        }
    }
}

pub struct ServoApp {
    event_loop: EventLoop<WakerEvent>,
}

impl ServoApp {
    pub fn new() -> Self {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        let event_loop = EventLoopBuilder::<WakerEvent>::with_user_event().build();
        Self { event_loop }
    }

    pub fn run(self, title: &str, initial_url: &str) -> Result<(), Box<dyn Error>> {
        let proxy = Arc::new(self.event_loop.create_proxy());
        let waker = ServoWaker(proxy);

        let window = WindowBuilder::new()
            .with_title(title)
            .with_inner_size(tao::dpi::LogicalSize::new(1024, 768))
            .build(&self.event_loop)?;

        let display_handle = window.display_handle()?;
        let window_handle = window.window_handle()?;
        let size = window.inner_size();

        let rendering_context = Rc::new(
            WindowRenderingContext::new(display_handle, window_handle, size)
                .map_err(|e| format!("Could not create RenderingContext: {:?}", e))?,
        );
        let _ = rendering_context.make_current();

        let mut preferences = Preferences::default();
        preferences.set_value("dom_wasm_tailwindcss_enabled", PrefValue::Bool(true));

        // 3. Build Servo engine instance
        let servo = ServoBuilder::default()
            .preferences(preferences)
            .event_loop_waker(Box::new(waker))
            .build();
        // servo.setup_logging();

        let app_state = Rc::new(AppState {
            window,
            servo,
            rendering_context: rendering_context.clone(),
            webview: RefCell::new(None),
            cursor_pos: Cell::new(DevicePoint::default()),
        });

        let url = Url::parse(initial_url)?;
        let webview = WebViewBuilder::new(&app_state.servo, rendering_context)
            .url(url)
            .hidpi_scale_factor(Scale::new(app_state.window.scale_factor() as f32))
            .delegate(app_state.clone())
            .build();

        webview.resize(app_state.window.inner_size());
        *app_state.webview.borrow_mut() = Some(webview);
        app_state.window.request_redraw();

        self.event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            app_state.servo.spin_event_loop();

            match event {
                tao::event::Event::WindowEvent {
                    event, window_id, ..
                } if window_id == app_state.window.id() => {
                    match event {
                        tao::event::WindowEvent::CloseRequested => {
                            *control_flow = ControlFlow::Exit;
                        }
                        // Continuous redraw on resize (avoids blank screen)
                        tao::event::WindowEvent::Resized(new_size) => {
                            if let Some(ref webview) = *app_state.webview.borrow() {
                                webview.resize(new_size);
                            }
                            app_state.window.request_redraw();
                        }
                        // Mouse Movement & Hover tracking
                        tao::event::WindowEvent::CursorMoved { position, .. } => {
                            let point = DevicePoint::new(position.x as f32, position.y as f32);
                            app_state.cursor_pos.set(point);

                            if let Some(ref webview) = *app_state.webview.borrow() {
                                webview.notify_input_event(InputEvent::MouseMove(
                                    MouseMoveEvent::new(WebViewPoint::Device(point)),
                                ));
                            }
                        }
                        // Mouse Click (Down & Up) -> Triggers Dioxus onClick handlers
                        tao::event::WindowEvent::MouseInput { state, button, .. } => {
                            let btn = match button {
                                tao::event::MouseButton::Left => MouseButton::Primary,
                                tao::event::MouseButton::Right => MouseButton::Secondary,
                                tao::event::MouseButton::Middle => MouseButton::Auxiliary,
                                _ => MouseButton::Primary,
                            };
                            let action = match state {
                                tao::event::ElementState::Pressed => MouseButtonAction::Down,
                                tao::event::ElementState::Released => MouseButtonAction::Up,
                                _ => unreachable!(),
                            };

                            if let Some(ref webview) = *app_state.webview.borrow() {
                                webview.notify_input_event(InputEvent::MouseButton(
                                    MouseButtonEvent::new(
                                        action,
                                        btn,
                                        WebViewPoint::Device(app_state.cursor_pos.get()),
                                    ),
                                ));
                            }
                        }
                        _ => {}
                    }
                }
                tao::event::Event::UserEvent(WakerEvent) => {
                    app_state.servo.spin_event_loop();
                    app_state.window.request_redraw();
                }
                tao::event::Event::RedrawRequested(_) => {
                    if let Some(ref webview) = *app_state.webview.borrow() {
                        let _ = app_state.rendering_context.make_current();
                        webview.paint();
                        app_state.rendering_context.present();
                    }
                }
                _ => {}
            }
        });
    }
}
