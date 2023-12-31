use std::{collections::HashMap, sync::Arc};

use async_recursion::async_recursion;
use lsp_textdocument::FullTextDocument;
use lsp_types::{
    Command, CompletionItem, CompletionItemKind, CompletionList, CompletionTextEdit, Documentation,
    InsertTextFormat, Position, Range, TextEdit,
};
use regex::Regex;
use tokio::sync::RwLock;

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
        html_parse::{HTMLDocument, Node},
        html_scanner::{Scanner, ScannerState, TokenType},
    },
    participant::{HtmlAttributeValueContext, HtmlContentContext, ICompletionParticipant},
    utils::{markdown::does_support_markdown, strings::is_letter_or_digit},
    LanguageServiceOptions,
};

pub struct HTMLCompletion {
    _ls_options: Arc<LanguageServiceOptions>,
    data_manager: Arc<RwLock<HTMLDataManager>>,
    supports_markdown: bool,
    completion_participants: Vec<Arc<RwLock<dyn ICompletionParticipant>>>,
}

impl HTMLCompletion {
    pub fn new(
        ls_options: Arc<LanguageServiceOptions>,
        data_manager: Arc<RwLock<HTMLDataManager>>,
    ) -> HTMLCompletion {
        HTMLCompletion {
            _ls_options: Arc::clone(&ls_options),
            data_manager,
            supports_markdown: does_support_markdown(Arc::clone(&ls_options)),
            completion_participants: vec![],
        }
    }

    pub fn set_completion_participants(
        &mut self,
        completion_participants: Vec<Arc<RwLock<dyn ICompletionParticipant>>>,
    ) {
        self.completion_participants = completion_participants;
    }

    pub async fn do_complete(
        &self,
        document: &FullTextDocument,
        position: &Position,
        html_document: &HTMLDocument,
        _document_context: impl DocumentContext,
        settings: Option<&CompletionConfiguration>,
    ) -> CompletionList {
        let data_manager = self.data_manager.read().await;
        let mut result = CompletionList::default();
        let mut data_providers = vec![];
        for provider in data_manager.get_data_providers() {
            if provider.read().await.is_applicable(document.language_id()) {
                if settings.is_none() {
                    data_providers.push(provider);
                } else {
                    let s = settings.unwrap();
                    let v = s.provider.get(provider.read().await.get_id());
                    if v.is_none() || *v.unwrap() {
                        data_providers.push(provider);
                    }
                }
            }
        }

        let void_elements = data_manager.get_void_elements(document.language_id()).await;

        let text = document.get_content(None);
        let offset = document.offset_at(*position).try_into().unwrap();

        let node = html_document.find_node_before(offset).await;

        if node.is_none() {
            return result;
        }
        let node = node.unwrap();

        let mut content = CompletionContext {
            offset,
            text,
            document,
            result: &mut result,
            data_providers,
            void_elements,
            settings,
            node: Arc::clone(&node),
            current_tag: None,
            does_support_markdown: self.supports_markdown,
            html_document,
            current_attribute_name: String::new(),
            completion_participants: &self.completion_participants,
            position,
            data_manager: Arc::clone(&self.data_manager),
        };

        let node = node.read().await;

        let mut scanner = Scanner::new(text, node.start, ScannerState::WithinContent);

        let mut token = scanner.scan();

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
                        content.collect_tag_suggestions(offset, end_pos).await;
                        return result;
                    }
                }
                TokenType::StartTag => {
                    if scanner.get_token_offset() <= offset && offset <= scanner.get_token_end() {
                        content
                            .collect_open_tag_suggestions(
                                scanner.get_token_offset(),
                                scanner.get_token_end(),
                            )
                            .await;
                        return result;
                    }
                    content.current_tag = Some(scanner.get_token_text().to_string());
                }
                TokenType::AttributeName => {
                    if scanner.get_token_offset() <= offset && offset <= scanner.get_token_end() {
                        content
                            .collect_attribute_name_suggestions(
                                scanner.get_token_offset(),
                                scanner.get_token_end(),
                            )
                            .await;
                        return result;
                    }
                    content.current_attribute_name = scanner.get_token_text().to_string();
                }
                TokenType::DelimiterAssign => {
                    if scanner.get_token_end() == offset {
                        let end_pos = content.scan_next_for_end_pos(
                            &mut scanner,
                            &mut token,
                            offset,
                            TokenType::AttributeValue,
                        );
                        content
                            .collect_attribute_value_suggestions(offset, end_pos)
                            .await;
                        return result;
                    }
                }
                TokenType::AttributeValue => {
                    if scanner.get_token_offset() <= offset && offset <= scanner.get_token_end() {
                        content
                            .collect_attribute_value_suggestions(
                                scanner.get_token_offset(),
                                scanner.get_token_end(),
                            )
                            .await;
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
                                content
                                    .collect_tag_suggestions(start_pos, end_tag_pos)
                                    .await;
                                return result;
                            }
                            ScannerState::WithinTag => {
                                content
                                    .collect_attribute_name_suggestions(
                                        scanner.get_token_end(),
                                        offset,
                                    )
                                    .await;
                                return result;
                            }
                            ScannerState::AfterAttributeName => {
                                content
                                    .collect_attribute_name_suggestions(
                                        scanner.get_token_end(),
                                        offset,
                                    )
                                    .await;
                                return result;
                            }
                            ScannerState::BeforeAttributeValue => {
                                content
                                    .collect_attribute_value_suggestions(
                                        scanner.get_token_end(),
                                        offset,
                                    )
                                    .await;
                                return result;
                            }
                            ScannerState::AfterOpeningEndTag => {
                                content
                                    .collect_close_tag_suggestions(
                                        scanner.get_token_offset() - 1,
                                        false,
                                        offset,
                                    )
                                    .await;
                                return result;
                            }
                            ScannerState::WithinContent => {
                                content.collect_inside_content().await;
                                return result;
                            }
                            _ => {}
                        }
                    }
                }
                TokenType::StartTagClose => {
                    if offset <= scanner.get_token_end() {
                        if content.current_tag.is_some() {
                            content
                                .collect_auto_close_tag_suggestion(
                                    scanner.get_token_end(),
                                    &content.current_tag.clone().unwrap(),
                                )
                                .await;
                            return result;
                        }
                    }
                }
                TokenType::Content => {
                    if offset <= scanner.get_token_end() {
                        content.collect_inside_content().await;
                        return result;
                    }
                }
                TokenType::EndTagOpen => {
                    if offset <= scanner.get_token_end() {
                        let after_open_bracket = scanner.get_token_offset() + 1;
                        let end_offset = content.scan_next_for_end_pos(
                            &mut scanner,
                            &mut token,
                            offset,
                            TokenType::EndTag,
                        );
                        content
                            .collect_close_tag_suggestions(after_open_bracket, false, end_offset)
                            .await;
                        return result;
                    }
                }
                TokenType::EndTag => {
                    if offset <= scanner.get_token_end() {
                        let mut start = scanner.get_token_offset() - 1;
                        while start > 0 {
                            let ch = text.bytes().nth(start).unwrap();
                            if ch == b'/' {
                                content
                                    .collect_close_tag_suggestions(
                                        start,
                                        false,
                                        scanner.get_token_end(),
                                    )
                                    .await;
                                return result;
                            } else if !is_white_space(std::str::from_utf8(&[ch]).unwrap()) {
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
}

struct CompletionContext<'a> {
    result: &'a mut CompletionList,
    text: &'a str,
    offset: usize,
    document: &'a FullTextDocument,
    data_providers: Vec<&'a Arc<RwLock<dyn IHTMLDataProvider>>>,
    void_elements: Vec<String>,
    settings: Option<&'a CompletionConfiguration>,
    node: Arc<RwLock<Node>>,
    current_tag: Option<String>,
    does_support_markdown: bool,
    html_document: &'a HTMLDocument,
    current_attribute_name: String,
    completion_participants: &'a Vec<Arc<RwLock<dyn ICompletionParticipant>>>,
    position: &'a Position,
    data_manager: Arc<RwLock<HTMLDataManager>>,
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

    async fn collect_tag_suggestions(&mut self, tag_start: usize, tag_end: usize) {
        self.collect_open_tag_suggestions(tag_start, tag_end).await;
        self.collect_close_tag_suggestions(tag_start, true, tag_end)
            .await;
    }

    async fn collect_open_tag_suggestions(
        &mut self,
        after_open_bracket: usize,
        tag_name_end: usize,
    ) {
        let range = self.get_replace_range(after_open_bracket, tag_name_end);
        for provider in &self.data_providers {
            for tag in provider.read().await.provide_tags() {
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
                let documentation = if let Some(documentation) = documentation {
                    Some(Documentation::MarkupContent(documentation))
                } else {
                    None
                };
                self.result.items.push(CompletionItem {
                    label: tag.name.clone(),
                    kind: Some(CompletionItemKind::PROPERTY),
                    documentation,
                    text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(
                        range,
                        tag.name.clone(),
                    ))),
                    insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                    ..Default::default()
                });
            }
        }
    }

    async fn collect_attribute_name_suggestions(&mut self, name_start: usize, name_end: usize) {
        let mut replace_end = self.offset;
        let text = self.document.get_content(None);
        while replace_end < name_end && text.as_bytes().get(replace_end).is_some_and(|c| *c != b'<')
        {
            replace_end += 1;
        }
        let current_attribute = if name_start > name_end {
            &text[name_end..name_start]
        } else {
            &text[name_start..name_end]
        };
        let range = self.get_replace_range(name_start, replace_end);
        let mut value = "";
        if !is_followed_by(
            text,
            name_end,
            ScannerState::AfterAttributeName,
            TokenType::DelimiterAssign,
        ) {
            let quotes = if let Some(settings) = self.settings {
                settings.attribute_default_value
            } else {
                Quotes::Double
            };
            match quotes {
                Quotes::None => value = "=$1",
                Quotes::Single => value = "='$1'",
                Quotes::Double => value = r#"="$1""#,
            }
        }

        let mut existing_attributes = self.get_existing_attributes().await;
        existing_attributes.insert(current_attribute.to_string(), false);

        for provider in &self.data_providers {
            for attr in provider
                .read()
                .await
                .provide_attributes(&self.current_tag.as_ref().unwrap())
            {
                if existing_attributes.get(&attr.name).is_some_and(|v| *v) {
                    continue;
                }
                existing_attributes.insert(attr.name.clone(), true);

                let mut code_snippet = attr.name.clone();
                let mut command: Option<Command> = None;

                if !(attr.value_set.as_ref().is_some_and(|v| v == "v") || value.len() == 0) {
                    code_snippet = code_snippet + value;
                    if attr.value_set.is_some() || attr.name == "style" {
                        command = Some(Command {
                            title: "Suggest".to_string(),
                            command: "editor.action.triggerSuggest".to_string(),
                            arguments: None,
                        });
                    }
                }

                let kind = Some(if attr.value_set.as_ref().is_some_and(|v| v == "handler") {
                    CompletionItemKind::FUNCTION
                } else {
                    CompletionItemKind::VALUE
                });
                let documentation = generate_documentation(
                    GenerateDocumentationItem {
                        description: attr.description.clone(),
                        references: attr.references.clone(),
                    },
                    GenerateDocumentationSetting {
                        documentation: true,
                        references: true,
                        does_support_markdown: self.does_support_markdown,
                    },
                );
                let documentation = if let Some(documentation) = documentation {
                    Some(Documentation::MarkupContent(documentation))
                } else {
                    None
                };
                self.result.items.push(CompletionItem {
                    label: attr.name.clone(),
                    kind,
                    documentation,
                    text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(range, code_snippet))),
                    insert_text_format: Some(InsertTextFormat::SNIPPET),
                    command,
                    ..Default::default()
                });
            }
        }
        self.collect_data_attributes_suggestions(range, &existing_attributes)
            .await;
    }

    async fn collect_data_attributes_suggestions(
        &mut self,
        range: Range,
        existing_attributes: &HashMap<String, bool>,
    ) {
        let data_attr = "data-";
        let mut data_attributes: HashMap<String, String> = HashMap::new();
        data_attributes.insert(data_attr.to_string(), format!(r#"{data_attr}$1="$2""#));

        #[async_recursion]
        async fn add_node_data_attributes(
            data_attributes: &mut HashMap<String, String>,
            node: Arc<RwLock<Node>>,
            existing_attributes: &HashMap<String, bool>,
            data_attr: &str,
        ) {
            let node = node.read().await;
            for attr in node.attribute_names() {
                if attr.starts_with(data_attr)
                    && !data_attributes.contains_key(&attr[..])
                    && !existing_attributes.contains_key(attr)
                {
                    data_attributes.insert(attr.to_string(), format!(r#"{attr}="$1""#));
                }
            }
            for child in &node.children {
                add_node_data_attributes(
                    data_attributes,
                    Arc::clone(child),
                    existing_attributes,
                    data_attr,
                )
                .await;
            }
        }

        for root in &self.html_document.roots {
            add_node_data_attributes(
                &mut data_attributes,
                Arc::clone(root),
                existing_attributes,
                data_attr,
            )
            .await;
        }

        for (attr, value) in data_attributes {
            self.result.items.push(CompletionItem {
                label: attr.to_string(),
                kind: Some(CompletionItemKind::VALUE),
                text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                    range,
                    new_text: value,
                })),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            });
        }
    }

    async fn collect_attribute_value_suggestions(&mut self, value_start: usize, value_end: usize) {
        let range: Range;
        let add_quotes: bool;
        let value_prefix;
        if self.offset > value_start
            && self.offset <= value_end
            && is_quote(&self.text[value_start..value_start + 1])
        {
            // inside quoted attribute
            let value_content_start = value_start + 1;
            let mut value_content_end = value_end;
            // valueEnd points to he char after quote, which encloses the replace range
            if value_end > value_start
                && self.text.as_bytes()[value_end - 1] == self.text.as_bytes()[value_start]
            {
                value_content_end -= 1;
            }

            let ws_before = get_word_start(self.text, self.offset, value_content_start);
            let ws_after = get_word_end(self.text, self.offset, value_content_end);
            range = self.get_replace_range(ws_before, ws_after);
            value_prefix = if self.offset >= value_content_start && self.offset < value_content_end
            {
                &self.text[value_content_start..self.offset]
            } else {
                ""
            };
            add_quotes = false;
        } else {
            range = self.get_replace_range(value_start, value_end);
            value_prefix = &self.text[value_start..self.offset];
            add_quotes = true;
        }

        if self.completion_participants.len() > 0 {
            let tag = self
                .current_tag
                .as_deref()
                .unwrap_or_default()
                .to_lowercase();
            let attribute = self.current_attribute_name.to_lowercase();
            let full_range = self.get_replace_range(value_start, value_end);
            for participant in self.completion_participants {
                self.result.items.append(
                    &mut participant
                        .read()
                        .await
                        .on_html_attribute_value(HtmlAttributeValueContext {
                            document: FullTextDocument::new(
                                self.document.language_id().to_string(),
                                self.document.version(),
                                self.document.get_content(None).to_string(),
                            ),
                            html_document: self.html_document.clone(),
                            position: *self.position,
                            tag: tag.clone(),
                            attribute: attribute.clone(),
                            value: value_prefix.to_string(),
                            range: full_range,
                        })
                        .await,
                );
            }
        }

        for provider in &self.data_providers {
            for value in provider.read().await.provide_values(
                &self.current_tag.clone().unwrap_or_default(),
                &self.current_attribute_name,
            ) {
                let insert_text = if add_quotes {
                    format!(r#""{}""#, value.name)
                } else {
                    value.name.clone()
                };

                let documentation = generate_documentation(
                    GenerateDocumentationItem {
                        description: value.description.clone(),
                        references: value.references.clone(),
                    },
                    GenerateDocumentationSetting {
                        documentation: true,
                        references: true,
                        does_support_markdown: self.does_support_markdown,
                    },
                );
                let documentation = if let Some(documentation) = documentation {
                    Some(Documentation::MarkupContent(documentation))
                } else {
                    None
                };
                self.result.items.push(CompletionItem {
                    label: value.name.clone(),
                    filter_text: Some(insert_text.clone()),
                    kind: Some(CompletionItemKind::UNIT),
                    documentation,
                    text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                        range,
                        new_text: insert_text.clone(),
                    })),
                    insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                    ..Default::default()
                });
            }
        }
    }

    async fn collect_close_tag_suggestions(
        &mut self,
        after_open_bracket: usize,
        in_open_tag: bool,
        tag_name_end: usize,
    ) {
        let range = self.get_replace_range(after_open_bracket, tag_name_end);
        let close_tag = if is_followed_by(
            self.text,
            tag_name_end,
            ScannerState::WithinEndTag,
            TokenType::EndTagClose,
        ) {
            ""
        } else {
            ">"
        };
        let mut cur = Some(Arc::clone(&self.node));
        if in_open_tag {
            cur = cur.unwrap().read().await.parent.upgrade();
        }
        while cur.is_some() {
            let c = cur.unwrap();
            let cur_node = c.read().await;
            let tag = &cur_node.tag;
            if tag.is_some()
                && (!cur_node.closed
                    || cur_node.end_tag_start.is_some()
                        && (cur_node.end_tag_start.is_some_and(|s| s > self.offset)))
            {
                let tag = tag.clone().unwrap();
                let mut text_edit = Some(CompletionTextEdit::Edit(TextEdit {
                    range,
                    new_text: format!("/{}{}", tag, close_tag),
                }));
                let mut filter_text = Some(format!("/{}", tag));
                let start_indent = self.get_line_indent(cur_node.start);
                let end_indent = self.get_line_indent(after_open_bracket - 1);
                if start_indent.is_some() && end_indent.is_some() && start_indent != end_indent {
                    let start_indent = start_indent.unwrap();
                    let end_indent = end_indent.unwrap();
                    let insert_text = format!("{}</{}{}", start_indent, tag, close_tag);
                    text_edit = Some(CompletionTextEdit::Edit(TextEdit {
                        range: self.get_replace_range(
                            after_open_bracket - 1 - end_indent.len(),
                            self.offset,
                        ),
                        new_text: insert_text,
                    }));
                    filter_text = Some(format!("{}</{}", end_indent, tag));
                }
                self.result.items.push(CompletionItem {
                    label: format!("/{}", tag),
                    kind: Some(CompletionItemKind::PROPERTY),
                    filter_text,
                    text_edit,
                    insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                    ..Default::default()
                });
                return;
            }
            cur = cur_node.parent.upgrade()
        }
        if in_open_tag {
            return;
        }

        for provider in &self.data_providers {
            for tag in provider.read().await.provide_tags() {
                let documentation = generate_documentation(
                    GenerateDocumentationItem {
                        description: tag.description.clone(),
                        references: tag.references.clone(),
                    },
                    GenerateDocumentationSetting {
                        documentation: true,
                        references: true,
                        does_support_markdown: self.does_support_markdown,
                    },
                );
                let documentation = if let Some(documentation) = documentation {
                    Some(Documentation::MarkupContent(documentation))
                } else {
                    None
                };
                self.result.items.push(CompletionItem {
                    label: format!("/{}", tag.name),
                    kind: Some(CompletionItemKind::PROPERTY),
                    documentation,
                    ..Default::default()
                });
            }
        }
    }

    async fn collect_auto_close_tag_suggestion(&mut self, tag_close_end: usize, tag: &str) {
        if self.settings.is_some() && self.settings.unwrap().hide_auto_complete_proposals {
            return;
        }
        if !self
            .data_manager
            .read()
            .await
            .is_void_element(tag, &self.void_elements)
        {
            let pos = self.document.position_at(tag_close_end as u32);
            let text_edit = Some(CompletionTextEdit::Edit(TextEdit {
                range: Range {
                    start: pos,
                    end: pos,
                },
                new_text: format!("$0</{}>", tag),
            }));
            self.result.items.push(CompletionItem {
                label: format!("</{}>", tag),
                kind: Some(CompletionItemKind::PROPERTY),
                filter_text: Some(format!("</{}>", tag)),
                text_edit,
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            });
        }
    }

    async fn collect_inside_content(&mut self) {
        for participant in self.completion_participants {
            self.result.items.append(
                &mut participant
                    .read()
                    .await
                    .on_html_content(HtmlContentContext {
                        document: FullTextDocument::new(
                            self.document.language_id().to_string(),
                            self.document.version(),
                            self.document.get_content(None).to_string(),
                        ),
                        html_document: self.html_document.clone(),
                        position: *self.position,
                    })
                    .await,
            );
        }
        self.collect_character_entity_proposals();
    }

    fn collect_character_entity_proposals(&mut self) {
        let mut k: i128 = self.offset as i128 - 1;
        let mut character_start = self.position.character;
        while k >= 0 && is_letter_or_digit(self.text, k as usize) {
            k -= 1;
            character_start -= 1;
        }
        if k >= 0 && self.text.as_bytes()[k as usize] == b'&' {
            let range = Range::new(
                Position {
                    line: self.position.line,
                    character: character_start - 1,
                },
                *self.position,
            );
            let entities = get_entities();
            for (entity, value) in entities {
                if entity.ends_with(";") {
                    let label = format!("&{}", entity);
                    self.result.items.push(CompletionItem {
                        label: label.clone(),
                        kind: Some(CompletionItemKind::KEYWORD),
                        documentation: Some(Documentation::String(format!(
                            "Character entity representing '{}",
                            value
                        ))),
                        text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                            range,
                            new_text: label,
                        })),
                        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                        ..Default::default()
                    });
                }
            }
        }
    }

    fn suggest_doctype(&mut self, replace_start: usize, replace_end: usize) {
        let range = self.get_replace_range(replace_start, replace_end);
        self.result.items.push(CompletionItem {
            label: "!DOCTYPE".to_string(),
            kind: Some(CompletionItemKind::PROPERTY),
            documentation: Some(Documentation::String(
                "A preamble for an HTML document.".to_string(),
            )),
            text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                range,
                new_text: "!DOCTYPE html>".to_string(),
            })),
            insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
            ..Default::default()
        });
    }

    async fn get_existing_attributes(&self) -> HashMap<String, bool> {
        let mut map: HashMap<String, bool> = HashMap::new();
        for name in self.node.read().await.attribute_names() {
            map.insert((*name).to_string(), true);
        }
        map
    }

    fn get_line_indent(&self, offset: usize) -> Option<String> {
        let mut start = offset;
        while start > 0 {
            let ch = self.text.as_bytes()[start - 1];
            if b'\n' == ch {
                return Some(self.text[start..offset].to_string());
            }
            if !is_white_space(std::str::from_utf8(&[ch]).unwrap()) {
                return None;
            }
            start -= 1;
        }
        Some(self.text[..offset].to_string())
    }
}

fn is_white_space(text: &str) -> bool {
    Regex::new(r"^\s*$").unwrap().is_match(text)
}

fn is_quote(text: &str) -> bool {
    Regex::new(r#"^["']*$"#).unwrap().is_match(text)
}

fn is_followed_by(
    s: &str,
    offset: usize,
    initial_state: ScannerState,
    expected_token: TokenType,
) -> bool {
    let mut scanner = Scanner::new(s, offset, initial_state);
    let mut token = scanner.scan();
    while token == TokenType::Whitespace {
        token = scanner.scan();
    }
    token == expected_token
}

fn get_word_start(s: &str, offset: usize, limit: usize) -> usize {
    let mut offset = offset;
    while offset > limit && !is_white_space(&s[offset - 1..offset]) {
        offset -= 1;
    }
    offset
}

fn get_word_end(s: &str, offset: usize, limit: usize) -> usize {
    let mut offset = offset;
    while offset < limit && !is_white_space(&s[offset..offset + 1]) {
        offset += 1;
    }
    offset
}

pub trait DocumentContext {
    fn resolve_reference(&self, reference: &str, base: &str) -> Option<&str>;
}

pub struct DefaultDocumentContext;

impl DocumentContext for DefaultDocumentContext {
    fn resolve_reference(&self, _reference: &str, _base: &str) -> Option<&str> {
        None
    }
}

pub struct CompletionConfiguration {
    hide_auto_complete_proposals: bool,
    attribute_default_value: Quotes,
    provider: HashMap<String, bool>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Quotes {
    None,
    Single,
    Double,
}

#[cfg(test)]
mod tests {
    use lsp_types::{MarkupContent, MarkupKind};

    use crate::LanguageService;

    use super::*;

    async fn test_completion_for(
        value: &str,
        expected: Expected,
        settings: Option<CompletionConfiguration>,
        ls_options: Option<LanguageServiceOptions>,
    ) {
        let offset = value.find('|').unwrap();
        let value: &str = &format!("{}{}", &value[..offset], &value[offset + 1..]);

        let ls_options = if let Some(ls_options) = ls_options {
            Arc::new(ls_options)
        } else {
            Arc::new(LanguageServiceOptions::default())
        };
        let ls = LanguageService::new(ls_options, None);

        let document = FullTextDocument::new("html".to_string(), 0, value.to_string());
        let position = document.position_at(offset as u32);
        let html_document = ls.parse_html_document(&document).await;
        let list = ls
            .do_complete(
                &document,
                &position,
                &html_document,
                DefaultDocumentContext,
                settings.as_ref(),
            )
            .await;

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
                    if let Documentation::MarkupContent(source) =
                        matches.documentation.clone().unwrap()
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

    #[tokio::test]
    async fn complete() {
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
        )
        .await;

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
        )
        .await;

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
        )
        .await;

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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;

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
        )
        .await;

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
        )
        .await;

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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
        test_completion_for(
            "<li/|>",
            Expected {
                count: Some(0),
                items: vec![],
            },
            None,
            None,
        )
        .await;
        test_completion_for(
            "  <div/|   ",
            Expected {
                count: Some(0),
                items: vec![],
            },
            None,
            None,
        )
        .await;
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
        )
        .await;
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
        )
        .await;
        test_completion_for(
            "<li><br/|>",
            Expected {
                count: Some(0),
                items: vec![],
            },
            None,
            None,
        )
        .await;
        test_completion_for(
            "<li><br>a/|",
            Expected {
                count: Some(0),
                items: vec![],
            },
            None,
            None,
        )
        .await;

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
        )
        .await;
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
        ).await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        ).await;
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
        ).await;
    }

    #[tokio::test]
    async fn references() {
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
        )
        .await;
    }

    #[tokio::test]
    async fn case_sensitivity() {
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
    }

    #[tokio::test]
    async fn handlebar_completion() {
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
        ).await;
    }

    #[tokio::test]
    async fn support_script_type() {
        test_completion_for(
            r#"<script id="html-template" type="text/html"> <| </script>"#,
            Expected {
                count: None,
                items: vec![ItemDescription {
                    label: "div",
                    result_text: Some(
                        r#"<script id="html-template" type="text/html"> <div </script>"#,
                    ),
                    ..Default::default()
                }],
            },
            None,
            None,
        )
        .await;
    }

    #[tokio::test]
    async fn complete_aria() {
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
        )
        .await;
        test_completion_for(
            "<span  |> </span >",
            Expected {
                count: None,
                items: expected_aria_attributes.clone(),
            },
            None,
            None,
        )
        .await;
        test_completion_for(
            "<input  |> </input >",
            Expected {
                count: None,
                items: expected_aria_attributes.clone(),
            },
            None,
            None,
        )
        .await;
    }

    #[tokio::test]
    async fn settings() {
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
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
        )
        .await;
    }

    #[derive(Default)]
    struct Expected {
        count: Option<usize>,
        items: Vec<ItemDescription>,
    }

    #[derive(Default, Clone)]
    struct ItemDescription {
        label: &'static str,
        result_text: Option<&'static str>,
        kind: Option<CompletionItemKind>,
        documentation: Option<Documentation>,
        filter_text: Option<&'static str>,
        not_available: Option<bool>,
    }
}
