use crate::html_language_types::HTMLLanguageServiceOptions;
use crate::parser::html_document::HTMLDocument;
use crate::parser::html_parse::HTMLParser;
use crate::parser::html_scanner::{Scanner, ScannerState};
use crate::participant::{ICompletionParticipant, IHoverParticipant};
use crate::services::html_completion::HTMLCompletion;
use crate::services::html_folding;
#[cfg(feature = "experimental")]
use crate::services::html_formatter;
use crate::services::html_highlight;
use crate::services::html_hover::HTMLHover;
use crate::services::html_links;
use crate::services::html_symbols;
#[cfg(feature = "experimental")]
use crate::HTMLFormatConfiguration;
use crate::{
    CompletionConfiguration, DocumentContext, FoldingRangeContext, HTMLDataManager, HoverSettings,
};
use lsp_types::{
    CompletionList, DocumentHighlight, DocumentLink, DocumentSymbol, FoldingRange, Hover, Position,
    SymbolInformation, Url,
};
#[cfg(feature = "experimental")]
use lsp_types::{Range, TextEdit};

use lsp_textdocument::FullTextDocument;

pub struct HTMLLanguageService {
    html_completion: HTMLCompletion,
    html_hover: HTMLHover,
}

impl HTMLLanguageService {
    pub fn new(options: HTMLLanguageServiceOptions) -> HTMLLanguageService {
        HTMLLanguageService {
            html_completion: HTMLCompletion::new(&options),
            html_hover: HTMLHover::new(&options),
        }
    }

    pub fn create_scanner(input: &str, initial_offset: usize) -> Scanner {
        Scanner::new(input, initial_offset, ScannerState::WithinContent, false)
    }

    pub fn parse_html_document(
        document: &FullTextDocument,
        data_manager: &HTMLDataManager,
    ) -> HTMLDocument {
        HTMLParser::parse_document(document, data_manager)
    }

    pub async fn do_complete(
        &self,
        document: &FullTextDocument,
        position: &Position,
        html_document: &HTMLDocument,
        document_context: impl DocumentContext,
        settings: Option<&CompletionConfiguration>,
        data_manager: &HTMLDataManager,
    ) -> CompletionList {
        self.html_completion
            .do_complete(
                document,
                position,
                html_document,
                document_context,
                settings,
                data_manager,
            )
            .await
    }

    pub fn set_completion_participants(
        &mut self,
        completion_participants: Vec<Box<dyn ICompletionParticipant>>,
    ) {
        self.html_completion
            .set_completion_participants(completion_participants);
    }

    pub fn do_quote_complete(
        document: &FullTextDocument,
        position: &Position,
        html_document: &HTMLDocument,
        settings: Option<&CompletionConfiguration>,
    ) -> Option<String> {
        HTMLCompletion::do_quote_complete(document, position, html_document, settings)
    }

    pub fn do_tag_complete(
        &self,
        document: &FullTextDocument,
        position: &Position,
        html_document: &HTMLDocument,
        data_manager: &HTMLDataManager,
    ) -> Option<String> {
        self.html_completion
            .do_tag_complete(document, position, html_document, data_manager)
    }

    pub async fn do_hover(
        &self,
        document: &FullTextDocument,
        position: &Position,
        html_document: &HTMLDocument,
        options: Option<HoverSettings>,
        data_manager: &HTMLDataManager,
    ) -> Option<Hover> {
        self.html_hover
            .do_hover(document, position, html_document, options, data_manager)
            .await
    }

    pub fn set_hover_participants(&mut self, hover_participants: Vec<Box<dyn IHoverParticipant>>) {
        self.html_hover.set_hover_participants(hover_participants);
    }

    /// Note: `format` is not prefect, it's under development
    #[cfg(feature = "experimental")]
    pub fn format(
        document: &FullTextDocument,
        range: Option<Range>,
        options: &HTMLFormatConfiguration,
    ) -> Vec<TextEdit> {
        html_formatter::format(document, &range, options)
    }

    pub fn find_document_highlights(
        document: &FullTextDocument,
        position: &Position,
        html_document: &HTMLDocument,
    ) -> Vec<DocumentHighlight> {
        html_highlight::find_document_highlights(document, position, html_document)
    }

    pub fn find_document_links(
        uri: &Url,
        document: &FullTextDocument,
        document_context: &impl DocumentContext,
        data_manager: &HTMLDataManager,
    ) -> Vec<DocumentLink> {
        html_links::find_document_links(uri, document, document_context, data_manager)
    }

    pub fn find_document_symbols(
        uri: &Url,
        document: &FullTextDocument,
        html_document: &HTMLDocument,
    ) -> Vec<SymbolInformation> {
        html_symbols::find_document_symbols(uri, document, html_document)
    }

    pub fn find_document_symbols2(
        document: &FullTextDocument,
        html_document: &HTMLDocument,
    ) -> Vec<DocumentSymbol> {
        html_symbols::find_document_symbols2(document, html_document)
    }

    pub fn get_folding_ranges(
        document: FullTextDocument,
        context: FoldingRangeContext,
        data_manager: &HTMLDataManager,
    ) -> Vec<FoldingRange> {
        html_folding::get_folding_ranges(document, context, data_manager)
    }
}
