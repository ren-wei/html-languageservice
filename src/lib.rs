pub mod html_data;
pub mod language_facts;
pub mod parser;
pub mod participant;
pub mod services;
mod utils;

pub use language_facts::data_manager::HTMLDataManager;
pub use parser::html_parse::parse_html_document;
use participant::{ICompletionParticipant, IHoverParticipant};
use tokio::sync::RwLock;

use std::sync::Arc;

use lsp_textdocument::FullTextDocument;
use lsp_types::{ClientCapabilities, CompletionList, Hover, Position};
use parser::html_parse::{HTMLDocument, HTMLParser};
use parser::html_scanner::{Scanner, ScannerState};
use services::html_completion::{CompletionConfiguration, DocumentContext, HTMLCompletion};
use services::html_hover::{HTMLHover, HoverSettings};

pub struct LanguageService {
    html_completion: HTMLCompletion,
    html_hover: HTMLHover,
}

impl LanguageService {
    pub fn new(options: LanguageServiceOptions) -> LanguageService {
        LanguageService {
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
        completion_participants: Vec<Arc<RwLock<dyn ICompletionParticipant>>>,
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

    pub fn set_hover_participants(
        &mut self,
        hover_participants: Vec<Arc<RwLock<dyn IHoverParticipant>>>,
    ) {
        self.html_hover.set_hover_participants(hover_participants);
    }
}

#[derive(Default)]
pub struct LanguageServiceOptions {
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
