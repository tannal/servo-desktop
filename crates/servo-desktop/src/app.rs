use std::cell::{Cell, RefCell};
use std::error::Error;
use std::rc::Rc;
use std::sync::Arc;

use euclid::Scale;
use keyboard_types::{Key as SKey, KeyState, NamedKey as SNamedKey};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use servo::{
    Cursor as ServoCursor, DevicePoint, InputEvent, KeyboardEvent, MouseButton, MouseButtonAction,
    MouseButtonEvent, MouseMoveEvent, PrefValue, RenderingContext, Servo, ServoBuilder, WebView,
    WebViewBuilder, WebViewDelegate, WebViewPoint, WindowRenderingContext,
};
use url::Url;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::keyboard::{Key as WKey, NamedKey as WNamedKey};
use winit::window::{CursorIcon, Window, WindowId};

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
        let icon = match cursor {
            ServoCursor::Pointer => CursorIcon::Pointer,
            ServoCursor::Text => CursorIcon::Text,
            ServoCursor::Crosshair => CursorIcon::Crosshair,
            ServoCursor::Move => CursorIcon::Move,
            ServoCursor::NotAllowed => CursorIcon::NotAllowed,
            ServoCursor::Wait => CursorIcon::Wait,
            _ => CursorIcon::Default,
        };
        self.window.set_cursor(icon);
    }

    fn notify_page_title_changed(&self, _: WebView, title: Option<String>) {
        if let Some(title) = title {
            self.window.set_title(&title);
        }
    }
}

pub enum AppLifecycle {
    Uninitialized {
        title: String,
        initial_url: String,
        waker: ServoWaker,
    },
    Running(Rc<AppState>),
}

pub struct ServoApp {
    lifecycle: AppLifecycle,
    event_loop: EventLoop<WakerEvent>,
}

impl ServoApp {
    pub fn new(title: &str, initial_url: &str) -> Self {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

        let event_loop = EventLoop::<WakerEvent>::with_user_event()
            .build()
            .expect("Failed to create winit EventLoop");

        let proxy = Arc::new(event_loop.create_proxy());
        let waker = ServoWaker(proxy);

        Self {
            lifecycle: AppLifecycle::Uninitialized {
                title: title.to_string(),
                initial_url: initial_url.to_string(),
                waker,
            },
            event_loop,
        }
    }

    pub fn run(mut self) -> Result<(), Box<dyn Error>> {
        Ok(self.event_loop.run_app(&mut self.lifecycle)?)
    }
}

impl ApplicationHandler<WakerEvent> for AppLifecycle {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let AppLifecycle::Uninitialized {
            title,
            initial_url,
            waker,
        } = self
        {
            // 1. Create native winit window
            let window_attributes = Window::default_attributes()
                .with_title(title.as_str())
                .with_inner_size(LogicalSize::new(1024, 768));

            let window = event_loop
                .create_window(window_attributes)
                .expect("Failed to create winit window");

            let display_handle = event_loop
                .display_handle()
                .expect("Failed to get display handle");
            let window_handle = window.window_handle().expect("Failed to get window handle");
            let size = window.inner_size();

            // 2. Initialize WebRender GPU rendering context
            let rendering_context = Rc::new(
                WindowRenderingContext::new(display_handle, window_handle, size)
                    .expect("Could not create RenderingContext for window"),
            );
            let _ = rendering_context.make_current();

            // 3. Initialize Servo engine with in-engine Tailwind JIT
            let servo = ServoBuilder::default()
                .event_loop_waker(Box::new(waker.clone()))
                .build();
            servo.setup_logging();
            servo.set_preference("dom_wasm_tailwindcss_enabled", PrefValue::Bool(true));

            let app_state = Rc::new(AppState {
                window,
                servo,
                rendering_context: rendering_context.clone(),
                webview: RefCell::new(None),
                cursor_pos: Cell::new(DevicePoint::default()),
            });

            // 4. Create the WebView loading the HTML / Wasm app
            let url = Url::parse(initial_url).expect("Invalid initial URL");
            let webview = WebViewBuilder::new(&app_state.servo, rendering_context)
                .url(url)
                .hidpi_scale_factor(Scale::new(app_state.window.scale_factor() as f32))
                .delegate(app_state.clone())
                .build();

            webview.resize(app_state.window.inner_size());
            webview.set_focused(true);

            *app_state.webview.borrow_mut() = Some(webview);
            app_state.window.request_redraw();

            *self = AppLifecycle::Running(app_state);
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: WakerEvent) {
        if let AppLifecycle::Running(state) = self {
            state.servo.spin_event_loop();
            state.window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let AppLifecycle::Running(state) = self else {
            return;
        };

        if window_id != state.window.id() {
            return;
        }

        state.servo.spin_event_loop();

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                if let Some(ref webview) = *state.webview.borrow() {
                    webview.resize(new_size);
                }
                state.window.request_redraw();
            }
            WindowEvent::Focused(is_focused) => {
                if let Some(ref webview) = *state.webview.borrow() {
                    webview.set_focused(is_focused);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let point = DevicePoint::new(position.x as f32, position.y as f32);
                state.cursor_pos.set(point);

                if let Some(ref webview) = *state.webview.borrow() {
                    webview.notify_input_event(InputEvent::MouseMove(MouseMoveEvent::new(
                        WebViewPoint::Device(point),
                    )));
                }
            }
            WindowEvent::MouseInput {
                state: button_state,
                button,
                ..
            } => {
                let btn = match button {
                    winit::event::MouseButton::Left => MouseButton::Primary,
                    winit::event::MouseButton::Right => MouseButton::Secondary,
                    winit::event::MouseButton::Middle => MouseButton::Auxiliary,
                    _ => MouseButton::Primary,
                };
                let action = match button_state {
                    winit::event::ElementState::Pressed => MouseButtonAction::Down,
                    winit::event::ElementState::Released => MouseButtonAction::Up,
                };

                if let Some(ref webview) = *state.webview.borrow() {
                    webview.notify_input_event(InputEvent::MouseButton(MouseButtonEvent::new(
                        action,
                        btn,
                        WebViewPoint::Device(state.cursor_pos.get()),
                    )));
                }
            }
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                let key_state = match key_event.state {
                    winit::event::ElementState::Pressed => KeyState::Down,
                    winit::event::ElementState::Released => KeyState::Up,
                };

                let key = match key_event.logical_key {
                    WKey::Character(ref s) => Some(SKey::Character(s.to_string())),
                    WKey::Named(named) => match named {
                        WNamedKey::Backspace => Some(SKey::Named(SNamedKey::Backspace)),
                        WNamedKey::Enter => Some(SKey::Named(SNamedKey::Enter)),
                        WNamedKey::Delete => Some(SKey::Named(SNamedKey::Delete)),
                        WNamedKey::ArrowLeft => Some(SKey::Named(SNamedKey::ArrowLeft)),
                        WNamedKey::ArrowRight => Some(SKey::Named(SNamedKey::ArrowRight)),
                        WNamedKey::ArrowUp => Some(SKey::Named(SNamedKey::ArrowUp)),
                        WNamedKey::ArrowDown => Some(SKey::Named(SNamedKey::ArrowDown)),
                        WNamedKey::Tab => Some(SKey::Named(SNamedKey::Tab)),
                        WNamedKey::Escape => Some(SKey::Named(SNamedKey::Escape)),
                        WNamedKey::Space => Some(SKey::Character(" ".into())),
                        _ => None,
                    },
                    _ => None,
                };

                if let Some(k) = key {
                    if let Some(ref webview) = *state.webview.borrow() {
                        webview.notify_input_event(InputEvent::Keyboard(
                            KeyboardEvent::from_state_and_key(key_state, k),
                        ));
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(ref webview) = *state.webview.borrow() {
                    let _ = state.rendering_context.make_current();
                    webview.paint();
                    state.rendering_context.present();
                }
            }
            _ => {}
        }
    }
}
