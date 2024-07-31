# html-languageservice

The basics of an HTML language server.

## Features

- [x] customize data providers
- [x] parse html document
- [x] scanner
- [x] complete
- [x] hover
- [x] format
- [x] findDocumentHighlights
- [x] findDocumentLinks
- [x] findDocumentSymbols
- [x] getFoldingRanges
- [x] getSelectionRanges
- [x] quoteComplete
- [x] tagComplete
- [ ] rename
- [ ] findMatchingTagPosition
- [ ] findLinkedEditingRanges

## Example

```rust
use std::sync::Arc;

use html_languageservice::{
    services::html_completion::DefaultDocumentContext, LanguageService, LanguageServiceOptions,
};
use lsp_textdocument::FullTextDocument;
use lsp_types::Position;

#[tokio::main]
async fn main() {
    // prepare
    let document = FullTextDocument::new("html".to_string(), 1, "<div></div>".to_string());
    let position = Position::new(0, 1);
    // init
    let data_manager = HTMLDataManager::new(true, None);
    let html_document = HTMLLanguageService::parse_html_document(&document, &data_manager);
    let ls = HTMLLanguageService::new(HTMLLanguageServiceOptions::default());
    // hover
    let hover = ls
        .do_hover(&document, &position, &html_document, None, &data_manager)
        .await;
    println!("hover: {:#?}", hover);
    // complete
    let document_context = DefaultDocumentContext;
    let completion_list = ls
        .do_complete(
            &document,
            &position,
            &html_document,
            document_context,
            None,
            &data_manager,
        )
        .await;
    println!("completion_list: {:#?}", completion_list);
}
```
