use lsp_textdocument::FullTextDocument;
use std::sync::{Arc, Weak};
use tokio::sync::RwLock;

use crate::{
    language_facts::data_manager::HTMLDataManager,
    parser::html_scanner::{Scanner, TokenType},
};

use super::{
    html_document::{HTMLDocument, Node, NodeAttribute},
    html_scanner::ScannerState,
};

pub struct HTMLParser;

impl HTMLParser {
    pub async fn parse_document(
        document: &FullTextDocument,
        data_manager: &HTMLDataManager,
    ) -> HTMLDocument {
        HTMLParser::parse(
            document.get_content(None),
            &document.language_id(),
            data_manager,
        )
        .await
    }

    pub async fn parse(
        text: &str,
        language_id: &str,
        data_manager: &HTMLDataManager,
    ) -> HTMLDocument {
        parse_html_document(text, language_id, &data_manager).await
    }
}

pub async fn parse_html_document(
    text: &str,
    language_id: &str,
    data_manager: &HTMLDataManager,
) -> HTMLDocument {
    let void_elements = data_manager.get_void_elements(language_id).await;
    let mut scanner = Scanner::new(text, 0, ScannerState::WithinContent, true);

    let html_document = Arc::new(RwLock::new(Node::new(0, text.len(), vec![], Weak::new())));
    let mut cur = Arc::clone(&html_document);
    let mut end_tag_start = None;
    let mut end_tag_name = None;
    let mut pending_attribute = None;
    let mut token = scanner.scan();
    while token != TokenType::EOS {
        match token {
            TokenType::StartTagOpen => {
                let child = Arc::new(RwLock::new(Node::new(
                    scanner.get_token_offset(),
                    text.len(),
                    vec![],
                    Arc::downgrade(&cur),
                )));
                cur.write().await.children.push(Arc::clone(&child));
                cur = child;
            }
            TokenType::StartTag => {
                cur.write().await.tag = Some(scanner.get_token_text().to_string());
            }
            TokenType::StartTagClose => {
                if cur.read().await.parent.upgrade().is_some() {
                    cur.write().await.end = scanner.get_token_end();
                    if scanner.get_token_length() > 0 {
                        let tag = cur.read().await.tag.clone();
                        cur.write().await.start_tag_end = Some(scanner.get_token_end());
                        if tag.is_some()
                            && data_manager.is_void_element(&tag.unwrap(), &void_elements)
                        {
                            cur.write().await.closed = true;
                            let parent = cur.read().await.parent.upgrade().unwrap();
                            cur = parent;
                        }
                    } else {
                        // pseudo close token from an incomplete start tag
                        let parent = cur.read().await.parent.upgrade().unwrap();
                        cur = parent;
                    }
                }
            }
            TokenType::StartTagSelfClose => {
                if cur.read().await.parent.upgrade().is_some() {
                    cur.write().await.closed = true;
                    cur.write().await.end = scanner.get_token_end();
                    cur.write().await.start_tag_end = Some(scanner.get_token_end());
                    let parent = cur.read().await.parent.upgrade().unwrap();
                    cur = parent;
                }
            }
            TokenType::EndTagOpen => {
                end_tag_start = Some(scanner.get_token_offset());
                end_tag_name = None;
            }
            TokenType::EndTag => {
                end_tag_name = Some(scanner.get_token_text().to_string().to_lowercase());
            }
            TokenType::EndTagClose => {
                let mut node = Arc::clone(&cur);
                // see if we can find a matching tag
                while !node.read().await.is_same_tag(end_tag_name.as_deref())
                    && node.read().await.parent.upgrade().is_some()
                {
                    let parent = node.read().await.parent.upgrade().unwrap();
                    node = parent;
                }
                if node.read().await.parent.upgrade().is_some() {
                    while !Arc::ptr_eq(&cur, &node) {
                        cur.write().await.end = end_tag_start.unwrap();
                        cur.write().await.closed = false;
                        let parent = cur.read().await.parent.upgrade().unwrap();
                        cur = parent;
                    }
                    cur.write().await.closed = true;
                    cur.write().await.end_tag_start = end_tag_start;
                    cur.write().await.end = scanner.get_token_end();
                    let parent = cur.read().await.parent.upgrade().unwrap();
                    cur = parent;
                }
            }
            TokenType::AttributeName => {
                let text = scanner.get_token_text();
                pending_attribute = Some(text.to_string());
                cur.write().await.attributes.insert(
                    text.to_string(),
                    NodeAttribute::new(None, scanner.get_token_offset()),
                ); // Support valueless attributes such as 'checked'
            }
            TokenType::AttributeValue => {
                let text = scanner.get_token_text();
                if let Some(attr) = pending_attribute {
                    let offset = scanner.get_token_offset() - 1 - attr.len();
                    cur.write()
                        .await
                        .attributes
                        .insert(attr, NodeAttribute::new(Some(text.to_string()), offset));
                    pending_attribute = None;
                }
            }
            _ => {}
        }
        token = scanner.scan();
    }
    while cur.read().await.parent.upgrade().is_some() {
        cur.write().await.end = text.len();
        cur.write().await.closed = false;
        let parent = cur.read().await.parent.upgrade().unwrap();
        cur = parent;
    }
    let mut roots = vec![];
    for root in html_document.read().await.children.iter() {
        roots.push(Arc::clone(root));
    }
    HTMLDocument { roots }
}
