use cef::{args::Args, *};
use message_router_lib::shared::app_renderer_impl::{RenderProcessHandlerImpl, RendererApp};
use tests_shared::common::client_app::ProcessType;

fn main() {
    let args = Args::new();

    #[cfg(all(target_os = "macos", feature = "sandbox"))]
    let _sandbox = {
        let mut sandbox = sandbox::Sandbox::new();
        sandbox.initialize(args.as_main_args());
        sandbox
    };

    #[cfg(target_os = "macos")]
    let _loader = {
        let loader = library_loader::LibraryLoader::new(&std::env::current_exe().unwrap(), true);
        assert!(loader.load());
        loader
    };

    _ = api_hash(sys::CEF_API_VERSION_LAST, 0);

    let cmd_line = args
        .as_cmd_line()
        .expect("Failed to parse command line arguments");
    let mut app = None;

    if matches!(ProcessType::from(&cmd_line), ProcessType::Renderer) {
        let render_process_handler = RenderProcessHandlerImpl::new(Default::default());
        app = Some(RendererApp::new(render_process_handler));
    }

    execute_process(
        Some(args.as_main_args()),
        app.as_mut(),
        std::ptr::null_mut(),
    );
}
