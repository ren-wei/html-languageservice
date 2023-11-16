use std::sync::{Arc, RwLock};

use lsp_textdocument::FullTextDocument;
use lsp_types::{Hover, HoverContents, MarkedString, MarkupContent, MarkupKind, Position, Range};
use regex::Regex;

use crate::{
    language_facts::{
        data_manager::HTMLDataManager,
        data_provider::{
            generate_documentation, GenerateDocumentationItem, GenerateDocumentationSetting,
            IHTMLDataProvider,
        },
    },
    parser::{
        html_entities::get_entities,
        html_parse::HTMLDocument,
        html_scanner::{Scanner, ScannerState, TokenType},
    },
    utils::{markdown::does_support_markdown, strings::is_letter_or_digit},
    LanguageServiceOptions,
};

pub struct HTMLHover {
    _ls_options: Arc<LanguageServiceOptions>,
    data_manager: Arc<RwLock<HTMLDataManager>>,
    supports_markdown: bool,
}

impl HTMLHover {
    pub fn new(
        ls_options: Arc<LanguageServiceOptions>,
        data_manager: Arc<RwLock<HTMLDataManager>>,
    ) -> HTMLHover {
        HTMLHover {
            _ls_options: Arc::clone(&ls_options),
            data_manager,
            supports_markdown: does_support_markdown(Arc::clone(&ls_options)),
        }
    }

    pub fn do_hover(
        &self,
        document: &FullTextDocument,
        position: &Position,
        html_document: &HTMLDocument,
        options: Option<HoverSettings>,
    ) -> Option<Hover> {
        let offset = document.offset_at(*position) as usize;
        let node = html_document.find_node_at(offset);
        let text = document.get_content(None);

        if node.is_none() || node.clone().is_some_and(|v| v.borrow().tag.is_none()) {
            return None;
        }

        let _node = node.unwrap();
        let node = _node.borrow();

        let data_manager = self.data_manager.read().unwrap();
        let data_providers: Vec<_> = data_manager
            .get_data_providers()
            .iter()
            .filter(|p| p.read().unwrap().is_applicable(document.language_id()))
            .collect();

        let options = if options.is_some() {
            options.unwrap()
        } else {
            HoverSettings {
                documentation: true,
                references: true,
            }
        };
        let mut context = HoverContext {
            options,
            data_providers,
            offset,
            position,
            document,
        };

        if node
            .end_tag_start
            .is_some_and(|end_tag_start| context.offset >= end_tag_start)
        {
            let tag_range = self.get_tag_name_range(
                TokenType::EndTag,
                node.end_tag_start.unwrap(),
                &mut context,
            );
            if tag_range.is_some() {
                return self.get_tag_hover(
                    &node.tag.clone().unwrap(),
                    tag_range.unwrap(),
                    false,
                    &mut context,
                );
            }
            return None;
        }

        let tag_range = self.get_tag_name_range(TokenType::StartTag, node.start, &mut context);
        if tag_range.is_some() {
            return self.get_tag_hover(
                &node.tag.clone().unwrap(),
                tag_range.unwrap(),
                true,
                &mut context,
            );
        }

        let attr_range =
            self.get_tag_name_range(TokenType::AttributeName, node.start, &mut context);
        if attr_range.is_some() {
            let tag = node.tag.clone().unwrap();
            let attr = document.get_content(attr_range);
            return self.get_attr_hover(&tag, attr, attr_range.unwrap(), &mut context);
        }

        let entity_range = self.get_entity_range(&mut context);
        if entity_range.is_some() {
            return self.get_entity_hover(text, entity_range.unwrap(), &mut context);
        }

        let attr_value_range =
            self.get_tag_name_range(TokenType::AttributeValue, node.start, &mut context);
        if attr_value_range.is_some() {
            let attr_value_range = attr_value_range.unwrap();
            let tag = node.tag.clone().unwrap();
            let attr_value = &HTMLHover::trim_quotes(document.get_content(Some(attr_value_range)));
            let match_attr = self.scan_attr_and_attr_value(
                node.start,
                document.offset_at(attr_value_range.start) as usize,
                &mut context,
            );
            if match_attr.is_some() {
                return self.get_attr_value_hover(
                    &tag,
                    &match_attr.unwrap(),
                    attr_value,
                    attr_value_range,
                    &mut context,
                );
            }
        }

        None
    }

    fn get_tag_hover(
        &self,
        cur_tag: &str,
        range: Range,
        _open: bool,
        context: &mut HoverContext,
    ) -> Option<Hover> {
        for provider in &context.data_providers {
            let mut hover = None;

            for tag in provider.read().unwrap().provide_tags() {
                if tag.name.to_lowercase() == cur_tag.to_lowercase() {
                    let markup_content = generate_documentation(
                        GenerateDocumentationItem {
                            description: tag.description.clone(),
                            references: tag.references.clone(),
                        },
                        GenerateDocumentationSetting {
                            documentation: context.options.documentation,
                            references: context.options.references,
                            does_support_markdown: self.supports_markdown,
                        },
                    )
                    .unwrap_or(MarkupContent {
                        kind: if self.supports_markdown {
                            MarkupKind::Markdown
                        } else {
                            MarkupKind::PlainText
                        },
                        value: "".to_string(),
                    });
                    hover = Some(Hover {
                        contents: self.convert_contents(HoverContents::Markup(markup_content)),
                        range: Some(range),
                    });
                }
            }
            if hover.is_some() {
                return hover;
            }
        }
        None
    }

    fn get_attr_hover(
        &self,
        cur_tag: &str,
        cur_attr: &str,
        range: Range,
        context: &mut HoverContext,
    ) -> Option<Hover> {
        for provider in &context.data_providers {
            let mut hover = None;

            for attr in provider.read().unwrap().provide_attributes(cur_tag) {
                if cur_attr == attr.name && attr.description.is_some() {
                    let contents = generate_documentation(
                        GenerateDocumentationItem {
                            description: attr.description.clone(),
                            references: attr.references.clone(),
                        },
                        GenerateDocumentationSetting {
                            documentation: context.options.documentation,
                            references: context.options.references,
                            does_support_markdown: self.supports_markdown,
                        },
                    );
                    if contents.is_some() {
                        hover = Some(Hover {
                            contents: self
                                .convert_contents(HoverContents::Markup(contents.unwrap())),
                            range: Some(range),
                        });
                    } else {
                        hover = None;
                    }
                }
            }
            if hover.is_some() {
                return hover;
            }
        }
        None
    }

    fn get_attr_value_hover(
        &self,
        cur_tag: &str,
        cur_attr: &str,
        cur_attr_value: &str,
        range: Range,
        context: &mut HoverContext,
    ) -> Option<Hover> {
        for provider in &context.data_providers {
            let mut hover = None;
            for attr_value in provider.read().unwrap().provide_values(cur_tag, cur_attr) {
                if cur_attr_value == attr_value.name && attr_value.description.is_some() {
                    let contents = generate_documentation(
                        GenerateDocumentationItem {
                            description: attr_value.description.clone(),
                            references: attr_value.references.clone(),
                        },
                        GenerateDocumentationSetting {
                            documentation: context.options.documentation,
                            references: context.options.references,
                            does_support_markdown: self.supports_markdown,
                        },
                    );
                    if contents.is_some() {
                        hover = Some(Hover {
                            contents: self
                                .convert_contents(HoverContents::Markup(contents.unwrap())),
                            range: Some(range),
                        })
                    } else {
                        hover = None
                    }
                }
            }
            if hover.is_some() {
                return hover;
            }
        }
        None
    }

    fn get_entity_hover(
        &self,
        text: &str,
        range: Range,
        context: &mut HoverContext,
    ) -> Option<Hover> {
        let cur_entity = self.filter_entity(text, context);

        for (entity, value) in get_entities() {
            let label = format!("&{}", entity);

            if cur_entity == label {
                let code = value
                    .as_bytes()
                    .iter()
                    .map(|b| format!("{:02X}", b))
                    .collect::<Vec<String>>()
                    .join("");
                let mut hex = String::from("U+");

                if code.len() < 4 {
                    let zeroes = 4 - code.len();
                    let mut k = 0;
                    while k < zeroes {
                        hex = hex + "0";
                        k += 1;
                    }
                }

                hex += &code;

                let content = format!(
                    "Character entity representing '{0}', unicode equivalent '{1}'",
                    value, hex
                );
                return Some(Hover {
                    contents: self.convert_contents(HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::PlainText,
                        value: content,
                    })),
                    range: Some(range),
                });
            }
        }

        None
    }

    fn get_tag_name_range(
        &self,
        token_type: TokenType,
        start_offset: usize,
        context: &mut HoverContext,
    ) -> Option<Range> {
        let mut scanner = Scanner::new(
            context.document.get_content(None),
            start_offset,
            ScannerState::WithinContent,
        );
        let mut token = scanner.scan();
        while token != TokenType::EOS
            && (scanner.get_token_end() < context.offset
                || scanner.get_token_end() == context.offset && token != token_type)
        {
            token = scanner.scan();
        }
        if token == token_type && context.offset <= scanner.get_token_end() {
            return Some(Range {
                start: context
                    .document
                    .position_at(scanner.get_token_offset() as u32),
                end: context.document.position_at(scanner.get_token_end() as u32),
            });
        }
        None
    }

    fn get_entity_range(&self, context: &mut HoverContext) -> Option<Range> {
        let mut k: isize = context.offset as isize - 1;
        let mut character_start = context.position.character;

        let text = context.document.get_content(None);

        while k >= 0 && is_letter_or_digit(text, k as usize) {
            k -= 1;
            character_start -= 1;
        }

        let mut n = (k + 1) as usize;
        let mut character_end = character_start;

        while is_letter_or_digit(text, n) {
            n += 1;
            character_end += 1;
        }

        if k >= 0 && text.as_bytes()[k as usize] == b'&' {
            return if text.as_bytes()[n] == b';' {
                Some(Range {
                    start: Position {
                        line: context.position.line,
                        character: character_start,
                    },
                    end: Position {
                        line: context.position.line,
                        character: character_end + 1,
                    },
                })
            } else {
                Some(Range {
                    start: Position {
                        line: context.position.line,
                        character: character_start,
                    },
                    end: Position {
                        line: context.position.line,
                        character: character_end,
                    },
                })
            };
        }
        None
    }

    fn filter_entity(&self, text: &str, context: &mut HoverContext) -> String {
        let mut k: isize = context.offset as isize - 1;
        let mut new_text = String::from("&");

        while k >= 0 && is_letter_or_digit(text, k as usize) {
            k -= 1;
        }

        let mut k = k as usize;

        k += 1;

        while is_letter_or_digit(text, k) {
            new_text += &text[k..k + 1];
            k += 1;
        }

        new_text += ";";

        new_text
    }

    fn scan_attr_and_attr_value(
        &self,
        node_start: usize,
        attr_value_start: usize,
        context: &mut HoverContext,
    ) -> Option<String> {
        let mut scanner = Scanner::new(
            context.document.get_content(None),
            node_start,
            ScannerState::WithinContent,
        );
        let mut token = scanner.scan();
        let mut prev_attr = None;

        while token != TokenType::EOS && scanner.get_token_end() <= attr_value_start {
            token = scanner.scan();
            if token == TokenType::AttributeName {
                prev_attr = Some(scanner.get_token_text().to_string());
            }
        }

        prev_attr
    }

    fn trim_quotes(s: &str) -> String {
        let mut s = s;
        if s.len() <= 1 {
            return Regex::new(r#"['"]"#).unwrap().replace(s, "").to_string();
        }

        if s.as_bytes()[0] == b'\'' || s.as_bytes()[0] == b'"' {
            s = &s[1..];
        }

        if s.as_bytes()[s.len() - 1] == b'\'' || s.as_bytes()[s.len() - 1] == b'"' {
            s = &s[..s.len() - 1];
        }

        s.to_string()
    }

    fn convert_contents(&self, contents: HoverContents) -> HoverContents {
        if !self.supports_markdown {
            return match contents {
                HoverContents::Array(contents) => HoverContents::Array(
                    contents
                        .iter()
                        .map(|c| match c {
                            MarkedString::String(c) => MarkedString::String(c.to_string()),
                            MarkedString::LanguageString(c) => {
                                MarkedString::String(c.value.clone())
                            }
                        })
                        .collect(),
                ),
                HoverContents::Markup(contents) => HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::PlainText,
                    value: contents.value,
                }),
                HoverContents::Scalar(contents) => HoverContents::Scalar(match contents {
                    MarkedString::String(c) => MarkedString::String(c),
                    MarkedString::LanguageString(c) => MarkedString::String(c.value),
                }),
            };
        }
        contents
    }
}

#[derive(Clone)]
pub struct HoverSettings {
    documentation: bool,
    references: bool,
}

struct HoverContext<'a> {
    options: HoverSettings,
    data_providers: Vec<&'a Arc<RwLock<dyn IHTMLDataProvider>>>,
    offset: usize,
    position: &'a Position,
    document: &'a FullTextDocument,
}
