use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
    sync::{Arc, RwLock},
};

use lsp_textdocument::FullTextDocument;

use crate::{
    language_facts::data_manager::HTMLDataManager,
    parser::html_scanner::{Scanner, TokenType},
};

use super::html_scanner::ScannerState;

pub struct Node {
    pub tag: Option<String>,
    pub start: usize,
    pub end: usize,
    pub children: Vec<Rc<RefCell<Node>>>,
    pub parent: Weak<RefCell<Node>>,
    pub closed: bool,
    pub start_tag_end: Option<usize>,
    pub end_tag_start: Option<usize>,
    pub attributes: HashMap<String, Option<String>>,
}

impl Node {
    pub fn new(
        start: usize,
        end: usize,
        children: Vec<Rc<RefCell<Node>>>,
        parent: Weak<RefCell<Node>>,
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

    pub fn first_child(&self) -> Option<Rc<RefCell<Node>>> {
        Some(Rc::clone(self.children.first()?))
    }

    pub fn last_child(&self) -> Option<Rc<RefCell<Node>>> {
        Some(Rc::clone(self.children.last()?))
    }

    pub fn find_node_before(node: Rc<RefCell<Node>>, offset: usize) -> Rc<RefCell<Node>> {
        let idx = if let Some((idx, _)) = node
            .borrow()
            .children
            .iter()
            .enumerate()
            .find(|(_, ref c)| offset <= c.borrow().start)
        {
            idx
        } else {
            node.borrow().children.len()
        };
        if idx > 0 {
            let child = Rc::clone(&node.borrow().children[idx - 1]);
            if offset > child.borrow().start {
                if offset < child.borrow().end {
                    return Node::find_node_before(child, offset);
                }
                if child
                    .borrow()
                    .last_child()
                    .is_some_and(|last_child| last_child.borrow().end == child.borrow().end)
                {
                    return Node::find_node_before(child, offset);
                }
                return child;
            }
        }
        node
    }

    pub fn find_node_at(node: Rc<RefCell<Node>>, offset: usize) -> Rc<RefCell<Node>> {
        let idx = if let Some((idx, _)) = node
            .borrow()
            .children
            .iter()
            .enumerate()
            .find(|(_, ref c)| offset <= c.borrow().start)
        {
            idx
        } else {
            node.borrow().children.len()
        };

        if idx > 0 {
            let child = Rc::clone(&node.borrow().children[idx - 1]);
            if offset > child.borrow().start && offset <= child.borrow().end {
                return Node::find_node_at(child, offset);
            }
        }
        node
    }
}

pub struct HTMLDocument {
    roots: Vec<Rc<RefCell<Node>>>,
}

impl HTMLDocument {
    pub fn find_node_before(&self, offset: usize) -> Option<Rc<RefCell<Node>>> {
        let idx = if let Some((idx, _)) = self
            .roots
            .iter()
            .enumerate()
            .find(|(_, ref c)| offset <= c.borrow().start)
        {
            idx
        } else {
            self.roots.len()
        };
        if idx > 0 {
            let child = Rc::clone(&self.roots[idx - 1]);
            if offset > child.borrow().start {
                if offset < child.borrow().end {
                    return Some(Node::find_node_before(child, offset));
                }
                if child
                    .borrow()
                    .last_child()
                    .is_some_and(|last_child| last_child.borrow().end == child.borrow().end)
                {
                    return Some(Node::find_node_before(child, offset));
                }
                return Some(child);
            }
        }
        None
    }

    pub fn find_node_at(&self, offset: usize) -> Option<Rc<RefCell<Node>>> {
        let idx = if let Some((idx, _)) = self
            .roots
            .iter()
            .enumerate()
            .find(|(_, ref c)| offset <= c.borrow().start)
        {
            idx
        } else {
            self.roots.len()
        };

        if idx > 0 {
            let child = Rc::clone(&self.roots[idx - 1]);
            if offset > child.borrow().start && offset <= child.borrow().end {
                return Some(Node::find_node_at(child, offset));
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

    pub fn parse_document(&self, document: FullTextDocument) -> HTMLDocument {
        self.parse(document.get_content(None), &document.language_id())
    }

    pub fn parse(&self, text: &str, language_id: &str) -> HTMLDocument {
        let manager = self.data_manager.read().unwrap();
        let void_elements = manager.get_void_elements(language_id);
        let mut scanner = Scanner::new(text, 0, ScannerState::WithinContent);

        let html_document = Rc::new(RefCell::new(Node::new(0, text.len(), vec![], Weak::new())));
        let mut cur = Rc::clone(&html_document);
        let mut end_tag_start = None;
        let mut end_tag_name = None;
        let mut pending_attribute = None;
        let mut token = scanner.scan();
        while token != TokenType::EOS {
            match token {
                TokenType::StartTagOpen => {
                    let child = Rc::new(RefCell::new(Node::new(
                        scanner.get_token_offset(),
                        text.len(),
                        vec![],
                        Rc::downgrade(&cur),
                    )));
                    cur.borrow_mut().children.push(Rc::clone(&child));
                    cur = child;
                }
                TokenType::StartTag => {
                    cur.borrow_mut().tag = Some(scanner.get_token_text().to_string());
                }
                TokenType::StartTagClose => {
                    if cur.borrow().parent.upgrade().is_some() {
                        cur.borrow_mut().end = scanner.get_token_end();
                        if scanner.get_token_length() > 0 {
                            let tag = cur.borrow().tag.clone();
                            cur.borrow_mut().start_tag_end = Some(scanner.get_token_end());
                            if tag.is_some()
                                && self
                                    .data_manager
                                    .read()
                                    .unwrap()
                                    .is_void_element(&tag.unwrap(), &void_elements)
                            {
                                cur.borrow_mut().closed = true;
                                let parent = cur.borrow().parent.upgrade().unwrap();
                                cur = parent;
                            }
                        } else {
                            // pseudo close token from an incomplete start tag
                            let parent = cur.borrow().parent.upgrade().unwrap();
                            cur = parent;
                        }
                    }
                }
                TokenType::StartTagSelfClose => {
                    if cur.borrow().parent.upgrade().is_some() {
                        cur.borrow_mut().closed = true;
                        cur.borrow_mut().end = scanner.get_token_end();
                        cur.borrow_mut().start_tag_end = Some(scanner.get_token_end());
                        let parent = cur.borrow().parent.upgrade().unwrap();
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
                    let mut node = Rc::clone(&cur);
                    // see if we can find a matching tag
                    while !node.borrow().is_same_tag(end_tag_name.as_deref())
                        && node.borrow().parent.upgrade().is_some()
                    {
                        let parent = node.borrow().parent.upgrade().unwrap();
                        node = parent;
                    }
                    if node.borrow().parent.upgrade().is_some() {
                        while !Rc::ptr_eq(&cur, &node) {
                            cur.borrow_mut().end = end_tag_start.unwrap();
                            cur.borrow_mut().closed = false;
                            let parent = cur.borrow().parent.upgrade().unwrap();
                            cur = parent;
                        }
                        cur.borrow_mut().closed = true;
                        cur.borrow_mut().end_tag_start = end_tag_start;
                        cur.borrow_mut().end = scanner.get_token_end();
                        let parent = cur.borrow().parent.upgrade().unwrap();
                        cur = parent;
                    }
                }
                TokenType::AttributeName => {
                    let text = scanner.get_token_text();
                    pending_attribute = Some(text.to_string());
                    cur.borrow_mut().attributes.insert(text.to_string(), None); // Support valueless attributes such as 'checked'
                }
                TokenType::AttributeValue => {
                    let text = scanner.get_token_text();
                    if let Some(attr) = pending_attribute {
                        cur.borrow_mut()
                            .attributes
                            .insert(attr, Some(text.to_string()));
                        pending_attribute = None;
                    }
                }
                _ => {}
            }
            token = scanner.scan();
        }
        while cur.borrow().parent.upgrade().is_some() {
            cur.borrow_mut().end = text.len();
            cur.borrow_mut().closed = false;
            let parent = cur.borrow().parent.upgrade().unwrap();
            cur = parent;
        }
        let mut roots = vec![];
        for root in html_document.borrow().children.iter() {
            roots.push(Rc::clone(root));
        }
        HTMLDocument { roots }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> HTMLDocument {
        let data_manager = Arc::new(RwLock::new(HTMLDataManager::new(true, None)));
        HTMLParser::new(data_manager).parse(text, "html")
    }

    fn to_json(node: Rc<RefCell<Node>>) -> NodeJSON {
        let node = node.borrow();
        NodeJSON {
            tag: node.tag.clone().unwrap_or_default(),
            start: node.start,
            end: node.end,
            end_tag_start: node.end_tag_start,
            closed: node.closed,
            children: node
                .children
                .iter()
                .map(|node| to_json(node.to_owned()))
                .collect(),
        }
    }

    fn to_json_with_attributes(node: Rc<RefCell<Node>>) -> NodeJSONWithAttributes {
        let node = node.borrow();
        NodeJSONWithAttributes {
            tag: node.tag.clone().unwrap_or_default(),
            attributes: node.attributes.clone(),
            children: node
                .children
                .iter()
                .map(|node| to_json_with_attributes(node.to_owned()))
                .collect(),
        }
    }

    fn assert_document(input: &str, expected: Vec<NodeJSON>) {
        let document = parse(input);
        let nodes: Vec<NodeJSON> = document
            .roots
            .iter()
            .map(|root| to_json(root.to_owned()))
            .collect();
        assert_eq!(nodes, expected)
    }

    fn assert_node_before(input: &str, offset: usize, expected_tag: Option<&str>) {
        let document = parse(input);
        let node = document.find_node_before(offset);
        if let Some(node) = node {
            assert_eq!(
                node.borrow().tag,
                Some(expected_tag.unwrap_or_default().to_string())
            );
        } else {
            assert_eq!(expected_tag, None);
        }
    }

    fn assert_attributes(input: &str, expected: Vec<NodeJSONWithAttributes>) {
        let document = parse(input);
        let nodes: Vec<NodeJSONWithAttributes> = document
            .roots
            .iter()
            .map(|root| to_json_with_attributes(root.to_owned()))
            .collect();
        assert_eq!(nodes, expected);
    }

    #[test]
    fn simple() {
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
        );
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
        );
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
    }

    #[test]
    fn self_close() {
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
        );
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
        );
    }

    #[test]
    fn empty_tag() {
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
        );
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
    }

    #[test]
    fn missing_tags() {
        assert_document("</meta>", vec![]);
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
        );
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
        );
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
        );
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
        );
    }

    #[test]
    fn missing_brackets() {
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
        );
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
        );
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
        );
    }

    #[test]
    fn find_node_before() {
        let input = r#"<div><input type="button"><span><br><hr></span></div>"#;
        assert_node_before(input, 0, None);
        assert_node_before(input, 1, Some("div"));
        assert_node_before(input, 5, Some("div"));
        assert_node_before(input, 6, Some("input"));
        assert_node_before(input, 25, Some("input"));
        assert_node_before(input, 26, Some("input"));
        assert_node_before(input, 27, Some("span"));
        assert_node_before(input, 32, Some("span"));
        assert_node_before(input, 33, Some("br"));
        assert_node_before(input, 36, Some("br"));
        assert_node_before(input, 37, Some("hr"));
        assert_node_before(input, 40, Some("hr"));
        assert_node_before(input, 41, Some("hr"));
        assert_node_before(input, 42, Some("hr"));
        assert_node_before(input, 47, Some("span"));
        assert_node_before(input, 48, Some("span"));
        assert_node_before(input, 52, Some("span"));
        assert_node_before(input, 53, Some("div"));
    }

    #[test]
    fn find_node_before_incomplete_node() {
        let input = "<div><span><br></div>";
        assert_node_before(input, 15, Some("br"));
        assert_node_before(input, 18, Some("br"));
        assert_node_before(input, 21, Some("div"));
    }

    #[test]
    fn attributes() {
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
    }

    #[test]
    fn attributes_without_value() {
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
        );
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
