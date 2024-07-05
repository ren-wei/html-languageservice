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
//!     // hover
//!     let data_manager = HTMLDataManager::new(true, None);
//!     let html_document = HTMLLanguageService::parse_html_document(&document, &data_manager);
//!     let ls = HTMLLanguageService::new(HTMLLanguageServiceOptions::default());
//!     let result = ls
//!         .do_hover(&document, &position, &html_document, None, &data_manager)
//!         .await;
//!     assert!(result.is_some());
//! }
//! ```

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

pub use services::html_completion::{
    CompletionConfiguration, DefaultDocumentContext, DocumentContext, Quotes,
};

pub use services::html_folding::FoldingRangeContext;

pub use services::html_formatter::HTMLFormatConfiguration;
pub use services::html_hover::HoverSettings;

pub use html_language_service::HTMLLanguageService;
pub use html_language_types::{FileStat, FileSystemProvider, FileType, HTMLLanguageServiceOptions};
