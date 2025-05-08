#[cfg(feature = "completion")]
use std::collections::HashMap;

#[cfg(feature = "completion")]
use html_languageservice::{
    CompletionConfiguration, DefaultDocumentContext, HTMLDataManager, HTMLLanguageService,
    HTMLLanguageServiceOptions, Quotes,
};
#[cfg(feature = "completion")]
use lsp_textdocument::FullTextDocument;
#[cfg(feature = "completion")]
use lsp_types::*;

#[cfg(feature = "completion")]
fn test_completion_for(
    value: &str,
    expected: Expected,
    settings: Option<CompletionConfiguration>,
    ls_options: Option<HTMLLanguageServiceOptions>,
) {
    let offset = value.find('|').unwrap();
    let value: &str = &format!("{}{}", &value[..offset], &value[offset + 1..]);

    let ls_options = if let Some(ls_options) = ls_options {
        ls_options
    } else {
        HTMLLanguageServiceOptions::default()
    };
    let ls = HTMLLanguageService::new(&ls_options);

    let document = FullTextDocument::new("html".to_string(), 0, value.to_string());
    let position = document.position_at(offset as u32);
    let data_manager = ls.create_data_manager(true, None);
    let html_document = ls.parse_html_document(&document, &data_manager);
    let list = ls.do_complete(
        &document,
        &position,
        &html_document,
        DefaultDocumentContext,
        settings.as_ref(),
        &HTMLDataManager::default(),
    );

    // no duplicate labels
    let mut labels: Vec<String> = list.items.iter().map(|i| i.label.clone()).collect();
    labels.sort();
    let mut previous = None;
    for label in &labels {
        assert!(
            previous != Some(label),
            "Duplicate label {} in {}",
            label,
            labels.join(",")
        );
        previous = Some(label);
    }
    if expected.count.is_some() {
        assert_eq!(list.items.len(), expected.count.unwrap());
    }
    if expected.items.len() > 0 {
        for item in &expected.items {
            assert_completion(&list, item, &document);
        }
    }
}

#[cfg(feature = "completion")]
fn assert_completion(
    completions: &CompletionList,
    expected: &ItemDescription,
    document: &FullTextDocument,
) {
    let matches: Vec<&CompletionItem> = completions
        .items
        .iter()
        .filter(|c| c.label == expected.label)
        .collect();
    if expected.not_available.is_some_and(|v| v) {
        assert_eq!(
            matches.len(),
            0,
            "{} should not existing is results",
            expected.label
        );
        return;
    }

    assert_eq!(
        matches.len(),
        1,
        "{} should only existing once: Actual: {}",
        expected.label,
        completions
            .items
            .iter()
            .map(|c| c.label.clone())
            .collect::<Vec<String>>()
            .join(", ")
    );
    let matches = matches[0];
    if expected.documentation.is_some() {
        match expected.documentation.clone().unwrap() {
            Documentation::String(documentation) => {
                if let Documentation::String(source) = matches.documentation.clone().unwrap() {
                    assert_eq!(source, documentation);
                } else {
                    panic!("{} type should is String", expected.label)
                }
            }
            Documentation::MarkupContent(documentation) => {
                if let Documentation::MarkupContent(source) = matches.documentation.clone().unwrap()
                {
                    assert_eq!(source.value, documentation.value)
                } else {
                    panic!("{} type should is MarkupContent", expected.label)
                }
            }
        }
    }
    if expected.kind.is_some() {
        assert_eq!(matches.kind, expected.kind);
    }
    // 检验修改后的文档是否与期望相同
    if expected.result_text.is_some() && matches.text_edit.is_some() {
        let edit = matches.text_edit.clone().unwrap();
        if let CompletionTextEdit::Edit(edit) = edit {
            let start_offset = document.offset_at(edit.range.start) as usize;
            let end_offset = document.offset_at(edit.range.end) as usize;
            let text = document.get_content(None);
            assert_eq!(
                format!(
                    "{}{}{}",
                    &text[..start_offset],
                    edit.new_text,
                    &text[end_offset..]
                ),
                expected.result_text.unwrap()
            );
        } else {
            panic!(
                "{} text_edit should is CompletionTextEdit::Edit",
                matches.label
            )
        }
    }
    if expected.filter_text.is_some() {
        assert_eq!(
            matches.filter_text.as_ref().unwrap(),
            expected.filter_text.unwrap()
        );
    }
}

#[cfg(feature = "completion")]
fn test_quote_completion(
    value: &str,
    expected: Option<String>,
    options: Option<&CompletionConfiguration>,
) {
    let offset = value.find('|').unwrap();
    let value: &str = &format!("{}{}", &value[..offset], &value[offset + 1..]);

    let document = FullTextDocument::new("html".to_string(), 0, value.to_string());
    let position = document.position_at(offset as u32);

    let ls = HTMLLanguageService::new(&HTMLLanguageServiceOptions::default());
    let data_manager = ls.create_data_manager(true, None);
    let html_document = ls.parse_html_document(&document, &data_manager);
    let actual = ls.do_quote_complete(&document, &position, &html_document, options);
    assert_eq!(actual, expected);
}

#[cfg(feature = "completion")]
fn test_tag_completion(value: &str, expected: Option<String>) {
    let offset = value.find('|').unwrap();
    let value: &str = &format!("{}{}", &value[..offset], &value[offset + 1..]);

    let ls_options = HTMLLanguageServiceOptions::default();
    let ls = HTMLLanguageService::new(&ls_options);

    let document = FullTextDocument::new("html".to_string(), 0, value.to_string());
    let position = document.position_at(offset as u32);
    let data_manager = HTMLDataManager::default();
    let html_document = ls.parse_html_document(&document, &data_manager);
    let actual = ls.do_tag_complete(&document, &position, &html_document, &data_manager);
    assert_eq!(actual, expected);
}

#[cfg(feature = "completion")]
#[test]
fn complete() {
    test_completion_for(
        "<|",
        Expected {
            count: None,
            items: vec![
                ItemDescription {
                    label: "!DOCTYPE",
                    result_text: Some("<!DOCTYPE html>"),
                    ..Default::default()
                },
                ItemDescription {
                    label: "iframe",
                    result_text: Some("<iframe"),
                    ..Default::default()
                },
                ItemDescription {
                    label: "h1",
                    result_text: Some("<h1"),
                    ..Default::default()
                },
                ItemDescription {
                    label: "div",
                    result_text: Some("<div"),
                    ..Default::default()
                },
            ],
        },
        None,
        None,
    );

    test_completion_for(
        "\n<|",
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "!DOCTYPE",
                not_available: Some(true),
                ..Default::default()
            }],
        },
        None,
        None,
    );

    test_completion_for(
        "< |",
        Expected {
            count: None,
            items: vec![
                ItemDescription {
                    label: "iframe",
                    result_text: Some("<iframe"),
                    ..Default::default()
                },
                ItemDescription {
                    label: "h1",
                    result_text: Some("<h1"),
                    ..Default::default()
                },
                ItemDescription {
                    label: "div",
                    result_text: Some("<div"),
                    ..Default::default()
                },
            ],
        },
        None,
        None,
    );

    test_completion_for(
        "<h|",
        Expected {
            count: None,
            items: vec![
                ItemDescription {
                    label: "html",
                    result_text: Some("<html"),
                    ..Default::default()
                },
                ItemDescription {
                    label: "h1",
                    result_text: Some("<h1"),
                    ..Default::default()
                },
                ItemDescription {
                    label: "header",
                    result_text: Some("<header"),
                    ..Default::default()
                },
            ],
        },
        None,
        None,
    );
    test_completion_for(
        "<input|",
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "input",
                result_text: Some("<input"),
                ..Default::default()
            }],
        },
        None,
        None,
    );
    test_completion_for(
        "<inp|ut",
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "input",
                result_text: Some("<input"),
                ..Default::default()
            }],
        },
        None,
        None,
    );
    test_completion_for(
        "<|inp",
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "input",
                result_text: Some("<input"),
                ..Default::default()
            }],
        },
        None,
        None,
    );
    test_completion_for(
        "<input |",
        Expected {
            count: None,
            items: vec![
                ItemDescription {
                    label: "type",
                    result_text: Some(r#"<input type="$1""#),
                    ..Default::default()
                },
                ItemDescription {
                    label: "style",
                    result_text: Some(r#"<input style="$1""#),
                    ..Default::default()
                },
                ItemDescription {
                    label: "onmousemove",
                    result_text: Some(r#"<input onmousemove="$1""#),
                    ..Default::default()
                },
            ],
        },
        None,
        None,
    );
    test_completion_for(
        "<input t|",
        Expected {
            count: None,
            items: vec![
                ItemDescription {
                    label: "type",
                    result_text: Some(r#"<input type="$1""#),
                    ..Default::default()
                },
                ItemDescription {
                    label: "tabindex",
                    result_text: Some(r#"<input tabindex="$1""#),
                    ..Default::default()
                },
            ],
        },
        None,
        None,
    );
    test_completion_for(
        "<input t|ype",
        Expected {
            count: None,
            items: vec![
                ItemDescription {
                    label: "type",
                    result_text: Some(r#"<input type="$1""#),
                    ..Default::default()
                },
                ItemDescription {
                    label: "tabindex",
                    result_text: Some(r#"<input tabindex="$1""#),
                    ..Default::default()
                },
            ],
        },
        None,
        None,
    );
    test_completion_for(
        r#"<input t|ype="text""#,
        Expected {
            count: None,
            items: vec![
                ItemDescription {
                    label: "type",
                    result_text: Some(r#"<input type="text""#),
                    ..Default::default()
                },
                ItemDescription {
                    label: "tabindex",
                    result_text: Some(r#"<input tabindex="text""#),
                    ..Default::default()
                },
            ],
        },
        None,
        None,
    );
    test_completion_for(
        r#"<input type="text" |"#,
        Expected {
            count: None,
            items: vec![
                ItemDescription {
                    label: "style",
                    result_text: Some(r#"<input type="text" style="$1""#),
                    ..Default::default()
                },
                ItemDescription {
                    label: "type",
                    not_available: Some(true),
                    ..Default::default()
                },
                ItemDescription {
                    label: "size",
                    result_text: Some(r#"<input type="text" size="$1""#),
                    ..Default::default()
                },
            ],
        },
        None,
        None,
    );
    test_completion_for(
        r#"<input | type="text""#,
        Expected {
            count: None,
            items: vec![
                ItemDescription {
                    label: "style",
                    result_text: Some(r#"<input style="$1" type="text""#),
                    ..Default::default()
                },
                ItemDescription {
                    label: "type",
                    not_available: Some(true),
                    ..Default::default()
                },
                ItemDescription {
                    label: "size",
                    result_text: Some(r#"<input size="$1" type="text""#),
                    ..Default::default()
                },
            ],
        },
        None,
        None,
    );
    test_completion_for(
        r#"<input type="text" type="number" |"#,
        Expected {
            count: None,
            items: vec![
                ItemDescription {
                    label: "style",
                    result_text: Some(r#"<input type="text" type="number" style="$1""#),
                    ..Default::default()
                },
                ItemDescription {
                    label: "type",
                    not_available: Some(true),
                    ..Default::default()
                },
                ItemDescription {
                    label: "size",
                    result_text: Some(r#"<input type="text" type="number" size="$1""#),
                    ..Default::default()
                },
            ],
        },
        None,
        None,
    );
    test_completion_for(
        r#"<input type="text" s|"#,
        Expected {
            count: None,
            items: vec![
                ItemDescription {
                    label: "style",
                    result_text: Some(r#"<input type="text" style="$1""#),
                    ..Default::default()
                },
                ItemDescription {
                    label: "src",
                    result_text: Some(r#"<input type="text" src="$1""#),
                    ..Default::default()
                },
                ItemDescription {
                    label: "size",
                    result_text: Some(r#"<input type="text" size="$1""#),
                    ..Default::default()
                },
            ],
        },
        None,
        None,
    );
    test_completion_for(
        r#"<input type="text" s|"#,
        Expected {
            count: None,
            items: vec![
                ItemDescription {
                    label: "style",
                    result_text: Some(r#"<input type="text" style="$1""#),
                    ..Default::default()
                },
                ItemDescription {
                    label: "src",
                    result_text: Some(r#"<input type="text" src="$1""#),
                    ..Default::default()
                },
                ItemDescription {
                    label: "size",
                    result_text: Some(r#"<input type="text" size="$1""#),
                    ..Default::default()
                },
            ],
        },
        None,
        None,
    );

    test_completion_for(
        r#"<input di| type="text""#,
        Expected {
            count: None,
            items: vec![
                ItemDescription {
                    label: "disabled",
                    result_text: Some(r#"<input disabled type="text""#),
                    ..Default::default()
                },
                ItemDescription {
                    label: "dir",
                    result_text: Some(r#"<input dir="$1" type="text""#),
                    ..Default::default()
                },
            ],
        },
        None,
        None,
    );

    test_completion_for(
        r#"<input disabled | type="text""#,
        Expected {
            count: None,
            items: vec![
                ItemDescription {
                    label: "dir",
                    result_text: Some(r#"<input disabled dir="$1" type="text""#),
                    ..Default::default()
                },
                ItemDescription {
                    label: "style",
                    result_text: Some(r#"<input disabled style="$1" type="text""#),
                    ..Default::default()
                },
            ],
        },
        None,
        None,
    );

    test_completion_for(
        r#"<input type=|"#,
        Expected {
            count: None,
            items: vec![
                ItemDescription {
                    label: "text",
                    result_text: Some(r#"<input type="text""#),
                    ..Default::default()
                },
                ItemDescription {
                    label: "checkbox",
                    result_text: Some(r#"<input type="checkbox""#),
                    ..Default::default()
                },
            ],
        },
        None,
        None,
    );
    test_completion_for(
        r#"<input type="c|"#,
        Expected {
            count: None,
            items: vec![
                ItemDescription {
                    label: "color",
                    result_text: Some(r#"<input type="color"#),
                    ..Default::default()
                },
                ItemDescription {
                    label: "checkbox",
                    result_text: Some(r#"<input type="checkbox"#),
                    ..Default::default()
                },
            ],
        },
        None,
        None,
    );
    test_completion_for(
        r#"<input type="|"#,
        Expected {
            count: None,
            items: vec![
                ItemDescription {
                    label: "color",
                    result_text: Some(r#"<input type="color"#),
                    ..Default::default()
                },
                ItemDescription {
                    label: "checkbox",
                    result_text: Some(r#"<input type="checkbox"#),
                    ..Default::default()
                },
            ],
        },
        None,
        None,
    );
    test_completion_for(
        r#"<input type= |"#,
        Expected {
            count: None,
            items: vec![
                ItemDescription {
                    label: "color",
                    result_text: Some(r#"<input type= "color""#),
                    ..Default::default()
                },
                ItemDescription {
                    label: "checkbox",
                    result_text: Some(r#"<input type= "checkbox""#),
                    ..Default::default()
                },
            ],
        },
        None,
        None,
    );
    test_completion_for(
        r#"<input src="c" type="color|" "#,
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "color",
                result_text: Some(r#"<input src="c" type="color" "#),
                ..Default::default()
            }],
        },
        None,
        None,
    );
    test_completion_for(
        r#"<iframe sandbox="allow-forms |"#,
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "allow-modals",
                result_text: Some(r#"<iframe sandbox="allow-forms allow-modals"#),
                ..Default::default()
            }],
        },
        None,
        None,
    );
    test_completion_for(
        r#"<iframe sandbox="allow-forms allow-modals|"#,
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "allow-modals",
                result_text: Some(r#"<iframe sandbox="allow-forms allow-modals"#),
                ..Default::default()
            }],
        },
        None,
        None,
    );
    test_completion_for(
        r#"<iframe sandbox="allow-forms all|""#,
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "allow-modals",
                result_text: Some(r#"<iframe sandbox="allow-forms allow-modals""#),
                ..Default::default()
            }],
        },
        None,
        None,
    );
    test_completion_for(
        r#"<iframe sandbox="allow-forms a|llow-modals ""#,
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "allow-modals",
                result_text: Some(r#"<iframe sandbox="allow-forms allow-modals ""#),
                ..Default::default()
            }],
        },
        None,
        None,
    );
    test_completion_for(
        r#"<input src="c" type=color| "#,
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "color",
                result_text: Some(r#"<input src="c" type="color" "#),
                ..Default::default()
            }],
        },
        None,
        None,
    );
    test_completion_for(
        r#"<div dir=|></div>"#,
        Expected {
            count: None,
            items: vec![
                ItemDescription {
                    label: "ltr",
                    result_text: Some(r#"<div dir="ltr"></div>"#),
                    ..Default::default()
                },
                ItemDescription {
                    label: "rtl",
                    result_text: Some(r#"<div dir="rtl"></div>"#),
                    ..Default::default()
                },
            ],
        },
        None,
        None,
    );
    test_completion_for(
        r#"<ul><|>"#,
        Expected {
            count: None,
            items: vec![
                ItemDescription {
                    label: "/ul",
                    result_text: Some(r#"<ul></ul>"#),
                    ..Default::default()
                },
                ItemDescription {
                    label: "li",
                    result_text: Some(r#"<ul><li>"#),
                    ..Default::default()
                },
            ],
        },
        None,
        None,
    );
    test_completion_for(
        r#"<ul><li><|"#,
        Expected {
            count: None,
            items: vec![
                ItemDescription {
                    label: "/li",
                    result_text: Some(r#"<ul><li></li>"#),
                    ..Default::default()
                },
                ItemDescription {
                    label: "a",
                    result_text: Some(r#"<ul><li><a"#),
                    ..Default::default()
                },
            ],
        },
        None,
        None,
    );
    test_completion_for(
        r#"<goo></|>"#,
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "/goo",
                result_text: Some(r#"<goo></goo>"#),
                ..Default::default()
            }],
        },
        None,
        None,
    );
    test_completion_for(
        r#"<foo></f|"#,
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "/foo",
                result_text: Some(r#"<foo></foo>"#),
                ..Default::default()
            }],
        },
        None,
        None,
    );
    test_completion_for(
        r#"<foo></f|o"#,
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "/foo",
                result_text: Some(r#"<foo></foo>"#),
                ..Default::default()
            }],
        },
        None,
        None,
    );
    test_completion_for(
        r#"<foo></|fo"#,
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "/foo",
                result_text: Some(r#"<foo></foo>"#),
                ..Default::default()
            }],
        },
        None,
        None,
    );
    test_completion_for(
        r#"<foo></ |>"#,
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "/foo",
                result_text: Some(r#"<foo></foo>"#),
                ..Default::default()
            }],
        },
        None,
        None,
    );
    test_completion_for(
        r#"<span></ s|"#,
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "/span",
                result_text: Some(r#"<span></span>"#),
                ..Default::default()
            }],
        },
        None,
        None,
    );
    test_completion_for(
        r#"<li><br></ |>"#,
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "/li",
                result_text: Some(r#"<li><br></li>"#),
                ..Default::default()
            }],
        },
        None,
        None,
    );
    test_completion_for(
        "<li/|>",
        Expected {
            count: Some(0),
            items: vec![],
        },
        None,
        None,
    );
    test_completion_for(
        "  <div/|   ",
        Expected {
            count: Some(0),
            items: vec![],
        },
        None,
        None,
    );
    test_completion_for(
        r#"<foo><br/></ f|>"#,
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "/foo",
                result_text: Some(r#"<foo><br/></foo>"#),
                ..Default::default()
            }],
        },
        None,
        None,
    );
    test_completion_for(
        r#"<li><div/></|"#,
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "/li",
                result_text: Some(r#"<li><div/></li>"#),
                ..Default::default()
            }],
        },
        None,
        None,
    );
    test_completion_for(
        "<li><br/|>",
        Expected {
            count: Some(0),
            items: vec![],
        },
        None,
        None,
    );
    test_completion_for(
        "<li><br>a/|",
        Expected {
            count: Some(0),
            items: vec![],
        },
        None,
        None,
    );

    test_completion_for(
        r#"<foo><bar></bar></|   "#,
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "/foo",
                result_text: Some(r#"<foo><bar></bar></foo>   "#),
                ..Default::default()
            }],
        },
        None,
        None,
    );
    test_completion_for(
            "<div>\n  <form>\n    <div>\n      <label></label>\n      <|\n    </div>\n  </form></div>",
            Expected {
                count: None,
                items: vec![
                    ItemDescription {
                        label: "span",
                        result_text: Some(
                            "<div>\n  <form>\n    <div>\n      <label></label>\n      <span\n    </div>\n  </form></div>",
                        ),
                        ..Default::default()
                    },
                    ItemDescription {
                        label: "/div",
                        result_text: Some(
                            "<div>\n  <form>\n    <div>\n      <label></label>\n    </div>\n    </div>\n  </form></div>",
                        ),
                        ..Default::default()
                    },
                ],
            },
            None,
            None,
        );
    test_completion_for(
        r#"<body><div><div></div></div></|  >"#,
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "/body",
                result_text: Some(r#"<body><div><div></div></div></body  >"#),
                ..Default::default()
            }],
        },
        None,
        None,
    );
    test_completion_for(
        "<body>\n  <div>\n    </|",
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "/div",
                result_text: Some("<body>\n  <div>\n  </div>"),
                ..Default::default()
            }],
        },
        None,
        None,
    );
    test_completion_for(
        "<div><a hre|</div>",
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "href",
                result_text: Some(r#"<div><a href="$1"</div>"#),
                ..Default::default()
            }],
        },
        None,
        None,
    );
    test_completion_for(
        "<a><b>foo</b><|f>",
        Expected {
            count: None,
            items: vec![
                ItemDescription {
                    label: "/a",
                    result_text: Some("<a><b>foo</b></a>"),
                    ..Default::default()
                },
                ItemDescription {
                    not_available: Some(true),
                    label: "/f",
                    ..Default::default()
                },
            ],
        },
        None,
        None,
    );
    test_completion_for(
        "<a><b>foo</b><| bar.",
        Expected {
            count: None,
            items: vec![
                ItemDescription {
                    label: "/a",
                    result_text: Some("<a><b>foo</b></a> bar."),
                    ..Default::default()
                },
                ItemDescription {
                    not_available: Some(true),
                    label: "/bar",
                    ..Default::default()
                },
            ],
        },
        None,
        None,
    );
    test_completion_for(
        "<div><h1><br><span></span><img></| </h1></div>",
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "/h1",
                result_text: Some(r#"<div><h1><br><span></span><img></h1> </h1></div>"#),
                ..Default::default()
            }],
        },
        None,
        None,
    );
    test_completion_for(
        "<div>|",
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "</div>",
                result_text: Some(r#"<div>$0</div>"#),
                ..Default::default()
            }],
        },
        None,
        None,
    );
    test_completion_for(
        r#"<div>|"#,
        Expected {
            count: None,
            items: vec![ItemDescription {
                not_available: Some(true),
                label: "</div>",
                ..Default::default()
            }],
        },
        Some(CompletionConfiguration {
            hide_auto_complete_proposals: true,
            attribute_default_value: Quotes::Double,
            provider: HashMap::new(),
        }),
        None,
    );
    test_completion_for(
        r#"<div d|"#,
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "data-",
                result_text: Some(r#"<div data-$1="$2""#),
                ..Default::default()
            }],
        },
        None,
        None,
    );
    test_completion_for(
        r#"<div no-data-test="no-data" d|"#,
        Expected {
            count: None,
            items: vec![ItemDescription {
                not_available: Some(true),
                label: "no-data-test",
                ..Default::default()
            }],
        },
        None,
        None,
    );
    test_completion_for(
        r#"<div data-custom="test"><div d|"#,
        Expected {
            count: None,
            items: vec![
                ItemDescription {
                    label: "data-",
                    result_text: Some(r#"<div data-custom="test"><div data-$1="$2""#),
                    ..Default::default()
                },
                ItemDescription {
                    label: "data-custom",
                    result_text: Some(r#"<div data-custom="test"><div data-custom="$1""#),
                    ..Default::default()
                },
            ],
        },
        None,
        None,
    );
    test_completion_for(
        r#"<div data-custom="test"><div data-custom-two="2"></div></div>\n <div d|"#,
        Expected {
            count: None,
            items: vec![
                ItemDescription {
                    label: "data-",
                    result_text: Some(
                        r#"<div data-custom="test"><div data-custom-two="2"></div></div>\n <div data-$1="$2""#,
                    ),
                    ..Default::default()
                },
                ItemDescription {
                    label: "data-custom",
                    result_text: Some(
                        r#"<div data-custom="test"><div data-custom-two="2"></div></div>\n <div data-custom="$1""#,
                    ),
                    ..Default::default()
                },
                ItemDescription {
                    label: "data-custom-two",
                    result_text: Some(
                        r#"<div data-custom="test"><div data-custom-two="2"></div></div>\n <div data-custom-two="$1""#,
                    ),
                    ..Default::default()
                },
            ],
        },
        None,
        None,
    );
    test_completion_for(
        r#"<body data-ng-app=""><div id="first" data-ng-include=" 'firstdoc.html' "></div><div id="second" inc|></div></body>"#,
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "data-ng-include",
                result_text: Some(
                    r#"<body data-ng-app=""><div id="first" data-ng-include=" 'firstdoc.html' "></div><div id="second" data-ng-include="$1"></div></body>"#,
                ),
                ..Default::default()
            }],
        },
        None,
        None,
    );
}

#[cfg(feature = "completion")]
#[test]
fn references() {
    let doc =
			"The div element has no special meaning at all. It represents its children. It can be used with the class, lang, and title attributes to mark up semantics common to a group of consecutive elements.".to_string() +
			"\n\n" +
			"[MDN Reference](https://developer.mozilla.org/docs/Web/HTML/Element/div)";

    test_completion_for(
        "<d|",
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "div",
                result_text: Some("<div"),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: doc,
                })),
                ..Default::default()
            }],
        },
        None,
        None,
    );
}

#[cfg(feature = "completion")]
#[test]
fn case_sensitivity() {
    test_completion_for(
        "<LI></|",
        Expected {
            count: None,
            items: vec![
                ItemDescription {
                    label: "/LI",
                    result_text: Some("<LI></LI>"),
                    ..Default::default()
                },
                ItemDescription {
                    label: "/li",
                    not_available: Some(true),
                    ..Default::default()
                },
            ],
        },
        None,
        None,
    );
    test_completion_for(
        "<lI></|",
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "/lI",
                result_text: Some("<lI></lI>"),
                ..Default::default()
            }],
        },
        None,
        None,
    );
    test_completion_for(
        "<iNpUt |",
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "type",
                result_text: Some(r#"<iNpUt type="$1""#),
                ..Default::default()
            }],
        },
        None,
        None,
    );
    test_completion_for(
        "<INPUT TYPE=|",
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "color",
                result_text: Some(r#"<INPUT TYPE="color""#),
                ..Default::default()
            }],
        },
        None,
        None,
    );
    test_completion_for(
        "<dIv>|",
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "</dIv>",
                result_text: Some("<dIv>$0</dIv>"),
                ..Default::default()
            }],
        },
        None,
        None,
    );
}

#[cfg(feature = "completion")]
#[test]
fn handlebar_completion() {
    test_completion_for(
        r#"<script id="entry-template" type="text/x-handlebars-template"> <| </script>"#,
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "div",
                result_text: Some(
                    r#"<script id="entry-template" type="text/x-handlebars-template"> <div </script>"#,
                ),
                ..Default::default()
            }],
        },
        None,
        None,
    );
}

#[cfg(feature = "completion")]
#[test]
fn support_script_type() {
    test_completion_for(
        r#"<script id="html-template" type="text/html"> <| </script>"#,
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "div",
                result_text: Some(r#"<script id="html-template" type="text/html"> <div </script>"#),
                ..Default::default()
            }],
        },
        None,
        None,
    );
}

#[cfg(feature = "completion")]
#[test]
fn complete_aria() {
    let expected_aria_attributes = vec![
        ItemDescription {
            label: "aria-activedescendant",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-atomic",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-autocomplete",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-busy",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-checked",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-colcount",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-colindex",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-colspan",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-controls",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-current",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-describedby",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-disabled",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-dropeffect",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-errormessage",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-expanded",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-flowto",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-grabbed",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-haspopup",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-hidden",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-invalid",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-label",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-labelledby",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-level",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-live",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-modal",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-multiline",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-multiselectable",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-orientation",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-owns",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-placeholder",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-posinset",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-pressed",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-readonly",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-relevant",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-required",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-roledescription",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-rowcount",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-rowindex",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-rowspan",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-selected",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-setsize",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-sort",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-valuemax",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-valuemin",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-valuenow",
            ..Default::default()
        },
        ItemDescription {
            label: "aria-valuetext",
            ..Default::default()
        },
    ];

    test_completion_for(
        "<div  |> </div >",
        Expected {
            count: None,
            items: expected_aria_attributes.clone(),
        },
        None,
        None,
    );
    test_completion_for(
        "<span  |> </span >",
        Expected {
            count: None,
            items: expected_aria_attributes.clone(),
        },
        None,
        None,
    );
    test_completion_for(
        "<input  |> </input >",
        Expected {
            count: None,
            items: expected_aria_attributes.clone(),
        },
        None,
        None,
    );
}

#[cfg(feature = "completion")]
#[test]
fn settings() {
    test_completion_for(
        "<|",
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "div",
                not_available: Some(true),
                ..Default::default()
            }],
        },
        Some(CompletionConfiguration {
            hide_auto_complete_proposals: false,
            attribute_default_value: Quotes::Double,
            provider: HashMap::from([("html5".to_string(), false)]),
        }),
        None,
    );
    test_completion_for(
        "<div clas|",
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "class",
                result_text: Some(r#"<div class="$1""#),
                ..Default::default()
            }],
        },
        Some(CompletionConfiguration {
            hide_auto_complete_proposals: false,
            attribute_default_value: Quotes::Double,
            provider: HashMap::new(),
        }),
        None,
    );
    test_completion_for(
        "<div clas|",
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "class",
                result_text: Some("<div class='$1'"),
                ..Default::default()
            }],
        },
        Some(CompletionConfiguration {
            hide_auto_complete_proposals: false,
            attribute_default_value: Quotes::Single,
            provider: HashMap::new(),
        }),
        None,
    );
    test_completion_for(
        "<div clas|",
        Expected {
            count: None,
            items: vec![ItemDescription {
                label: "class",
                result_text: Some("<div class=$1"),
                ..Default::default()
            }],
        },
        Some(CompletionConfiguration {
            hide_auto_complete_proposals: false,
            attribute_default_value: Quotes::None,
            provider: HashMap::new(),
        }),
        None,
    );
}

#[cfg(feature = "completion")]
#[test]
fn do_quote_complete() {
    test_quote_completion("<a foo=|", Some(r#""$1""#.to_string()), None);
    test_quote_completion(
        "<a foo=|",
        Some("'$1'".to_string()),
        Some(&CompletionConfiguration {
            attribute_default_value: Quotes::Single,
            hide_auto_complete_proposals: false,
            provider: HashMap::new(),
        }),
    );
    test_quote_completion(
        "<a foo=|",
        None,
        Some(&CompletionConfiguration {
            attribute_default_value: Quotes::None,
            hide_auto_complete_proposals: false,
            provider: HashMap::new(),
        }),
    );
    test_quote_completion("<a foo=|=", None, None);
    test_quote_completion(r#"<a foo=|"bar""#, None, None);
    test_quote_completion("<a foo=|></a>", Some(r#""$1""#.to_string()), None);
    test_quote_completion(r#"<a foo="bar=|""#, None, None);
    test_quote_completion(r#"<a baz=| foo="bar">"#, Some(r#""$1""#.to_string()), None);
    test_quote_completion("<a>< foo=| /a>", None, None);
    test_quote_completion("<a></ foo=| a>", None, None);
    test_quote_completion(
        r#"<a foo="bar" \n baz=| ></a>"#,
        Some(r#""$1""#.to_string()),
        None,
    );
}

#[cfg(feature = "completion")]
#[test]
fn do_tag_complete() {
    test_tag_completion("<div>|", Some("$0</div>".to_string()));
    test_tag_completion("<div>|</div>", None);
    test_tag_completion(r#"<div class="">|"#, Some("$0</div>".to_string()));
    test_tag_completion("<img>|", None);
    test_tag_completion("<div><br></|", Some("div>".to_string()));
    test_tag_completion("<div><br><span></span></|", Some("div>".to_string()));
    test_tag_completion(
        "<div><h1><br><span></span><img></| </h1></div>",
        Some("h1>".to_string()),
    );
    test_tag_completion(
        "<ng-template><td><ng-template></|   </td> </ng-template>",
        Some("ng-template>".to_string()),
    );
    test_tag_completion("<div><br></|>", Some("div".to_string()));
}

#[cfg(feature = "completion")]
#[derive(Default)]
struct Expected {
    count: Option<usize>,
    items: Vec<ItemDescription>,
}

#[cfg(feature = "completion")]
#[derive(Default, Clone)]
struct ItemDescription {
    label: &'static str,
    result_text: Option<&'static str>,
    kind: Option<CompletionItemKind>,
    documentation: Option<Documentation>,
    filter_text: Option<&'static str>,
    not_available: Option<bool>,
}
