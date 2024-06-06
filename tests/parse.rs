use std::{collections::HashMap, sync::Arc};

use async_recursion::async_recursion;
use html_languageservice::{
    parser::{html_parse::*, html_scanner::TokenType},
    HTMLDataManager,
};
use tokio::sync::RwLock;

async fn parse(text: &str) -> HTMLDocument {
    let data_manager = HTMLDataManager::new(true, None);
    HTMLParser::parse(text, "html", &data_manager).await
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
                    NodeAttribute::new(Some(r#""these are my-classes""#.to_string()), 5),
                ),
                (
                    "id".to_string(),
                    NodeAttribute::new(Some(r#""test""#.to_string()), 34),
                ),
            ]),
            children: vec![NodeJSONWithAttributes {
                tag: "span".to_string(),
                attributes: HashMap::from([(
                    "aria-describedby".to_string(),
                    NodeAttribute::new(Some(r#""test""#.to_string()), 50),
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
                ("checked".to_string(), NodeAttribute::new(None, 5)),
                (
                    "id".to_string(),
                    NodeAttribute::new(Some(r#""test""#.to_string()), 13),
                ),
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
    attributes: HashMap<String, NodeAttribute>,
    children: Vec<NodeJSONWithAttributes>,
}
