use servo::EventLoopWaker;
use std::sync::{Arc, Mutex};
use tao::event_loop::EventLoopProxy;

#[derive(Debug, Clone, Copy)]
pub enum DesktopEvent {
    ServoWakeup,
}

#[derive(Clone)]
pub struct TaoEventLoopWaker {
    pub proxy: Arc<Mutex<EventLoopProxy<DesktopEvent>>>,
}

impl TaoEventLoopWaker {
    pub fn new(proxy: EventLoopProxy<DesktopEvent>) -> Self {
        Self {
            proxy: Arc::new(Mutex::new(proxy)),
        }
    }
}

impl EventLoopWaker for TaoEventLoopWaker {
    fn wake(&self) {
        if let Ok(proxy) = self.proxy.lock() {
            let _ = proxy.send_event(DesktopEvent::ServoWakeup);
        }
    }

    fn clone_box(&self) -> Box<dyn EventLoopWaker> {
        Box::new(self.clone())
    }
}
