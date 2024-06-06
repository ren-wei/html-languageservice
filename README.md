# html-languageservice

The basics of an HTML language server.

## Features

- [x] customize data providers
- [x] parse html document
- [x] scanner
- [x] complete
- [x] hover
- [x] format
- [ ] findDocumentHighlights
- [ ] findDocumentLinks
- [ ] findDocumentSymbols
- [ ] getFoldingRanges
- [ ] getSelectionRanges
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
    // init
    let options = Arc::new(LanguageServiceOptions::default());
    let language_service = LanguageService::new(options, None);
    // prepare
    let document = FullTextDocument::new("html".to_string(), 1, "<div></div>".to_string());
    let html_document = language_service.parse_html_document(&document).await;
    let position = Position::new(0, 1);
    let document_context = DefaultDocumentContext {};
    // complete
    let completion_list = language_service
        .do_complete(&document, &position, &html_document, document_context, None)
        .await;
    println!("completion_list: {:#?}", completion_list);
    // hover
    let hover = language_service
        .do_hover(&document, &position, &html_document, None)
        .await;
    println!("hover: {:#?}", hover);
}
```
