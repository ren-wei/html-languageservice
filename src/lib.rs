//! # HTMLLanguageService
//!
//! The basics of an HTML language server.
//!
//! # Examples
//!
//! ```rust
//! use html_languageservice::{
//!     parse_html_document, HTMLDataManager, HTMLLanguageService, HTMLLanguageServiceOptions,
//! };
//! use lsp_textdocument::FullTextDocument;
//! use lsp_types::Position;
//!
//! #[tokio::main]
//! async fn main() {
//!     // prepare
//!     let document = FullTextDocument::new("html".to_string(), 1, "<div></div>".to_string());
//!     let position = Position::new(0, 1);
//!     // hover
//!     let data_manager = HTMLDataManager::new(true, None);
//!     let html_document = parse_html_document(
//!         document.get_content(None),
//!         document.language_id(),
//!         &data_manager,
//!     )
//!     .await;
//!     let ls = HTMLLanguageService::new(HTMLLanguageServiceOptions::default());
//!     let result = ls
//!         .do_hover(&document, &position, &html_document, None, &data_manager)
//!         .await;
//!     assert!(result.is_some());
//! }
//! ```

mod beautify;
pub mod html_data;
pub mod language_facts;
pub mod parser;
pub mod participant;
pub mod services;
mod utils;

pub use language_facts::data_manager::HTMLDataManager;
pub use parser::html_parse::parse_html_document;
use participant::{ICompletionParticipant, IHoverParticipant};

use lsp_textdocument::FullTextDocument;
use lsp_types::{
    ClientCapabilities, CompletionList, DocumentHighlight, Hover, Position, Range, TextEdit,
};
use parser::html_parse::{HTMLDocument, HTMLParser};
use parser::html_scanner::{Scanner, ScannerState};
use services::html_completion::{CompletionConfiguration, DocumentContext, HTMLCompletion};
use services::html_formatter::{format, HTMLFormatConfiguration};
use services::html_highlight::find_document_highlights;
use services::html_hover::{HTMLHover, HoverSettings};

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

    pub async fn parse_html_document(
        document: &FullTextDocument,
        data_manager: &HTMLDataManager,
    ) -> HTMLDocument {
        HTMLParser::parse_document(document, data_manager).await
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

    pub async fn do_quote_complete(
        document: &FullTextDocument,
        position: &Position,
        html_document: &HTMLDocument,
        settings: Option<&CompletionConfiguration>,
    ) -> Option<String> {
        HTMLCompletion::do_quote_complete(document, position, html_document, settings).await
    }

    pub async fn do_tag_complete(
        &self,
        document: &FullTextDocument,
        position: &Position,
        html_document: &HTMLDocument,
        data_manager: &HTMLDataManager,
    ) -> Option<String> {
        self.html_completion
            .do_tag_complete(document, position, html_document, data_manager)
            .await
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
    pub async fn format(
        document: &FullTextDocument,
        range: Option<Range>,
        options: &HTMLFormatConfiguration,
    ) -> Vec<TextEdit> {
        format(document, &range, options).await
    }

    pub async fn find_document_highlights(
        document: &FullTextDocument,
        position: &Position,
        html_document: &HTMLDocument,
    ) -> Vec<DocumentHighlight> {
        find_document_highlights(document, position, html_document).await
    }
}

#[derive(Default)]
pub struct HTMLLanguageServiceOptions {
    /**
     * Unless set to false, the default HTML data provider will be used
     * along with the providers from customDataProviders.
     * Defaults to true.
     */
    pub use_default_data_provider: Option<bool>,

    /**
     * Provide data that could enhance the service's understanding of
     * HTML tag / attribute / attribute-value
     */
    // pub custom_data_providers: Option<Vec<Box<dyn IHTMLDataProvider>>>,

    /**
     * Abstract file system access away from the service.
     * Used for path completion, etc.
     */
    pub file_system_provider: Option<Box<dyn FileSystemProvider>>,

    /**
     * Describes the LSP capabilities the client supports.
     */
    pub client_capabilities: Option<ClientCapabilities>,
}

pub trait FileSystemProvider: Send + Sync {
    fn stat(&self, uri: DocumentUri) -> FileStat;

    fn read_directory(&self, uri: DocumentUri) -> (String, FileType);
}

pub type DocumentUri = String;

pub struct FileStat {
    /// The type of the file, e.g. is a regular file, a directory, or symbolic link
    /// to a file.
    pub file_type: FileType,
    /// The creation timestamp in milliseconds elapsed since January 1, 1970 00:00:00 UTC.
    pub ctime: i128,
    /// The modification timestamp in milliseconds elapsed since January 1, 1970 00:00:00 UTC.
    pub mtime: i128,
    /// The size in bytes.
    pub size: usize,
}

pub enum FileType {
    /// The file type is unknown.
    Unknown = 0,
    /// A regular file.
    File = 1,
    /// A directory.
    Directory = 2,
    /// A symbolic link to a file.
    SymbolicLink = 64,
}
