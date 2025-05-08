#[cfg(feature = "matching_tag_position")]
use html_languageservice::{HTMLDataManager, HTMLLanguageService};
#[cfg(feature = "matching_tag_position")]
use lsp_textdocument::FullTextDocument;

#[cfg(feature = "matching_tag_position")]
fn test_matching_tag_position(content: &str) {
    use html_languageservice::HTMLLanguageServiceOptions;

    let mut offset = content.find('|').unwrap();
    let mut value = format!("{}{}", &content[..offset], &content[offset + 1..]);
    let mirror_offset = value.find('$').unwrap();
    value = format!("{}{}", &value[..mirror_offset], &value[mirror_offset + 1..]);
    if mirror_offset < offset {
        offset -= 1;
    }

    let document = FullTextDocument::new("html".to_string(), 0, value);

    let position = document.position_at(offset as u32);
    let ls = HTMLLanguageService::new(&HTMLLanguageServiceOptions::default());
    let html_document = ls.parse_html_document(&document, &HTMLDataManager::default());

    let mirror_position =
        HTMLLanguageService::find_matching_tag_position(&document, position, &html_document)
            .expect("Failed to find mirror position");

    assert_eq!(
        document.offset_at(mirror_position),
        mirror_offset as u32,
        "{}",
        content
    );
}

#[cfg(feature = "matching_tag_position")]
#[test]
fn matching_position() {
    test_matching_tag_position("<|div></$div>");
    test_matching_tag_position("<d|iv></d$iv>");
    test_matching_tag_position("<di|v></di$v>");
    test_matching_tag_position("<div|></div$>");

    test_matching_tag_position("<$div></|div>");
    test_matching_tag_position("<d$iv></d|iv>");
    test_matching_tag_position("<di$v></di|v>");
    test_matching_tag_position("<div$></div|>");

    test_matching_tag_position("<div| ></div$>");
    test_matching_tag_position(r#"<div| id="foo"></div$>"#);

    test_matching_tag_position("<div$ ></div|>");
    test_matching_tag_position(r#"<div$ id="foo"></div|>"#);
}
