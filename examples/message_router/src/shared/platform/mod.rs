#[cfg(target_os = "macos")]
mod mac;

#[cfg(target_os = "macos")]
pub use mac::{platform_show_window, platform_title_change};

#[cfg(not(target_os = "macos"))]
pub fn platform_show_window(_browser: Option<&mut cef::Browser>) {
    // Not needed when using Views framework (the default).
}

#[cfg(not(target_os = "macos"))]
pub fn platform_title_change(_browser: Option<&mut cef::Browser>, _title: Option<&cef::CefString>) {
    // Not needed when using Views framework (the default).
}
