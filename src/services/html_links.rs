use std::collections::HashMap;

use lsp_textdocument::FullTextDocument;
use lsp_types::{DocumentLink, Url};

use crate::{
    parser::html_scanner::{Scanner, ScannerState, TokenType},
    DocumentContext, HTMLDataManager,
};

pub fn find_document_links(
    uri: &Url,
    document: &FullTextDocument,
    document_context: &impl DocumentContext,
    data_manager: &HTMLDataManager,
) -> Vec<DocumentLink> {
    let mut links = vec![];
    let mut scanner = Scanner::new(
        document.get_content(None),
        0,
        ScannerState::WithinContent,
        false,
    );
    let mut last_attribute_name = None;
    let mut last_tag_name = None;
    let mut in_base_tag = false;
    let mut base = None;
    let mut id_locations = HashMap::new();

    let mut token = scanner.scan();
    while token != TokenType::EOS {
        match token {
            TokenType::StartTag => {
                last_tag_name = Some(scanner.get_token_text().to_lowercase());
                if !in_base_tag {
                    in_base_tag = last_tag_name.as_ref().unwrap() == "base";
                }
            }
            TokenType::AttributeName => {
                last_attribute_name = Some(scanner.get_token_text().to_lowercase());
            }
            TokenType::AttributeValue => {
                if last_tag_name.is_some() && last_attribute_name.is_some() {
                    let tag_name = last_tag_name.as_ref().unwrap();
                    let attribute_name = last_attribute_name.as_ref().unwrap();
                    if data_manager.is_path_attribute(&tag_name, &attribute_name) {
                        let attribute_value = scanner.get_token_text();
                        if !in_base_tag {
                            // don't highlight the base link itself
                            if let Some(link) = create_link(
                                document,
                                document_context,
                                attribute_value,
                                scanner.get_token_offset(),
                                scanner.get_token_end(),
                                &base,
                            ) {
                                links.push(link);
                            }
                        }
                        if in_base_tag && base.is_none() {
                            base = Some(normalize_ref(attribute_value).to_string());
                            if base.as_ref().is_some_and(|base| base.len() > 0) {
                                if let Some(uri) = document_context
                                    .resolve_reference(base.as_ref().unwrap(), uri.as_str())
                                {
                                    base = Some(uri.to_string());
                                }
                            }
                        }
                        in_base_tag = false;
                        last_attribute_name = None;
                    } else if attribute_name == "id" {
                        let id = normalize_ref(scanner.get_token_text());
                        id_locations.insert(id.to_string(), scanner.get_token_offset());
                    }
                }
            }
            _ => {}
        }
        token = scanner.scan();
    }

    for link in &mut links {
        let local_with_hash = format!("{}#", uri);
        if let Some(target) = &mut link.target {
            let target = target.to_string();
            if target.starts_with(&local_with_hash) {
                let hash = &target[local_with_hash.len()..];
                if let Some(offset) = id_locations.get(hash) {
                    let pos = document.position_at(*offset as u32);
                    link.target = Some(
                        Url::parse(&format!(
                            "{}{},{}",
                            local_with_hash,
                            pos.line + 1,
                            pos.character + 1
                        ))
                        .unwrap(),
                    )
                } else {
                    link.target = Some(uri.clone());
                }
            }
        }
    }

    links
}

fn create_link(
    document: &FullTextDocument,
    document_context: &impl DocumentContext,
    attribute_value: &str,
    start_offset: usize,
    end_offset: usize,
    base: &Option<String>,
) -> Option<DocumentLink> {
    todo!()
}

fn normalize_ref(url: &str) -> &str {
    todo!()
}
