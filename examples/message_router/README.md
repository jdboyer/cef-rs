# cef-rs-message-router

Rust port of CEF's official ["message_router" example](https://github.com/chromiumembedded/cef-project/tree/master/examples/message_router)
implementation.
It demonstrates how to create JavaScript bindings using CefMessageRouter
while providing a practical sample for the cef-rs ecosystem.

Currently, this port has only been confirmed to work on macOS.

## Usage

1. [Install Shared CEF Binaries](https://github.com/tauri-apps/cef-rs/tree/dev?tab=readme-ov-file#install-shared-cef-binaries)
2. [Set Environment Variables](https://github.com/tauri-apps/cef-rs/tree/dev?tab=readme-ov-file#set-environment-variables)
3. `cargo run --bin bundle` will create the application bundle on `./target/bundle/message_router.app`

## Acknowledgments

- Original C++ implementation: https://github.com/chromiumembedded/cef-project/tree/master/examples/message_router
- cef-rs' cefsimple example: https://github.com/tauri-apps/cef-rs/tree/dev/examples/cefsimple
- cef-rs' tests_shared utilities: https://github.com/tauri-apps/cef-rs/tree/dev/examples/tests_shared

## License

This port is provided under the [Unlicense](LICENSE).

However, this project contains code and assets derived from the
[CEF project](https://github.com/chromiumembedded/cef-project)
and [cef-rs](https://github.com/tauri-apps/cef-rs),
which is subject to the following license:

- **cef-project**: [BSD 3-Clause License](third_party/cef-project-license.txt)
- **cef-rs**
    - [MIT License](third_party/cef-rs-license-mit.txt)
    - [Apache-2.0 License](third_party/cef-rs-license-apache.txt)
