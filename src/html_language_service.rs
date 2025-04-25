#[cfg(any(feature = "completion", feature = "hover"))]
use crate::html_language_types::HTMLLanguageServiceOptions;
use crate::parser::html_document::HTMLDocument;
use crate::parser::html_parse::HTMLParser;
use crate::parser::html_scanner::{Scanner, ScannerState};
#[cfg(feature = "completion")]
use crate::participant::ICompletionParticipant;
#[cfg(feature = "hover")]
use crate::participant::IHoverParticipant;
#[cfg(feature = "completion")]
use crate::services::html_completion::HTMLCompletion;
#[cfg(feature = "folding")]
use crate::services::html_folding;
#[cfg(feature = "formatter")]
use crate::services::html_formatter;
#[cfg(feature = "highlight")]
use crate::services::html_highlight;
#[cfg(feature = "hover")]
use crate::services::html_hover::HTMLHover;
#[cfg(feature = "linked_editing")]
use crate::services::html_linked_editing;
#[cfg(feature = "links")]
use crate::services::html_links;
#[cfg(feature = "matching_tag_position")]
use crate::services::html_matching_tag_position;
#[cfg(feature = "rename")]
use crate::services::html_rename;
#[cfg(feature = "selection_range")]
use crate::services::html_selection_range;
#[cfg(feature = "symbols")]
use crate::services::html_symbols;

#[cfg(feature = "formatter")]
use crate::HTMLFormatConfiguration;

#[cfg(feature = "completion")]
use crate::CompletionConfiguration;
#[cfg(any(feature = "completion", feature = "links"))]
use crate::DocumentContext;
#[cfg(feature = "folding")]
use crate::FoldingRangeContext;
use crate::HTMLDataManager;
#[cfg(feature = "hover")]
use crate::HoverSettings;

#[cfg(feature = "completion")]
use lsp_types::CompletionList;
#[cfg(feature = "highlight")]
use lsp_types::DocumentHighlight;
#[cfg(feature = "links")]
use lsp_types::DocumentLink;
#[cfg(feature = "folding")]
use lsp_types::FoldingRange;
#[cfg(feature = "hover")]
use lsp_types::Hover;
#[cfg(any(
    feature = "formatter",
    feature = "completion",
    feature = "hover",
    feature = "highlight",
    feature = "selection_range",
    feature = "rename",
    feature = "matching_tag_position",
    feature = "linked_editing"
))]
use lsp_types::Position;
#[cfg(any(feature = "formatter", feature = "linked_editing"))]
use lsp_types::Range;
#[cfg(feature = "selection_range")]
use lsp_types::SelectionRange;
#[cfg(feature = "formatter")]
use lsp_types::TextEdit;
#[cfg(any(feature = "links", feature = "symbols", feature = "rename"))]
use lsp_types::Uri;
#[cfg(feature = "rename")]
use lsp_types::WorkspaceEdit;
#[cfg(feature = "symbols")]
use lsp_types::{DocumentSymbol, SymbolInformation};

use lsp_textdocument::FullTextDocument;

/// This is a collection of features necessary to implement an HTML language server
///
/// Make sure you activated the features you need of the `html-languageservice` crate on `Cargo.toml`
///
/// # Features
///
/// - completion
/// - hover
/// - formatter
/// - highlight
/// - links
/// - symbols
/// - folding
/// - selection_range
/// - rename
/// - matching_tag_position
/// - linked_editing
pub struct HTMLLanguageService {
    #[cfg(feature = "completion")]
    html_completion: HTMLCompletion,
    #[cfg(feature = "hover")]
    html_hover: HTMLHover,
}

impl HTMLLanguageService {
    #[cfg(any(feature = "completion", feature = "hover"))]
    pub fn new(options: &HTMLLanguageServiceOptions) -> HTMLLanguageService {
        HTMLLanguageService {
            #[cfg(feature = "completion")]
            html_completion: HTMLCompletion::new(options),
            #[cfg(feature = "hover")]
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

    /// Provide completion proposals for a given location
    #[cfg(feature = "completion")]
    pub fn do_complete(
        &self,
        document: &FullTextDocument,
        position: &Position,
        html_document: &HTMLDocument,
        document_context: impl DocumentContext,
        settings: Option<&CompletionConfiguration>,
        data_manager: &HTMLDataManager,
    ) -> CompletionList {
        self.html_completion.do_complete(
            document,
            position,
            html_document,
            document_context,
            settings,
            data_manager,
        )
    }

    /// Add additional completion items to the completion proposal
    #[cfg(feature = "completion")]
    pub fn set_completion_participants(
        &mut self,
        completion_participants: Vec<Box<dyn ICompletionParticipant>>,
    ) {
        self.html_completion
            .set_completion_participants(completion_participants);
    }

    /// Provide quotes completion when `=` is entered
    #[cfg(feature = "completion")]
    pub fn do_quote_complete(
        document: &FullTextDocument,
        position: &Position,
        html_document: &HTMLDocument,
        settings: Option<&CompletionConfiguration>,
    ) -> Option<String> {
        HTMLCompletion::do_quote_complete(document, position, html_document, settings)
    }

    /// Completes the tag when `>` or `/` is entered
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

    /// Provides hover information at a given location
    #[cfg(feature = "hover")]
    pub fn do_hover(
        &self,
        document: &FullTextDocument,
        position: &Position,
        html_document: &HTMLDocument,
        options: Option<HoverSettings>,
        data_manager: &HTMLDataManager,
    ) -> Option<Hover> {
        self.html_hover
            .do_hover(document, position, html_document, options, data_manager)
    }

    /// Add additional hover to the hover proposal
    #[cfg(feature = "hover")]
    pub fn set_hover_participants(&mut self, hover_participants: Vec<Box<dyn IHoverParticipant>>) {
        self.html_hover.set_hover_participants(hover_participants);
    }

    /// Formats the code at the given range
    ///
    /// Note: `format` is not prefect, it's under development
    #[cfg(feature = "formatter")]
    pub fn format(
        document: &FullTextDocument,
        range: Option<Range>,
        options: &HTMLFormatConfiguration,
    ) -> Vec<TextEdit> {
        html_formatter::format(document, &range, options)
    }

    /// Provides document highlights capability
    #[cfg(feature = "highlight")]
    pub fn find_document_highlights(
        document: &FullTextDocument,
        position: &Position,
        html_document: &HTMLDocument,
    ) -> Vec<DocumentHighlight> {
        html_highlight::find_document_highlights(document, position, html_document)
    }

    /// Finds all links in the document
    #[cfg(feature = "links")]
    pub fn find_document_links(
        uri: &Uri,
        document: &FullTextDocument,
        document_context: &impl DocumentContext,
        data_manager: &HTMLDataManager,
    ) -> Vec<DocumentLink> {
        html_links::find_document_links(uri, document, document_context, data_manager)
    }

    /// Finds all the symbols in the document, it returns `SymbolInformation`
    #[cfg(feature = "symbols")]
    pub fn find_document_symbols(
        uri: &Uri,
        document: &FullTextDocument,
        html_document: &HTMLDocument,
    ) -> Vec<SymbolInformation> {
        html_symbols::find_document_symbols(uri, document, html_document)
    }

    /// Finds all the symbols in the document, it returns `DocumentSymbol`
    #[cfg(feature = "symbols")]
    pub fn find_document_symbols2(
        document: &FullTextDocument,
        html_document: &HTMLDocument,
    ) -> Vec<DocumentSymbol> {
        html_symbols::find_document_symbols2(document, html_document)
    }

    /// Get folding ranges for the given document
    #[cfg(feature = "folding")]
    pub fn get_folding_ranges(
        document: FullTextDocument,
        context: FoldingRangeContext,
        data_manager: &HTMLDataManager,
    ) -> Vec<FoldingRange> {
        html_folding::get_folding_ranges(document, context, data_manager)
    }

    /// Get the selection ranges for the given document
    #[cfg(feature = "selection_range")]
    pub fn get_selection_ranges(
        document: &FullTextDocument,
        positions: &Vec<Position>,
        html_document: &HTMLDocument,
    ) -> Vec<SelectionRange> {
        html_selection_range::get_selection_ranges(document, positions, html_document)
    }

    /// Rename the matching tag
    #[cfg(feature = "rename")]
    pub fn do_rename(
        uri: Uri,
        document: &FullTextDocument,
        position: Position,
        new_name: &str,
        html_document: &HTMLDocument,
    ) -> Option<WorkspaceEdit> {
        html_rename::do_rename(uri, document, position, new_name, html_document)
    }

    /// Get the location of the matching tag
    #[cfg(feature = "matching_tag_position")]
    pub fn find_matching_tag_position(
        document: &FullTextDocument,
        position: Position,
        html_document: &HTMLDocument,
    ) -> Option<Position> {
        html_matching_tag_position::find_matching_tag_position(document, position, html_document)
    }

    /// Provides linked editing range capability
    #[cfg(feature = "linked_editing")]
    pub fn find_linked_editing_ranges(
        document: &FullTextDocument,
        position: Position,
        html_document: &HTMLDocument,
    ) -> Option<Vec<Range>> {
        html_linked_editing::find_linked_editing_ranges(document, position, html_document)
    }
}
