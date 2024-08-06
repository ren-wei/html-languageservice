use crate::html_language_types::HTMLLanguageServiceOptions;
use crate::parser::html_document::HTMLDocument;
use crate::parser::html_parse::HTMLParser;
use crate::parser::html_scanner::{Scanner, ScannerState};
#[cfg(feature = "completion")]
use crate::participant::ICompletionParticipant;
use crate::participant::IHoverParticipant;
#[cfg(feature = "completion")]
use crate::services::html_completion::HTMLCompletion;
#[cfg(feature = "folding")]
use crate::services::html_folding;
#[cfg(feature = "formatter")]
use crate::services::html_formatter;
use crate::services::html_hover::HTMLHover;
use crate::services::html_selection_range;
use crate::services::{html_highlight, html_rename};
use crate::services::{html_linked_editing, html_symbols};
use crate::services::{html_links, html_matching_tag_position};

#[cfg(feature = "formatter")]
use crate::HTMLFormatConfiguration;

#[cfg(feature = "completion")]
use crate::CompletionConfiguration;
#[cfg(feature = "folding")]
use crate::FoldingRangeContext;
use crate::{DocumentContext, HTMLDataManager, HoverSettings};

#[cfg(feature = "completion")]
use lsp_types::CompletionList;
#[cfg(feature = "folding")]
use lsp_types::FoldingRange;
#[cfg(feature = "formatter")]
use lsp_types::TextEdit;
use lsp_types::{
    DocumentHighlight, DocumentLink, DocumentSymbol, Hover, Position, Range, SelectionRange,
    SymbolInformation, Url, WorkspaceEdit,
};

use lsp_textdocument::FullTextDocument;

pub struct HTMLLanguageService {
    #[cfg(feature = "formatter")]
    html_completion: HTMLCompletion,
    html_hover: HTMLHover,
}

impl HTMLLanguageService {
    pub fn new(options: &HTMLLanguageServiceOptions) -> HTMLLanguageService {
        HTMLLanguageService {
            #[cfg(feature = "formatter")]
            html_completion: HTMLCompletion::new(options),
            html_hover: HTMLHover::new(options),
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

    #[cfg(feature = "formatter")]
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

    #[cfg(feature = "completion")]
    pub fn set_completion_participants(
        &mut self,
        completion_participants: Vec<Box<dyn ICompletionParticipant>>,
    ) {
        self.html_completion
            .set_completion_participants(completion_participants);
    }

    #[cfg(feature = "formatter")]
    pub fn do_quote_complete(
        document: &FullTextDocument,
        position: &Position,
        html_document: &HTMLDocument,
        settings: Option<&CompletionConfiguration>,
    ) -> Option<String> {
        HTMLCompletion::do_quote_complete(document, position, html_document, settings)
    }

    #[cfg(feature = "completion")]
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
    #[cfg(feature = "formatter")]
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

    #[cfg(feature = "folding")]
    pub fn get_folding_ranges(
        document: FullTextDocument,
        context: FoldingRangeContext,
        data_manager: &HTMLDataManager,
    ) -> Vec<FoldingRange> {
        html_folding::get_folding_ranges(document, context, data_manager)
    }

    pub fn get_selection_ranges(
        document: &FullTextDocument,
        positions: &Vec<Position>,
        html_document: &HTMLDocument,
    ) -> Vec<SelectionRange> {
        html_selection_range::get_selection_ranges(document, positions, html_document)
    }

    pub fn do_rename(
        uri: Url,
        document: &FullTextDocument,
        position: Position,
        new_name: &str,
        html_document: &HTMLDocument,
    ) -> Option<WorkspaceEdit> {
        html_rename::do_rename(uri, document, position, new_name, html_document)
    }

    pub fn find_matching_tag_position(
        document: &FullTextDocument,
        position: Position,
        html_document: &HTMLDocument,
    ) -> Option<Position> {
        html_matching_tag_position::find_matching_tag_position(document, position, html_document)
    }

    pub fn find_linked_editing_ranges(
        document: &FullTextDocument,
        position: Position,
        html_document: &HTMLDocument,
    ) -> Option<Vec<Range>> {
        html_linked_editing::find_linked_editing_ranges(document, position, html_document)
    }
}
