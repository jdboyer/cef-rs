use cef::{
    ResourceHandler,
    wrapper::{resource_manager::get_mime_type, stream_resource_handler::StreamResourceHandler},
};
use tests_shared::browser::resource_util::get_binary_resource_reader;

pub const TEST_ORIGIN: &str = "https://example.com/";

/// Returns |url| without the query or fragment components, if any.
fn get_url_without_query_or_fragment(url: &str) -> String {
    // Find the first instance of '?' or '#'.
    let q_pos = url.find('?').unwrap_or(url.len());
    let f_pos = url.find('#').unwrap_or(url.len());
    let pos = q_pos.min(f_pos);

    url[..pos].to_string()
}

pub fn get_resource_path(url: &str) -> Option<String> {
    if !url.starts_with(TEST_ORIGIN) {
        return None;
    }

    let url_no_query = get_url_without_query_or_fragment(url);
    Some(url_no_query[TEST_ORIGIN.len()..].to_string())
}

pub fn get_resource_handler(resource_path: &str) -> Option<ResourceHandler> {
    let reader = get_binary_resource_reader(resource_path)?;

    Some(StreamResourceHandler::new_with_stream(
        get_mime_type(resource_path),
        reader,
    ))
}
