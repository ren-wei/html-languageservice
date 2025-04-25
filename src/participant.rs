use lsp_textdocument::FullTextDocument;
use lsp_types::{CompletionItem, Hover, Position, Range};

use crate::parser::html_document::HTMLDocument;

pub trait ICompletionParticipant: Send + Sync {
    fn on_html_attribute_value(&self, context: HtmlAttributeValueContext) -> Vec<CompletionItem>;
    fn on_html_content(&self, context: HtmlContentContext) -> Vec<CompletionItem>;
}

pub trait IHoverParticipant: Send + Sync {
    fn on_html_attribute_value<'a>(
        &'a self,
        context: HtmlAttributeValueContext<'a>,
    ) -> Option<Hover>;
    fn on_html_content<'a>(&'a self, context: HtmlContentContext<'a>) -> Option<Hover>;
}

pub struct HtmlAttributeValueContext<'a> {
    pub document: &'a FullTextDocument,
    pub html_document: &'a HTMLDocument,
    pub position: Position,
    pub tag: &'a str,
    pub attribute: &'a str,
    pub value: &'a str,
    pub range: Range,
}

pub struct HtmlContentContext<'a> {
    pub document: &'a FullTextDocument,
    pub html_document: &'a HTMLDocument,
    pub position: Position,
}
