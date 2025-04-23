use lsp_textdocument::FullTextDocument;
use lsp_types::{DocumentSymbol, Location, Range, SymbolInformation, SymbolKind, Uri};

use crate::parser::html_document::{HTMLDocument, Node};

pub fn find_document_symbols(
    uri: &Uri,
    document: &FullTextDocument,
    html_document: &HTMLDocument,
) -> Vec<SymbolInformation> {
    let mut symbols = vec![];
    let symbols2 = find_document_symbols2(document, html_document);

    for symbol in &symbols2 {
        walk(uri, symbol, None, &mut symbols);
    }

    symbols
}

pub fn find_document_symbols2(
    document: &FullTextDocument,
    html_document: &HTMLDocument,
) -> Vec<DocumentSymbol> {
    let mut symbols = vec![];

    for root in &html_document.roots {
        provide_file_symbols_internal(document, root, &mut symbols);
    }

    symbols
}

fn provide_file_symbols_internal(
    document: &FullTextDocument,
    node: &Node,
    symbols: &mut Vec<DocumentSymbol>,
) {
    let name = node_to_name(node);
    let range = Range::new(
        document.position_at(node.start as u32),
        document.position_at(node.end as u32),
    );

    let mut children = vec![];

    for child in &node.children {
        provide_file_symbols_internal(document, &child, &mut children);
    }

    #[allow(deprecated)]
    let symbol = DocumentSymbol {
        name,
        detail: None,
        kind: SymbolKind::FIELD,
        range: range.clone(),
        selection_range: range,
        tags: None,
        children: Some(children),
        deprecated: None,
    };

    symbols.push(symbol);
}

fn walk(
    uri: &Uri,
    node: &DocumentSymbol,
    parent: Option<&DocumentSymbol>,
    symbols: &mut Vec<SymbolInformation>,
) {
    #[allow(deprecated)]
    let symbol = SymbolInformation {
        name: node.name.clone(),
        kind: node.kind.clone(),
        tags: None,
        location: Location::new(uri.clone(), node.range),
        deprecated: None,
        container_name: parent.map(|v| v.name.clone()),
    };

    symbols.push(symbol);

    if let Some(children) = &node.children {
        for child in children {
            walk(uri, child, Some(&node), symbols)
        }
    }
}

fn node_to_name(node: &Node) -> String {
    if let Some(mut name) = node.tag.clone() {
        if !node.attributes.is_empty() {
            let id = node.attributes.get("id").map(|v| v.value.clone()).flatten();
            let class = node
                .attributes
                .get("class")
                .map(|v| v.value.clone())
                .flatten();
            if let Some(id) = id {
                name += &format!("#{}", id.replace("\"", "").replace("'", ""));
            }
            if let Some(class) = class {
                name += &class
                    .replace("\"", "")
                    .replace("'", "")
                    .split_ascii_whitespace()
                    .map(|class_name| format!(".{}", class_name))
                    .collect::<Vec<_>>()
                    .join("");
            }
        }
        name
    } else {
        "?".to_string()
    }
}
