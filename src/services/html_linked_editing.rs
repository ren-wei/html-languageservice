use lsp_textdocument::FullTextDocument;
use lsp_types::{Position, Range};

use crate::parser::html_document::HTMLDocument;

pub fn find_linked_editing_ranges(
    document: &FullTextDocument,
    position: Position,
    html_document: &HTMLDocument,
) -> Option<Vec<Range>> {
    let offset = document.offset_at(position) as usize;
    let node = html_document.find_node_at(offset, &mut vec![])?;

    let tag_len = if let Some(tag) = &node.tag {
        tag.len()
    } else {
        0
    };

    let end_tag_start = node.end_tag_start?;

    if (node.start + "<".len() <= offset && offset <= node.start + "<".len() + tag_len)
        || (end_tag_start + "</".len() <= offset && offset <= end_tag_start + "</".len() + tag_len)
    {
        Some(vec![
            Range::new(
                document.position_at((node.start + "<".len()) as u32),
                document.position_at((node.start + "<".len() + tag_len) as u32),
            ),
            Range::new(
                document.position_at((end_tag_start + "</".len()) as u32),
                document.position_at((end_tag_start + "</".len() + tag_len) as u32),
            ),
        ])
    } else {
        None
    }
}
