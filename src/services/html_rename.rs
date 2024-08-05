use std::collections::HashMap;

use lsp_textdocument::FullTextDocument;
use lsp_types::{Position, Range, TextEdit, Url, WorkspaceEdit};

use crate::parser::html_document::{HTMLDocument, Node};

pub fn do_rename(
    uri: Url,
    document: &FullTextDocument,
    position: Position,
    new_name: &str,
    html_document: &HTMLDocument,
) -> Option<WorkspaceEdit> {
    let offset = document.offset_at(position) as usize;
    let node = html_document.find_node_at(offset, &mut vec![])?;

    let tag = node.tag.as_ref()?;

    if !is_within_tag_range(node, offset, tag) {
        return None;
    }

    let mut edits = vec![];

    let start_tag_range = Range::new(
        document.position_at((node.start + "<".len()) as u32),
        document.position_at((node.start + "<".len() + tag.len()) as u32),
    );

    edits.push(TextEdit::new(start_tag_range, new_name.to_string()));

    if let Some(end_tag_start) = node.end_tag_start {
        let end_tag_range = Range::new(
            document.position_at((end_tag_start + "</".len()) as u32),
            document.position_at((end_tag_start + "</".len() + tag.len()) as u32),
        );
        edits.push(TextEdit::new(end_tag_range, new_name.to_string()));
    }

    let changes: HashMap<Url, Vec<TextEdit>> = HashMap::from([(uri, edits)]);

    Some(WorkspaceEdit::new(changes))
}

fn is_within_tag_range(node: &Node, offset: usize, tag: &str) -> bool {
    // Self-closing tag
    if let Some(end_tag_start) = node.end_tag_start {
        if end_tag_start + "</".len() <= offset && offset <= end_tag_start + "</".len() + tag.len()
        {
            return true;
        }
    }

    node.start + "<".len() <= offset && offset <= node.start + "<".len() + tag.len()
}
