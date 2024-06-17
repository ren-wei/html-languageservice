use lsp_textdocument::FullTextDocument;
use lsp_types::{HoverContents, MarkupContent, MarkupKind};

use html_languageservice::{
    language_facts::data_manager::HTMLDataManager, HTMLLanguageService, HTMLLanguageServiceOptions,
    HoverSettings,
};

async fn assert_hover(
    value: &str,
    expected_hover_content: Option<MarkupContent>,
    expected_hover_offset: Option<u32>,
) {
    let offset = value.find('|').unwrap();
    let value = format!("{}{}", &value[..offset], &value[offset + 1..]);

    let document = FullTextDocument::new("html".to_string(), 0, value);

    let position = document.position_at(offset as u32);
    let ls = HTMLLanguageService::new(HTMLLanguageServiceOptions::default());
    let data_manager = HTMLDataManager::default();
    let html_document = HTMLLanguageService::parse_html_document(&document, &data_manager).await;
    let hover = ls
        .do_hover(&document, &position, &html_document, None, &data_manager)
        .await;
    if let Some(hover) = hover {
        assert_eq!(
            hover.clone().contents,
            HoverContents::Markup(expected_hover_content.unwrap())
        );
        assert_eq!(
            document.offset_at(hover.range.unwrap().start),
            expected_hover_offset.unwrap()
        );
    } else {
        assert_eq!(expected_hover_content, None);
        assert_eq!(expected_hover_offset, None);
    }
}

async fn assert_hover_range(
    value: &str,
    contents: HoverContents,
    range_text: &str,
    ls_options: Option<HTMLLanguageServiceOptions>,
    hover_setting: Option<HoverSettings>,
) {
    let offset = value.find('|').unwrap();
    let value = format!("{}{}", &value[..offset], &value[offset + 1..]);

    let document = FullTextDocument::new("html".to_string(), 0, value);

    let position = document.position_at(offset as u32);
    let ls = if let Some(ls_options) = ls_options {
        HTMLLanguageService::new(ls_options)
    } else {
        HTMLLanguageService::new(HTMLLanguageServiceOptions::default())
    };

    let data_manager = HTMLDataManager::default();
    let html_document = HTMLLanguageService::parse_html_document(&document, &data_manager).await;
    let hover = ls
        .do_hover(
            &document,
            &position,
            &html_document,
            hover_setting,
            &data_manager,
        )
        .await;
    if let Some(hover) = hover {
        assert_eq!(hover.contents, contents);
        if hover.range.is_some() {
            assert_eq!(document.get_content(hover.range), range_text);
        }
    }
}

#[tokio::test]
async fn single() {
    let description_and_reference = "The html element represents the root of an HTML document."
        .to_string()
        + "\n\n"
        + "[MDN Reference](https://developer.mozilla.org/docs/Web/HTML/Element/html)";

    let html_content = MarkupContent {
        kind: MarkupKind::Markdown,
        value: description_and_reference.clone(),
    };
    let close_html_content = MarkupContent {
        kind: MarkupKind::Markdown,
        value: description_and_reference.clone(),
    };

    assert_hover("|<html></html>", None, None).await;
    assert_hover("<|html></html>", Some(html_content.clone()), Some(1)).await;
    assert_hover("<h|tml></html>", Some(html_content.clone()), Some(1)).await;
    assert_hover("<htm|l></html>", Some(html_content.clone()), Some(1)).await;
    assert_hover("<html|></html>", Some(html_content.clone()), Some(1)).await;
    assert_hover("<html>|</html>", None, None).await;
    assert_hover("<html><|/html>", None, None).await;
    assert_hover("<html></|html>", Some(close_html_content.clone()), Some(8)).await;
    assert_hover("<html></h|tml>", Some(close_html_content.clone()), Some(8)).await;
    assert_hover("<html></ht|ml>", Some(close_html_content.clone()), Some(8)).await;
    assert_hover("<html></htm|l>", Some(close_html_content.clone()), Some(8)).await;
    assert_hover("<html></html|>", Some(close_html_content.clone()), Some(8)).await;
    assert_hover("<html></html>|", None, None).await;

    let entity_description =
        "Character entity representing '\u{00A0}', unicode equivalent 'U+00A0'";

    assert_hover_range(
        "<html>|&nbsp;</html>",
        HoverContents::Markup(MarkupContent {
            kind: MarkupKind::PlainText,
            value: "".to_string(),
        }),
        "",
        None,
        None,
    )
    .await;
    assert_hover_range(
        "<html>&|nbsp;</html>",
        HoverContents::Markup(MarkupContent {
            kind: MarkupKind::PlainText,
            value: entity_description.to_string(),
        }),
        "nbsp;",
        None,
        None,
    )
    .await;
    assert_hover_range(
        "<html>&n|bsp;</html>",
        HoverContents::Markup(MarkupContent {
            kind: MarkupKind::PlainText,
            value: entity_description.to_string(),
        }),
        "nbsp;",
        None,
        None,
    )
    .await;
    assert_hover_range(
        "<html>&nb|sp;</html>",
        HoverContents::Markup(MarkupContent {
            kind: MarkupKind::PlainText,
            value: entity_description.to_string(),
        }),
        "nbsp;",
        None,
        None,
    )
    .await;
    assert_hover_range(
        "<html>&nbs|p;</html>",
        HoverContents::Markup(MarkupContent {
            kind: MarkupKind::PlainText,
            value: entity_description.to_string(),
        }),
        "nbsp;",
        None,
        None,
    )
    .await;
    assert_hover_range(
        "<html>&nbsp|;</html>",
        HoverContents::Markup(MarkupContent {
            kind: MarkupKind::PlainText,
            value: entity_description.to_string(),
        }),
        "nbsp;",
        None,
        None,
    )
    .await;
    assert_hover_range(
        "<html>&nbsp;|</html>",
        HoverContents::Markup(MarkupContent {
            kind: MarkupKind::PlainText,
            value: "".to_string(),
        }),
        "",
        None,
        None,
    )
    .await;

    let no_description = MarkupContent {
        kind: MarkupKind::Markdown,
        value: "[MDN Reference](https://developer.mozilla.org/docs/Web/HTML/Element/html)"
            .to_string(),
    };
    assert_hover_range(
        "<html|></html>",
        HoverContents::Markup(no_description),
        "html",
        None,
        Some(HoverSettings {
            documentation: false,
            references: true,
        }),
    )
    .await;

    let no_references = MarkupContent {
        kind: MarkupKind::Markdown,
        value: "The html element represents the root of an HTML document.".to_string(),
    };
    assert_hover_range(
        "<html|></html>",
        HoverContents::Markup(no_references),
        "html",
        None,
        Some(HoverSettings {
            documentation: true,
            references: false,
        }),
    )
    .await;
}
