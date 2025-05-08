# html-languageservice

> The project is a rewrite of [vscode-html-languageservice](https://github.com/Microsoft/vscode-html-languageservice) use `rust`. It has all the features of the original project while still having higher performance.

This project is a collection of features necessary to implement an HTML language server.

## Features

- customize data providers
- parse html document
- scanner
- completion - `completion` feature activate
- hover - `hover` feature activate
- formatter - `formatter` feature activate
- find document highlights - `highlight` feature activate
- find document links - `links` feature activate
- find document symbols - `symbols` feature activate
- get folding ranges - `folding` feature activate
- get selection ranges - `selection_range` feature activate
- quote complete - `completion` feature activate
- tag complete - `completion` feature activate
- rename - `rename` feature activate
- find matching tag position - `matching_tag_position` feature activate
- find linked editing ranges - `linked_editing` feature activate

## Usage

Make sure you activated the full features of the `html-languageservice` crate on `Cargo.toml`:

```toml
html-languageservice = { version = "0.6.1", features = ["full"] }
```

You can also activate only some of the features you need on `Cargo.toml`:

```toml
html-languageservice = { version = "0.6.1", features = ["completion", "hover"] }
```

Create the `HTMLLanguageService` struct in first.

Second, You need to prepare: `document` and `position`.

Then, parse `document` as `html_document` you need to `HTMLDataManager`, tags, attributes, and attribute value data are stored in the `HTMLDataManager`.

Finally, call a function or method to get the result.

For more see [docs.rs](https://docs.rs/html-languageservice/latest/html_languageservice/)
