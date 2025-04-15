use std::str::FromStr;

use lsp_types::Uri;

pub fn create_uri_from_str(s: &str) -> Option<Uri> {
    let s = s.replace("\\", "/");
    if let Ok(uri) = Uri::from_str(&s) {
        Some(uri)
    } else {
        None
    }
}
