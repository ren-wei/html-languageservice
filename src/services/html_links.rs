use std::collections::HashMap;

use lazy_static::lazy_static;
use lsp_textdocument::FullTextDocument;
use lsp_types::{DocumentLink, Range, Url};
use regex::Regex;

use crate::{
    parser::html_scanner::{Scanner, ScannerState, TokenType},
    DocumentContext, HTMLDataManager,
};

lazy_static! {
    static ref REG_REF: Regex =
        Regex::new(r"\b(\w[\w\d+.-]*:\/\/)?[^\s()<>]+(?:\([\w\d]+\)|([^[:punct:]\s]|\/?))")
            .unwrap();
    static ref REG_JAVASCRIPT: Regex = Regex::new(r"(^\s*(?i)javascript\:)|([\n\r])").unwrap();
    static ref REG_SCHEMA: Regex = Regex::new(r"^(\w[\w\d+.-]*):").unwrap();
}

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
                                uri,
                                document,
                                document_context,
                                &attribute_value,
                                scanner.get_token_offset(),
                                scanner.get_token_end(),
                                &base,
                            ) {
                                links.push(link);
                            }
                        }
                        if in_base_tag && base.is_none() {
                            base = Some(normalize_ref(&attribute_value).to_string());
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
                        let text = scanner.get_token_text();
                        let id = normalize_ref(&text);
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
    uri: &Url,
    document: &FullTextDocument,
    document_context: &impl DocumentContext,
    attribute_value: &str,
    mut start_offset: usize,
    mut end_offset: usize,
    base: &Option<String>,
) -> Option<DocumentLink> {
    let token_content = normalize_ref(attribute_value);
    if !validate_ref(token_content) {
        return None;
    }
    if token_content.len() < attribute_value.len() {
        start_offset += 1;
        end_offset -= 1;
    }
    let workspace_url = get_workspace_url(uri, token_content, document_context, base)?;
    let target = validate_and_clean_uri(&workspace_url, uri);

    Some(DocumentLink {
        range: Range::new(
            document.position_at(start_offset as u32),
            document.position_at(end_offset as u32),
        ),
        target,
        tooltip: None,
        data: None,
    })
}

fn normalize_ref(url: &str) -> &str {
    if url.len() > 0 {
        let first = url.get(0..1);
        let last = url.get(url.len() - 1..url.len());
        if first == last && (first == Some("'") || first == Some(r#"""#)) {
            return &url[1..url.len() - 1];
        }
    }
    url
}

fn validate_ref(url: &str) -> bool {
    if url.len() == 0 {
        return false;
    }
    REG_REF.is_match(url)
}

fn get_workspace_url(
    document_uri: &Url,
    token_content: &str,
    document_context: &impl DocumentContext,
    base: &Option<String>,
) -> Option<String> {
    if REG_JAVASCRIPT.is_match(token_content) {
        return None;
    }

    let token_content = token_content.trim_start();
    let caps = REG_SCHEMA.captures(&token_content);
    if let Some(caps) = caps {
        // Absolute link that needs no treatment
        let schema = caps.get(1).unwrap().as_str().to_lowercase();
        if schema == "http" || schema == "https" || schema == "file" {
            return Some(token_content.to_string());
        }
        return None;
    }
    if token_content.starts_with("#") {
        return Some(format!("{}{}", document_uri.to_string(), token_content));
    }

    if token_content.starts_with("//") {
        // Absolute link (that does not name the protocol)
        let picked_scheme = if document_uri.to_string().starts_with("https://") {
            "https".to_string()
        } else {
            "http".to_string()
        };
        return Some(picked_scheme + ":" + token_content.trim_start());
    }

    let document_uri = document_uri.to_string();
    document_context
        .resolve_reference(
            &token_content,
            if base.is_some() {
                base.as_ref().unwrap()
            } else {
                &document_uri
            },
        )
        .map(|v| v.to_string())
}

fn validate_and_clean_uri(uri_str: &str, document_uri: &Url) -> Option<Url> {
    if let Ok(mut uri) = Url::parse(uri_str) {
        if uri.scheme() == "file" && uri.query().is_some() {
            // see https://github.com/microsoft/vscode/issues/194577 & https://github.com/microsoft/vscode/issues/206238
            uri.set_query(None);
        }
        let uri_str = uri.to_string();
        if uri.scheme() == "file"
            && uri.fragment().is_some()
            && !(uri_str.starts_with(&document_uri.to_string())
                && uri_str
                    .get(document_uri.as_str().len()..document_uri.as_str().len() + 1)
                    .is_some_and(|c| c == "#"))
        {
            uri.set_fragment(None);
            return Some(uri);
        }
        Some(Url::parse(&uri_str).unwrap())
    } else {
        None
    }
}
