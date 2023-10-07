pub mod html_data;
pub mod language_facts;
pub mod parser;

use language_facts::{data_manager::HTMLDataManager, data_provider::IHTMLDataProvider};
use lsp_types::{ClientCapabilities, TextDocumentItem};
use parser::html_parse::{HTMLDocument, HTMLParser};
use parser::html_scanner::{Scanner, ScannerState};

pub struct LanguageService {
    html_parse: HTMLParser,
}

impl LanguageService {
    pub fn new<'a>(options: LanguageServiceOptions) -> LanguageService {
        let data_manager = HTMLDataManager::new(true, options.custom_data_providers);
        LanguageService {
            html_parse: HTMLParser::new(data_manager),
        }
    }

    pub fn create_scanner(input: &str, initial_offset: usize) -> Scanner {
        Scanner::new(input, initial_offset, ScannerState::WithinContent)
    }

    pub fn parse_html_document(&self, document: TextDocumentItem) -> HTMLDocument {
        self.html_parse.parse_document(document)
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
    pub custom_data_providers: Option<Vec<Box<dyn IHTMLDataProvider>>>,

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

pub trait FileSystemProvider {
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
