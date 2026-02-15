use std::sync::{Arc, Mutex, OnceLock, Weak};

use cef::{
    wrapper::message_router::{
        BrowserSideCallback, BrowserSideHandler, BrowserSideRouter, HandlerId,
        MessageRouterBrowserSide, MessageRouterBrowserSideHandlerCallbacks, MessageRouterConfig,
    },
    *,
};

use crate::shared::{
    platform::{platform_show_window, platform_title_change},
    resource_util::{get_resource_handler, get_resource_path},
};

const TEST_MESSAGE_NAME: &str = "MessageRouterTest";

#[derive(Debug)]
pub struct MessageHandler {
    startup_url: String,
}

impl BrowserSideHandler for MessageHandler {
    // Called due to cefQuery execution in message_router.html.
    fn on_query_str(
        &self,
        _browser: Option<Browser>,
        frame: Option<Frame>,
        _query_id: i64,
        request: &str,
        _persistent: bool,
        callback: Arc<Mutex<dyn BrowserSideCallback>>,
    ) -> bool {
        // Only handle messages from the startup URL.
        if !frame.as_ref().is_some_and(|f| {
            CefString::from(&f.url())
                .to_string()
                .contains(&self.startup_url)
        }) {
            return false;
        }

        let message_name = request;
        if message_name.starts_with(TEST_MESSAGE_NAME) {
            // Reverse the string and return.
            let prefix_len = TEST_MESSAGE_NAME.len() + 1;
            let result: String = message_name[prefix_len..].chars().rev().collect();

            callback
                .lock()
                .expect("Failed to lock callback")
                .success_str(&result);

            return true;
        }

        false
    }
}

pub struct ClientManager {
    weak_self: Weak<Mutex<ClientManager>>,

    startup_url: String,
    browser_ct: usize,
    message_router: Option<Arc<BrowserSideRouter>>,
    message_handler_id: Option<HandlerId>,

    browser_list: Vec<Browser>,
    is_closing: bool,
}

static CLIENT_MANAGER_INSTANCE: OnceLock<Weak<Mutex<ClientManager>>> = OnceLock::new();

impl ClientManager {
    pub fn instance() -> Option<Arc<Mutex<Self>>> {
        CLIENT_MANAGER_INSTANCE
            .get()
            .and_then(|manager| manager.upgrade())
    }

    pub fn new(startup_url: String) -> Arc<Mutex<Self>> {
        Arc::new_cyclic(|weak_self| {
            if let Err(instance) = CLIENT_MANAGER_INSTANCE.set(weak_self.clone()) {
                assert_eq!(instance.strong_count(), 0, "Replacing a viable instance");
            }

            Mutex::new(Self {
                weak_self: weak_self.clone(),
                startup_url,
                browser_ct: 0,
                message_router: None,
                message_handler_id: None,
                browser_list: Vec::new(),
                is_closing: false,
            })
        })
    }

    pub fn is_closing(&self) -> bool {
        self.is_closing
    }

    pub fn show_main_window(&self) {
        let thread_id = ThreadId::UI;
        if currently_on(thread_id) == 0 {
            // Execute on the UI thread.
            let this = self
                .weak_self
                .upgrade()
                .expect("Weak reference to ClientManager is None");
            let mut task = ShowMainWindow::new(this);
            post_task(thread_id, Some(&mut task));
            return;
        }

        let Some(mut main_browser) = self.browser_list.first().cloned() else {
            return;
        };

        let browser_view = browser_view_get_for_browser(Some(&mut main_browser));
        if let Some(browser_view) = browser_view {
            if let Some(window) = browser_view.window() {
                window.show();
            }
        } else {
            platform_show_window(Some(&mut main_browser));
        }
    }

    pub fn close_all_browsers(&mut self, force_close: bool) {
        let thread_id = ThreadId::UI;
        if currently_on(thread_id) == 0 {
            // Execute on the UI thread.
            let this = self
                .weak_self
                .upgrade()
                .expect("Weak reference to ClientManager is None");
            let mut task = CloseAllBrowsers::new(this, force_close);
            post_task(thread_id, Some(&mut task));
            return;
        }

        for browser in self.browser_list.iter() {
            let browser_host = browser.host().expect("BrowserHost is None");
            browser_host.close_browser(force_close.into());
        }
    }

    // CefDisplayHandler method
    pub fn on_title_change(&self, mut browser: Option<Browser>, title: Option<&CefString>) {
        debug_assert_ne!(currently_on(ThreadId::UI), 0);

        let browser_view = browser_view_get_for_browser(browser.as_mut());
        if let Some(browser_view) = browser_view {
            if let Some(window) = browser_view.window() {
                window.set_title(title);
            }
        } else {
            platform_title_change(browser.as_mut(), title);
        }
    }

    // CefClient method
    pub fn on_process_message_received(
        &self,
        browser: Option<Browser>,
        frame: Option<Frame>,
        source_process: ProcessId,
        message: Option<ProcessMessage>,
    ) -> bool {
        debug_assert_ne!(currently_on(ThreadId::UI), 0);

        if let Some(message_router) = self.message_router.as_ref() {
            return message_router.on_process_message_received(
                browser,
                frame,
                source_process,
                message,
            );
        }

        false
    }

    // CefLifeSpanHandler method
    pub fn on_after_created(&mut self, browser: Option<Browser>) {
        debug_assert_ne!(currently_on(ThreadId::UI), 0);

        if self.message_router.is_none() {
            // Create the browser-side router for query handling.
            let config = MessageRouterConfig::default();
            self.message_router = Some(BrowserSideRouter::new(config));

            // Register handlers with the router.
            let message_handler = MessageHandler {
                startup_url: self.startup_url.clone(),
            };

            if let Some(message_router) = self.message_router.as_ref() {
                self.message_handler_id = Some(
                    message_router
                        .add_handler(Arc::new(message_handler), false)
                        .expect("Failed to add message handler"),
                );
            }
        }

        self.browser_ct += 1;
        if let Some(browser) = browser {
            self.browser_list.push(browser);
        }
    }

    // CefLifeSpanHandler method
    pub fn do_close(&mut self, _browser: Option<Browser>) -> i32 {
        debug_assert_ne!(currently_on(ThreadId::UI), 0);

        // Closing the main window requires special handling. See the DoClose()
        // documentation in the CEF header for a detailed destription of this
        // process.
        if self.browser_list.len() == 1 {
            // The last browser window is closing.
            self.is_closing = true;
        }

        // Allow the close. For windowed browsers this will result in the OS close
        // event being sent.
        0
    }

    // CefLifeSpanHandler method
    pub fn on_before_close(&mut self, mut browser: Option<Browser>) {
        debug_assert_ne!(currently_on(ThreadId::UI), 0);

        self.browser_ct -= 1;
        if self.browser_ct == 0 {
            // Free the router when the last browser is closed.
            if let Some(message_router) = self.message_router.take()
                && let Some(message_handler_id) = self.message_handler_id.take()
            {
                message_router.remove_handler(message_handler_id);
            }
        }

        // Remove from the list of existing browsers.
        for (i, browser_item) in self.browser_list.iter().enumerate() {
            if browser_item.is_same(browser.as_mut()) != 0 {
                self.browser_list.remove(i);
                break;
            }
        }

        if self.browser_list.is_empty() {
            // All browser windows have closed. Quit the application message loop.
            quit_message_loop();
        }
    }

    // CefRequestHandler method
    pub fn on_before_browse(&self, browser: Option<Browser>, frame: Option<Frame>) -> bool {
        debug_assert_ne!(currently_on(ThreadId::UI), 0);

        if let Some(message_router) = self.message_router.as_ref() {
            message_router.on_before_browse(browser, frame);
        }

        false
    }

    // CefRequestHandler method
    pub fn on_render_process_terminated(&self, browser: Option<Browser>) {
        debug_assert_ne!(currently_on(ThreadId::UI), 0);

        if let Some(message_router) = self.message_router.as_ref() {
            message_router.on_render_process_terminated(browser);
        }
    }

    // CefResourceRequestHandler method
    pub fn resource_handler(&self, request: Option<Request>) -> Option<ResourceHandler> {
        debug_assert_ne!(currently_on(ThreadId::IO), 0);
        let url = CefString::from(&request?.url()).to_string();

        // This is a minimal implementation of resource loading. For more complex
        // usage (multiple files, zip archives, custom handlers, etc.) you might want
        // to use CefResourceManager. See the "resource_manager" target for an
        // example implementation.
        if let Some(resource_path) = get_resource_path(&url) {
            return get_resource_handler(&resource_path);
        }

        None
    }
}

pub fn setup_client(startup_url: String) -> Client {
    let manager = ClientManager::new(startup_url);

    let display_handler = DisplayHandlerImpl::new(manager.clone());
    let life_span_handler = LifeSpanHandlerImpl::new(manager.clone());
    let resource_request_handler = ResourceRequestHandlerImpl::new(manager.clone());
    let request_handler = RequestHandlerImpl::new(manager.clone(), resource_request_handler);

    ClientImpl::new(manager, display_handler, life_span_handler, request_handler)
}

wrap_client! {
    struct ClientImpl {
        manager: Arc<Mutex<ClientManager>>,
        display_handler: DisplayHandler,
        life_span_handler: LifeSpanHandler,
        request_handler: RequestHandler,
    }

    impl Client {
        fn on_process_message_received(
            &self,
            browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            source_process: ProcessId,
            message: Option<&mut ProcessMessage>,
        ) -> i32 {
            let manager = self.manager.lock().expect("Failed to lock manager");
            manager.on_process_message_received(
                browser.cloned(),
                frame.cloned(),
                source_process,
                message.cloned(),
            ).into()
        }

        fn display_handler(&self) -> Option<DisplayHandler> {
            Some(self.display_handler.clone())
        }

        fn life_span_handler(&self) -> Option<LifeSpanHandler> {
            Some(self.life_span_handler.clone())
        }

        fn request_handler(&self) -> Option<RequestHandler> {
            Some(self.request_handler.clone())
        }
    }
}

wrap_display_handler! {
    struct DisplayHandlerImpl {
        inner: Arc<Mutex<ClientManager>>,
    }

    impl DisplayHandler {
        fn on_title_change(&self, browser: Option<&mut Browser>, title: Option<&CefString>) {
            let inner = self.inner.lock().expect("Failed to lock inner");
            inner.on_title_change(browser.cloned(), title);
        }
    }
}

wrap_life_span_handler! {
    struct LifeSpanHandlerImpl {
        inner: Arc<Mutex<ClientManager>>,
    }

    impl LifeSpanHandler {
        fn on_after_created(&self, browser: Option<&mut Browser>) {
            let mut inner = self.inner.lock().expect("Failed to lock inner");
            inner.on_after_created(browser.cloned());
        }

        fn do_close(&self, browser: Option<&mut Browser>) -> i32 {
            let mut inner = self.inner.lock().expect("Failed to lock inner");
            inner.do_close(browser.cloned())
        }

        fn on_before_close(&self, browser: Option<&mut Browser>) {
            let mut inner = self.inner.lock().expect("Failed to lock inner");
            inner.on_before_close(browser.cloned());
        }
    }
}

wrap_request_handler! {
    struct RequestHandlerImpl {
        inner: Arc<Mutex<ClientManager>>,
        resource_request_handler: ResourceRequestHandler,
    }

    impl RequestHandler {
        fn on_before_browse(
            &self,
            browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            request: Option<&mut Request>,
            user_gesture: i32,
            is_redirect: i32
        ) -> i32 {
            let inner = self.inner.lock().expect("Failed to lock inner");
            inner.on_before_browse(browser.cloned(), frame.cloned()).into()
        }

        fn resource_request_handler(
            &self,
            browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            request: Option<&mut Request>,
            is_navigation: ::std::os::raw::c_int,
            is_download: ::std::os::raw::c_int,
            request_initiator: Option<&CefString>,
            disable_default_handling: Option<&mut ::std::os::raw::c_int>,
        ) -> Option<ResourceRequestHandler> {
            debug_assert_ne!(currently_on(ThreadId::IO), 0);
            Some(self.resource_request_handler.clone())
        }

        fn on_render_process_terminated(
            &self,
            browser: Option<&mut Browser>,
            status: TerminationStatus,
            error_code: i32,
            error_string: Option<&CefString>
        ) {
            let inner = self.inner.lock().expect("Failed to lock inner");
            inner.on_render_process_terminated(browser.cloned());
        }
    }
}

wrap_resource_request_handler! {
    struct ResourceRequestHandlerImpl {
        inner: Arc<Mutex<ClientManager>>,
    }

    impl ResourceRequestHandler {
        fn on_before_resource_load(
            &self,
            browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            request: Option<&mut Request>,
            callback: Option<&mut Callback>,
        ) -> ReturnValue {
            ReturnValue::CONTINUE
        }

        fn resource_handler(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            request: Option<&mut Request>,
        ) -> Option<ResourceHandler> {
            let inner = self.inner.lock().expect("Failed to lock inner");
            inner.resource_handler(request.cloned())
        }
    }
}

wrap_task! {
    struct ShowMainWindow {
        inner: Arc<Mutex<ClientManager>>,
    }

    impl Task {
        fn execute(&self) {
            debug_assert_ne!(currently_on(ThreadId::UI), 0);

            let inner = self.inner.lock().expect("Failed to lock inner");
            inner.show_main_window();
        }
    }
}

wrap_task! {
    struct CloseAllBrowsers {
        inner: Arc<Mutex<ClientManager>>,
        force_close: bool,
    }

    impl Task {
        fn execute(&self) {
            debug_assert_ne!(currently_on(ThreadId::UI), 0);

            let mut inner = self.inner.lock().expect("Failed to lock inner");
            inner.close_all_browsers(self.force_close);
        }
    }
}
