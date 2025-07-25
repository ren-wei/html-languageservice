use std::{collections::HashMap, fmt::Debug};

use lazy_static::lazy_static;
use lsp_textdocument::FullTextDocument;
use lsp_types::{Hover, HoverContents, MarkedString, MarkupContent, MarkupKind, Position, Range};
use regex::Regex;

use crate::{
    language_facts::{
        data_manager::HTMLDataManager,
        data_provider::{
            self, GenerateDocumentationItem, GenerateDocumentationSetting, HTMLDataProviderContent,
            IHTMLDataProvider,
        },
    },
    parser::{
        html_document::HTMLDocument,
        html_entities,
        html_scanner::{Scanner, ScannerState, TokenType},
    },
    participant::{HtmlAttributeValueContext, HtmlContentContext, IHoverParticipant},
    utils::{markdown, strings},
    HTMLLanguageServiceOptions,
};

lazy_static! {
    static ref REG_QUOTE: Regex = Regex::new(r#"['"]"#).unwrap();
}

pub struct HTMLHover {
    supports_markdown: bool,
    hover_participants: Vec<Box<dyn IHoverParticipant>>,
    case_sensitive: bool,
}

impl HTMLHover {
    pub fn new(ls_options: &HTMLLanguageServiceOptions) -> HTMLHover {
        HTMLHover {
            supports_markdown: markdown::does_support_markdown(&ls_options),
            hover_participants: vec![],
            case_sensitive: ls_options.case_sensitive.unwrap_or(false),
        }
    }

    pub fn set_hover_participants(&mut self, hover_participants: Vec<Box<dyn IHoverParticipant>>) {
        self.hover_participants = hover_participants;
    }

    pub fn do_hover(
        &self,
        document: &FullTextDocument,
        position: &Position,
        html_document: &HTMLDocument,
        options: Option<HoverSettings>,
        data_manager: &HTMLDataManager,
    ) -> Option<Hover> {
        let offset = document.offset_at(*position) as usize;
        let node = html_document.find_node_at(offset, &mut vec![]);
        let text = document.get_content(None);

        if node.is_none() {
            return None;
        }
        if let Some(node) = &node {
            if node.tag.is_none() {
                return None;
            }
        }

        let node = node.unwrap();

        let mut data_providers = vec![];
        for provider in data_manager.get_data_providers() {
            if provider.is_applicable(document.language_id()) {
                data_providers.push(provider);
            }
        }

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
            html_document,
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

        for participant in &self.hover_participants {
            let hover = participant.on_html_content(HtmlContentContext {
                document,
                html_document,
                position: *position,
            });
            if let Some(hover) = hover {
                return Some(hover);
            }
        }

        None
    }

    fn get_tag_hover<'a>(
        &self,
        cur_tag: &str,
        range: Range,
        _open: bool,
        context: &mut HoverContext<'a>,
    ) -> Option<Hover> {
        for provider in &context.data_providers {
            let mut hover = None;

            for tag in provider.provide_tags() {
                let equal = if self.case_sensitive {
                    tag.name == cur_tag
                } else {
                    tag.name.to_lowercase() == cur_tag.to_lowercase()
                };
                if equal {
                    let markup_content = data_provider::generate_documentation(
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

    fn get_attr_hover<'a>(
        &self,
        cur_tag: &str,
        cur_attr: &str,
        range: Range,
        context: &mut HoverContext<'a>,
    ) -> Option<Hover> {
        for provider in &context.data_providers {
            let mut hover = None;

            for attr in provider.provide_attributes(
                cur_tag,
                &HTMLDataProviderContent {
                    document: context.document,
                    html_document: context.html_document,
                    offset: context.offset,
                },
            ) {
                if cur_attr == attr.name && attr.description.is_some() {
                    let contents = data_provider::generate_documentation(
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

    fn get_attr_value_hover<'a>(
        &self,
        cur_tag: &str,
        cur_attr: &str,
        cur_attr_value: &str,
        range: Range,
        context: &mut HoverContext<'a>,
    ) -> Option<Hover> {
        for hover_participant in &self.hover_participants {
            if let Some(hover) =
                hover_participant.on_html_attribute_value(HtmlAttributeValueContext {
                    document: context.document,
                    html_document: context.html_document,
                    position: *context.position,
                    tag: cur_tag,
                    attribute: cur_attr,
                    value: cur_attr_value,
                    range,
                })
            {
                return Some(hover);
            }
        }
        for provider in &context.data_providers {
            for attr_value in provider.provide_values(cur_tag, cur_attr) {
                if cur_attr_value == attr_value.name && attr_value.description.is_some() {
                    let contents = data_provider::generate_documentation(
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
                        return Some(Hover {
                            contents: self
                                .convert_contents(HoverContents::Markup(contents.unwrap())),
                            range: Some(range),
                        });
                    }
                }
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

        let entities: &HashMap<_, _> = &html_entities::ENTITIES;
        for (entity, value) in entities {
            let label = format!("&{}", entity);

            if cur_entity == label {
                let code = value
                    .chars()
                    .map(|b| format!("{:02X}", b as u32))
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
            false,
            self.case_sensitive,
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
        let mut k = context.offset;
        let mut character_start = context.position.character;

        let text = context.document.get_content(None);

        while k > 0 && strings::is_letter_or_digit(text, k - 1) {
            k -= 1;
            character_start -= 1;
        }

        let mut n = k;
        let mut character_end = character_start;

        while strings::is_letter_or_digit(text, n) {
            n += 1;
            character_end += 1;
        }

        if k > 0 && text.get(k - 1..k) == Some("&") {
            return if text.get(n..n + 1) == Some(";") {
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

        while k >= 0 && strings::is_letter_or_digit(text, k as usize) {
            k -= 1;
        }

        let mut k = k as usize;

        k += 1;

        while strings::is_letter_or_digit(text, k) {
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
            false,
            self.case_sensitive,
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
            return REG_QUOTE.replace(s, "").to_string();
        }

        if s.get(0..1) == Some("'") || s.get(0..1) == Some(r#"""#) {
            s = &s[1..];
        }

        if s.get(s.len() - 1..s.len()) == Some("'") || s.get(s.len() - 1..s.len()) == Some(r#"""#) {
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

impl Debug for HTMLHover {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HTMLHover")
            .field("supports_markdown", &self.supports_markdown)
            .field("hover_participants(len)", &self.hover_participants.len())
            .field("case_sensitive", &self.case_sensitive)
            .finish()
    }
}

#[derive(Clone)]
pub struct HoverSettings {
    pub documentation: bool,
    pub references: bool,
}

struct HoverContext<'a> {
    options: HoverSettings,
    data_providers: Vec<&'a Box<dyn IHTMLDataProvider>>,
    offset: usize,
    position: &'a Position,
    document: &'a FullTextDocument,
    html_document: &'a HTMLDocument,
}
