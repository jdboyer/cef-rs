#[cfg(target_os = "macos")]
mod mac;

#[cfg(target_os = "macos")]
pub use mac::{platform_show_window, platform_title_change};

#[cfg(not(target_os = "macos"))]
pub fn platform_show_window(_browser: Option<&mut cef::Browser>) {
    todo!("Implement platform_show_window for non-macOS platforms");
}

#[cfg(not(target_os = "macos"))]
pub fn platform_title_change(_browser: Option<&mut cef::Browser>, _title: Option<&cef::CefString>) {
    todo!("Implement platform_title_change for non-macOS platforms");
}
