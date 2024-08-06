#[cfg(feature = "links")]
use html_languageservice::{DocumentContext, HTMLDataManager, HTMLLanguageService};
#[cfg(feature = "links")]
use lsp_textdocument::FullTextDocument;
#[cfg(feature = "links")]
use lsp_types::{DocumentLink, Position, Range, Url};

#[cfg(feature = "links")]
struct LinkDocumentContent;

#[cfg(feature = "links")]
impl DocumentContext for LinkDocumentContent {
    fn resolve_reference(&self, reference: &str, base: &str) -> Option<String> {
        if let Ok(uri) = Url::parse(base) {
            if let Ok(uri) = uri.join(reference) {
                Some(uri.to_string())
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[cfg(feature = "links")]
fn test_link_creation(model_url: &str, token_content: &str, expected: Option<&str>) {
    let language_id = if let Some(index) = model_url.rfind(".") {
        let lang = model_url[index..].to_string();
        if lang == "hbs" {
            lang
        } else {
            "html".to_string()
        }
    } else {
        "html".to_string()
    };
    let uri = Url::parse(model_url).unwrap();
    let document = FullTextDocument::new(language_id, 0, format!(r#"<a href="{}""#, token_content));
    let mut data_manager = HTMLDataManager::default();
    let links = HTMLLanguageService::find_document_links(
        &uri,
        &document,
        &LinkDocumentContent,
        &mut data_manager,
    );
    assert_eq!(
        if links.len() > 0 {
            links[0].target.as_ref().map(|v| v.to_string())
        } else {
            None
        },
        expected.map(|v| v.to_string())
    );
}

#[cfg(feature = "links")]
fn test_link_detection(value: &str, expected_links: Vec<DocumentLink>) {
    let uri = Url::parse("file:///test/data/abc/test.html").unwrap();
    let document = FullTextDocument::new("html".to_string(), 0, value.to_string());
    let mut data_manager = HTMLDataManager::default();
    let links = HTMLLanguageService::find_document_links(
        &uri,
        &document,
        &LinkDocumentContent,
        &mut data_manager,
    );

    assert_eq!(links, expected_links);
}

#[cfg(feature = "links")]
#[test]
fn link_creation() {
    test_link_creation("http://model/1.html", "javascript:void;", None);
    test_link_creation("http://model/1.html", " \tjavascript:alert(7);", None);
    test_link_creation(
        "http://model/1.html",
        " #relative",
        Some("http://model/1.html"),
    );
    test_link_creation(
        "http://model/1.html",
        "file:///C:\\Alex\\src\\path\\to\\file.txt",
        Some("file:///C:/Alex/src/path/to/file.txt"),
    );
    test_link_creation(
        "http://model/1.html",
        "http://www.microsoft.com/",
        Some("http://www.microsoft.com/"),
    );
    test_link_creation(
        "http://model/1.html",
        "https://www.microsoft.com/",
        Some("https://www.microsoft.com/"),
    );
    test_link_creation(
        "http://model/1.html",
        "//www.microsoft.com/",
        Some("http://www.microsoft.com/"),
    );
    test_link_creation("http://model/x/1.html", "a.js", Some("http://model/x/a.js"));
    test_link_creation(
        "http://model/x/1.html",
        "./a2.js",
        Some("http://model/x/a2.js"),
    );
    test_link_creation("http://model/x/1.html", "/b.js", Some("http://model/b.js"));
    test_link_creation(
        "http://model/x/y/1.html",
        "../../c.js",
        Some("http://model/c.js"),
    );

    test_link_creation(
        "file:///C:/Alex/src/path/to/file.html",
        "javascript:void;",
        None,
    );
    test_link_creation(
        "file:///C:/Alex/src/path/to/file.html",
        " \tjavascript:alert(7);",
        None,
    );
    test_link_creation(
        "file:///C:/Alex/src/path/to/file.html",
        " #relative",
        Some("file:///C:/Alex/src/path/to/file.html"),
    );
    test_link_creation(
        "file:///C:/Alex/src/path/to/file.html",
        "file:///C:\\Alex\\src\\path\\to\\file.txt",
        Some("file:///C:/Alex/src/path/to/file.txt"),
    );
    test_link_creation(
        "file:///C:/Alex/src/path/to/file.html",
        "http://www.microsoft.com/",
        Some("http://www.microsoft.com/"),
    );
    test_link_creation(
        "file:///C:/Alex/src/path/to/file.html",
        "https://www.microsoft.com/",
        Some("https://www.microsoft.com/"),
    );
    test_link_creation(
        "file:///C:/Alex/src/path/to/file.html",
        "  //www.microsoft.com/",
        Some("http://www.microsoft.com/"),
    );
    test_link_creation(
        "file:///C:/Alex/src/path/to/file.html",
        "a.js",
        Some("file:///C:/Alex/src/path/to/a.js"),
    );
    test_link_creation(
        "file:///C:/Alex/src/path/to/file.html",
        "/a.js",
        Some("file:///C:/a.js"),
    );

    test_link_creation(
        "https://www.test.com/path/to/file.html",
        "file:///C:\\Alex\\src\\path\\to\\file.txt",
        Some("file:///C:/Alex/src/path/to/file.txt"),
    );
    test_link_creation(
        "https://www.test.com/path/to/file.html",
        "//www.microsoft.com/",
        Some("https://www.microsoft.com/"),
    );
    test_link_creation(
        "https://www.test.com/path/to/file.html",
        "//www.microsoft.com/",
        Some("https://www.microsoft.com/"),
    );

    // invalid uris are ignored
    test_link_creation("https://www.test.com/path/to/file.html", "%", None);

    test_link_creation(
        "file:///c:/Alex/working_dir/18314-link-detection/test.html",
        "/class/class.js",
        Some("file:///c:/class/class.js"),
    );

    test_link_creation(
        "http://foo/bar.hbs",
        "/class/class.js",
        Some("http://foo/class/class.js"),
    );
}

#[cfg(feature = "links")]
#[test]
fn link_detection() {
    test_link_detection(
        r#"<img src="foo.png">"#,
        vec![DocumentLink {
            range: Range::new(Position::new(0, 10), Position::new(0, 17)),
            target: Some(Url::parse("file:///test/data/abc/foo.png").unwrap()),
            tooltip: None,
            data: None,
        }],
    );
    test_link_detection(
        r#"<a href="http://server/foo.html">"#,
        vec![DocumentLink {
            range: Range::new(Position::new(0, 9), Position::new(0, 31)),
            target: Some(Url::parse("http://server/foo.html").unwrap()),
            tooltip: None,
            data: None,
        }],
    );
    test_link_detection(r#"<img src="">"#, vec![]);
    test_link_detection(
        r#"<LINK HREF="a.html">"#,
        vec![DocumentLink {
            range: Range::new(Position::new(0, 12), Position::new(0, 18)),
            target: Some(Url::parse("file:///test/data/abc/a.html").unwrap()),
            tooltip: None,
            data: None,
        }],
    );
    test_link_detection(&format!("{}{}", r#"<LINK HREF="a.html"#, "\n>\n"), vec![]);
    test_link_detection(
        r#"<a href=http://www.example.com></a>"#,
        vec![DocumentLink {
            range: Range::new(Position::new(0, 8), Position::new(0, 30)),
            target: Some(Url::parse("http://www.example.com").unwrap()),
            tooltip: None,
            data: None,
        }],
    );

    test_link_detection(
        r#"<html><base href="docs/"><img src="foo.png"></html>"#,
        vec![DocumentLink {
            range: Range::new(Position::new(0, 35), Position::new(0, 42)),
            target: Some(Url::parse("file:///test/data/abc/docs/foo.png").unwrap()),
            tooltip: None,
            data: None,
        }],
    );
    test_link_detection(
        r#"<html><base href="http://www.example.com/page.html"><img src="foo.png"></html>"#,
        vec![DocumentLink {
            range: Range::new(Position::new(0, 62), Position::new(0, 69)),
            target: Some(Url::parse("http://www.example.com/foo.png").unwrap()),
            tooltip: None,
            data: None,
        }],
    );
    test_link_detection(
        r#"<html><base href=".."><img src="foo.png"></html>"#,
        vec![DocumentLink {
            range: Range::new(Position::new(0, 32), Position::new(0, 39)),
            target: Some(Url::parse("file:///test/data/foo.png").unwrap()),
            tooltip: None,
            data: None,
        }],
    );
    test_link_detection(
        r#"<html><base href="."><img src="foo.png"></html>"#,
        vec![DocumentLink {
            range: Range::new(Position::new(0, 31), Position::new(0, 38)),
            target: Some(Url::parse("file:///test/data/abc/foo.png").unwrap()),
            tooltip: None,
            data: None,
        }],
    );
    test_link_detection(
        r#"<html><base href="/docs/"><img src="foo.png"></html>"#,
        vec![DocumentLink {
            range: Range::new(Position::new(0, 36), Position::new(0, 43)),
            target: Some(Url::parse("file:///docs/foo.png").unwrap()),
            tooltip: None,
            data: None,
        }],
    );

    test_link_detection(
        r#"<a href="mailto:<%- mail %>@<%- domain %>" > <% - mail %>@<% - domain %> </a>"#,
        vec![],
    );

    test_link_detection(
        r#"<link rel="icon" type="image/x-icon" href="data:@file/x-icon;base64#,AAABAAIAQEAAAAEAIAAoQgAAJgA">"#,
        vec![],
    );
    test_link_detection(
        r#"<blockquote cite="foo.png">"#,
        vec![DocumentLink {
            range: Range::new(Position::new(0, 18), Position::new(0, 25)),
            target: Some(Url::parse("file:///test/data/abc/foo.png").unwrap()),
            tooltip: None,
            data: None,
        }],
    );
    test_link_detection(
        r#"<style src="styles.css?t=345">"#,
        vec![DocumentLink {
            range: Range::new(Position::new(0, 12), Position::new(0, 28)),
            target: Some(Url::parse("file:///test/data/abc/styles.css").unwrap()),
            tooltip: None,
            data: None,
        }],
    );
    test_link_detection(
        r#"<a href="https://werkenvoor.be/nl/jobs?f%5B0%5D=activitydomains%3A115&f%5B1%5D=lang%3Anl">link</a>"#,
        vec![DocumentLink {
            range: Range::new(Position::new(0, 9), Position::new(0, 88)),
            target:
                Some(Url::parse("https://werkenvoor.be/nl/jobs?f%5B0%5D=activitydomains%3A115&f%5B1%5D=lang%3Anl").unwrap()),
            tooltip: None,
            data: None,
        }],
    );
    test_link_detection(
        r#"<a href="jobs.html?f=bar">link</a>"#,
        vec![DocumentLink {
            range: Range::new(Position::new(0, 9), Position::new(0, 24)),
            target: Some(Url::parse("file:///test/data/abc/jobs.html").unwrap()),
            tooltip: None,
            data: None,
        }],
    );
}

#[cfg(feature = "links")]
#[test]
fn local_targets() {
    test_link_detection(
        r##"<body><h1 id="title"></h1><a href="#title"</a></body>"##,
        vec![DocumentLink {
            range: Range::new(Position::new(0, 35), Position::new(0, 41)),
            target: Some(Url::parse("file:///test/data/abc/test.html#1,14").unwrap()),
            tooltip: None,
            data: None,
        }],
    );
    test_link_detection(
        r#"<body><h1 id="title"></h1><a href="file:///test/data/abc/test.html#title"</a></body>"#,
        vec![DocumentLink {
            range: Range::new(Position::new(0, 35), Position::new(0, 72)),
            target: Some(Url::parse("file:///test/data/abc/test.html#1,14").unwrap()),
            tooltip: None,
            data: None,
        }],
    );
    test_link_detection(
        r##"<body><h1 id="title"></h1><a href="#body"</a></body>"##,
        vec![DocumentLink {
            range: Range::new(Position::new(0, 35), Position::new(0, 40)),
            target: Some(Url::parse("file:///test/data/abc/test.html").unwrap()),
            tooltip: None,
            data: None,
        }],
    );
}
