use std::collections::HashMap;

use super::html_scanner::TokenType;

#[derive(Debug, Clone)]
pub struct Node {
    /// It's None only when new
    pub tag: Option<String>,
    pub start: usize,
    pub end: usize,
    pub children: Vec<Node>,
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
    pub fn new(start: usize, end: usize, children: Vec<Node>) -> Node {
        Node {
            tag: None,
            start,
            end,
            children,
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

    pub fn first_child(&self) -> Option<&Node> {
        Some(self.children.first()?)
    }

    pub fn last_child(&self) -> Option<&Node> {
        Some(self.children.last()?)
    }

    pub fn find_node_before<'a>(
        node: &'a Node,
        offset: usize,
        parent_list: &mut Vec<&'a Node>,
    ) -> &'a Node {
        let mut idx = node.children.len();
        for (i, child) in node.children.iter().enumerate() {
            if offset <= child.start {
                idx = i;
                break;
            }
        }
        if idx > 0 {
            let child = &node.children[idx - 1];
            if offset > child.start {
                if offset < child.end {
                    parent_list.push(&node);
                    return Node::find_node_before(child, offset, parent_list);
                }
                if let Some(last_child) = child.last_child() {
                    if last_child.end == child.end {
                        parent_list.push(&node);
                        return Node::find_node_before(child, offset, parent_list);
                    }
                }
                parent_list.push(&node);
                return child;
            }
        }
        node
    }

    pub fn find_node_at<'a>(
        node: &'a Node,
        offset: usize,
        parent_list: &mut Vec<&'a Node>,
    ) -> &'a Node {
        let mut idx = node.children.len();
        for (i, child) in node.children.iter().enumerate() {
            if offset < child.start {
                idx = i;
                break;
            }
        }

        if idx > 0 {
            let child = &node.children[idx - 1];
            if offset >= child.start && offset < child.end {
                parent_list.push(&node);
                return Node::find_node_at(child, offset, parent_list);
            }
        }
        node
    }

    /// Find TokenType in node at offset
    ///
    /// it return StartTagOpen, StartTag, StartTagClose, StartTagSelfClose, Content, EndTagOpen, EndTag, EndTagClose, Unknown
    ///
    /// if offset in children, then it's Content
    /// if offset outside of node then it's Unknown
    pub fn find_token_type_in_node(node: &Node, offset: usize) -> TokenType {
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
    pub roots: Vec<Node>,
}

impl HTMLDocument {
    /// `parent_list` is a list where the previous node is the parent node of the latter node
    pub fn find_node_before<'a>(
        &'a self,
        offset: usize,
        parent_list: &mut Vec<&'a Node>,
    ) -> Option<&'a Node> {
        let mut idx = self.roots.len();
        for (i, child) in self.roots.iter().enumerate() {
            if offset <= child.start {
                idx = i;
                break;
            }
        }
        if idx > 0 {
            let child = &self.roots[idx - 1];
            if offset > child.start {
                if offset < child.end {
                    return Some(Node::find_node_before(child, offset, parent_list));
                }
                if let Some(last_child) = child.last_child() {
                    if last_child.end == child.end {
                        return Some(Node::find_node_before(child, offset, parent_list));
                    }
                }
                return Some(child);
            }
        }
        None
    }

    /// `parent_list` is a list where the previous node is the parent node of the latter node
    pub fn find_node_at<'a>(
        &'a self,
        offset: usize,
        parent_list: &mut Vec<&'a Node>,
    ) -> Option<&'a Node> {
        let mut idx = self.roots.len();
        for (i, child) in self.roots.iter().enumerate() {
            if offset < child.start {
                idx = i;
                break;
            }
        }

        if idx > 0 {
            let child = &self.roots[idx - 1];
            if offset >= child.start && offset < child.end {
                return Some(Node::find_node_at(child, offset, parent_list));
            }
        }
        None
    }

    pub fn find_root_at(&self, offset: usize) -> Option<&Node> {
        for root in &self.roots {
            if offset <= root.end {
                return Some(root);
            }
        }
        None
    }
}
