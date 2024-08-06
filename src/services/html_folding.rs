use std::cmp::Ordering;

use lazy_static::lazy_static;
use lsp_textdocument::FullTextDocument;
use lsp_types::{FoldingRange, FoldingRangeKind};
use regex::Regex;

use crate::{parser::html_scanner::TokenType, HTMLDataManager, HTMLLanguageService};

lazy_static! {
    static ref REG_REGION: Regex = Regex::new(r"^\s*#(region\b)|(endregion\b)").unwrap();
}

pub fn get_folding_ranges(
    document: FullTextDocument,
    context: FoldingRangeContext,
    data_manager: &HTMLDataManager,
) -> Vec<FoldingRange> {
    let void_elements = data_manager.get_void_elements(document.language_id());
    let mut scanner = HTMLLanguageService::create_scanner(document.get_content(None), 0);
    let mut token = scanner.scan();
    let mut ranges = vec![];
    let mut stack = vec![]; // Vec<(startLine: usize, tag_name: String)>
    let mut last_tag_name: Option<String> = None;
    let mut prev_start = u32::MAX;

    while token != TokenType::EOS {
        match token {
            TokenType::StartTag => {
                let tag_name = scanner.get_token_text();
                let start_line = document.position_at(scanner.get_token_offset() as u32).line;
                stack.push((start_line, tag_name.to_string()));
                last_tag_name = Some(tag_name.to_string());
            }
            TokenType::EndTag => {
                last_tag_name = Some(scanner.get_token_text().to_string());
            }
            TokenType::StartTagClose | TokenType::EndTagClose | TokenType::StartTagSelfClose => {
                if stack.len() > 0
                    && (token != TokenType::StartTagClose
                        || last_tag_name.is_some()
                            && data_manager
                                .is_void_element(last_tag_name.as_ref().unwrap(), &void_elements))
                {
                    let mut i = stack.len() - 1;
                    let mut is_find = true;
                    while Some(&stack[i].1) != last_tag_name.as_ref() {
                        if i == 0 {
                            is_find = false;
                            break;
                        } else {
                            i -= 1;
                        }
                    }
                    if is_find {
                        let start_line = stack[i].0;
                        stack.truncate(i);
                        let line = document.position_at(scanner.get_token_end() as u32).line;
                        if line > start_line + 1 && prev_start != start_line {
                            ranges.push(FoldingRange {
                                start_line,
                                end_line: line - 1,
                                ..Default::default()
                            });
                            prev_start = start_line;
                        }
                    }
                }
            }
            TokenType::Comment => {
                let mut start_line = document.position_at(scanner.get_token_offset() as u32).line;
                let text = scanner.get_token_text();
                if let Some(caps) = REG_REGION.captures(&text) {
                    if caps.get(1).is_some() {
                        stack.push((start_line, String::new()));
                    } else if stack.len() > 0 {
                        let mut i = stack.len() - 1;
                        let mut is_find = true;
                        while !stack[i].1.is_empty() {
                            if i == 0 {
                                is_find = false;
                                break;
                            } else {
                                i -= 1;
                            }
                        }
                        if is_find {
                            let end_line = start_line;
                            start_line = stack[i].0;
                            stack.truncate(i);
                            if end_line > start_line && prev_start != start_line {
                                ranges.push(FoldingRange {
                                    start_line,
                                    end_line,
                                    kind: Some(FoldingRangeKind::Region),
                                    ..Default::default()
                                });
                                prev_start = start_line;
                            }
                        }
                    }
                } else {
                    let end_line = document
                        .position_at(scanner.get_token_end() as u32 + 3)
                        .line;
                    if start_line < end_line {
                        ranges.push(FoldingRange {
                            start_line,
                            end_line,
                            kind: Some(FoldingRangeKind::Comment),
                            ..Default::default()
                        });
                        prev_start = start_line;
                    }
                }
            }
            _ => {}
        }
        token = scanner.scan();
    }

    let range_limit = context.range_limit.unwrap_or(usize::MAX);
    if ranges.len() > range_limit {
        limit_ranges(ranges, range_limit)
    } else {
        ranges
    }
}

fn limit_ranges(mut ranges: Vec<FoldingRange>, range_limit: usize) -> Vec<FoldingRange> {
    ranges.sort_by(|r1, r2| {
        let order = r1.start_line.cmp(&r2.start_line);
        if order == Ordering::Equal {
            r1.end_line.cmp(&r2.end_line)
        } else {
            order
        }
    });

    // compute each range's nesting level in 'nesting_levels'.
    // count the number of ranges for each level in 'nesting_level_counts'
    let mut top = None;
    let mut previous = vec![];
    let mut nesting_levels = vec![];
    let mut nesting_level_counts = vec![];

    // compute nesting levels and sanitize
    for i in 0..ranges.len() {
        let entry = &ranges[i];
        if top.is_none() {
            top = Some(entry);
            set_nesting_level(i, 0, &mut nesting_levels, &mut nesting_level_counts);
        } else if entry.start_line > top.unwrap().start_line {
            if entry.end_line <= top.unwrap().end_line {
                previous.push(top.unwrap());
                top = Some(entry);
                set_nesting_level(
                    i,
                    previous.len(),
                    &mut nesting_levels,
                    &mut nesting_level_counts,
                );
            } else if entry.start_line > top.unwrap().end_line {
                top = previous.pop();
                while top.is_some() && entry.start_line > top.unwrap().end_line {
                    top = previous.pop();
                }
                if top.is_some() {
                    previous.push(top.unwrap());
                }
                top = Some(entry);
                set_nesting_level(
                    i,
                    previous.len(),
                    &mut nesting_levels,
                    &mut nesting_level_counts,
                );
            }
        }
    }

    let mut entries = 0;
    let mut max_level = 0;
    for i in 0..nesting_level_counts.len() {
        let n = nesting_level_counts[i];
        if n > 0 {
            if n + entries > range_limit {
                max_level = i;
                break;
            }
            entries += n;
        }
    }

    let mut result = vec![];
    let mut i = 0;
    for range in ranges {
        let level = nesting_levels[i];
        if level <= max_level {
            if level == max_level {
                if entries < range_limit {
                    result.push(range);
                }
                entries += 1;
            } else {
                result.push(range);
            }
        }
        i += 1;
    }

    result
}

fn set_nesting_level(
    index: usize,
    level: usize,
    nesting_levels: &mut Vec<usize>,
    nesting_level_counts: &mut Vec<usize>,
) {
    while nesting_levels.len() < index + 1 {
        nesting_levels.push(0);
    }
    nesting_levels[index] = level;
    if level < 30 {
        while nesting_level_counts.len() < level + 1 {
            nesting_level_counts.push(0);
        }
        nesting_level_counts[level] += 1;
    }
}

#[derive(Default, Clone)]
pub struct FoldingRangeContext {
    pub range_limit: Option<usize>,
}
