use embedder_traits::{Cursor, EmbedderMethods, MediaSessionActionType, ScreenSize};
use servo_url::ServoUrl;
use tao::event_loop::EventLoopProxy;

#[derive(Debug, Clone, Copy)]
pub enum ServoEvent {
    RedrawRequested,
    Wakeup,
}

pub struct MinimalServoEmbedder {
    pub proxy: EventLoopProxy<ServoEvent>,
}

impl MinimalServoEmbedder {
    pub fn new(proxy: EventLoopProxy<ServoEvent>) -> Self {
        Self { proxy }
    }
}

impl EmbedderMethods for MinimalServoEmbedder {
    fn notify_ready_to_present(&self) {
        let _ = self.proxy.send_event(ServoEvent::RedrawRequested);
    }

    fn set_cursor(&self, _cursor: Cursor) {}
    fn load_started(&self) {}
    fn load_complete(&self) {}
    fn title_changed(&self, _title: Option<String>) {}
    fn allow_navigation(&self, _url: ServoUrl) -> bool {
        true
    }
    fn handle_media_session_action(&self, _action: MediaSessionActionType) {}

    fn screen_size(&self) -> ScreenSize {
        ScreenSize {
            width: 1920,
            height: 1080,
        }
    }

    fn screen_avail_size(&self) -> ScreenSize {
        ScreenSize {
            width: 1920,
            height: 1040,
        }
    }
}
