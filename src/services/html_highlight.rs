use lsp_textdocument::FullTextDocument;
use lsp_types::{DocumentHighlight, DocumentHighlightKind, Position, Range};

use crate::parser::{
    html_parse::HTMLDocument,
    html_scanner::{Scanner, ScannerState, TokenType},
};

pub async fn find_document_highlights(
    document: &FullTextDocument,
    position: &Position,
    html_document: &HTMLDocument,
) -> Vec<DocumentHighlight> {
    let offset = document.offset_at(*position);
    if let Some(node) = html_document.find_node_at(offset as usize).await {
        let node = node.read().await;

        if node.tag.is_none() {
            return vec![];
        }

        let mut result = vec![];
        let start_tag_range = get_tag_name_range(TokenType::StartTag, document, node.start);
        let end_tag_range = if node.is_self_closing() {
            None
        } else {
            get_tag_name_range(TokenType::EndTag, document, node.end_tag_start.unwrap())
        };

        if start_tag_range.is_some_and(|range| covers(&range, position))
            || end_tag_range.is_some_and(|range| covers(&range, position))
        {
            if let Some(range) = start_tag_range {
                result.push(DocumentHighlight {
                    range,
                    kind: Some(DocumentHighlightKind::READ),
                });
            }
            if let Some(range) = end_tag_range {
                result.push(DocumentHighlight {
                    range,
                    kind: Some(DocumentHighlightKind::READ),
                });
            }
        }

        result
    } else {
        vec![]
    }
}

fn is_before_or_equal(pos1: &Position, pos2: &Position) -> bool {
    pos1.line < pos2.line || (pos1.line == pos2.line && pos1.character <= pos2.character)
}

fn covers(range: &Range, position: &Position) -> bool {
    is_before_or_equal(&range.start, position) && is_before_or_equal(position, &range.end)
}

fn get_tag_name_range(
    token_type: TokenType,
    document: &FullTextDocument,
    start_offset: usize,
) -> Option<Range> {
    let mut scanner = Scanner::new(
        document.get_content(None),
        start_offset,
        ScannerState::WithinContent,
        false,
    );
    let mut token = scanner.scan();
    while token != TokenType::EOS && token != token_type {
        token = scanner.scan();
    }
    if token != TokenType::EOS {
        Some(Range {
            start: document.position_at(scanner.get_token_offset() as u32),
            end: document.position_at(scanner.get_token_end() as u32),
        })
    } else {
        None
    }
}
