use cef::*;
use tests_shared::common::client_app::ProcessType;

pub mod app_browser_impl;
pub mod app_renderer_impl;
pub mod client_impl;
pub mod platform;
pub mod resource_util;

use crate::shared::{
    app_browser_impl::{BrowserApp, BrowserProcessHandlerImpl},
    app_renderer_impl::{RenderProcessHandlerImpl, RendererApp},
};

#[cfg(target_os = "macos")]
pub type Library = library_loader::LibraryLoader;

#[cfg(not(target_os = "macos"))]
pub struct Library;

#[allow(dead_code)]
pub fn load_cef() -> Library {
    #[cfg(target_os = "macos")]
    let library = {
        let loader = library_loader::LibraryLoader::new(&std::env::current_exe().unwrap(), false);
        assert!(loader.load());
        loader
    };
    #[cfg(not(target_os = "macos"))]
    let library = Library;

    // Initialize the CEF API version.
    let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);

    #[cfg(target_os = "macos")]
    crate::mac::setup_simple_application();

    library
}

#[allow(dead_code)]
pub fn run_main(main_args: &MainArgs, cmd_line: &CommandLine, sandbox_info: *mut u8) {
    let switch = CefString::from("type");
    let process_type = ProcessType::from(cmd_line);

    if matches!(process_type, ProcessType::Browser) {
        let ret = execute_process(Some(main_args), None, sandbox_info);
        println!("launch browser process");
        assert_eq!(ret, -1, "cannot execute browser process");
    } else {
        let mut app = None;
        if matches!(process_type, ProcessType::Renderer) {
            let render_process_handler = RenderProcessHandlerImpl::new(Default::default());
            app = Some(RendererApp::new(render_process_handler));
        }

        let ret = execute_process(Some(main_args), app.as_mut(), sandbox_info);

        let process_type = CefString::from(&cmd_line.switch_value(Some(&switch)));
        println!("launch process {process_type}");
        assert!(ret >= 0, "cannot execute non-browser process");
        // non-browser process does not initialize cef
        return;
    }

    let browser_process_handler = BrowserProcessHandlerImpl::new(Default::default());
    let mut app = BrowserApp::new(browser_process_handler);

    let settings = Settings {
        no_sandbox: !cfg!(feature = "sandbox") as _,
        ..Default::default()
    };
    assert_eq!(
        initialize(
            Some(main_args),
            Some(&settings),
            Some(&mut app),
            sandbox_info,
        ),
        1
    );

    #[cfg(target_os = "macos")]
    let _delegate = crate::mac::setup_simple_app_delegate();

    run_message_loop();

    shutdown();
}
