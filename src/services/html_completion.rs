use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use lsp_textdocument::FullTextDocument;
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionList, CompletionTextEdit, Documentation,
    InsertTextFormat, Position, Range, TextDocumentItem, TextEdit,
};
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
        html_parse::HTMLDocument,
        html_scanner::{Scanner, ScannerState, TokenType},
    },
    LanguageServiceOptions,
};

pub struct HTMLCompletion {
    ls_options: Arc<LanguageServiceOptions>,
    data_manager: Arc<RwLock<HTMLDataManager>>,
    supports_markdown: bool,
    completion_participants: Vec<Box<dyn ICompletionParticipant>>,
}

impl HTMLCompletion {
    pub fn new(
        ls_options: Arc<LanguageServiceOptions>,
        data_manager: Arc<RwLock<HTMLDataManager>>,
    ) -> HTMLCompletion {
        HTMLCompletion {
            ls_options: Arc::clone(&ls_options),
            data_manager,
            supports_markdown: HTMLCompletion::does_support_markdown(Arc::clone(&ls_options)),
            completion_participants: vec![],
        }
    }

    pub fn set_completion_participants(
        &mut self,
        registered_completion_participants: Vec<Box<dyn ICompletionParticipant>>,
    ) {
        self.completion_participants = registered_completion_participants;
    }

    pub fn do_complete(
        &self,
        document: &FullTextDocument,
        position: &Position,
        html_document: &HTMLDocument,
        document_context: impl DocumentContext,
        settings: Option<&CompletionConfiguration>,
    ) -> CompletionList {
        let data_manager = self.data_manager.read().unwrap();
        let mut result = CompletionList::default();
        let data_providers: Vec<_> = data_manager
            .get_data_providers()
            .iter()
            .filter(|p| {
                p.is_applicable(document.language_id())
                    && (settings.is_none()
                        || settings.is_some_and(|s| {
                            let v = s.provider.get(p.get_id());
                            v.is_none() || *v.unwrap()
                        }))
            })
            .collect();

        let void_elements = data_manager.get_void_elements(document.language_id());

        let text = document.get_content(None);
        let offset = document.offset_at(*position).try_into().unwrap();

        let node = html_document.find_node_before(offset);

        if node.is_none() {
            return result;
        }
        let node = node.unwrap();
        let node = node.borrow();

        let mut scanner = Scanner::new(text, node.start, ScannerState::WithinContent);
        let mut current_tag = None;
        let mut current_attribute_name = "";

        let mut token = scanner.scan();

        let mut content = CompletionContext {
            offset,
            document,
            result: &mut result,
            data_providers,
            void_elements,
        };

        while token != TokenType::EOS && scanner.get_token_offset() < offset {
            match token {
                TokenType::StartTagOpen => {
                    if scanner.get_token_end() == offset {
                        let end_pos = content.scan_next_for_end_pos(
                            &mut scanner,
                            &mut token,
                            offset,
                            TokenType::StartTag,
                        );
                        if position.line == 0 {
                            content.suggest_doctype(offset, end_pos);
                        }
                        content.collect_tag_suggestions(offset, end_pos);
                        return result;
                    }
                }
                TokenType::StartTag => {
                    if scanner.get_token_offset() <= offset && offset <= scanner.get_token_end() {
                        content.collect_open_tag_suggestions(
                            scanner.get_token_offset(),
                            scanner.get_token_end(),
                        );
                        return result;
                    }
                    current_tag = Some(scanner.get_token_text().to_string());
                }
                TokenType::AttributeName => {
                    if scanner.get_token_offset() <= offset && offset <= scanner.get_token_end() {
                        content.collect_attribute_name_suggestions(
                            scanner.get_token_offset(),
                            scanner.get_token_end(),
                        );
                        return result;
                    }
                    current_attribute_name = scanner.get_token_text();
                }
                TokenType::DelimiterAssign => {
                    if scanner.get_token_end() == offset {
                        let end_pos = content.scan_next_for_end_pos(
                            &mut scanner,
                            &mut token,
                            offset,
                            TokenType::AttributeValue,
                        );
                        content.collect_attribute_value_suggestions(offset, end_pos);
                        return result;
                    }
                }
                TokenType::AttributeValue => {
                    if scanner.get_token_offset() <= offset && offset <= scanner.get_token_end() {
                        content.collect_attribute_value_suggestions(
                            scanner.get_token_offset(),
                            scanner.get_token_end(),
                        );
                        return result;
                    }
                }
                TokenType::Whitespace => {
                    if offset <= scanner.get_token_end() {
                        match scanner.get_scanner_state() {
                            ScannerState::AfterOpeningStartTag => {
                                let start_pos = scanner.get_token_offset();
                                let end_tag_pos = content.scan_next_for_end_pos(
                                    &mut scanner,
                                    &mut token,
                                    offset,
                                    TokenType::StartTag,
                                );
                                content.collect_tag_suggestions(start_pos, end_tag_pos);
                                return result;
                            }
                            ScannerState::WithinTag => {
                                content.collect_attribute_name_suggestions(
                                    scanner.get_token_end(),
                                    offset,
                                );
                                return result;
                            }
                            ScannerState::AfterAttributeName => {
                                content.collect_attribute_name_suggestions(
                                    scanner.get_token_end(),
                                    offset,
                                );
                                return result;
                            }
                            ScannerState::BeforeAttributeValue => {
                                content.collect_attribute_value_suggestions(
                                    scanner.get_token_end(),
                                    offset,
                                );
                                return result;
                            }
                            ScannerState::AfterOpeningEndTag => {
                                content.collect_close_tag_suggestions(
                                    scanner.get_token_offset() - 1,
                                    false,
                                    offset,
                                );
                                return result;
                            }
                            ScannerState::WithinContent => {
                                content.collect_inside_content();
                                return result;
                            }
                            _ => {}
                        }
                    }
                }
                TokenType::StartTagClose => {
                    if offset <= scanner.get_token_end() {
                        if current_tag.is_some() {
                            content.collect_auto_close_tag_suggestion(
                                scanner.get_token_end(),
                                &current_tag.unwrap(),
                            );
                            return result;
                        }
                    }
                }
                TokenType::Content => {
                    if offset <= scanner.get_token_end() {
                        content.collect_inside_content();
                        return result;
                    }
                }
                TokenType::EndTagOpen => {
                    if offset < scanner.get_token_end() {
                        let after_open_bracket = scanner.get_token_offset() + 1;
                        let end_offset = content.scan_next_for_end_pos(
                            &mut scanner,
                            &mut token,
                            offset,
                            TokenType::EndTag,
                        );
                        content.collect_close_tag_suggestions(
                            after_open_bracket,
                            false,
                            end_offset,
                        );
                        return result;
                    }
                }
                TokenType::EndTag => {
                    if offset < scanner.get_token_end() {
                        let mut start = scanner.get_token_offset() - 1;
                        while start > 0 {
                            let ch = text.bytes().nth(start).unwrap();
                            if ch == b'/' {
                                content.collect_close_tag_suggestions(
                                    start,
                                    false,
                                    scanner.get_token_end(),
                                );
                                return result;
                            } else if !is_white_space(&ch.to_string()) {
                                break;
                            }
                            start -= 1;
                        }
                    }
                }
                _ => {
                    if offset < scanner.get_token_end() {
                        return result;
                    }
                }
            }
            token = scanner.scan();
        }

        result
    }

    fn does_support_markdown(ls_options: Arc<LanguageServiceOptions>) -> bool {
        if let Some(client_capabilities) = &ls_options.client_capabilities {
            if let Some(text_document) = &client_capabilities.text_document {
                if let Some(completion) = &text_document.completion {
                    if let Some(completion_item) = &completion.completion_item {
                        if let Some(documentation_format) = &completion_item.documentation_format {
                            return documentation_format.contains(&lsp_types::MarkupKind::Markdown);
                        }
                    }
                }
            }
        } else {
            return true;
        }
        false
    }
}

struct CompletionContext<'a> {
    result: &'a mut CompletionList,
    offset: usize,
    document: &'a FullTextDocument,
    data_providers: Vec<&'a Box<dyn IHTMLDataProvider>>,
    void_elements: Vec<&'a str>,
}

impl CompletionContext<'_> {
    fn get_replace_range(&self, replace_start: usize, replace_end: usize) -> Range {
        let mut replace_start = replace_start;
        if replace_start > self.offset {
            replace_start = self.offset;
        }
        Range {
            start: self.document.position_at(replace_start.try_into().unwrap()),
            end: self.document.position_at(replace_end.try_into().unwrap()),
        }
    }

    fn scan_next_for_end_pos(
        &mut self,
        scanner: &mut Scanner,
        token: &mut TokenType,
        offset: usize,
        next_token: TokenType,
    ) -> usize {
        if offset == scanner.get_token_end() {
            *token = scanner.scan();
            if *token == next_token && scanner.get_token_offset() == offset {
                return scanner.get_token_end();
            }
        }
        offset
    }

    fn collect_tag_suggestions(&mut self, tag_start: usize, tag_end: usize) {
        self.collect_open_tag_suggestions(tag_start, tag_end);
        self.collect_close_tag_suggestions(tag_start, true, tag_end);
    }

    fn collect_open_tag_suggestions(&mut self, after_open_bracket: usize, tag_name_end: usize) {
        let range = self.get_replace_range(after_open_bracket, tag_name_end);
        for provider in &self.data_providers {
            for tag in provider.provide_tags() {
                let mut item = CompletionItem::default();
                item.label = tag.name.clone();
                item.kind = Some(CompletionItemKind::PROPERTY);
                let documentation = generate_documentation(
                    GenerateDocumentationItem {
                        description: tag.description.clone(),
                        references: tag.references.clone(),
                    },
                    GenerateDocumentationSetting {
                        documentation: true,
                        references: true,
                        does_support_markdown: true,
                    },
                );
                if let Some(documentation) = documentation {
                    item.documentation = Some(Documentation::MarkupContent(documentation));
                } else {
                    item.documentation = None;
                }
                item.text_edit = Some(CompletionTextEdit::Edit(TextEdit::new(
                    range,
                    tag.name.clone(),
                )));
                item.insert_text_format = Some(InsertTextFormat::PLAIN_TEXT);
                self.result.items.push(item);
            }
        }
    }

    fn collect_attribute_name_suggestions(&mut self, name_start: usize, name_end: usize) {
        todo!()
    }

    fn collect_attribute_value_suggestions(&mut self, value_start: usize, value_end: usize) {
        todo!()
    }

    fn collect_close_tag_suggestions(
        &mut self,
        after_open_bracket: usize,
        in_open_tag: bool,
        tag_name_end: usize,
    ) {
        todo!()
    }

    fn collect_auto_close_tag_suggestion(&mut self, tag_close_end: usize, tag: &str) {
        todo!()
    }

    fn collect_inside_content(&mut self) {
        todo!()
    }

    fn suggest_doctype(&mut self, offset: usize, end_pos: usize) {
        todo!()
    }
}

fn is_white_space(text: &str) -> bool {
    Regex::new(r"^\s*$").unwrap().is_match(text)
}

pub trait ICompletionParticipant: Send + Sync {
    fn on_html_attribute_value(&self, context: HtmlAttributeValueContext);
    fn on_html_content(&self, context: HtmlContentContext);
}

pub struct HtmlAttributeValueContext {
    pub document: TextDocumentItem,
    pub position: Position,
    pub tag: String,
    pub attribute: String,
    pub value: String,
    pub range: Range,
}

pub struct HtmlContentContext {
    pub document: TextDocumentItem,
    pub position: Position,
}

pub trait DocumentContext {
    fn resolve_reference(&self, reference: &str, base: &str) -> Option<&str>;
}

pub struct CompletionConfiguration {
    hide_auto_complete_proposals: bool,
    attribute_default_value: bool,
    provider: HashMap<String, bool>,
}
