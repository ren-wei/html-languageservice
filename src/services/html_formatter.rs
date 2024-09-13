use lsp_textdocument::FullTextDocument;
use lsp_types::{Position, Range, TextEdit};
use regex::Regex;

use crate::beautify::beautify_html::html_beautify;

pub fn format(
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
        let range = if document
            .get_content(None)
            .get(start_offset - 1..start_offset)
            .is_some_and(|v| v == "\n")
        {
            let start = document.position_at(start_offset as u32);
            Range::new(
                Position {
                    line: start.line + 1,
                    character: 0,
                },
                document.position_at(end_offset as u32),
            )
        } else {
            Range::new(
                document.position_at(start_offset as u32),
                document.position_at(end_offset as u32),
            )
        };

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

    let mut result = html_beautify(&trim_left(value), &options);

    if initial_indent_level > 0 {
        let indent = if options.insert_spaces {
            " ".repeat(tab_size as usize * initial_indent_level)
        } else {
            "\t".repeat(initial_indent_level)
        };
        if result.ends_with('\n') {
            result = result[..result.len() - 1]
                .split("\n")
                .collect::<Vec<_>>()
                .join(&format!("\n{}", &indent));
            result += "\n";
        } else {
            result = result
                .split("\n")
                .collect::<Vec<_>>()
                .join(&format!("\n{}", &indent));
        }
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
    let mut bytes = content.bytes().skip(i - 1);
    while i < length {
        let ch = bytes.next().unwrap();
        if ch == b' ' {
            n_chars += 1;
        } else if ch == b'\t' {
            n_chars += tab_size;
        } else {
            break;
        }
        i += 1;
    }
    n_chars / tab_size
}

fn is_eol(text: &str, offset: usize) -> bool {
    text.get(offset..offset + 1).is_some_and(|c| c == "\n")
}

fn is_whitespace(text: &str, offset: usize) -> bool {
    text.get(offset..offset + 1)
        .is_some_and(|c| vec![" ", "\t"].contains(&c))
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
