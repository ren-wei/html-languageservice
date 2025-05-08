use std::{fs, time};

use html_languageservice::{HTMLDataManager, HTMLLanguageService, HTMLLanguageServiceOptions};
use lsp_textdocument::FullTextDocument;

fn main() {
    let content = fs::read_to_string("examples/parse-speed/index.html").unwrap();

    let document = FullTextDocument::new("html".to_string(), 0, content);

    let start_time = time::SystemTime::now();
    let ls = HTMLLanguageService::new(&HTMLLanguageServiceOptions::default());
    ls.parse_html_document(&document, &HTMLDataManager::default());
    let end_time = time::SystemTime::now();
    let duration = end_time.duration_since(start_time).unwrap();
    println!("{:?}", duration);
}
