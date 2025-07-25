use std::borrow::Cow;

use lazy_static::lazy_static;
use multi_line_stream::MultiLineStream;
use regex::Regex;

lazy_static! {
    static ref REG_DOCTYPE: Regex = Regex::new(r"^!(?i)doctype").unwrap();
    static ref REG_NON_SPECIAL_START: Regex = Regex::new(r#"^[^\s"'`=<>]+"#).unwrap();
    static ref REG_SCRIPT_COMMENT: Regex = Regex::new(r"<!--|-->|<\/?script\s*\/?>?").unwrap();
    static ref REG_ELEMENT_NAME: Regex = Regex::new(r"^[_:\w][_:\w\-.\d]*").unwrap();
    static ref REG_NON_ELEMENT_NAME: Regex =
        Regex::new(r#"^[^\s"'></=\x00-\x0F\x7F\x80-\x9F]+"#).unwrap();
    static ref REG_STYLE: Regex = Regex::new(r"<\/style").unwrap();
}

/// Scan the input string with byte as the base unit to generate a token stream
pub struct Scanner<'a> {
    state: ScannerState,
    token_type: TokenType,
    token_offset: usize,
    token_error: Option<&'static str>,
    stream: MultiLineStream<'a>,

    emit_pseudo_close_tags: bool,
    has_space_after_tag: bool,
    last_tag: Option<Cow<'a, str>>,
    last_attribute_name: Option<Cow<'a, str>>,
    last_type_value: Option<Cow<'a, str>>,
    case_sensitive: bool,
}

impl<'a> Scanner<'a> {
    pub fn new(
        input: &'a str,
        initial_offset: usize,
        initial_state: ScannerState,
        emit_pseudo_close_tags: bool,
        case_sensitive: bool,
    ) -> Scanner<'a> {
        let stream = MultiLineStream::new(input, initial_offset);
        let token_offset = 0;
        let token_type = TokenType::Unknown;
        Scanner {
            state: initial_state,
            token_type,
            token_offset,
            token_error: None,
            stream,
            emit_pseudo_close_tags,
            has_space_after_tag: false,
            last_tag: None,
            last_attribute_name: None,
            last_type_value: None,
            case_sensitive,
        }
    }

    pub fn scan(&mut self) -> TokenType {
        let offset = self.stream.pos();
        let old_state = &self.state.clone();
        self.internal_scan();
        if self.token_type != TokenType::EOS
            && offset == self.stream.pos()
            && !(self.emit_pseudo_close_tags
                && [TokenType::StartTagClose, TokenType::EndTagClose].contains(&self.token_type))
        {
            eprintln!(
                "Scanner.scan has not advanced at offset {}, state before: {:?} after: {:?}",
                offset, old_state, self.state,
            );
            self.stream.advance(1);
            return self.finish_token(offset, TokenType::Unknown, None);
        }
        self.token_type
    }

    pub fn get_token_type(&self) -> TokenType {
        self.token_type
    }

    pub fn get_token_offset(&self) -> usize {
        self.token_offset
    }

    pub fn get_token_length(&self) -> usize {
        self.stream.pos() - self.token_offset
    }

    pub fn get_token_end(&self) -> usize {
        self.stream.pos()
    }

    pub fn get_token_text(&self) -> &'a str {
        &self.stream.source[self.get_token_offset()..self.get_token_end()]
    }

    pub fn get_scanner_state(&self) -> ScannerState {
        self.state
    }

    pub fn get_token_error(&self) -> Option<&'static str> {
        self.token_error
    }

    pub fn get_source_len(&self) -> usize {
        self.stream.source.len()
    }

    fn internal_scan(&mut self) -> TokenType {
        let offset = self.stream.pos();
        if self.stream.eos() {
            return self.finish_token(offset, TokenType::EOS, None);
        }
        let error_message;

        match self.state {
            ScannerState::WithinComment => {
                if self.stream.advance_if_chars("-->") {
                    // -->
                    self.state = ScannerState::WithinContent;
                    return self.finish_token(offset, TokenType::EndCommentTag, None);
                }
                self.stream.advance_until_chars("-->"); // -->
                return self.finish_token(offset, TokenType::Comment, None);
            }

            ScannerState::WithinDoctype => {
                if self.stream.advance_if_char(b'>') {
                    self.state = ScannerState::WithinContent;
                    return self.finish_token(offset, TokenType::EndDoctypeTag, None);
                }
                self.stream.advance_until_char(b'>'); // >
                return self.finish_token(offset, TokenType::Doctype, None);
            }

            ScannerState::WithinContent => {
                if self.stream.advance_if_char(b'<') {
                    // <
                    if !self.stream.eos() && self.stream.peek_char(0) == Some(b'!') {
                        // !
                        if self.stream.advance_if_chars("!--") {
                            // <!--
                            self.state = ScannerState::WithinComment;
                            return self.finish_token(offset, TokenType::StartCommentTag, None);
                        }
                        if self.stream.advance_if_regexp(&REG_DOCTYPE).is_some() {
                            self.state = ScannerState::WithinDoctype;
                            return self.finish_token(offset, TokenType::StartDoctypeTag, None);
                        }
                    }
                    if self.stream.advance_if_char(b'/') {
                        // /
                        self.state = ScannerState::AfterOpeningEndTag;
                        return self.finish_token(offset, TokenType::EndTagOpen, None);
                    }
                    self.state = ScannerState::AfterOpeningStartTag;
                    return self.finish_token(offset, TokenType::StartTagOpen, None);
                }
                self.stream.advance_until_char(b'<');
                return self.finish_token(offset, TokenType::Content, None);
            }

            ScannerState::AfterOpeningEndTag => {
                let tag_name = self.next_element_name();
                if tag_name.is_some() {
                    self.state = ScannerState::WithinEndTag;
                    return self.finish_token(offset, TokenType::EndTag, None);
                }
                if self.stream.skip_whitespace() {
                    // white space is not valid here
                    return self.finish_token(
                        offset,
                        TokenType::Whitespace,
                        Some("Tag name must directly follow the open bracket."),
                    );
                }
                self.state = ScannerState::WithinEndTag;
                self.stream.advance_until_char(b'>');
                if offset < self.stream.pos() {
                    return self.finish_token(
                        offset,
                        TokenType::Unknown,
                        Some("End tag name expected."),
                    );
                }
                return self.internal_scan();
            }

            ScannerState::WithinEndTag => {
                if self.stream.skip_whitespace() {
                    // white space is valid here
                    return self.finish_token(offset, TokenType::Whitespace, None);
                }
                if self.stream.advance_if_char(b'>') {
                    // >
                    self.state = ScannerState::WithinContent;
                    return self.finish_token(offset, TokenType::EndTagClose, None);
                }
                if self.emit_pseudo_close_tags && self.stream.peek_char(0) == Some(b'<') {
                    // <
                    self.state = ScannerState::WithinContent;
                    return self.finish_token(
                        offset,
                        TokenType::EndTagClose,
                        Some("Closing bracket missing."),
                    );
                }
                error_message = Some("Closing bracket expected.");
            }

            ScannerState::AfterOpeningStartTag => {
                self.last_tag = self.next_element_name();
                self.last_type_value = None;
                self.last_attribute_name = None;
                if self.last_tag.is_some() {
                    self.has_space_after_tag = false;
                    self.state = ScannerState::WithinTag;
                    return self.finish_token(offset, TokenType::StartTag, None);
                }
                if self.stream.skip_whitespace() {
                    // white space is not valid here
                    return self.finish_token(
                        offset,
                        TokenType::Whitespace,
                        Some("Tag name must directly follow the open bracket."),
                    );
                }
                self.state = ScannerState::WithinTag;
                self.stream.advance_until_char(b'>');
                if offset < self.stream.pos() {
                    return self.finish_token(
                        offset,
                        TokenType::Unknown,
                        Some("Start tag name expected."),
                    );
                }
                return self.internal_scan();
            }

            ScannerState::WithinTag => {
                if self.stream.skip_whitespace() {
                    self.has_space_after_tag = true; // remember that we have seen a whitespace
                    return self.finish_token(offset, TokenType::Whitespace, None);
                }
                if self.has_space_after_tag {
                    self.last_attribute_name = self.next_attribute_name();
                    if self.last_attribute_name.is_some() {
                        self.state = ScannerState::AfterAttributeName;
                        self.has_space_after_tag = false;
                        return self.finish_token(offset, TokenType::AttributeName, None);
                    }
                    if self.stream.peek_char(0) == Some(b'=') {
                        self.has_space_after_tag = false;
                    }
                }
                if self.stream.advance_if_chars("/>") {
                    // />
                    self.state = ScannerState::WithinContent;
                    return self.finish_token(offset, TokenType::StartTagSelfClose, None);
                }
                if self.stream.advance_if_char(b'>') {
                    // >
                    if self
                        .last_tag
                        .as_ref()
                        .is_some_and(|v| v.as_ref() == "script")
                    {
                        if self.last_type_value.is_some() {
                            // stay in html
                            self.state = ScannerState::WithinContent;
                        } else {
                            self.state = ScannerState::WithinScriptContent;
                        }
                    } else if self
                        .last_tag
                        .as_ref()
                        .is_some_and(|v| v.as_ref() == "style")
                    {
                        self.state = ScannerState::WithinStyleContent;
                    } else {
                        self.state = ScannerState::WithinContent;
                    }
                    return self.finish_token(offset, TokenType::StartTagClose, None);
                }
                if self.emit_pseudo_close_tags && self.stream.peek_char(0) == Some(b'<') {
                    // <
                    self.state = ScannerState::WithinContent;
                    return self.finish_token(
                        offset,
                        TokenType::StartTagClose,
                        Some("Closing bracket missing."),
                    );
                }
                self.stream.advance(1);
                return self.finish_token(
                    offset,
                    TokenType::Unknown,
                    Some("Unexpected character in tag."),
                );
            }

            ScannerState::AfterAttributeName => {
                if self.stream.skip_whitespace() {
                    self.has_space_after_tag = true;
                    return self.finish_token(offset, TokenType::Whitespace, None);
                }

                if self.stream.advance_if_char(b'=') {
                    self.state = ScannerState::BeforeAttributeValue;
                    return self.finish_token(offset, TokenType::DelimiterAssign, None);
                }
                self.state = ScannerState::WithinTag;
                return self.internal_scan(); // no advance yet - jump to WithinTag
            }

            ScannerState::BeforeAttributeValue => {
                if self.stream.skip_whitespace() {
                    self.state = ScannerState::WithinTag;
                    self.has_space_after_tag = true;
                    return self.finish_token(offset, TokenType::Whitespace, None);
                }
                let cur_char = self.stream.peek_char(0);
                let prev_char = self.stream.peek_char(-1);
                let attribute_value = self.stream.advance_if_regexp(&REG_NON_SPECIAL_START);
                if let Some(mut attribute_value) = attribute_value {
                    let mut is_go_back = false;
                    if cur_char == Some(b'>') && prev_char == Some(b'/') {
                        // <foo bar=http://foo/>
                        is_go_back = true;
                        attribute_value = &attribute_value[..attribute_value.len() - 1];
                    }
                    if self.stream.advance_if_char(b'\'') || self.stream.advance_if_char(b'"') {
                        attribute_value = &self.stream.source
                            [self.stream.pos() - attribute_value.len() - 1..self.stream.pos()]
                    }
                    if self
                        .last_attribute_name
                        .as_ref()
                        .is_some_and(|v| v.as_ref() == "type")
                    {
                        let s = attribute_value;
                        self.last_type_value = if s.len() != 0 {
                            Some(Cow::Borrowed(s))
                        } else {
                            None
                        };
                    }
                    let attribute_value_len = attribute_value.len();
                    if is_go_back {
                        self.stream.go_back(1);
                    }
                    if attribute_value_len > 0 {
                        self.state = ScannerState::WithinTag;
                        self.has_space_after_tag = false;
                        return self.finish_token(offset, TokenType::AttributeValue, None);
                    }
                }
                let ch = self.stream.peek_char(0);
                if let Some(ch) = ch {
                    if ch == b'\'' || ch == b'"' {
                        self.stream.advance(1); // consume quote
                        if self.stream.advance_until_char(ch) {
                            self.stream.advance(1); // consume quote
                        }
                        if self
                            .last_attribute_name
                            .as_ref()
                            .is_some_and(|v| v.as_ref() == "type")
                        {
                            let s = &self.stream.source[if offset + 1 > self.stream.pos() - 1 {
                                self.stream.pos() - 1..offset + 1
                            } else {
                                offset + 1..self.stream.pos() - 1
                            }];
                            self.last_type_value = if s.len() != 0 {
                                Some(Cow::Borrowed(s))
                            } else {
                                None
                            }
                        }
                        self.state = ScannerState::WithinTag;
                        self.has_space_after_tag = false;
                        return self.finish_token(offset, TokenType::AttributeValue, None);
                    }
                }
                self.state = ScannerState::WithinTag;
                self.has_space_after_tag = false;
                return self.internal_scan(); // no advance yet - jump to WithinTag
            }

            ScannerState::WithinScriptContent => {
                // see http://stackoverflow.com/questions/14574471/how-do-browsers-parse-a-script-tag-exactly
                let mut script_state: u8 = 1;
                while !self.stream.eos() {
                    let m = self.stream.advance_if_regexp(&REG_SCRIPT_COMMENT);
                    if m.is_none() {
                        self.stream.go_to_end();
                        return self.finish_token(offset, TokenType::Script, None);
                    } else if m == Some("<!--") {
                        if script_state == 1 {
                            script_state = 2;
                        }
                    } else if m == Some("-->") {
                        script_state = 1;
                    } else if m.is_some_and(|m| &m[1..2] != "/") {
                        // <script
                        if script_state == 2 {
                            script_state = 3;
                        }
                    } else {
                        // </script
                        if script_state == 3 {
                            script_state = 2;
                        } else {
                            let length = m.map(|v| v.len()).unwrap_or_default();
                            self.stream.go_back(length); // to the beginning of the closing tag
                            break;
                        }
                    }
                }
                self.state = ScannerState::WithinContent;
                if offset < self.stream.pos() {
                    return self.finish_token(offset, TokenType::Script, None);
                }
                return self.internal_scan(); // no advance yet - jump to content
            }

            ScannerState::WithinStyleContent => {
                self.stream.advance_until_regexp(&REG_STYLE);
                self.state = ScannerState::WithinContent;
                if offset < self.stream.pos() {
                    return self.finish_token(offset, TokenType::Styles, None);
                }
                return self.internal_scan(); // no advance yet - jump to content
            }
        }

        self.stream.advance(1);
        self.state = ScannerState::WithinContent;
        return self.finish_token(offset, TokenType::Unknown, error_message);
    }

    fn finish_token(
        &mut self,
        offset: usize,
        token_type: TokenType,
        error_message: Option<&'static str>,
    ) -> TokenType {
        self.token_type = token_type;
        self.token_offset = offset;
        self.token_error = error_message;
        self.token_type
    }

    fn next_element_name(&mut self) -> Option<Cow<'a, str>> {
        if self.case_sensitive {
            Some(Cow::Borrowed(
                self.stream.advance_if_regexp(&REG_ELEMENT_NAME)?,
            ))
        } else {
            Some(Cow::Owned(
                self.stream
                    .advance_if_regexp(&REG_ELEMENT_NAME)?
                    .to_lowercase(),
            ))
        }
    }

    fn next_attribute_name(&mut self) -> Option<Cow<'a, str>> {
        if self.case_sensitive {
            Some(Cow::Borrowed(
                self.stream.advance_if_regexp(&REG_NON_ELEMENT_NAME)?,
            ))
        } else {
            Some(Cow::Owned(
                self.stream
                    .advance_if_regexp(&REG_NON_ELEMENT_NAME)?
                    .to_lowercase(),
            ))
        }
    }
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum TokenType {
    StartCommentTag,
    Comment,
    EndCommentTag,
    StartTagOpen,
    StartTagClose,
    StartTagSelfClose,
    StartTag,
    EndTagOpen,
    EndTagClose,
    EndTag,
    DelimiterAssign,
    AttributeName,
    AttributeValue,
    StartDoctypeTag,
    Doctype,
    EndDoctypeTag,
    Content,
    Whitespace,
    Unknown,
    Script,
    Styles,
    EOS,
}

#[derive(Debug, Clone, Copy)]
pub enum ScannerState {
    WithinContent,
    AfterOpeningStartTag,
    AfterOpeningEndTag,
    WithinDoctype,
    WithinTag,
    WithinEndTag,
    WithinComment,
    WithinScriptContent,
    WithinStyleContent,
    AfterAttributeName,
    BeforeAttributeValue,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_tokens(tests: Vec<TestItem>) {
        let mut scanner_state = ScannerState::WithinContent;

        for t in tests {
            let mut scanner = Scanner::new(&t.input, 0, scanner_state, false, false);
            let mut token_type = scanner.scan();
            let mut actual = vec![];
            while token_type != TokenType::EOS {
                let offset = scanner.get_token_offset();
                let mut actual_token = Token {
                    offset,
                    token_type: token_type,
                    content: None,
                };
                if [TokenType::StartTag, TokenType::EndTag].contains(&token_type) {
                    actual_token.content = Some(
                        t.input[scanner.get_token_offset()..scanner.get_token_end()].to_string(),
                    );
                }
                actual.push(actual_token);
                token_type = scanner.scan();
            }
            assert_eq!(actual, t.tokens);
            scanner_state = scanner.get_scanner_state();
        }
    }

    #[test]
    fn open_start_tag() {
        assert_tokens(vec![TestItem {
            input: "<abc".to_string(),
            tokens: vec![
                Token {
                    offset: 0,
                    token_type: TokenType::StartTagOpen,
                    content: None,
                },
                Token {
                    offset: 1,
                    token_type: TokenType::StartTag,
                    content: Some("abc".to_string()),
                },
            ],
        }]);
        assert_tokens(vec![TestItem {
            input: "<input".to_string(),
            tokens: vec![
                Token {
                    offset: 0,
                    token_type: TokenType::StartTagOpen,
                    content: None,
                },
                Token {
                    offset: 1,
                    token_type: TokenType::StartTag,
                    content: Some("input".to_string()),
                },
            ],
        }]);
    }

    struct TestItem {
        input: String,
        tokens: Vec<Token>,
    }

    #[derive(PartialEq, Debug)]
    struct Token {
        offset: usize,
        token_type: TokenType,
        content: Option<String>,
    }
}
