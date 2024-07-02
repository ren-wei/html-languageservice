use std::vec;

use html_languageservice::{HTMLDataManager, HTMLLanguageService};
use lsp_textdocument::FullTextDocument;
use lsp_types::{DocumentSymbol, Location, Position, Range, SymbolInformation, SymbolKind, Url};

const TEST_URL: &'static str = "test://test/test.html";

fn test_symbol_informations_for(value: &str, expected: Vec<SymbolInformation>) {
    let document = FullTextDocument::new("html".to_string(), 0, value.to_string());
    let uri = Url::parse(&TEST_URL).unwrap();
    let html_document =
        HTMLLanguageService::parse_html_document(&document, &mut HTMLDataManager::default());
    let symbols = HTMLLanguageService::find_document_symbols(&uri, &document, &html_document);
    assert_eq!(symbols, expected);
}

fn test_document_symbols_for(value: &str, expected: Vec<DocumentSymbol>) {
    let document = FullTextDocument::new("html".to_string(), 0, value.to_string());
    let html_document =
        HTMLLanguageService::parse_html_document(&document, &mut HTMLDataManager::default());
    let symbols = HTMLLanguageService::find_document_symbols2(&document, &html_document);
    assert_eq!(symbols, expected);
}

#[test]
fn simple() {
    let uri = Url::parse(&TEST_URL).unwrap();
    test_symbol_informations_for(
        "<div></div>",
        vec![
            #[allow(deprecated)]
            SymbolInformation {
                name: "div".to_string(),
                kind: SymbolKind::FIELD,
                tags: None,
                deprecated: None,
                location: Location::new(
                    uri.clone(),
                    Range::new(Position::new(0, 0), Position::new(0, 11)),
                ),
                container_name: None,
            },
        ],
    );
    test_symbol_informations_for(
        r#"<div><input checked id="test" class="checkbox"></div>"#,
        vec![
            #[allow(deprecated)]
            SymbolInformation {
                name: "div".to_string(),
                kind: SymbolKind::FIELD,
                tags: None,
                deprecated: None,
                location: Location::new(
                    uri.clone(),
                    Range::new(Position::new(0, 0), Position::new(0, 53)),
                ),
                container_name: None,
            },
            #[allow(deprecated)]
            SymbolInformation {
                name: "input#test.checkbox".to_string(),
                kind: SymbolKind::FIELD,
                tags: None,
                deprecated: None,
                location: Location::new(
                    uri.clone(),
                    Range::new(Position::new(0, 5), Position::new(0, 47)),
                ),
                container_name: Some("div".to_string()),
            },
        ],
    );

    test_document_symbols_for(
        "<div></div>",
        vec![
            #[allow(deprecated)]
            DocumentSymbol {
                name: "div".to_string(),
                detail: None,
                kind: SymbolKind::FIELD,
                tags: None,
                deprecated: None,
                range: Range::new(Position::new(0, 0), Position::new(0, 11)),
                selection_range: Range::new(Position::new(0, 0), Position::new(0, 11)),
                children: Some(vec![]),
            },
        ],
    );

    test_document_symbols_for(
        r#"<div><input checked id="test" class="checkbox"></div>"#,
        vec![
            #[allow(deprecated)]
            DocumentSymbol {
                name: "div".to_string(),
                detail: None,
                kind: SymbolKind::FIELD,
                tags: None,
                deprecated: None,
                range: Range::new(Position::new(0, 0), Position::new(0, 53)),
                selection_range: Range::new(Position::new(0, 0), Position::new(0, 53)),
                children: Some(vec![
                    #[allow(deprecated)]
                    DocumentSymbol {
                        name: "input#test.checkbox".to_string(),
                        detail: None,
                        kind: SymbolKind::FIELD,
                        tags: None,
                        deprecated: None,
                        range: Range::new(Position::new(0, 5), Position::new(0, 47)),
                        selection_range: Range::new(Position::new(0, 5), Position::new(0, 47)),
                        children: Some(vec![]),
                    },
                ]),
            },
        ],
    );
}

#[test]
fn id_and_classes() {
    let uri = Url::parse(&TEST_URL).unwrap();
    let content =
        r#"<html id='root'><body id="Foo" class="bar"><div class="a b"></div></body></html>"#;

    test_symbol_informations_for(
        &content,
        vec![
            #[allow(deprecated)]
            SymbolInformation {
                name: "html#root".to_string(),
                kind: SymbolKind::FIELD,
                tags: None,
                deprecated: None,
                location: Location::new(
                    uri.clone(),
                    Range::new(Position::new(0, 0), Position::new(0, 80)),
                ),
                container_name: None,
            },
            #[allow(deprecated)]
            SymbolInformation {
                name: "body#Foo.bar".to_string(),
                kind: SymbolKind::FIELD,
                tags: None,
                deprecated: None,
                location: Location::new(
                    uri.clone(),
                    Range::new(Position::new(0, 16), Position::new(0, 73)),
                ),
                container_name: Some("html#root".to_string()),
            },
            #[allow(deprecated)]
            SymbolInformation {
                name: "div.a.b".to_string(),
                kind: SymbolKind::FIELD,
                tags: None,
                deprecated: None,
                location: Location::new(
                    uri.clone(),
                    Range::new(Position::new(0, 43), Position::new(0, 66)),
                ),
                container_name: Some("body#Foo.bar".to_string()),
            },
        ],
    );

    test_document_symbols_for(
        &content,
        vec![
            #[allow(deprecated)]
            DocumentSymbol {
                name: "html#root".to_string(),
                detail: None,
                kind: SymbolKind::FIELD,
                tags: None,
                deprecated: None,
                range: Range::new(Position::new(0, 0), Position::new(0, 80)),
                selection_range: Range::new(Position::new(0, 0), Position::new(0, 80)),
                children: Some(vec![
                    #[allow(deprecated)]
                    DocumentSymbol {
                        name: "body#Foo.bar".to_string(),
                        detail: None,
                        kind: SymbolKind::FIELD,
                        tags: None,
                        deprecated: None,
                        range: Range::new(Position::new(0, 16), Position::new(0, 73)),
                        selection_range: Range::new(Position::new(0, 16), Position::new(0, 73)),
                        children: Some(vec![
                            #[allow(deprecated)]
                            DocumentSymbol {
                                name: "div.a.b".to_string(),
                                detail: None,
                                kind: SymbolKind::FIELD,
                                tags: None,
                                deprecated: None,
                                range: Range::new(Position::new(0, 43), Position::new(0, 66)),
                                selection_range: Range::new(
                                    Position::new(0, 43),
                                    Position::new(0, 66),
                                ),
                                children: Some(vec![]),
                            },
                        ]),
                    },
                ]),
            },
        ],
    )
}

#[test]
fn self_closing() {
    let uri = Url::parse(&TEST_URL).unwrap();
    let content = r#"<html><br id="Foo"><br id=Bar></html>"#;

    test_symbol_informations_for(
        &content,
        vec![
            #[allow(deprecated)]
            SymbolInformation {
                name: "html".to_string(),
                kind: SymbolKind::FIELD,
                tags: None,
                deprecated: None,
                location: Location::new(
                    uri.clone(),
                    Range::new(Position::new(0, 0), Position::new(0, 37)),
                ),
                container_name: None,
            },
            #[allow(deprecated)]
            SymbolInformation {
                name: "br#Foo".to_string(),
                kind: SymbolKind::FIELD,
                tags: None,
                deprecated: None,
                location: Location::new(
                    uri.clone(),
                    Range::new(Position::new(0, 6), Position::new(0, 19)),
                ),
                container_name: Some("html".to_string()),
            },
            #[allow(deprecated)]
            SymbolInformation {
                name: "br#Bar".to_string(),
                kind: SymbolKind::FIELD,
                tags: None,
                deprecated: None,
                location: Location::new(
                    uri.clone(),
                    Range::new(Position::new(0, 19), Position::new(0, 30)),
                ),
                container_name: Some("html".to_string()),
            },
        ],
    );

    test_document_symbols_for(
        &content,
        vec![
            #[allow(deprecated)]
            DocumentSymbol {
                name: "html".to_string(),
                detail: None,
                kind: SymbolKind::FIELD,
                tags: None,
                deprecated: None,
                range: Range::new(Position::new(0, 0), Position::new(0, 37)),
                selection_range: Range::new(Position::new(0, 0), Position::new(0, 37)),
                children: Some(vec![
                    #[allow(deprecated)]
                    DocumentSymbol {
                        name: "br#Foo".to_string(),
                        detail: None,
                        kind: SymbolKind::FIELD,
                        tags: None,
                        deprecated: None,
                        range: Range::new(Position::new(0, 6), Position::new(0, 19)),
                        selection_range: Range::new(Position::new(0, 6), Position::new(0, 19)),
                        children: Some(vec![]),
                    },
                    #[allow(deprecated)]
                    DocumentSymbol {
                        name: "br#Bar".to_string(),
                        detail: None,
                        kind: SymbolKind::FIELD,
                        tags: None,
                        deprecated: None,
                        range: Range::new(Position::new(0, 19), Position::new(0, 30)),
                        selection_range: Range::new(Position::new(0, 19), Position::new(0, 30)),
                        children: Some(vec![]),
                    },
                ]),
            },
        ],
    );
}

#[test]
fn no_attributes() {
    let uri = Url::parse(&TEST_URL).unwrap();
    let content = "<html><body><div></div></body></html>";

    test_symbol_informations_for(
        &content,
        vec![
            #[allow(deprecated)]
            SymbolInformation {
                name: "html".to_string(),
                kind: SymbolKind::FIELD,
                tags: None,
                deprecated: None,
                location: Location::new(
                    uri.clone(),
                    Range::new(Position::new(0, 0), Position::new(0, 37)),
                ),
                container_name: None,
            },
            #[allow(deprecated)]
            SymbolInformation {
                name: "body".to_string(),
                kind: SymbolKind::FIELD,
                tags: None,
                deprecated: None,
                location: Location::new(
                    uri.clone(),
                    Range::new(Position::new(0, 6), Position::new(0, 30)),
                ),
                container_name: Some("html".to_string()),
            },
            #[allow(deprecated)]
            SymbolInformation {
                name: "div".to_string(),
                kind: SymbolKind::FIELD,
                tags: None,
                deprecated: None,
                location: Location::new(
                    uri.clone(),
                    Range::new(Position::new(0, 12), Position::new(0, 23)),
                ),
                container_name: Some("body".to_string()),
            },
        ],
    );

    test_document_symbols_for(
        &content,
        vec![
            #[allow(deprecated)]
            DocumentSymbol {
                name: "html".to_string(),
                detail: None,
                kind: SymbolKind::FIELD,
                tags: None,
                deprecated: None,
                range: Range::new(Position::new(0, 0), Position::new(0, 37)),
                selection_range: Range::new(Position::new(0, 0), Position::new(0, 37)),
                children: Some(vec![
                    #[allow(deprecated)]
                    DocumentSymbol {
                        name: "body".to_string(),
                        detail: None,
                        kind: SymbolKind::FIELD,
                        tags: None,
                        deprecated: None,
                        range: Range::new(Position::new(0, 6), Position::new(0, 30)),
                        selection_range: Range::new(Position::new(0, 6), Position::new(0, 30)),
                        children: Some(vec![
                            #[allow(deprecated)]
                            DocumentSymbol {
                                name: "div".to_string(),
                                detail: None,
                                kind: SymbolKind::FIELD,
                                tags: None,
                                deprecated: None,
                                range: Range::new(Position::new(0, 12), Position::new(0, 23)),
                                selection_range: Range::new(
                                    Position::new(0, 12),
                                    Position::new(0, 23),
                                ),
                                children: Some(vec![]),
                            },
                        ]),
                    },
                ]),
            },
        ],
    );
}
