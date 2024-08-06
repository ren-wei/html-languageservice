//! # HTMLLanguageService
//!
//! The basics of an HTML language server.
//!
//! [HTMLLanguageService]
//!
//! # Examples
//!
//! ```rust
//! use html_languageservice::{HTMLDataManager, HTMLLanguageService, HTMLLanguageServiceOptions};
//! use lsp_textdocument::FullTextDocument;
//! use lsp_types::Position;
//!
//! #[tokio::main]
//! async fn main() {
//!     // prepare
//!     let document = FullTextDocument::new("html".to_string(), 1, "<div></div>".to_string());
//!     let position = Position::new(0, 1);
//!     // parse_html_document
//!     let data_manager = HTMLDataManager::new(true, None);
//!     let html_document = HTMLLanguageService::parse_html_document(&document, &data_manager);
//!     assert!(html_document.roots.len() > 0);
//! }
//! ```

#[cfg(feature = "formatter")]
mod beautify;
pub mod html_data;
mod html_language_service;
mod html_language_types;
pub mod language_facts;
pub mod parser;
pub mod participant;
mod services;
mod utils;

pub use language_facts::data_manager::HTMLDataManager;
pub use parser::html_parse::parse_html_document;

#[cfg(feature = "completion")]
pub use services::html_completion::{CompletionConfiguration, DefaultDocumentContext, Quotes};

#[cfg(feature = "folding")]
pub use services::html_folding::FoldingRangeContext;

#[cfg(feature = "formatter")]
pub use services::html_formatter::HTMLFormatConfiguration;
#[cfg(feature = "hover")]
pub use services::html_hover::HoverSettings;

pub use html_language_service::HTMLLanguageService;
pub use html_language_types::{
    DocumentContext, FileStat, FileSystemProvider, FileType, HTMLLanguageServiceOptions,
};
