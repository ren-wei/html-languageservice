pub mod html_data;
pub mod language_facts;
pub mod parser;
pub mod services;
pub mod utils;

pub use parser::html_parse::parse_html_document;

use std::sync::{Arc, RwLock};

use language_facts::{data_manager::HTMLDataManager, data_provider::IHTMLDataProvider};
use lsp_textdocument::FullTextDocument;
use lsp_types::{ClientCapabilities, CompletionList, Hover, Position};
use parser::html_parse::{HTMLDocument, HTMLParser};
use parser::html_scanner::{Scanner, ScannerState};
use services::html_completion::{
    CompletionConfiguration, DocumentContext, HTMLCompletion, ICompletionParticipant,
};
use services::html_hover::{HTMLHover, HoverSettings};

pub struct LanguageService {
    data_manager: Arc<RwLock<HTMLDataManager>>,
    html_parse: HTMLParser,
    html_completion: HTMLCompletion,
    html_hover: HTMLHover,
}

impl LanguageService {
    pub fn new(
        options: Arc<LanguageServiceOptions>,
        custom_data_providers: Option<Vec<Arc<RwLock<dyn IHTMLDataProvider>>>>,
    ) -> LanguageService {
        let data_manager = Arc::new(RwLock::new(HTMLDataManager::new(
            true,
            custom_data_providers,
        )));
        LanguageService {
            data_manager: Arc::clone(&data_manager),
            html_parse: HTMLParser::new(Arc::clone(&data_manager)),
            html_completion: HTMLCompletion::new(Arc::clone(&options), Arc::clone(&data_manager)),
            html_hover: HTMLHover::new(Arc::clone(&options), Arc::clone(&data_manager)),
        }
    }

    pub fn set_data_providers(
        &mut self,
        built_in: bool,
        providers: Vec<Arc<RwLock<dyn IHTMLDataProvider>>>,
    ) {
        self.data_manager
            .write()
            .unwrap()
            .set_data_providers(built_in, providers);
    }

    pub fn create_scanner(input: &str, initial_offset: usize) -> Scanner {
        Scanner::new(input, initial_offset, ScannerState::WithinContent)
    }

    pub fn parse_html_document(&self, document: &FullTextDocument) -> HTMLDocument {
        self.html_parse.parse_document(document)
    }

    pub fn do_complete(
        &self,
        document: &FullTextDocument,
        position: &Position,
        html_document: &HTMLDocument,
        document_context: impl DocumentContext,
        settings: Option<&CompletionConfiguration>,
    ) -> CompletionList {
        self.html_completion.do_complete(
            document,
            position,
            html_document,
            document_context,
            settings,
        )
    }

    pub fn set_completion_participants(
        &mut self,
        registered_completion_participants: Vec<Arc<dyn ICompletionParticipant>>,
    ) {
        self.html_completion
            .set_completion_participants(registered_completion_participants);
    }

    pub fn do_hover(
        &self,
        document: &FullTextDocument,
        position: &Position,
        html_document: &HTMLDocument,
        options: Option<HoverSettings>,
    ) -> Option<Hover> {
        self.html_hover
            .do_hover(document, position, html_document, options)
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
