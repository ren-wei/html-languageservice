use html_languageservice::{HTMLDataManager, HTMLLanguageService, HTMLLanguageServiceOptions};
use lsp_textdocument::FullTextDocument;
use lsp_types::Position;

#[tokio::main]
async fn main() {
    // prepare
    let document = FullTextDocument::new("html".to_string(), 1, "<div></div>".to_string());
    let position = Position::new(0, 1);
    // hover
    let data_manager = HTMLDataManager::new(true, None);
    let html_document = HTMLLanguageService::parse_html_document(&document, &data_manager);
    let ls = HTMLLanguageService::new(HTMLLanguageServiceOptions::default());
    let result = ls
        .do_hover(&document, &position, &html_document, None, &data_manager)
        .await;
    assert!(result.is_some());
}
