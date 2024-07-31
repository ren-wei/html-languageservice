use std::vec;

use lsp_textdocument::FullTextDocument;
use lsp_types::{Position, Range, SelectionRange};

use crate::{
    parser::{
        html_document::{HTMLDocument, Node},
        html_scanner::TokenType,
    },
    HTMLLanguageService,
};

pub fn get_selection_ranges(
    document: &FullTextDocument,
    positions: &Vec<Position>,
    html_document: &HTMLDocument,
) -> Vec<SelectionRange> {
    positions
        .iter()
        .map(|position| get_selection_range(position, document, html_document))
        .collect()
}

fn get_selection_range(
    position: &Position,
    document: &FullTextDocument,
    html_document: &HTMLDocument,
) -> SelectionRange {
    let applicable_ranges = get_applicable_ranges(position, document, html_document);
    let mut prev: Option<(usize, usize)> = None;
    let mut current: Option<Box<SelectionRange>> = None;
    if applicable_ranges.len() > 0 {
        let mut index = applicable_ranges.len() - 1;
        loop {
            let range = applicable_ranges[index];
            if !prev.is_some_and(|v| range == v) {
                current = Some(Box::new(SelectionRange {
                    range: Range::new(
                        document.position_at(range.0 as u32),
                        document.position_at(range.1 as u32),
                    ),
                    parent: current,
                }));
            }
            prev = Some(range);
            if index > 0 {
                index -= 1;
            } else {
                break;
            }
        }
    }
    if current.is_none() {
        SelectionRange {
            range: Range::new(*position, *position),
            parent: None,
        }
    } else {
        *current.unwrap()
    }
}

fn get_applicable_ranges(
    position: &Position,
    document: &FullTextDocument,
    html_document: &HTMLDocument,
) -> Vec<(usize, usize)> {
    let curr_offset = document.offset_at(*position) as usize;
    let mut parent_list = vec![];
    let curr_node = html_document.find_node_at(curr_offset, &mut parent_list);

    let mut result = get_all_parent_tag_ranges(parent_list, html_document);
    if let Some(curr_node) = curr_node {
        // Self-closing or void elements
        if curr_node.start_tag_end.is_some() && curr_node.end_tag_start.is_none() {
            let start_tag_end = curr_node.start_tag_end.unwrap() as u32;
            // The rare case of unmatching tag pairs like <div></div1>
            if start_tag_end != curr_node.end as u32 {
                return vec![(curr_node.start, curr_node.end)];
            }

            let close_range = Range::new(
                document.position_at(start_tag_end - 2),
                document.position_at(start_tag_end),
            );
            let close_text = document.get_content(Some(close_range));

            if close_text == "/>" {
                // Self-closing element
                result.insert(0, (curr_node.start + 1, start_tag_end as usize - 2));
            } else {
                // Void element
                result.insert(0, (curr_node.start + 1, start_tag_end as usize - 1))
            }

            let mut attribute_level_ranges =
                get_attribute_level_ranges(document, curr_node, curr_offset);
            attribute_level_ranges.append(&mut result);
            result = attribute_level_ranges;
            return result;
        }

        if curr_node.start_tag_end.is_none() || curr_node.end_tag_start.is_none() {
            return result;
        }

        let start_tag_end = curr_node.start_tag_end.unwrap();
        let end_tag_start = curr_node.end_tag_start.unwrap();

        // For html like
        // `<div class="foo">bar</div>`
        result.insert(0, (curr_node.start, curr_node.end));

        // Cursor inside `<div class="foo">`
        if curr_node.start < curr_offset && curr_offset < start_tag_end {
            result.insert(0, (curr_node.start + 1, start_tag_end - 1));
            let mut attribute_level_ranges =
                get_attribute_level_ranges(document, curr_node, curr_offset);
            attribute_level_ranges.append(&mut result);
            result = attribute_level_ranges;
            return result;
        }

        // Cursor inside `bar`
        if start_tag_end <= curr_offset && curr_offset <= end_tag_start {
            result.insert(0, (start_tag_end, end_tag_start));
            return result;
        }

        // Cursor inside `</div>`
        if curr_offset >= end_tag_start + 2 {
            result.insert(0, (end_tag_start + 2, curr_node.end - 1));
        }
    }
    result
}

fn get_all_parent_tag_ranges(
    mut parent_list: Vec<&Node>,
    html_document: &HTMLDocument,
) -> Vec<(usize, usize)> {
    let mut result = vec![];

    while parent_list.len() > 0 {
        let curr_node = parent_list.pop();
        if let Some(node) = curr_node {
            result.append(&mut get_node_ranges(&node));
        }
    }

    if html_document.roots.len() > 0 {
        result.push((
            html_document.roots[0].start,
            html_document.roots[html_document.roots.len() - 1].end,
        ));
    }

    result
}

fn get_node_ranges(node: &Node) -> Vec<(usize, usize)> {
    if node.start_tag_end.is_some() && node.end_tag_start.is_some() {
        let start_tag_end = node.start_tag_end.unwrap();
        let end_tag_start = node.end_tag_start.unwrap();
        if start_tag_end < end_tag_start {
            return vec![(start_tag_end, end_tag_start), (node.start, node.end)];
        }
    }
    vec![(node.start, node.end)]
}

fn get_attribute_level_ranges(
    document: &FullTextDocument,
    curr_node: &Node,
    curr_offset: usize,
) -> Vec<(usize, usize)> {
    let curr_node_range = Range::new(
        document.position_at(curr_node.start as u32),
        document.position_at(curr_node.end as u32),
    );
    let curr_node_text = document.get_content(Some(curr_node_range));
    let relative_offset = curr_offset - curr_node.start;

    // Tag level semantic selection

    let mut scanner = HTMLLanguageService::create_scanner(curr_node_text, 0);
    let mut token = scanner.scan();

    // For text like
    // <div class="foo">bar</div>

    let position_offset = curr_node.start;

    let mut result = vec![];

    let mut is_inside_attribute = false;
    let mut attr_start = 0;

    while token != TokenType::EOS {
        match token {
            TokenType::AttributeName => {
                if relative_offset < scanner.get_token_offset() {
                    is_inside_attribute = false;
                } else {
                    if relative_offset <= scanner.get_token_end() {
                        // `class`
                        result.insert(0, (scanner.get_token_offset(), scanner.get_token_end()));
                    }
                    is_inside_attribute = true;
                    attr_start = scanner.get_token_offset();
                }
            }
            TokenType::AttributeValue => {
                if is_inside_attribute {
                    let value_text = scanner.get_token_text();
                    if relative_offset < scanner.get_token_offset() {
                        // `class="foo"`
                        result.push((attr_start, scanner.get_token_end()));
                    } else if relative_offset >= scanner.get_token_offset()
                        && relative_offset <= scanner.get_token_end()
                    {
                        // `"foo"`
                        result.insert(0, (scanner.get_token_offset(), scanner.get_token_end()));
                        // `foo`
                        let first_ch = value_text.get(0..1);
                        let end_ch = value_text.get((value_text.len() - 1)..);
                        if (first_ch.is_some_and(|ch| ch == r#"""#)
                            && end_ch.is_some_and(|ch| ch == r#"""#))
                            || (first_ch.is_some_and(|ch| ch == "'")
                                && end_ch.is_some_and(|ch| ch == "'"))
                        {
                            if relative_offset >= scanner.get_token_offset() + 1
                                && relative_offset <= scanner.get_token_end() - 1
                            {
                                result.insert(
                                    0,
                                    (scanner.get_token_offset() + 1, scanner.get_token_end() - 1),
                                );
                            }
                        }
                        // `class="foo"`
                        result.push((attr_start, scanner.get_token_end()));
                    }
                }
            }
            _ => {}
        }
        token = scanner.scan();
    }
    result
        .iter()
        .map(|pair| (pair.0 + position_offset, pair.1 + position_offset))
        .collect()
}
