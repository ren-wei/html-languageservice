use lsp_textdocument::FullTextDocument;
use lsp_types::Position;

use crate::parser::html_document::HTMLDocument;

pub fn find_matching_tag_position(
    document: &FullTextDocument,
    position: Position,
    html_document: &HTMLDocument,
) -> Option<Position> {
    let offset = document.offset_at(position) as usize;
    let node = html_document.find_node_at(offset, &mut vec![])?;

    let tag = node.tag.as_ref()?;

    let end_tag_start = node.end_tag_start?;

    // Within open tag, compute close tag
    if node.start + "<".len() <= offset && offset <= node.start + "<".len() + tag.len() {
        let mirror_offset = (offset - "<".len() - node.start) + end_tag_start + "</".len();
        return Some(document.position_at(mirror_offset as u32));
    }

    // Within closing tag, compute open tag
    if end_tag_start + "</".len() <= offset && offset <= end_tag_start + "</".len() + tag.len() {
        let mirror_offset = (offset - "</".len() - end_tag_start) + node.start + "<".len();
        return Some(document.position_at(mirror_offset as u32));
    }

    None
}
