#[cfg(feature = "selection_range")]
use html_languageservice::{HTMLDataManager, HTMLLanguageService};
#[cfg(feature = "selection_range")]
use lsp_textdocument::FullTextDocument;

#[cfg(feature = "selection_range")]
fn assert_ranges(content: &str, expected: Vec<(u32, &str)>) {
    let offset = content.find('|').unwrap();
    let value = format!("{}{}", &content[..offset], &content[offset + 1..]);

    let document = FullTextDocument::new("html".to_string(), 0, value);

    let position = document.position_at(offset as u32);
    let html_document =
        HTMLLanguageService::parse_html_document(&document, &HTMLDataManager::default());

    let actual_ranges =
        HTMLLanguageService::get_selection_ranges(&document, &vec![position], &html_document);

    assert_eq!(actual_ranges.len(), 1);

    let mut offset_pairs = vec![];
    let mut curr = actual_ranges.get(0);
    while let Some(c) = curr {
        offset_pairs.push((
            document.offset_at(c.range.start),
            document.get_content(Some(c.range)),
        ));
        curr = c.parent.as_deref();
    }

    assert_eq!(offset_pairs, expected, "{}", content);
}

#[cfg(feature = "selection_range")]
#[test]
fn basic() {
    assert_ranges("<div|>foo</div>", vec![(1, "div"), (0, "<div>foo</div>")]);
    assert_ranges("<|div>foo</div>", vec![(1, "div"), (0, "<div>foo</div>")]);
    assert_ranges("<d|iv>foo</div>", vec![(1, "div"), (0, "<div>foo</div>")]);

    assert_ranges("<div>|foo</div>", vec![(5, "foo"), (0, "<div>foo</div>")]);
    assert_ranges("<div>f|oo</div>", vec![(5, "foo"), (0, "<div>foo</div>")]);
    assert_ranges("<div>foo|</div>", vec![(5, "foo"), (0, "<div>foo</div>")]);

    assert_ranges("<div>foo<|/div>", vec![(0, "<div>foo</div>")]);

    assert_ranges("<div>foo</|div>", vec![(10, "div"), (0, "<div>foo</div>")]);
    assert_ranges("<div>foo</di|v>", vec![(10, "div"), (0, "<div>foo</div>")]);
    assert_ranges("<div>foo</div|>", vec![(10, "div"), (0, "<div>foo</div>")]);
}

#[cfg(feature = "selection_range")]
#[test]
fn attribute_name() {
    assert_ranges(
        r#"<div |class="foo">foo</div>"#,
        vec![
            (5, "class"),
            (5, r#"class="foo""#),
            (1, r#"div class="foo""#),
            (0, r#"<div class="foo">foo</div>"#),
        ],
    );
    assert_ranges(
        r#"<div cl|ass="foo">foo</div>"#,
        vec![
            (5, "class"),
            (5, r#"class="foo""#),
            (1, r#"div class="foo""#),
            (0, r#"<div class="foo">foo</div>"#),
        ],
    );
    assert_ranges(
        r#"<div class|="foo">foo</div>"#,
        vec![
            (5, "class"),
            (5, r#"class="foo""#),
            (1, r#"div class="foo""#),
            (0, r#"<div class="foo">foo</div>"#),
        ],
    );
}

#[cfg(feature = "selection_range")]
#[test]
fn attribute_value() {
    assert_ranges(
        r#"<div class=|"foo">foo</div>"#,
        vec![
            (11, r#""foo""#),
            (5, r#"class="foo""#),
            (1, r#"div class="foo""#),
            (0, r#"<div class="foo">foo</div>"#),
        ],
    );
    assert_ranges(
        r#"<div class="foo"|>foo</div>"#,
        vec![
            (11, r#""foo""#),
            (5, r#"class="foo""#),
            (1, r#"div class="foo""#),
            (0, r#"<div class="foo">foo</div>"#),
        ],
    );

    assert_ranges(
        r#"<div class="|foo">foo</div>"#,
        vec![
            (12, "foo"),
            (11, r#""foo""#),
            (5, r#"class="foo""#),
            (1, r#"div class="foo""#),
            (0, r#"<div class="foo">foo</div>"#),
        ],
    );
    assert_ranges(
        r#"<div class="f|oo">foo</div>"#,
        vec![
            (12, "foo"),
            (11, r#""foo""#),
            (5, r#"class="foo""#),
            (1, r#"div class="foo""#),
            (0, r#"<div class="foo">foo</div>"#),
        ],
    );
}

#[cfg(feature = "selection_range")]
#[test]
fn unquoted_attribute_value() {
    assert_ranges(
        r#"<div class=|foo>foo</div>"#,
        vec![
            (11, "foo"),
            (5, "class=foo"),
            (1, "div class=foo"),
            (0, "<div class=foo>foo</div>"),
        ],
    );
}

#[cfg(feature = "selection_range")]
#[test]
fn multiple_attribute_value() {
    assert_ranges(
        r#"<div class="foo" id="|bar">foo</div>"#,
        vec![
            (21, "bar"),
            (20, r#""bar""#),
            (17, r#"id="bar""#),
            (1, r#"div class="foo" id="bar""#),
            (0, r#"<div class="foo" id="bar">foo</div>"#),
        ],
    );
}

#[cfg(feature = "selection_range")]
#[test]
fn self_closing_tags() {
    assert_ranges(
        r#"<br class="|foo"/>"#,
        vec![
            (11, "foo"),
            (10, r#""foo""#),
            (4, r#"class="foo""#),
            (1, r#"br class="foo""#),
            (0, r#"<br class="foo"/>"#),
        ],
    );

    //Todo@Pine: We need the range `br` too. Sync with Joh to see what selection ranges should provider return.
    assert_ranges(
        r#"<b|r class="foo"/>"#,
        vec![(1, r#"br class="foo""#), (0, r#"<br class="foo"/>"#)],
    );
}

#[cfg(feature = "selection_range")]
#[test]
fn nested() {
    assert_ranges(
        r#"<div><div>|foo</div></div>"#,
        vec![
            (10, "foo"),
            (5, "<div>foo</div>"),
            (0, "<div><div>foo</div></div>"),
        ],
    );

    assert_ranges(
        "<div>\n<p>|foo</p>\n</div>",
        vec![
            (9, "foo"),
            (6, "<p>foo</p>"),
            (5, "\n<p>foo</p>\n"),
            (0, "<div>\n<p>foo</p>\n</div>"),
        ],
    );
}

#[cfg(feature = "selection_range")]
#[test]
fn void_elements() {
    assert_ranges(
        "<meta charset='|UTF-8'>",
        vec![
            (15, "UTF-8"),
            (14, "'UTF-8'"),
            (6, "charset='UTF-8'"),
            (1, "meta charset='UTF-8'"),
            (0, "<meta charset='UTF-8'>"),
        ],
    );

    assert_ranges(
        "<meta c|harset='UTF-8'>",
        vec![
            (6, "charset"),
            (6, "charset='UTF-8'"),
            (1, "meta charset='UTF-8'"),
            (0, "<meta charset='UTF-8'>"),
        ],
    );

    assert_ranges(
        "<html><meta c|harset='UTF-8'></html>",
        vec![
            (12, "charset"),
            (12, "charset='UTF-8'"),
            (7, "meta charset='UTF-8'"),
            (6, "<meta charset='UTF-8'>"),
            (0, "<html><meta charset='UTF-8'></html>"),
        ],
    );
}

#[cfg(feature = "selection_range")]
#[test]
fn unmatching_tags() {
    assert_ranges("<div></div|1>", vec![(0, "<div></div1>")]);
}

#[cfg(feature = "selection_range")]
#[test]
fn unhandled() {
    // We do not handle comments. This semantic selection is handled by VS Code's default provider, which returns
    // - foo
    // - <!-- foo -->
    assert_ranges("<!-- f|oo -->", vec![(6, "")]);

    // Same for DOCTYPE
    assert_ranges("<!DOCTYPE h|tml>", vec![(11, "")]);
}
