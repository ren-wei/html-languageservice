use html_languageservice::{HTMLDataManager, HTMLLanguageService};
use lsp_textdocument::FullTextDocument;

fn test_linked_editing(content: &str, expected: Vec<(usize, &str)>) {
    let offset = content.find('|').unwrap();
    let value = format!("{}{}", &content[..offset], &content[offset + 1..]);

    let document = FullTextDocument::new("html".to_string(), 0, value);
    let position = document.position_at(offset as u32);
    let html_document =
        HTMLLanguageService::parse_html_document(&document, &HTMLDataManager::default());

    let synced_regions =
        HTMLLanguageService::find_linked_editing_ranges(&document, position, &html_document);

    if synced_regions.is_none() {
        if expected.len() > 0 {
            panic!(
                "No linked editing ranges for {} but expecting\n{:?}",
                content, expected
            );
        } else {
            return;
        }
    }

    let actual: Vec<(usize, &str)> = synced_regions
        .unwrap()
        .iter()
        .map(|r| {
            (
                document.offset_at(r.start) as usize,
                document.get_content(Some(r.clone())),
            )
        })
        .collect();

    assert_eq!(actual, expected);
}

#[test]
fn linked_editing() {
    test_linked_editing("|<div></div>", vec![]);
    test_linked_editing("<|div></div>", vec![(1, "div"), (7, "div")]);
    test_linked_editing("<d|iv></div>", vec![(1, "div"), (7, "div")]);
    test_linked_editing("<di|v></div>", vec![(1, "div"), (7, "div")]);
    test_linked_editing("<div|></div>", vec![(1, "div"), (7, "div")]);

    test_linked_editing("<div>|</div>", vec![]);
    test_linked_editing("<div><|/div>", vec![]);

    test_linked_editing("<div></|div>", vec![(1, "div"), (7, "div")]);
    test_linked_editing("<div></d|iv>", vec![(1, "div"), (7, "div")]);
    test_linked_editing("<div></di|v>", vec![(1, "div"), (7, "div")]);
    test_linked_editing("<div></div|>", vec![(1, "div"), (7, "div")]);

    test_linked_editing("<div></div>|", vec![]);
    test_linked_editing("<div><div|</div>", vec![]);
    test_linked_editing("<div><div><div|</div></div>", vec![]);

    test_linked_editing("<div| ></div>", vec![(1, "div"), (8, "div")]);
    test_linked_editing(r#"<div| id="foo"></div>"#, vec![(1, "div"), (16, "div")]);

    test_linked_editing("<|></>", vec![(1, ""), (4, "")]);
    test_linked_editing("<><div></div></|>", vec![(1, ""), (15, "")]);
}
