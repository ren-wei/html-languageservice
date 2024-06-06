use async_recursion::async_recursion;
use lsp_textdocument::FullTextDocument;
use std::{
    collections::HashMap,
    sync::{Arc, Weak},
};
use tokio::sync::RwLock;

use crate::{
    language_facts::data_manager::HTMLDataManager,
    parser::html_scanner::{Scanner, TokenType},
};

use super::html_scanner::ScannerState;

#[derive(Debug)]
pub struct Node {
    /// It's None only when new
    pub tag: Option<String>,
    pub start: usize,
    pub end: usize,
    pub children: Vec<Arc<RwLock<Node>>>,
    pub parent: Weak<RwLock<Node>>,
    /// Whether part of end tag exists
    pub closed: bool,
    /// It's None only when new, it larger than end of start tag
    pub start_tag_end: Option<usize>,
    /// It's None only when it's self-closing tag or it miss part of end tag, it equals start of end tag
    pub end_tag_start: Option<usize>,
    pub attributes: HashMap<String, NodeAttribute>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NodeAttribute {
    /// include quote
    pub value: Option<String>,
    /// start offfset of attribute name
    pub offset: usize,
}

impl NodeAttribute {
    pub fn new(value: Option<String>, offset: usize) -> NodeAttribute {
        NodeAttribute { value, offset }
    }
}

impl Node {
    pub fn new(
        start: usize,
        end: usize,
        children: Vec<Arc<RwLock<Node>>>,
        parent: Weak<RwLock<Node>>,
    ) -> Node {
        Node {
            tag: None,
            start,
            end,
            children,
            parent,
            closed: false,
            start_tag_end: None,
            end_tag_start: None,
            attributes: HashMap::new(),
        }
    }

    pub fn attribute_names(&self) -> Vec<&String> {
        self.attributes.keys().collect()
    }

    pub fn attribute_names_by_order(&self) -> Vec<&String> {
        let mut attributes = self.attribute_names();
        attributes.sort_by(|a, b| {
            let a = self.attributes.get(*a).unwrap().offset;
            let b = self.attributes.get(*b).unwrap().offset;
            a.cmp(&b)
        });
        attributes
    }

    pub fn is_self_closing(&self) -> bool {
        self.end_tag_start.is_none()
    }

    pub fn is_same_tag(&self, tag_in_lowercase: Option<&str>) -> bool {
        if self.tag.is_none() {
            tag_in_lowercase.is_none()
        } else {
            let tag: &str = &self.tag.as_ref().unwrap();
            tag_in_lowercase.is_some_and(|tag_in_lowercase| {
                tag.len() == tag_in_lowercase.len() && tag.to_lowercase() == tag_in_lowercase
            })
        }
    }

    pub fn first_child(&self) -> Option<Arc<RwLock<Node>>> {
        Some(Arc::clone(self.children.first()?))
    }

    pub fn last_child(&self) -> Option<Arc<RwLock<Node>>> {
        Some(Arc::clone(self.children.last()?))
    }

    #[async_recursion]
    pub async fn find_node_before(node: Arc<RwLock<Node>>, offset: usize) -> Arc<RwLock<Node>> {
        let raw_node = node;
        let node = raw_node.read().await;
        let mut idx = node.children.len();
        for (i, child) in node.children.iter().enumerate() {
            if offset <= child.read().await.start {
                idx = i;
                break;
            }
        }
        if idx > 0 {
            let raw_child = Arc::clone(&node.children[idx - 1]);
            let child = raw_child.read().await;
            if offset > child.start {
                if offset < child.end {
                    drop(child);
                    return Node::find_node_before(raw_child, offset).await;
                }
                if let Some(last_child) = child.last_child() {
                    if last_child.read().await.end == child.end {
                        drop(child);
                        return Node::find_node_before(raw_child, offset).await;
                    }
                }
                drop(child);
                return raw_child;
            }
        }
        drop(node);
        raw_node
    }

    #[async_recursion]
    pub async fn find_node_at(node: Arc<RwLock<Node>>, offset: usize) -> Arc<RwLock<Node>> {
        let raw_node = node;
        let node = raw_node.read().await;
        let mut idx = node.children.len();
        for (i, child) in node.children.iter().enumerate() {
            if offset < child.read().await.start {
                idx = i;
                break;
            }
        }

        if idx > 0 {
            let raw_child = Arc::clone(&node.children[idx - 1]);
            let child = raw_child.read().await;
            if offset >= child.start && offset < child.end {
                drop(child);
                return Node::find_node_at(raw_child, offset).await;
            }
        }
        drop(node);
        raw_node
    }

    /// Find TokenType in node at offset
    ///
    /// it return StartTagOpen, StartTag, StartTagClose, StartTagSelfClose, Content, EndTagOpen, EndTag, EndTagClose, Unknown
    ///
    /// if offset in children, then it's Content
    /// if offset outside of node then it's Unknown
    pub async fn find_token_type_in_node(node: Arc<RwLock<Node>>, offset: usize) -> TokenType {
        let node = node.read().await;
        if node.start > offset || node.end <= offset {
            return TokenType::Unknown;
        }
        let tag = node.tag.as_ref().unwrap();
        if node.start == offset {
            return TokenType::StartTagOpen;
        }
        if offset < node.start + 1 + tag.len() {
            return TokenType::StartTag;
        }
        let start_tag_end = *node.start_tag_end.as_ref().unwrap();
        if offset >= start_tag_end {
            if let Some(end_tag_start) = node.end_tag_start {
                if offset < end_tag_start {
                    return TokenType::Content;
                } else if offset == end_tag_start || offset == end_tag_start + 1 {
                    return TokenType::EndTagOpen;
                } else if offset < node.end - 1 {
                    return TokenType::EndTag;
                } else {
                    return TokenType::EndTagClose;
                }
            } else if start_tag_end == node.end {
                if offset >= node.end - 2 {
                    return TokenType::StartTagSelfClose;
                }
            }
        } else {
            if start_tag_end == node.end {
                if offset >= start_tag_end - 2 {
                    return TokenType::StartTagSelfClose;
                }
            } else {
                if offset >= start_tag_end - 1 {
                    return TokenType::StartTagClose;
                }
            }
        }
        TokenType::Unknown
    }
}

#[derive(Clone)]
pub struct HTMLDocument {
    pub roots: Vec<Arc<RwLock<Node>>>,
}

impl HTMLDocument {
    #[async_recursion]
    pub async fn find_node_before(&self, offset: usize) -> Option<Arc<RwLock<Node>>> {
        let mut idx = self.roots.len();
        for (i, child) in self.roots.iter().enumerate() {
            if offset <= child.read().await.start {
                idx = i;
                break;
            }
        }
        if idx > 0 {
            let raw_child = Arc::clone(&self.roots[idx - 1]);
            let child = raw_child.read().await;
            if offset > child.start {
                if offset < child.end {
                    drop(child);
                    return Some(Node::find_node_before(raw_child, offset).await);
                }
                if let Some(last_child) = child.last_child() {
                    if last_child.read().await.end == child.end {
                        drop(child);
                        return Some(Node::find_node_before(raw_child, offset).await);
                    }
                }
                drop(child);
                return Some(raw_child);
            }
        }
        None
    }

    #[async_recursion]
    pub async fn find_node_at(&self, offset: usize) -> Option<Arc<RwLock<Node>>> {
        let mut idx = self.roots.len();
        for (i, child) in self.roots.iter().enumerate() {
            if offset < child.read().await.start {
                idx = i;
                break;
            }
        }

        if idx > 0 {
            let raw_child = Arc::clone(&self.roots[idx - 1]);
            let child = raw_child.read().await;
            if offset >= child.start && offset < child.end {
                drop(child);
                return Some(Node::find_node_at(raw_child, offset).await);
            }
        }
        None
    }

    pub async fn find_root_at(&self, offset: usize) -> Option<Arc<RwLock<Node>>> {
        for root in &self.roots {
            if offset <= root.read().await.end {
                return Some(Arc::clone(root));
            }
        }
        None
    }
}

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
