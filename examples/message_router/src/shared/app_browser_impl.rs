use std::cell::RefCell;

use cef::*;

use crate::shared::{client_impl::setup_client, resource_util::TEST_ORIGIN};

fn get_startup_url() -> String {
    format!("{}{}", TEST_ORIGIN, "message_router_test.html")
}

wrap_app! {
    pub struct BrowserApp {
        browser_process_handler: BrowserProcessHandler,
    }

    impl App {
        fn on_before_command_line_processing(&self, process_type: Option<&CefString>, command_line: Option<&mut CommandLine>) {
            // Command-line flags can be modified in this callback.
            // |process_type| is empty for the browser process.
            if process_type.is_none() {
                // Disable the macOS keychain prompt. Cookies will not be encrypted.
                #[cfg(target_os = "macos")]
                if let Some(command_line) = command_line {
                    command_line.append_switch(Some(&"use-mock-keychain".into()));
                }
            }
        }

        fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
            Some(self.browser_process_handler.clone())
        }
    }
}

wrap_browser_process_handler! {
    pub struct BrowserProcessHandlerImpl {
        client: RefCell<Option<Client>>
    }

    impl BrowserProcessHandler {
        fn on_context_initialized(&self) {
            debug_assert_ne!(currently_on(ThreadId::UI), 0);

            let startup_url = get_startup_url();

            // Check if Alloy style will be used.
            let command_line = command_line_get_global().expect("Failed to get command line");
            let use_alloy_style =
                command_line.has_switch(Some(&CefString::from("use-alloy-style"))) != 0;
            let runtime_style = if use_alloy_style {
                RuntimeStyle::ALLOY
            } else {
                RuntimeStyle::DEFAULT
            };

            // SimpleHandler implements browser-level callbacks.
            self.client.replace(Some(setup_client(startup_url.clone())));
            let mut client = self.default_client();

            // Specify CEF browser settings here.
            let settings = BrowserSettings::default();

            // Views is enabled by default (add `--use-native` to disable).
            let use_views = command_line.has_switch(Some(&CefString::from("use-native"))) != 0;

            // If using Views create the browser using the Views framework, otherwise
            // create the browser using the native platform framework.
            if use_views {
                // Create the BrowserView.
                let mut delegate = SimpleBrowserViewDelegate::new(runtime_style);
                let browser_view = browser_view_create(
                    client.as_mut(),
                    Some(&startup_url.as_str().into()),
                    Some(&settings),
                    None,
                    None,
                    Some(&mut delegate),
                );

                // Create the Window. It will show itself after creation.
                let mut delegate = SimpleWindowDelegate::new(
                    RefCell::new(browser_view),
                    runtime_style,
                    ShowState::NORMAL,
                );
                window_create_top_level(Some(&mut delegate));
            } else {
                // Information used when creating the native window.
                let window_info = WindowInfo {
                    runtime_style,
                    ..Default::default()
                };

                #[cfg(target_os = "windows")]
                let window_info = window_info.set_as_popup(Default::default(), "cefsimple");

                browser_host_create_browser(
                    Some(&window_info),
                    client.as_mut(),
                    Some(&startup_url.as_str().into()),
                    Some(&settings),
                    None,
                    None,
                );
            }
        }

        fn default_client(&self) -> Option<Client> {
            self.client.borrow().clone()
        }
    }
}

wrap_window_delegate! {
    struct SimpleWindowDelegate {
        browser_view: RefCell<Option<BrowserView>>,
        runtime_style: RuntimeStyle,
        initial_show_state: ShowState,
    }

    impl ViewDelegate {
        fn preferred_size(&self, _view: Option<&mut View>) -> Size {
            Size {
                width: 800,
                height: 600,
            }
        }
    }

    impl PanelDelegate {}

    impl WindowDelegate {
        fn on_window_created(&self, window: Option<&mut Window>) {
            // Add the browser view and show the window.
            let browser_view = self.browser_view.borrow();
            let (Some(window), Some(browser_view)) = (window, browser_view.as_ref()) else {
                return;
            };
            let mut view = View::from(browser_view);
            window.add_child_view(Some(&mut view));

            if self.initial_show_state != ShowState::HIDDEN {
                window.show();
            }

            browser_view.request_focus();
        }

        fn on_window_destroyed(&self, _window: Option<&mut Window>) {
            let mut browser_view = self.browser_view.borrow_mut();
            *browser_view = None;
        }

        fn can_close(&self, _window: Option<&mut Window>) -> i32 {
            // Allow the window to close if the browser says it's OK.
            let browser_view = self.browser_view.borrow();
            let browser_view = browser_view.as_ref().expect("BrowserView is None");

            if let Some(browser) = browser_view.browser() {
                let browser_host = browser.host().expect("BrowserHost is None");
                browser_host.try_close_browser()
            } else {
                1
            }
        }

        fn initial_show_state(&self, _window: Option<&mut Window>) -> ShowState {
            self.initial_show_state
        }

        fn window_runtime_style(&self) -> RuntimeStyle {
            self.runtime_style
        }
    }
}

wrap_browser_view_delegate! {
    struct SimpleBrowserViewDelegate {
        runtime_style: RuntimeStyle,
    }

    impl ViewDelegate {}

    impl BrowserViewDelegate {
        fn on_popup_browser_view_created(
            &self,
            _browser_view: Option<&mut BrowserView>,
            popup_browser_view: Option<&mut BrowserView>,
            _is_devtools: i32,
        ) -> i32 {
            // Create a new top-level Window for the popup. It will show itself after
            // creation.
            let mut window_delegate = SimpleWindowDelegate::new(
                RefCell::new(popup_browser_view.cloned()),
                self.runtime_style,
                ShowState::NORMAL,
            );
            window_create_top_level(Some(&mut window_delegate));

            // We created the Window.
            1
        }

        fn browser_runtime_style(&self) -> RuntimeStyle {
            self.runtime_style
        }
    }
}
