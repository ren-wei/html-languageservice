#[cfg(feature = "experimental")]
use html_languageservice::{HTMLFormatConfiguration, HTMLLanguageService};
#[cfg(feature = "experimental")]
use lsp_textdocument::FullTextDocument;
#[cfg(feature = "experimental")]
use lsp_types::*;

#[cfg(feature = "experimental")]
fn format(unformatted: &str, expected: &str, options: &HTMLFormatConfiguration) {
    let range_start = unformatted.find('|');
    let range_end = unformatted.rfind('|');
    let mut range = None;
    let document = if let Some(range_start) = range_start {
        let range_end = range_end.unwrap();
        let content = format!(
            "{}{}{}",
            &unformatted[..range_start],
            &unformatted[range_start + 1..range_end],
            &unformatted[range_end + 1..]
        );
        let document = FullTextDocument::new("html".to_string(), 0, content);
        range = Some(Range::new(
            document.position_at(range_start as u32),
            document.position_at(range_end as u32),
        ));
        document
    } else {
        FullTextDocument::new("html".to_string(), 0, unformatted.to_string())
    };

    let edits = HTMLLanguageService::format(&document, range, &options);

    let content = document.get_content(None);
    let mut formatted = content.to_string();
    for edit in edits {
        let start = document.offset_at(edit.range.start) as usize;
        let end = document.offset_at(edit.range.end) as usize;
        formatted = format!("{}{}{}", &content[..start], edit.new_text, &content[end..]);
    }

    assert_eq!(formatted, expected);
}

#[cfg(feature = "experimental")]
#[test]
fn full_document() {
    let unformatted = [
        r#"<div  class = "foo">"#, // wrap
        "<br>",
        " </div>",
    ]
    .join("\n");
    let expected = [
        r#"<div class="foo">"#, // wrap
        "  <br />",
        "</div>",
    ]
    .join("\n");
    let options = HTMLFormatConfiguration {
        tab_size: 2,
        ..Default::default()
    };
    format(&unformatted, &expected, &options);
}

#[cfg(feature = "experimental")]
#[test]
fn text_content() {
    let unformatted = [
        r#"<div  class = "foo">"#, // wrap
        "text  text2",
        " </div>",
    ]
    .join("\n");
    let expected = [
        r#"<div class="foo">"#, // wrap
        "  text text2",
        "</div>",
    ]
    .join("\n");
    let options = HTMLFormatConfiguration {
        tab_size: 2,
        ..Default::default()
    };
    format(&unformatted, &expected, &options);
}

#[cfg(feature = "experimental")]
#[test]
fn inline_text_content() {
    let unformatted = [
        r#"<div  class = "foo">  text  text2  </div>"#, // wrap
    ]
    .join("\n");
    let expected = [
        r#"<div class="foo">text text2</div>"#, // wrap
    ]
    .join("\n");
    let options = HTMLFormatConfiguration {
        tab_size: 2,
        ..Default::default()
    };
    format(&unformatted, &expected, &options);
}

#[cfg(feature = "experimental")]
#[test]
fn brother_text_content() {
    let unformatted = [
        r#"<div  class = "foo">"#, // wrap
        "text  text2",
        "<div></div>",
        "text3  text4",
        " </div>",
    ]
    .join("\n");
    let expected = [
        r#"<div class="foo">"#, // wrap
        "  text text2",
        "  <div></div>",
        "  text3 text4",
        "</div>",
    ]
    .join("\n");
    let options = HTMLFormatConfiguration {
        tab_size: 2,
        ..Default::default()
    };
    format(&unformatted, &expected, &options);
}

#[cfg(feature = "experimental")]
#[test]
fn indent_empty_lines() {
    let unformatted = [
        "<div>",  // wrap
        " ",      // wrap
        "<div>",  // wrap
        "",       // wrap
        "</div>", // wrap
        "",       // wrap
        "</div>", // wrap
    ]
    .join("\n");
    let expected_false = [
        "<div>",    // wrap
        "",         // wrap
        "  <div>",  // wrap
        "",         // wrap
        "  </div>", // wrap
        "",         // wrap
        "</div>",   // wrap
    ]
    .join("\n");
    let expected_true = [
        "<div>",    // wrap
        "  ",       // wrap
        "  <div>",  // wrap
        "    ",     // wrap
        "  </div>", // wrap
        "  ",       // wrap
        "</div>",   // wrap
    ]
    .join("\n");

    let mut options = HTMLFormatConfiguration {
        tab_size: 2,
        indent_empty_lines: false,
        ..Default::default()
    };
    format(&unformatted, &expected_false, &options);
    options.indent_empty_lines = true;
    format(&unformatted, &expected_true, &options);
}

#[cfg(feature = "experimental")]
#[test]
fn self_closing_tag() {
    let unformatted = [
        r#"<img  src = "https://exsample.com"/>"#, // wrap
    ]
    .join("\n");
    let expected = [
        r#"<img src="https://exsample.com" />"#, // wrap
    ]
    .join("\n");
    let options = HTMLFormatConfiguration {
        ..Default::default()
    };
    format(&unformatted, &expected, &options);
}

#[cfg(feature = "experimental")]
#[test]
fn wrap_line_length() {
    let unformatted = [
        r#"<div title="a div container" data-id="123456" data-type="node">content</div>"#, // wrap
    ]
    .join("\n");

    let mut options = HTMLFormatConfiguration {
        tab_size: 2,
        wrap_line_length: Some(70),
        ..Default::default()
    };
    let expected = [
        r#"<div title="a div container" data-id="123456" data-type="node">"#,
        "  content",
        "</div>",
    ]
    .join("\n");
    format(&unformatted, &expected, &options);

    options.wrap_line_length = Some(60);
    let expected = [
        r#"<div"#,
        r#"  title="a div container""#,
        r#"  data-id="123456""#,
        r#"  data-type="node""#,
        ">",
        "  content",
        "</div>",
    ]
    .join("\n");
    format(&unformatted, &expected, &options);
}

#[cfg(feature = "experimental")]
#[test]
fn self_closing_tag_wrap_line_length() {
    let unformatted = [
            r#"<img  src = "https://exsample.com" title="a image container" data-id="123456" data-type="node"/>"#,
        ]
        .join("\n");
    let expected = [
        r#"<img"#,
        r#"  src="https://exsample.com""#,
        r#"  title="a image container""#,
        r#"  data-id="123456""#,
        r#"  data-type="node""#,
        "/>",
    ]
    .join("\n");
    let options = HTMLFormatConfiguration {
        wrap_line_length: Some(70),
        wrap_attributes_indent_size: Some(2),
        ..Default::default()
    };
    format(&unformatted, &expected, &options);
}

#[cfg(feature = "experimental")]
#[test]
fn preserve_new_lines() {
    let unformatted = [
        r#"<div  class = "foo">"#, // wrap
        "",
        "",
        "<br>",
        " </div>",
    ]
    .join("\n");
    let expected = [
        r#"<div class="foo">"#, // wrap
        "  <br />",
        "</div>",
    ]
    .join("\n");
    let options = HTMLFormatConfiguration {
        tab_size: 2,
        preserve_new_lines: false,
        ..Default::default()
    };
    format(&unformatted, &expected, &options);
}

#[cfg(feature = "experimental")]
#[test]
fn max_preserve_new_lines() {
    let unformatted = [
        r#"<div  class = "foo">"#, // wrap
        "",
        "",
        "<br>",
        " </div>",
    ]
    .join("\n");
    let expected = [
        r#"<div class="foo">"#, // wrap
        "",
        "  <br />",
        "</div>",
    ]
    .join("\n");
    let options = HTMLFormatConfiguration {
        tab_size: 2,
        max_preserve_new_lines: Some(1),
        ..Default::default()
    };
    format(&unformatted, &expected, &options);
}

#[cfg(feature = "experimental")]
#[test]
fn end_with_newline() {
    let unformatted = [
        r#"<div  class = "foo">"#, // wrap
        "<br>",
        " </div>",
    ]
    .join("\n");
    let expected = [
        r#"<div class="foo">"#, // wrap
        "  <br />",
        "</div>",
        "",
    ]
    .join("\n");
    let options = HTMLFormatConfiguration {
        tab_size: 2,
        end_with_newline: true,
        ..Default::default()
    };
    format(&unformatted, &expected, &options);
}

#[cfg(feature = "experimental")]
#[test]
fn range() {
    let unformatted = [
        r#"<div  class = "foo">"#,
        r#"  |<img  src = "foo">|"#,
        r#" </div>"#,
    ]
    .join("\n");
    let expected = [
        r#"<div  class = "foo">"#,
        r#"  <img src="foo" />"#,
        r#" </div>"#,
    ]
    .join("\n");
    let options = HTMLFormatConfiguration {
        tab_size: 2,
        end_with_newline: true,
        ..Default::default()
    };
    format(&unformatted, &expected, &options);
}
