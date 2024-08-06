use std::collections::HashMap;

use lazy_static::lazy_static;
use lsp_textdocument::FullTextDocument;
use lsp_types::{
    Command, CompletionItem, CompletionItemKind, CompletionList, CompletionTextEdit, Documentation,
    InsertTextFormat, Position, Range, TextEdit,
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
        html_document::{HTMLDocument, Node},
        html_entities::get_entities,
        html_scanner::{Scanner, ScannerState, TokenType},
    },
    participant::{HtmlAttributeValueContext, HtmlContentContext, ICompletionParticipant},
    utils::{markdown::does_support_markdown, strings::is_letter_or_digit},
    HTMLLanguageServiceOptions,
};

lazy_static! {
    static ref REG_WHITE_SPACE: Regex = Regex::new(r"^\s*$").unwrap();
    static ref REG_QUOTE: Regex = Regex::new(r#"^["']*$"#).unwrap();
}

pub struct HTMLCompletion {
    supports_markdown: bool,
    completion_participants: Vec<Box<dyn ICompletionParticipant>>,
}

impl HTMLCompletion {
    pub fn new(ls_options: &HTMLLanguageServiceOptions) -> HTMLCompletion {
        HTMLCompletion {
            supports_markdown: does_support_markdown(&ls_options),
            completion_participants: vec![],
        }
    }

    pub fn set_completion_participants(
        &mut self,
        completion_participants: Vec<Box<dyn ICompletionParticipant>>,
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
        data_manager: &HTMLDataManager,
    ) -> CompletionList {
        let mut result = CompletionList::default();
        let mut data_providers = vec![];
        for provider in data_manager.get_data_providers() {
            if provider.is_applicable(document.language_id()) {
                if settings.is_none() {
                    data_providers.push(provider);
                } else {
                    let s = settings.unwrap();
                    let v = s.provider.get(provider.get_id());
                    if v.is_none() || *v.unwrap() {
                        data_providers.push(provider);
                    }
                }
            }
        }

        let void_elements = data_manager.get_void_elements(document.language_id());

        let text = document.get_content(None);
        let offset = document.offset_at(*position).try_into().unwrap();

        let mut parent_list = vec![];
        let node = html_document.find_node_before(offset, &mut parent_list);

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
            node: &node,
            parent_list,
            current_tag: None,
            does_support_markdown: self.supports_markdown,
            html_document,
            current_attribute_name: String::new(),
            completion_participants: &self.completion_participants,
            position,
            data_manager,
        };

        let mut scanner = Scanner::new(text, node.start, ScannerState::WithinContent, true);

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
                    content.current_tag = Some(scanner.get_token_text().to_string());
                }
                TokenType::AttributeName => {
                    if scanner.get_token_offset() <= offset && offset <= scanner.get_token_end() {
                        content.collect_attribute_name_suggestions(
                            scanner.get_token_offset(),
                            scanner.get_token_end(),
                        );
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
                                content
                                    .collect_attribute_value_suggestions(
                                        scanner.get_token_end(),
                                        offset,
                                    )
                                    .await;
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
                            content.collect_auto_close_tag_suggestion(
                                scanner.get_token_end(),
                                &content.current_tag.clone().unwrap(),
                            );
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
                        content.collect_close_tag_suggestions(
                            after_open_bracket,
                            false,
                            end_offset,
                        );
                        return result;
                    }
                }
                TokenType::EndTag => {
                    if offset <= scanner.get_token_end() {
                        let mut start = scanner.get_token_offset() - 1;
                        while start > 0 {
                            let ch = text.chars().nth(start).unwrap();
                            if ch == '/' {
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

    pub fn do_quote_complete(
        document: &FullTextDocument,
        position: &Position,
        html_document: &HTMLDocument,
        settings: Option<&CompletionConfiguration>,
    ) -> Option<String> {
        let offset = document.offset_at(*position) as usize;
        if offset == 0 {
            return None;
        }
        if document.get_content(None).chars().nth(offset - 1) != Some('=') {
            return None;
        }
        let default_value = if let Some(settings) = settings {
            settings.attribute_default_value
        } else {
            Quotes::Double
        };
        if default_value == Quotes::None {
            return None;
        }
        let value = if default_value == Quotes::Double {
            r#""$1""#.to_string()
        } else {
            "'$1'".to_string()
        };
        let node = html_document.find_node_before(offset, &mut vec![])?;
        if node.start < offset
            && !node
                .end_tag_start
                .is_some_and(|end_tag_start| end_tag_start <= offset)
        {
            let mut scanner = Scanner::new(
                document.get_content(None),
                node.start,
                ScannerState::WithinContent,
                false,
            );
            let mut token = scanner.scan();
            while token != TokenType::EOS && scanner.get_token_end() <= offset {
                if token == TokenType::AttributeName && scanner.get_token_end() == offset - 1 {
                    // Ensure the token is a valid standalone attribute name
                    token = scanner.scan();
                    if token != TokenType::DelimiterAssign {
                        return None;
                    }
                    token = scanner.scan();
                    // Any non-attribute valid tag
                    if token == TokenType::Unknown || token == TokenType::AttributeValue {
                        return None;
                    }
                    return Some(value);
                }
                token = scanner.scan();
            }
        }
        None
    }

    pub fn do_tag_complete(
        &self,
        document: &FullTextDocument,
        position: &Position,
        html_document: &HTMLDocument,
        data_manager: &HTMLDataManager,
    ) -> Option<String> {
        let offset = document.offset_at(*position) as usize;
        if offset == 0 {
            return None;
        }
        let char = document.get_content(None).chars().nth(offset - 1);
        if char == Some('>') {
            let void_elements = data_manager.get_void_elements(document.language_id());
            let node = html_document.find_node_before(offset, &mut vec![])?;
            let node_tag = node.tag.as_ref()?;
            if !data_manager.is_void_element(&node_tag, &void_elements)
                && node.start < offset
                && !node
                    .end_tag_start
                    .is_some_and(|end_tag_start| end_tag_start <= offset)
            {
                let mut scanner = Scanner::new(
                    document.get_content(None),
                    node.start,
                    ScannerState::WithinContent,
                    false,
                );
                let mut token = scanner.scan();
                while token != TokenType::EOS && scanner.get_token_end() <= offset {
                    if token == TokenType::StartTagClose && scanner.get_token_end() == offset {
                        return Some(format!("$0</{}>", node_tag));
                    }
                    token = scanner.scan();
                }
            }
        } else if char == Some('/') {
            let mut parent_list = vec![];
            let mut node = html_document.find_node_before(offset, &mut parent_list)?;
            loop {
                if !node.closed
                    || node
                        .end_tag_start
                        .is_some_and(|end_tag_start| end_tag_start > offset)
                {
                    break;
                }
                node = parent_list.pop()?;
            }
            let node_tag = node.tag.as_ref()?;
            let mut scanner = Scanner::new(
                document.get_content(None),
                node.start,
                ScannerState::WithinContent,
                false,
            );
            let mut token = scanner.scan();
            while token != TokenType::EOS && scanner.get_token_end() <= offset {
                if token == TokenType::EndTagOpen && scanner.get_token_end() == offset {
                    if document.get_content(None).chars().nth(offset) != Some('>') {
                        return Some(format!("{}>", node_tag));
                    } else {
                        return Some(node_tag.clone());
                    }
                }
                token = scanner.scan();
            }
        }
        None
    }
}

struct CompletionContext<'a> {
    result: &'a mut CompletionList,
    text: &'a str,
    offset: usize,
    document: &'a FullTextDocument,
    data_providers: Vec<&'a Box<dyn IHTMLDataProvider>>,
    void_elements: Vec<String>,
    settings: Option<&'a CompletionConfiguration>,
    node: &'a Node,
    parent_list: Vec<&'a Node>,
    current_tag: Option<String>,
    does_support_markdown: bool,
    html_document: &'a HTMLDocument,
    current_attribute_name: String,
    completion_participants: &'a Vec<Box<dyn ICompletionParticipant>>,
    position: &'a Position,
    data_manager: &'a HTMLDataManager,
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

    fn collect_attribute_name_suggestions(&mut self, name_start: usize, name_end: usize) {
        let mut replace_end = self.offset;
        let text = self.document.get_content(None);
        while replace_end < name_end && text.chars().nth(replace_end).is_some_and(|c| c != '<') {
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

        let mut existing_attributes = self.get_existing_attributes();
        existing_attributes.insert(current_attribute.to_string(), false);

        for provider in &self.data_providers {
            for attr in provider.provide_attributes(&self.current_tag.as_ref().unwrap()) {
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
        self.collect_data_attributes_suggestions(range, &existing_attributes);
    }

    fn collect_data_attributes_suggestions(
        &mut self,
        range: Range,
        existing_attributes: &HashMap<String, bool>,
    ) {
        let data_attr = "data-";
        let mut data_attributes: HashMap<String, String> = HashMap::new();
        data_attributes.insert(data_attr.to_string(), format!(r#"{data_attr}$1="$2""#));

        fn add_node_data_attributes(
            data_attributes: &mut HashMap<String, String>,
            node: &Node,
            existing_attributes: &HashMap<String, bool>,
            data_attr: &str,
        ) {
            for attr in node.attribute_names() {
                if attr.starts_with(data_attr)
                    && !data_attributes.contains_key(&attr[..])
                    && !existing_attributes.contains_key(attr)
                {
                    data_attributes.insert(attr.to_string(), format!(r#"{attr}="$1""#));
                }
            }
            for child in &node.children {
                add_node_data_attributes(data_attributes, child, existing_attributes, data_attr);
            }
        }

        for root in &self.html_document.roots {
            add_node_data_attributes(&mut data_attributes, root, existing_attributes, data_attr);
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
                && self.text.chars().nth(value_end - 1) == self.text.chars().nth(value_start)
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
            for value in provider.provide_values(
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

    fn collect_close_tag_suggestions(
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
        let mut cur = Some(self.node);
        let mut cur_parent_list = self.parent_list.clone();
        if in_open_tag {
            cur = cur_parent_list.pop();
        }
        while cur.is_some() {
            let cur_node = cur.unwrap();
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
            cur = cur_parent_list.pop();
        }
        if in_open_tag {
            return;
        }

        for provider in &self.data_providers {
            for tag in provider.provide_tags() {
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

    fn collect_auto_close_tag_suggestion(&mut self, tag_close_end: usize, tag: &str) {
        if self.settings.is_some() && self.settings.unwrap().hide_auto_complete_proposals {
            return;
        }
        if !self.data_manager.is_void_element(tag, &self.void_elements) {
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
        if k >= 0 && self.text.chars().nth(k as usize) == Some('&') {
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

    fn get_existing_attributes(&self) -> HashMap<String, bool> {
        let mut map: HashMap<String, bool> = HashMap::new();
        for name in self.node.attribute_names() {
            map.insert((*name).to_string(), true);
        }
        map
    }

    fn get_line_indent(&self, offset: usize) -> Option<String> {
        let mut start = offset;
        while start > 0 {
            let ch = self.text.chars().nth(start - 1);
            if ch == Some('\n') {
                return Some(self.text[start..offset].to_string());
            }
            if let Some(ch) = ch {
                if !is_white_space(&ch.to_string()) {
                    return None;
                }
            }
            start -= 1;
        }
        Some(self.text[..offset].to_string())
    }
}

fn is_white_space(text: &str) -> bool {
    REG_WHITE_SPACE.is_match(text)
}

fn is_quote(text: &str) -> bool {
    REG_QUOTE.is_match(text)
}

fn is_followed_by(
    s: &str,
    offset: usize,
    initial_state: ScannerState,
    expected_token: TokenType,
) -> bool {
    let mut scanner = Scanner::new(s, offset, initial_state, false);
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
    fn resolve_reference(&self, reference: &str, base: &str) -> Option<String>;
}

pub struct DefaultDocumentContext;

impl DocumentContext for DefaultDocumentContext {
    fn resolve_reference(&self, _reference: &str, _base: &str) -> Option<String> {
        None
    }
}

pub struct CompletionConfiguration {
    pub hide_auto_complete_proposals: bool,
    pub attribute_default_value: Quotes,
    pub provider: HashMap<String, bool>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Quotes {
    None,
    Single,
    Double,
}
