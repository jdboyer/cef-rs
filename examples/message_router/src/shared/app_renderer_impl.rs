use std::sync::{Arc, Mutex};

use cef::{
    wrapper::message_router::{
        MessageRouterConfig, MessageRouterRendererSide, MessageRouterRendererSideHandlerCallbacks,
        RendererSideRouter,
    },
    *,
};

wrap_app! {
    pub struct RendererApp {
        render_process_handler: RenderProcessHandler,
    }

    impl App {
        fn render_process_handler(&self) -> Option<RenderProcessHandler> {
            Some(self.render_process_handler.clone())
        }
    }
}

wrap_render_process_handler! {
    pub struct RenderProcessHandlerImpl {
        message_router: Arc<Mutex<Option<Arc<RendererSideRouter>>>>,
    }

    impl RenderProcessHandler {
        fn on_web_kit_initialized(&self) {
            // Create the renderer-side router for query handling.
            let config = MessageRouterConfig::default();
            self.message_router.lock().expect("Failed to lock message_router")
                .replace(RendererSideRouter::new(config));
        }

        fn on_context_created(
            &self,
            browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            context: Option<&mut V8Context>,
        ) {
            let message_router = self.message_router.lock().expect("Failed to lock message_router");
            if let Some(message_router) = message_router.as_ref() {
                message_router.on_context_created(browser.cloned(), frame.cloned(), context.cloned());
            }
        }

        fn on_context_released(
            &self,
            browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            context: Option<&mut V8Context>,
        ) {
            let message_router = self.message_router.lock().expect("Failed to lock message_router");
            if let Some(message_router) = message_router.as_ref() {
                message_router.on_context_released(browser.cloned(), frame.cloned(), context.cloned());
            }
        }

        fn on_process_message_received(
            &self,
            browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            source_process: ProcessId,
            message: Option<&mut ProcessMessage>,
        ) -> i32 {
            let message_router = self.message_router.lock().expect("Failed to lock message_router");

            if let Some(message_router) = message_router.as_ref() {
                return message_router.on_process_message_received(
                    browser.cloned(),
                    frame.cloned(),
                    Some(source_process),
                    message.cloned(),
                ).into();
            }

            0
        }
    }
}
