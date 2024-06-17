use html_languageservice::{HTMLDataManager, HTMLLanguageService};
use lsp_textdocument::FullTextDocument;

async fn assert_highlights(value: &str, expected_matches: &[usize], element_name: Option<&str>) {
    let offset = value.find('|').unwrap();
    let value = format!("{}{}", &value[..offset], &value[offset + 1..]);

    let document = FullTextDocument::new("html".to_string(), 0, value);

    let position = document.position_at(offset as u32);
    let data_manager = HTMLDataManager::default();
    let html_document = HTMLLanguageService::parse_html_document(&document, &data_manager).await;

    let hightlights =
        HTMLLanguageService::find_document_highlights(&document, &position, &html_document).await;
    assert_eq!(hightlights.len(), expected_matches.len());

    for (i, hightlight) in hightlights.iter().enumerate() {
        let actual_start_offset = document.offset_at(hightlight.range.start) as usize;
        assert_eq!(actual_start_offset, expected_matches[i]);
        let actual_end_offset = document.offset_at(hightlight.range.end) as usize;
        assert_eq!(
            actual_end_offset,
            expected_matches[i] + element_name.unwrap().len()
        );

        assert_eq!(
            &document.get_content(None)[actual_start_offset..actual_end_offset].to_lowercase(),
            element_name.unwrap()
        );
    }
}

#[tokio::test]
async fn single() {
    assert_highlights("|<html></html>", &[], None).await;
    assert_highlights("<|html></html>", &[1, 8], Some("html")).await;
    assert_highlights("<h|tml></html>", &[1, 8], Some("html")).await;
    assert_highlights("<htm|l></html>", &[1, 8], Some("html")).await;
    assert_highlights("<html|></html>", &[1, 8], Some("html")).await;
    assert_highlights("<html>|</html>", &[], None).await;
    assert_highlights("<html><|/html>", &[], None).await;
    assert_highlights("<html></|html>", &[1, 8], Some("html")).await;
    assert_highlights("<html></h|tml>", &[1, 8], Some("html")).await;
    assert_highlights("<html></ht|ml>", &[1, 8], Some("html")).await;
    assert_highlights("<html></htm|l>", &[1, 8], Some("html")).await;
    assert_highlights("<html></html|>", &[1, 8], Some("html")).await;
    assert_highlights("<html></html>|", &[], None).await;
}

#[tokio::test]
async fn nested() {
    assert_highlights("<html>|<div></div></html>", &[], None).await;
    assert_highlights("<html><|div></div></html>", &[7, 13], Some("div")).await;
    assert_highlights("<html><div>|</div></html>", &[], None).await;
    assert_highlights("<html><div></di|v></html>", &[7, 13], Some("div")).await;
    assert_highlights(
        "<html><div><div></div></di|v></html>",
        &[7, 24],
        Some("div"),
    )
    .await;
    assert_highlights(
        "<html><div><div></div|></div></html>",
        &[12, 18],
        Some("div"),
    )
    .await;
    assert_highlights(
        "<html><div><div|></div></div></html>",
        &[12, 18],
        Some("div"),
    )
    .await;
    assert_highlights(
        "<html><div><div></div></div></h|tml>",
        &[1, 30],
        Some("html"),
    )
    .await;
    assert_highlights(
        "<html><di|v></div><div></div></html>",
        &[7, 13],
        Some("div"),
    )
    .await;
    assert_highlights(
        "<html><div></div><div></d|iv></html>",
        &[18, 24],
        Some("div"),
    )
    .await;
}

#[tokio::test]
async fn self_closed() {
    assert_highlights("<html><|div/></html>", &[7], Some("div")).await;
    assert_highlights("<html><|br></html>", &[7], Some("br")).await;
    assert_highlights("<html><div><d|iv/></div></html>", &[12], Some("div")).await;
}

#[tokio::test]
async fn case_insensivity() {
    assert_highlights(
        "<HTML><diV><Div></dIV></dI|v></html>",
        &[7, 24],
        Some("div"),
    )
    .await;
    assert_highlights(
        "<HTML><diV|><Div></dIV></dIv></html>",
        &[7, 24],
        Some("div"),
    )
    .await;
}

#[tokio::test]
async fn incomplete() {
    assert_highlights("<div><ol><li></li></ol></p></|div>", &[1, 29], Some("div")).await;
}
