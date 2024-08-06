#[cfg(feature = "folding")]
use html_languageservice::{FoldingRangeContext, HTMLDataManager, HTMLLanguageService};
#[cfg(feature = "folding")]
use lsp_textdocument::FullTextDocument;
#[cfg(feature = "folding")]
use lsp_types::FoldingRangeKind;

#[cfg(feature = "folding")]
fn assert_ranges(
    lines: &[&str],
    expected: &[ExpectedIndentRange],
    message: Option<&str>,
    range_limit: Option<usize>,
) {
    let document = FullTextDocument::new("json".to_string(), 1, lines.join("\n"));
    let actual = HTMLLanguageService::get_folding_ranges(
        document,
        FoldingRangeContext { range_limit },
        &HTMLDataManager::default(),
    );

    let mut actual_ranges = vec![];
    for i in 0..actual.len() {
        actual_ranges.push(ExpectedIndentRange::new(
            actual[i].start_line,
            actual[i].end_line,
            actual[i].kind.clone(),
        ))
    }
    actual_ranges.sort_by(|r1, r2| r1.start_line.cmp(&r2.start_line));
    if let Some(message) = message {
        assert_eq!(actual_ranges, expected, "{message}");
    } else {
        assert_eq!(actual_ranges, expected);
    }
}

#[cfg(feature = "folding")]
fn r(start_line: u32, end_line: u32) -> ExpectedIndentRange {
    ExpectedIndentRange::new(start_line, end_line, None)
}

#[cfg(feature = "folding")]
fn rc(start_line: u32, end_line: u32) -> ExpectedIndentRange {
    ExpectedIndentRange::new(start_line, end_line, Some(FoldingRangeKind::Comment))
}

#[cfg(feature = "folding")]
fn rr(start_line: u32, end_line: u32) -> ExpectedIndentRange {
    ExpectedIndentRange::new(start_line, end_line, Some(FoldingRangeKind::Region))
}

#[cfg(feature = "folding")]
#[test]
fn fold_one_level() {
    assert_ranges(
        &[
            "<html>",  // 0
            "Hello",   // 1
            "</html>", // 2
        ],
        &[r(0, 1)],
        None,
        None,
    );
}

#[cfg(feature = "folding")]
#[test]
fn fold_two_level() {
    assert_ranges(
        &[
            "<html>",  // 0
            "<head>",  // 1
            "Hello",   // 2
            "</head>", // 3
            "</html>", // 4
        ],
        &[r(0, 3), r(1, 2)],
        None,
        None,
    );
}

#[cfg(feature = "folding")]
#[test]
fn fold_siblings() {
    assert_ranges(
        &[
            "<html>",              // 0
            "<head>",              // 1
            "Head",                // 2
            "</head>",             // 3
            r#"<body class="f">"#, // 4
            "Body",                // 5
            "</body>",             // 6
            "</html>",             // 7
        ],
        &[r(0, 6), r(1, 2), r(4, 5)],
        None,
        None,
    );
}

#[cfg(feature = "folding")]
#[test]
fn fold_self_closing_tags() {
    assert_ranges(
        &[
            "<div>",              // 0
            r#"<a href="top"/>"#, // 1
            r#"<img src="s">"#,   // 2
            "<br/>",              // 3
            "<br>",               // 4
            r#"<img class="c""#,  // 5
            r#"     src="top""#,  // 6
            ">",                  // 7
            "</div>",             // 8
        ],
        &[r(0, 7), r(5, 6)],
        None,
        None,
    );
}

#[cfg(feature = "folding")]
#[test]
fn fold_comment() {
    assert_ranges(
        &[
            "<!--",                 // 0
            " multi line",          // 1
            "-->",                  // 2
            "<!-- some stuff",      // 3
            " some more stuff -->", // 4
        ],
        &[rc(0, 2), rc(3, 4)],
        None,
        None,
    );
}

#[cfg(feature = "folding")]
#[test]
fn fold_regions() {
    assert_ranges(
        &[
            "<!-- #region -->",    // 0
            "<!-- #region -->",    // 1
            "<!-- #endregion -->", // 2
            "<!-- #endregion -->", // 3
        ],
        &[rr(0, 3), rr(1, 2)],
        None,
        None,
    );
}

#[cfg(feature = "folding")]
#[test]
fn fold_incomplete() {
    assert_ranges(
        &[
            "<body>",      // 0
            "<div></div>", // 1
            "Hello",       // 2
            "</div>",      // 3
            "</body>",     // 4
        ],
        &[r(0, 3)],
        None,
        None,
    );
    assert_ranges(
        &[
            "<be><div>",           // 0
            "<!-- #endregion -->", // 1
            "</div>",              // 2
        ],
        &[r(0, 1)],
        None,
        None,
    );
}

#[cfg(feature = "folding")]
#[test]
fn fold_intersecting_region() {
    assert_ranges(
        &[
            "<body>",              // 0
            "<!-- #region -->",    // 1
            "Hello",               // 2
            "<div></div>",         // 3
            "</body>",             // 4
            "<!-- #endregion -->", // 5
        ],
        &[r(0, 3)],
        None,
        None,
    );

    assert_ranges(
        &[
            "<!-- #region -->",    // 0
            "<body>",              // 1
            "Hello",               // 2
            "<!-- #endregion -->", // 3
            "<div></div>",         // 4
            "</body>",             // 5
        ],
        &[rr(0, 3)],
        None,
        None,
    );
}

#[cfg(feature = "folding")]
#[test]
fn test_limit() {
    let input = [
        "<div>",      //  0
        " <span>",    //  1
        "  <b>",      //  2
        "  ",         //  3
        "  </b>,",    //  4
        "  <b>",      //  5
        "   <pre>",   //  6
        "  ",         //  7
        "   </pre>,", //  8
        "   <pre>",   //  9
        "  ",         // 10
        "   </pre>,", // 11
        "  </b>,",    // 12
        "  <b>",      // 13
        "  ",         // 14
        "  </b>,",    // 15
        "  <b>",      // 16
        "  ",         // 17
        "  </b>",     // 18
        " </span>",   // 19
        "</div>",     // 20
    ];
    assert_ranges(
        &input,
        &[
            r(0, 19),
            r(1, 18),
            r(2, 3),
            r(5, 11),
            r(6, 7),
            r(9, 10),
            r(13, 14),
            r(16, 17),
        ],
        Some("no limit"),
        None,
    );

    assert_ranges(
        &input,
        &[
            r(0, 19),
            r(1, 18),
            r(2, 3),
            r(5, 11),
            r(6, 7),
            r(9, 10),
            r(13, 14),
            r(16, 17),
        ],
        Some("limit 8"),
        Some(8),
    );
    assert_ranges(
        &input,
        &[
            r(0, 19),
            r(1, 18),
            r(2, 3),
            r(5, 11),
            r(6, 7),
            r(13, 14),
            r(16, 17),
        ],
        Some("limit 7"),
        Some(7),
    );
    assert_ranges(
        &input,
        &[r(0, 19), r(1, 18), r(2, 3), r(5, 11), r(13, 14), r(16, 17)],
        Some("limit 6"),
        Some(6),
    );
    assert_ranges(
        &input,
        &[r(0, 19), r(1, 18), r(2, 3), r(5, 11), r(13, 14)],
        Some("limit 5"),
        Some(5),
    );
    assert_ranges(
        &input,
        &[r(0, 19), r(1, 18), r(2, 3), r(5, 11)],
        Some("limit 4"),
        Some(4),
    );
    assert_ranges(
        &input,
        &[r(0, 19), r(1, 18), r(2, 3)],
        Some("limit 3"),
        Some(3),
    );
    assert_ranges(&input, &[r(0, 19), r(1, 18)], Some("limit 2"), Some(2));
    assert_ranges(&input, &[r(0, 19)], Some("limit 1"), Some(1));
}

#[cfg(feature = "folding")]
#[derive(PartialEq, Debug)]
struct ExpectedIndentRange {
    start_line: u32,
    end_line: u32,
    kind: Option<FoldingRangeKind>,
}

#[cfg(feature = "folding")]
impl ExpectedIndentRange {
    pub fn new(start_line: u32, end_line: u32, kind: Option<FoldingRangeKind>) -> Self {
        ExpectedIndentRange {
            start_line,
            end_line,
            kind,
        }
    }
}
