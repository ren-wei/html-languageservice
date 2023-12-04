use lsp_textdocument::FullTextDocument;
use lsp_types::{CompletionItem, Hover, Position, Range};

pub trait ICompletionParticipant: Send + Sync {
    fn on_html_attribute_value(&self, context: HtmlAttributeValueContext) -> Vec<CompletionItem>;
    fn on_html_content(&self, context: HtmlContentContext) -> Vec<CompletionItem>;
}

pub trait IHoverParticipant: Send + Sync {
    fn on_html_attribute_value(&self, context: HtmlAttributeValueContext) -> Option<Hover>;
    fn on_html_content(&self, context: HtmlContentContext) -> Option<Hover>;
}

pub struct HtmlAttributeValueContext<'a> {
    pub document: &'a FullTextDocument,
    pub position: &'a Position,
    pub tag: String,
    pub attribute: String,
    pub value: String,
    pub range: Range,
}

pub struct HtmlContentContext<'a> {
    pub document: &'a FullTextDocument,
    pub position: &'a Position,
}
