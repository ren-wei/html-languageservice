use lsp_textdocument::FullTextDocument;
use lsp_types::{Position, Range, TextEdit};
use regex::Regex;

use crate::beautify::beautify_html::html_beautify;

pub async fn format(
    document: &FullTextDocument,
    range: &Option<Range>,
    options: &HTMLFormatConfiguration,
) -> Vec<TextEdit> {
    let mut value = document.get_content(None);
    let mut initial_indent_level = 0;
    let tab_size = options.tab_size;
    let range = if let Some(range) = range {
        let mut start_offset = document.offset_at(range.start) as usize;

        // include all leading whitespace if at the beginning of the line
        let mut extended_start = start_offset;
        while extended_start > 0 && is_whitespace(value, extended_start - 1) {
            extended_start -= 1;
        }
        if extended_start == 0 || is_eol(value, extended_start - 1) {
            start_offset = extended_start;
        } else {
            // else keep at least one whitespace
            if extended_start < start_offset {
                start_offset = extended_start + 1;
            }
        }

        // include all following whitespace until the end of the line
        let mut end_offset = document.offset_at(range.end) as usize;
        let mut extended_end = end_offset;
        while extended_end < value.len() && is_whitespace(value, extended_end) {
            extended_end += 1;
        }
        if extended_end == value.len() || is_eol(value, extended_end) {
            end_offset = extended_end;
        }
        let range = Range::new(
            document.position_at(start_offset as u32),
            document.position_at(end_offset as u32),
        );

        // Do not modify if substring starts in inside an element
        // Ending inside an element is fine as it doesn't cause formatting errors
        let first_half = &value[0..start_offset];
        if Regex::new(".*[<][^>]*$").unwrap().is_match(first_half) {
            // return without modification
            let value = &value[start_offset..end_offset];
            return vec![TextEdit::new(range, value.to_string())];
        }

        value = &value[start_offset..end_offset];

        if start_offset != 0 {
            let start_of_line_offset =
                document.offset_at(Position::new(range.start.line, 0)) as usize;
            initial_indent_level =
                compute_indent_level(document.get_content(None), start_of_line_offset, &options);
        }
        range
    } else {
        Range::new(
            Position::new(0, 0),
            document.position_at(value.len() as u32),
        )
    };

    let mut result = html_beautify(&trim_left(value), &options).await;

    if initial_indent_level > 0 {
        let indent = if options.insert_spaces {
            " ".repeat(tab_size as usize * initial_indent_level)
        } else {
            "\t".repeat(initial_indent_level)
        };
        result = result.split("\n").collect::<Vec<_>>().join(&indent);
        if range.start.character == 0 {
            result = indent + &result;
        }
    }

    vec![TextEdit::new(range, result)]
}

fn trim_left(value: &str) -> String {
    Regex::new("^\\s+").unwrap().replace(value, "").to_string()
}

fn compute_indent_level(content: &str, offset: usize, options: &HTMLFormatConfiguration) -> usize {
    let mut i = offset;
    let mut n_chars = 0;
    let tab_size = options.tab_size as usize;
    let length = content.len();
    let mut content = content.chars();
    while i < length {
        let ch = content.nth(i).unwrap();
        if ch == ' ' {
            n_chars += 1;
        } else if ch == '\t' {
            n_chars += tab_size;
        } else {
            break;
        }
        i += 1;
    }
    n_chars / tab_size
}

fn is_eol(text: &str, offset: usize) -> bool {
    text.chars().nth(offset).is_some_and(|c| c == '\n')
}

fn is_whitespace(text: &str, offset: usize) -> bool {
    text.chars()
        .nth(offset)
        .is_some_and(|c| vec![' ', '\t'].contains(&c))
}

pub struct HTMLFormatConfiguration {
    pub tab_size: u8,
    pub insert_spaces: bool,
    pub indent_empty_lines: bool,
    pub wrap_line_length: Option<usize>,
    // pub unformatted: Option<Vec<String>>,
    // pub content_unformatted: Option<Vec<String>>,
    // pub indent_inner_html: bool,
    // pub wrap_attributes: HtmlWrapAttributes,
    /// default same of tab_size if None
    pub wrap_attributes_indent_size: Option<u8>,
    pub preserve_new_lines: bool,
    pub max_preserve_new_lines: Option<usize>,
    // pub indent_handlebars: bool,
    pub end_with_newline: bool,
    // pub extra_liners: Option<Vec<String>>,
    // pub indent_scripts: HtmlIndentScripts,
    // pub templating: Vec<HtmlTemplating>,
    // pub unformatted_content_delimiter: String,
}

impl Default for HTMLFormatConfiguration {
    fn default() -> Self {
        Self {
            tab_size: 4,
            insert_spaces: true,
            indent_empty_lines: false,
            wrap_line_length: Some(120),
            // unformatted: None,
            // content_unformatted: None,
            // indent_inner_html: false,
            // wrap_attributes: HtmlWrapAttributes::default(),
            wrap_attributes_indent_size: None,
            preserve_new_lines: true,
            max_preserve_new_lines: Some(32786),
            // indent_handlebars: false,
            end_with_newline: false,
            // extra_liners: None,
            // indent_scripts: HtmlIndentScripts::default(),
            // templating: vec![HtmlTemplating::default()],
            // unformatted_content_delimiter: "".to_string(),
        }
    }
}

// pub enum HtmlIndentScripts {
//     Keep,
//     Separate,
//     Normal,
// }

// impl Default for HtmlIndentScripts {
//     fn default() -> Self {
//         HtmlIndentScripts::Normal
//     }
// }

// pub enum HtmlWrapAttributes {
//     Auto,
//     Force,
//     ForceAligned,
//     ForceExpandMultiline,
//     AlignedMultiple,
//     Preserve,
//     PreserveAligned,
// }

// impl Default for HtmlWrapAttributes {
//     fn default() -> Self {
//         HtmlWrapAttributes::Auto
//     }
// }

// pub enum HtmlTemplating {
//     Auto,
//     None,
//     Angular,
//     Django,
//     Erb,
//     Handlebars,
//     Php,
//     Smarty,
// }

// impl Default for HtmlTemplating {
//     fn default() -> Self {
//         HtmlTemplating::Auto
//     }
// }

#[cfg(test)]
mod tests {
    use lsp_textdocument::FullTextDocument;
    use lsp_types::Range;

    use crate::HTMLLanguageService;

    use super::HTMLFormatConfiguration;

    async fn format(unformatted: &str, expected: &str, options: &HTMLFormatConfiguration) {
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

        let edits = HTMLLanguageService::format(&document, range, &options).await;

        let content = document.get_content(None);
        let mut formatted = content.to_string();
        for edit in edits {
            let start = document.offset_at(edit.range.start) as usize;
            let end = document.offset_at(edit.range.end) as usize;
            formatted = format!("{}{}{}", &content[..start], edit.new_text, &content[end..]);
        }

        assert_eq!(formatted, expected);
    }

    #[tokio::test]
    async fn full_document() {
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
        format(&unformatted, &expected, &options).await;
    }

    #[tokio::test]
    async fn text_content() {
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
        format(&unformatted, &expected, &options).await;
    }

    #[tokio::test]
    async fn inline_text_content() {
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
        format(&unformatted, &expected, &options).await;
    }

    #[tokio::test]
    async fn brother_text_content() {
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
        format(&unformatted, &expected, &options).await;
    }

    #[tokio::test]
    async fn indent_empty_lines() {
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
        format(&unformatted, &expected_false, &options).await;
        options.indent_empty_lines = true;
        format(&unformatted, &expected_true, &options).await;
    }

    #[tokio::test]
    async fn self_closing_tag() {
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
        format(&unformatted, &expected, &options).await;
    }

    #[tokio::test]
    async fn wrap_line_length() {
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
        format(&unformatted, &expected, &options).await;

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
        format(&unformatted, &expected, &options).await;
    }

    #[tokio::test]
    async fn self_closing_tag_wrap_line_length() {
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
        format(&unformatted, &expected, &options).await;
    }

    #[tokio::test]
    async fn preserve_new_lines() {
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
        format(&unformatted, &expected, &options).await;
    }

    #[tokio::test]
    async fn max_preserve_new_lines() {
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
        format(&unformatted, &expected, &options).await;
    }

    #[tokio::test]
    async fn end_with_newline() {
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
        format(&unformatted, &expected, &options).await;
    }
}
