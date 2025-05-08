use crate::{
    language_facts::data_manager::HTMLDataManager,
    parser::html_scanner::{Scanner, TokenType},
};
use lsp_textdocument::FullTextDocument;

use super::{
    html_document::{HTMLDocument, Node, NodeAttribute},
    html_scanner::ScannerState,
};

pub struct HTMLParser;

impl HTMLParser {
    pub fn parse_document(
        document: &FullTextDocument,
        data_manager: &HTMLDataManager,
        case_sensitive: bool,
    ) -> HTMLDocument {
        HTMLParser::parse(
            document.get_content(None),
            &document.language_id(),
            data_manager,
            case_sensitive,
        )
    }

    pub fn parse(
        text: &str,
        language_id: &str,
        data_manager: &HTMLDataManager,
        case_sensitive: bool,
    ) -> HTMLDocument {
        parse_html_document(text, language_id, &data_manager, case_sensitive)
    }
}

pub fn parse_html_document(
    text: &str,
    language_id: &str,
    data_manager: &HTMLDataManager,
    case_sensitive: bool,
) -> HTMLDocument {
    let void_elements = data_manager.get_void_elements(language_id);
    let mut scanner = Scanner::new(text, 0, ScannerState::WithinContent, true, case_sensitive);

    let mut html_document = Node::new(0, scanner.get_source_len(), vec![]);
    let mut cur = &mut html_document as *mut Node;
    let mut parent_list: Vec<*mut Node> = vec![];
    let mut end_tag_start = None;
    let mut end_tag_name = None;
    let mut pending_attribute = None;
    let mut token = scanner.scan();
    unsafe {
        while token != TokenType::EOS {
            match token {
                TokenType::StartTagOpen => {
                    let child =
                        Node::new(scanner.get_token_offset(), scanner.get_source_len(), vec![]);
                    let length = (*cur).children.len();
                    (*cur).children.push(child);
                    parent_list.push(cur);
                    cur = &mut (*cur).children[length];
                }
                TokenType::StartTag => {
                    (*cur).tag = Some(scanner.get_token_text().to_string());
                }
                TokenType::StartTagClose => {
                    if !parent_list.is_empty() {
                        (*cur).end = scanner.get_token_end();
                        if scanner.get_token_length() > 0 {
                            let tag = (*cur).tag.clone();
                            (*cur).start_tag_end = Some(scanner.get_token_end());
                            if tag.is_some()
                                && data_manager.is_void_element(&tag.unwrap(), &void_elements)
                            {
                                (*cur).closed = true;
                                cur = parent_list.pop().unwrap();
                            }
                        } else {
                            // pseudo close token from an incomplete start tag
                            cur = parent_list.pop().unwrap();
                        }
                    }
                }
                TokenType::StartTagSelfClose => {
                    if !parent_list.is_empty() {
                        (*cur).closed = true;
                        (*cur).end = scanner.get_token_end();
                        (*cur).start_tag_end = Some(scanner.get_token_end());
                        cur = parent_list.pop().unwrap();
                    }
                }
                TokenType::EndTagOpen => {
                    end_tag_start = Some(scanner.get_token_offset());
                    end_tag_name = None;
                }
                TokenType::EndTag => {
                    if case_sensitive {
                        end_tag_name = Some(scanner.get_token_text().to_string());
                    } else {
                        end_tag_name = Some(scanner.get_token_text().to_lowercase());
                    }
                }
                TokenType::EndTagClose => {
                    let mut node = cur;
                    let mut node_parent_list_length = parent_list.len();
                    let end_tag_name = end_tag_name.as_deref();
                    // see if we can find a matching tag
                    while !(*node).is_same_tag(end_tag_name, case_sensitive)
                        && node_parent_list_length > 0
                    {
                        node_parent_list_length -= 1;
                        node = parent_list[node_parent_list_length];
                    }
                    if node_parent_list_length > 0 {
                        while node_parent_list_length != parent_list.len() {
                            (*cur).end = end_tag_start.unwrap();
                            (*cur).closed = false;
                            cur = parent_list.pop().unwrap();
                        }
                        (*cur).closed = true;
                        (*cur).end_tag_start = end_tag_start;
                        (*cur).end = scanner.get_token_end();
                        cur = parent_list.pop().unwrap();
                    }
                }
                TokenType::AttributeName => {
                    let text = scanner.get_token_text();
                    pending_attribute = Some(text.to_string());
                    (*cur).attributes.insert(
                        text.to_string(),
                        NodeAttribute::new(None, scanner.get_token_offset()),
                    ); // Support valueless attributes such as 'checked'
                }
                TokenType::AttributeValue => {
                    let text = scanner.get_token_text();
                    if let Some(attr) = pending_attribute {
                        let offset = scanner.get_token_offset() - 1 - attr.len();
                        (*cur)
                            .attributes
                            .insert(attr, NodeAttribute::new(Some(text.to_string()), offset));
                        pending_attribute = None;
                    }
                }
                _ => {}
            }
            token = scanner.scan();
        }
        while !parent_list.is_empty() {
            (*cur).end = scanner.get_source_len();
            (*cur).closed = false;
            cur = parent_list.pop().unwrap();
        }
    }
    let mut roots = vec![];
    for root in html_document.children {
        roots.push(root);
    }
    HTMLDocument { roots }
}
