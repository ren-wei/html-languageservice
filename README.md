# html-languageservice

> The project is a rewrite of [vscode-html-languageservice](https://github.com/Microsoft/vscode-html-languageservice) use `rust`. It has all the features of the original project while still having higher performance.

This project is a collection of features necessary to implement an HTML language server.

## Features

- customize data providers
- parse html document
- scanner
- completion - `completion` feature activate
- hover - `hover` feature activate
- formatter - `formatter` feature activate
- find document highlights - `highlight` feature activate
- find document links - `links` feature activate
- find document symbols - `symbols` feature activate
- get folding ranges - `folding` feature activate
- get selection ranges - `selection_range` feature activate
- quote complete - `completion` feature activate
- tag complete - `completion` feature activate
- rename - `rename` feature activate
- find matching tag position - `matching_tag_position` feature activate
- find linked editing ranges - `linked_editing` feature activate

## Usage

The `hover` and `complete` related to the data need to create the `HTMLLanguageService` struct first, and the other functions are not related to the data, but are used as its association functions and can be called directly.

Make sure you activated the full features of the `html-languageservice` crate on `Cargo.toml`:

```toml
html-languageservice = { version = "0.6.1", features = ["full"] }
```

You can also activate only some of the features you need on `Cargo.toml`:

```toml
html-languageservice = { version = "0.6.1", features = ["completion", "hover"] }
```

Second, You need to prepare: `document` and `position`.

Then, parse `document` as `html_document` you need to `HTMLDataManager`, tags, attributes, and attribute value data are stored in the `HTMLDataManager`.

Finally, call a function or method to get the result.

## Example

```rust
use std::sync::Arc;

use html_languageservice::{
    services::html_completion::DefaultDocumentContext, LanguageService, LanguageServiceOptions,
};
use lsp_textdocument::FullTextDocument;
use lsp_types::Position;

fn main() {
    // prepare
    let document = FullTextDocument::new("html".to_string(), 1, "<div></div>".to_string());
    let position = Position::new(0, 1);
    // init
    let data_manager = HTMLDataManager::new(true, None);
    let html_document = HTMLLanguageService::parse_html_document(&document, &data_manager);
    let ls = HTMLLanguageService::new(HTMLLanguageServiceOptions::default());
    // hover
    let hover = ls.do_hover(&document, &position, &html_document, None, &data_manager);
    assert!(hover.is_some());
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
        );
    assert!(completion_list.items.len() > 0);
}
```
