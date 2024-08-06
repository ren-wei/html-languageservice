#[cfg(feature = "rename")]
use html_languageservice::{HTMLDataManager, HTMLLanguageService};
#[cfg(feature = "rename")]
use lsp_textdocument::FullTextDocument;
#[cfg(feature = "rename")]
use lsp_types::{TextEdit, Url};

#[cfg(feature = "rename")]
fn test_rename(value: &str, new_name: &str, expected: &str) {
    let offset = value.find('|').unwrap();
    let value = format!("{}{}", &value[..offset], &value[offset + 1..]);

    let document = FullTextDocument::new("html".to_string(), 0, value);

    let uri = Url::parse("test://test/test.html").unwrap();
    let position = document.position_at(offset as u32);
    let html_document =
        HTMLLanguageService::parse_html_document(&document, &HTMLDataManager::default());

    let workspace_edit =
        HTMLLanguageService::do_rename(uri.clone(), &document, position, new_name, &html_document);

    if workspace_edit.is_none()
        || workspace_edit
            .as_ref()
            .is_some_and(|edit| edit.changes.is_none())
    {
        panic!("No workspace edits");
    }

    let changes = workspace_edit.unwrap().changes.unwrap();
    let edits = changes.get(&uri);

    if edits.is_none() {
        panic!("No edits for file at {}", uri);
    }

    let new_content = apply_edits(&document, edits.unwrap());
    assert_eq!(new_content, expected);
}

#[cfg(feature = "rename")]
fn test_no_rename(value: &str, new_name: &str) {
    let offset = value.find('|').unwrap();
    let value = format!("{}{}", &value[..offset], &value[offset + 1..]);

    let document = FullTextDocument::new("html".to_string(), 0, value);

    let uri = Url::parse("test://test/test.html").unwrap();
    let position = document.position_at(offset as u32);
    let html_document =
        HTMLLanguageService::parse_html_document(&document, &HTMLDataManager::default());

    let workspace_edit =
        HTMLLanguageService::do_rename(uri.clone(), &document, position, new_name, &html_document);

    assert!(
        workspace_edit.is_none() || workspace_edit.is_some_and(|v| v.changes.is_none()),
        "Should not rename but rename happened"
    );
}

#[cfg(feature = "rename")]
fn apply_edits(document: &FullTextDocument, edits: &Vec<TextEdit>) -> String {
    let content = document.get_content(None);
    let mut new_content = String::new();
    let mut prev_offset = 0;
    for edit in edits {
        let start_offset = document.offset_at(edit.range.start) as usize;
        new_content += &format!("{}{}", &content[prev_offset..start_offset], edit.new_text);
        prev_offset = document.offset_at(edit.range.end) as usize;
    }
    new_content += &content[prev_offset..];

    new_content
}

#[cfg(feature = "rename")]
#[test]
fn rename_tag() {
    test_rename("<|div></div>", "h1", "<h1></h1>");
    test_rename("<d|iv></div>", "h1", "<h1></h1>");
    test_rename("<di|v></div>", "h1", "<h1></h1>");
    test_rename("<div|></div>", "h1", "<h1></h1>");
    test_rename("<|div></div>", "h1", "<h1></h1>");
    test_rename("<|div></div>", "h1", "<h1></h1>");

    test_no_rename("|<div></div>", "h1");
    test_no_rename("<div>|</div>", "h1");
    test_no_rename("<div><|/div>", "h1");
    test_no_rename("<div></div>|", "h1");

    test_no_rename(r#"<div |id="foo"></div>"#, "h1");
    test_no_rename(r#"<div i|d="foo"></div>"#, "h1");
    test_no_rename(r#"<div id|="foo"></div>"#, "h1");
    test_no_rename(r#"<div id=|"foo"></div>"#, "h1");
    test_no_rename(r#"<div id="|foo"></div>"#, "h1");
    test_no_rename(r#"<div id="f|oo"></div>"#, "h1");
    test_no_rename(r#"<div id="fo|o"></div>"#, "h1");
    test_no_rename(r#"<div id="foo|"></div>"#, "h1");
    test_no_rename(r#"<div id="foo"|></div>"#, "h1");
}

#[cfg(feature = "rename")]
#[test]
fn rename_self_closing_tag() {
    test_rename("<|br>", "h1", "<h1>");
    test_rename("<|br/>", "h1", "<h1/>");
    test_rename("<|br />", "h1", "<h1 />");
}

#[cfg(feature = "rename")]
#[test]
fn rename_inner_tag() {
    test_rename("<div><|h1></h1></div>", "h2", "<div><h2></h2></div>");
}

#[cfg(feature = "rename")]
#[test]
fn rename_unmatched_tag() {
    test_rename("<div><|h1></div>", "h2", "<div><h2></div>");
    test_rename("<|div><h1></h1></div>", "span", "<span><h1></h1></span>");
}
