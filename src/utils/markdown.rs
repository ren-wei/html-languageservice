use crate::HTMLLanguageServiceOptions;

pub fn does_support_markdown(ls_options: &HTMLLanguageServiceOptions) -> bool {
    if let Some(client_capabilities) = &ls_options.client_capabilities {
        if let Some(text_document) = &client_capabilities.text_document {
            if let Some(completion) = &text_document.completion {
                if let Some(completion_item) = &completion.completion_item {
                    if let Some(documentation_format) = &completion_item.documentation_format {
                        return documentation_format.contains(&lsp_types::MarkupKind::Markdown);
                    }
                }
            }
        }
    } else {
        return true;
    }
    false
}
