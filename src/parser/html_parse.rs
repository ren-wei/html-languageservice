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
    /// It's None only when it's closed or it miss part of end tag, it equals start of end tag
    pub end_tag_start: Option<usize>,
    pub attributes: HashMap<String, Option<String>>,
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

pub struct HTMLParser {
    pub data_manager: Arc<RwLock<HTMLDataManager>>,
}

impl HTMLParser {
    pub fn new(data_manager: Arc<RwLock<HTMLDataManager>>) -> HTMLParser {
        HTMLParser { data_manager }
    }

    pub async fn parse_document(&self, document: &FullTextDocument) -> HTMLDocument {
        self.parse(document.get_content(None), &document.language_id())
            .await
    }

    pub async fn parse(&self, text: &str, language_id: &str) -> HTMLDocument {
        parse_html_document(text, language_id, Arc::clone(&self.data_manager)).await
    }
}

pub async fn parse_html_document(
    text: &str,
    language_id: &str,
    data_manager: Arc<RwLock<HTMLDataManager>>,
) -> HTMLDocument {
    let manager = data_manager.read().await;
    let void_elements = manager.get_void_elements(language_id).await;
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
                            && data_manager
                                .read()
                                .await
                                .is_void_element(&tag.unwrap(), &void_elements)
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
                cur.write().await.attributes.insert(text.to_string(), None); // Support valueless attributes such as 'checked'
            }
            TokenType::AttributeValue => {
                let text = scanner.get_token_text();
                if let Some(attr) = pending_attribute {
                    cur.write()
                        .await
                        .attributes
                        .insert(attr, Some(text.to_string()));
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

#[cfg(test)]
mod tests {
    use super::*;

    async fn parse(text: &str) -> HTMLDocument {
        let data_manager = Arc::new(RwLock::new(HTMLDataManager::new(true, None)));
        HTMLParser::new(data_manager).parse(text, "html").await
    }

    #[async_recursion]
    async fn to_json(node: Arc<RwLock<Node>>) -> NodeJSON {
        let raw_node = node;
        let node = raw_node.read().await;
        let mut children = vec![];
        for child in &node.children {
            children.push(to_json(child.to_owned()).await);
        }
        NodeJSON {
            tag: node.tag.clone().unwrap_or_default(),
            start: node.start,
            end: node.end,
            end_tag_start: node.end_tag_start,
            closed: node.closed,
            children,
        }
    }

    #[async_recursion]
    async fn to_json_with_attributes(node: Arc<RwLock<Node>>) -> NodeJSONWithAttributes {
        let node = node.read().await;
        let mut children = vec![];
        for child in &node.children {
            children.push(to_json_with_attributes(child.to_owned()).await)
        }
        NodeJSONWithAttributes {
            tag: node.tag.clone().unwrap_or_default(),
            attributes: node.attributes.clone(),
            children,
        }
    }

    async fn assert_document(input: &str, expected: Vec<NodeJSON>) {
        let document = parse(input).await;
        let mut nodes = vec![];
        for root in document.roots {
            nodes.push(to_json(root.to_owned()).await)
        }
        assert_eq!(nodes, expected)
    }

    async fn assert_node_before(input: &str, offset: usize, expected_tag: Option<&str>) {
        let document = parse(input).await;
        let node = document.find_node_before(offset).await;
        if let Some(node) = node {
            assert_eq!(
                node.read().await.tag,
                Some(expected_tag.unwrap_or_default().to_string())
            );
        } else {
            assert_eq!(None, expected_tag);
        }
    }

    async fn assert_find_token_type_in_node(
        input: &str,
        offset: usize,
        expected_token_type: TokenType,
    ) {
        let document = parse(input).await;
        let node = document.find_node_at(offset).await;
        println!("{:#?}", node);
        if let Some(node) = node {
            assert_eq!(
                Node::find_token_type_in_node(node, offset).await,
                expected_token_type
            );
        } else {
            assert_eq!(TokenType::Unknown, expected_token_type);
        }
    }

    async fn assert_attributes(input: &str, expected: Vec<NodeJSONWithAttributes>) {
        let document = parse(input).await;
        let mut nodes = vec![];
        for root in document.roots {
            nodes.push(to_json_with_attributes(root.to_owned()).await);
        }
        assert_eq!(nodes, expected);
    }

    #[tokio::test]
    async fn simple() {
        assert_document(
            "<html></html>",
            vec![NodeJSON {
                tag: "html".to_string(),
                start: 0,
                end: 13,
                end_tag_start: Some(6),
                closed: true,
                children: vec![],
            }],
        )
        .await;
        assert_document(
            "<html><body></body></html>",
            vec![NodeJSON {
                tag: "html".to_string(),
                start: 0,
                end: 26,
                end_tag_start: Some(19),
                closed: true,
                children: vec![NodeJSON {
                    tag: "body".to_string(),
                    start: 6,
                    end: 19,
                    end_tag_start: Some(12),
                    closed: true,
                    children: vec![],
                }],
            }],
        )
        .await;
        assert_document(
            "<html><head></head><body></body></html>",
            vec![NodeJSON {
                tag: "html".to_string(),
                start: 0,
                end: 39,
                end_tag_start: Some(32),
                closed: true,
                children: vec![
                    NodeJSON {
                        tag: "head".to_string(),
                        start: 6,
                        end: 19,
                        end_tag_start: Some(12),
                        closed: true,
                        children: vec![],
                    },
                    NodeJSON {
                        tag: "body".to_string(),
                        start: 19,
                        end: 32,
                        end_tag_start: Some(25),
                        closed: true,
                        children: vec![],
                    },
                ],
            }],
        )
        .await;
    }

    #[tokio::test]
    async fn self_close() {
        assert_document(
            "<br/>",
            vec![NodeJSON {
                tag: "br".to_string(),
                start: 0,
                end: 5,
                end_tag_start: None,
                closed: true,
                children: vec![],
            }],
        )
        .await;
        assert_document(
            "<div><br/><span></span></div>",
            vec![NodeJSON {
                tag: "div".to_string(),
                start: 0,
                end: 29,
                end_tag_start: Some(23),
                closed: true,
                children: vec![
                    NodeJSON {
                        tag: "br".to_string(),
                        start: 5,
                        end: 10,
                        end_tag_start: None,
                        closed: true,
                        children: vec![],
                    },
                    NodeJSON {
                        tag: "span".to_string(),
                        start: 10,
                        end: 23,
                        end_tag_start: Some(16),
                        closed: true,
                        children: vec![],
                    },
                ],
            }],
        )
        .await;
    }

    #[tokio::test]
    async fn empty_tag() {
        assert_document(
            "<meta>",
            vec![NodeJSON {
                tag: "meta".to_string(),
                start: 0,
                end: 6,
                end_tag_start: None,
                closed: true,
                children: vec![],
            }],
        )
        .await;
        assert_document(
            r#"<div><input type="button"><span><br><br></span></div>"#,
            vec![NodeJSON {
                tag: "div".to_string(),
                start: 0,
                end: 53,
                end_tag_start: Some(47),
                closed: true,
                children: vec![
                    NodeJSON {
                        tag: "input".to_string(),
                        start: 5,
                        end: 26,
                        end_tag_start: None,
                        closed: true,
                        children: vec![],
                    },
                    NodeJSON {
                        tag: "span".to_string(),
                        start: 26,
                        end: 47,
                        end_tag_start: Some(40),
                        closed: true,
                        children: vec![
                            NodeJSON {
                                tag: "br".to_string(),
                                start: 32,
                                end: 36,
                                end_tag_start: None,
                                closed: true,
                                children: vec![],
                            },
                            NodeJSON {
                                tag: "br".to_string(),
                                start: 36,
                                end: 40,
                                end_tag_start: None,
                                closed: true,
                                children: vec![],
                            },
                        ],
                    },
                ],
            }],
        )
        .await;
    }

    #[tokio::test]
    async fn missing_tags() {
        assert_document("</meta>", vec![]).await;
        assert_document(
            "<div></div></div>",
            vec![NodeJSON {
                tag: "div".to_string(),
                start: 0,
                end: 11,
                end_tag_start: Some(5),
                closed: true,
                children: vec![],
            }],
        )
        .await;
        assert_document(
            "<div><div></div>",
            vec![NodeJSON {
                tag: "div".to_string(),
                start: 0,
                end: 16,
                end_tag_start: None,
                closed: false,
                children: vec![NodeJSON {
                    tag: "div".to_string(),
                    start: 5,
                    end: 16,
                    end_tag_start: Some(10),
                    closed: true,
                    children: vec![],
                }],
            }],
        )
        .await;
        assert_document(
            "<title><div></title>",
            vec![NodeJSON {
                tag: "title".to_string(),
                start: 0,
                end: 20,
                end_tag_start: Some(12),
                closed: true,
                children: vec![NodeJSON {
                    tag: "div".to_string(),
                    start: 7,
                    end: 12,
                    end_tag_start: None,
                    closed: false,
                    children: vec![],
                }],
            }],
        )
        .await;
        assert_document(
            "<h1><div><span></h1>",
            vec![NodeJSON {
                tag: "h1".to_string(),
                start: 0,
                end: 20,
                end_tag_start: Some(15),
                closed: true,
                children: vec![NodeJSON {
                    tag: "div".to_string(),
                    start: 4,
                    end: 15,
                    end_tag_start: None,
                    closed: false,
                    children: vec![NodeJSON {
                        tag: "span".to_string(),
                        start: 9,
                        end: 15,
                        end_tag_start: None,
                        closed: false,
                        children: vec![],
                    }],
                }],
            }],
        )
        .await;
    }

    #[tokio::test]
    async fn missing_brackets() {
        assert_document(
            "<div><div</div>",
            vec![NodeJSON {
                tag: "div".to_string(),
                start: 0,
                end: 15,
                end_tag_start: Some(9),
                closed: true,
                children: vec![NodeJSON {
                    tag: "div".to_string(),
                    start: 5,
                    end: 9,
                    end_tag_start: None,
                    closed: false,
                    children: vec![],
                }],
            }],
        )
        .await;
        assert_document(
            "<div><div\n</div>",
            vec![NodeJSON {
                tag: "div".to_string(),
                start: 0,
                end: 16,
                end_tag_start: Some(10),
                closed: true,
                children: vec![NodeJSON {
                    tag: "div".to_string(),
                    start: 5,
                    end: 10,
                    end_tag_start: None,
                    closed: false,
                    children: vec![],
                }],
            }],
        )
        .await;
        assert_document(
            "<div><div></div</div>",
            vec![NodeJSON {
                tag: "div".to_string(),
                start: 0,
                end: 21,
                end_tag_start: Some(15),
                closed: true,
                children: vec![NodeJSON {
                    tag: "div".to_string(),
                    start: 5,
                    end: 15,
                    end_tag_start: Some(10),
                    closed: true,
                    children: vec![],
                }],
            }],
        )
        .await;
    }

    #[tokio::test]
    async fn find_node_before() {
        let input = r#"<div><input type="button"><span><br><hr></span></div>"#;
        assert_node_before(input, 0, None).await;
        assert_node_before(input, 1, Some("div")).await;
        assert_node_before(input, 5, Some("div")).await;
        assert_node_before(input, 6, Some("input")).await;
        assert_node_before(input, 25, Some("input")).await;
        assert_node_before(input, 26, Some("input")).await;
        assert_node_before(input, 27, Some("span")).await;
        assert_node_before(input, 32, Some("span")).await;
        assert_node_before(input, 33, Some("br")).await;
        assert_node_before(input, 36, Some("br")).await;
        assert_node_before(input, 37, Some("hr")).await;
        assert_node_before(input, 40, Some("hr")).await;
        assert_node_before(input, 41, Some("hr")).await;
        assert_node_before(input, 42, Some("hr")).await;
        assert_node_before(input, 47, Some("span")).await;
        assert_node_before(input, 48, Some("span")).await;
        assert_node_before(input, 52, Some("span")).await;
        assert_node_before(input, 53, Some("div")).await;
    }

    #[tokio::test]
    async fn find_node_before_incomplete_node() {
        let input = "<div><span><br></div>";
        assert_node_before(input, 15, Some("br")).await;
        assert_node_before(input, 18, Some("br")).await;
        assert_node_before(input, 21, Some("div")).await;
    }

    #[tokio::test]
    async fn find_token_type_in_node() {
        // ------------------0----5---10---15---20---25---30---35---40---45---50-2
        let input = r#"<div><input type="button"/><span>content</span></div>"#;
        assert_find_token_type_in_node(&input, 0, TokenType::StartTagOpen).await;
        assert_find_token_type_in_node(&input, 1, TokenType::StartTag).await;
        assert_find_token_type_in_node(&input, 3, TokenType::StartTag).await;
        assert_find_token_type_in_node(&input, 4, TokenType::StartTagClose).await;
        assert_find_token_type_in_node(&input, 5, TokenType::StartTagOpen).await;
        assert_find_token_type_in_node(&input, 6, TokenType::StartTag).await;
        assert_find_token_type_in_node(&input, 10, TokenType::StartTag).await;
        assert_find_token_type_in_node(&input, 11, TokenType::Unknown).await;
        assert_find_token_type_in_node(&input, 24, TokenType::Unknown).await;
        assert_find_token_type_in_node(&input, 25, TokenType::StartTagSelfClose).await;
        assert_find_token_type_in_node(&input, 26, TokenType::StartTagSelfClose).await;
        assert_find_token_type_in_node(&input, 27, TokenType::StartTagOpen).await;
        assert_find_token_type_in_node(&input, 28, TokenType::StartTag).await;
        assert_find_token_type_in_node(&input, 31, TokenType::StartTag).await;
        assert_find_token_type_in_node(&input, 32, TokenType::StartTagClose).await;
        assert_find_token_type_in_node(&input, 33, TokenType::Content).await;
        assert_find_token_type_in_node(&input, 39, TokenType::Content).await;
        assert_find_token_type_in_node(&input, 40, TokenType::EndTagOpen).await;
        assert_find_token_type_in_node(&input, 41, TokenType::EndTagOpen).await;
        assert_find_token_type_in_node(&input, 42, TokenType::EndTag).await;
        assert_find_token_type_in_node(&input, 45, TokenType::EndTag).await;
        assert_find_token_type_in_node(&input, 46, TokenType::EndTagClose).await;
        assert_find_token_type_in_node(&input, 47, TokenType::EndTagOpen).await;
        assert_find_token_type_in_node(&input, 48, TokenType::EndTagOpen).await;
        assert_find_token_type_in_node(&input, 49, TokenType::EndTag).await;
        assert_find_token_type_in_node(&input, 51, TokenType::EndTag).await;
        assert_find_token_type_in_node(&input, 52, TokenType::EndTagClose).await;
    }

    #[tokio::test]
    async fn attributes() {
        let input = r#"<div class="these are my-classes" id="test"><span aria-describedby="test"></span></div>"#;
        assert_attributes(
            input,
            vec![NodeJSONWithAttributes {
                tag: "div".to_string(),
                attributes: HashMap::from([
                    (
                        "class".to_string(),
                        Some(r#""these are my-classes""#.to_string()),
                    ),
                    ("id".to_string(), Some(r#""test""#.to_string())),
                ]),
                children: vec![NodeJSONWithAttributes {
                    tag: "span".to_string(),
                    attributes: HashMap::from([(
                        "aria-describedby".to_string(),
                        Some(r#""test""#.to_string()),
                    )]),
                    children: vec![],
                }],
            }],
        )
        .await;
    }

    #[tokio::test]
    async fn attributes_without_value() {
        let input = r#"<div checked id="test"></div>"#;
        assert_attributes(
            input,
            vec![NodeJSONWithAttributes {
                tag: "div".to_string(),
                attributes: HashMap::from([
                    ("checked".to_string(), None),
                    ("id".to_string(), Some(r#""test""#.to_string())),
                ]),
                children: vec![],
            }],
        )
        .await;
    }

    #[derive(PartialEq, Debug)]
    struct NodeJSON {
        tag: String,
        start: usize,
        end: usize,
        end_tag_start: Option<usize>,
        closed: bool,
        children: Vec<NodeJSON>,
    }

    #[derive(PartialEq, Debug)]
    struct NodeJSONWithAttributes {
        tag: String,
        attributes: HashMap<String, Option<String>>,
        children: Vec<NodeJSONWithAttributes>,
    }
}
