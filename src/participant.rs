use async_trait::async_trait;
use lsp_textdocument::FullTextDocument;
use lsp_types::{CompletionItem, Hover, Position, Range};

use crate::parser::html_document::HTMLDocument;

#[async_trait]
pub trait ICompletionParticipant: Send + Sync {
    async fn on_html_attribute_value(
        &self,
        context: HtmlAttributeValueContext,
    ) -> Vec<CompletionItem>;
    async fn on_html_content(&self, context: HtmlContentContext) -> Vec<CompletionItem>;
}

#[async_trait]
pub trait IHoverParticipant: Send + Sync {
    async fn on_html_attribute_value(&self, context: HtmlAttributeValueContext) -> Option<Hover>;
    async fn on_html_content(&self, context: HtmlContentContext) -> Option<Hover>;
}

pub struct HtmlAttributeValueContext {
    pub document: FullTextDocument,
    pub html_document: HTMLDocument,
    pub position: Position,
    pub tag: String,
    pub attribute: String,
    pub value: String,
    pub range: Range,
}

pub struct HtmlContentContext {
    pub document: FullTextDocument,
    pub html_document: HTMLDocument,
    pub position: Position,
}
