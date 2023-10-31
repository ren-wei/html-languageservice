use lsp_types::{MarkupContent, MarkupKind};

use crate::html_data::Description;

pub fn normalize_markup_content(input: Description) -> MarkupContent {
    match input {
        Description::String(input) => MarkupContent {
            kind: MarkupKind::Markdown,
            value: input,
        },
        Description::MarkupContent(input) => MarkupContent {
            kind: MarkupKind::Markdown,
            value: input.value,
        },
    }
}
