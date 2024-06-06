use regex::Regex;

pub struct Scanner<'a> {
    state: ScannerState,
    token_type: TokenType,
    token_offset: usize,
    token_error: Option<&'static str>,
    stream: MultiLineStream<'a>,

    emit_pseudo_close_tags: bool,
    has_space_after_tag: bool,
    last_tag: Option<String>,
    last_attribute_name: Option<String>,
    last_type_value: Option<String>,
}

impl Scanner<'_> {
    pub fn new<'a>(
        input: &'a str,
        initial_offset: usize,
        initial_state: ScannerState,
        emit_pseudo_close_tags: bool,
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

    pub fn get_token_text(&self) -> &str {
        &self.stream.get_source()[self.token_offset..self.get_token_end()]
    }

    pub fn get_scanner_state(&self) -> ScannerState {
        self.state
    }

    pub fn get_token_error(&self) -> Option<&'static str> {
        self.token_error
    }

    fn internal_scan(&mut self) -> TokenType {
        let offset = self.stream.pos();
        if self.stream.eos() {
            return self.finish_token(offset, TokenType::EOS, None);
        }
        let error_message;

        match self.state {
            ScannerState::WithinComment => {
                if self.stream.advance_if_chars(vec![_MIN, _MIN, _RAN]) {
                    // -->
                    self.state = ScannerState::WithinContent;
                    return self.finish_token(offset, TokenType::EndCommentTag, None);
                }
                self.stream.advance_until_chars(vec![_MIN, _MIN, _RAN]); // -->
                return self.finish_token(offset, TokenType::Comment, None);
            }

            ScannerState::WithinDoctype => {
                if self.stream.advance_if_char(_RAN) {
                    self.state = ScannerState::WithinContent;
                    return self.finish_token(offset, TokenType::EndDoctypeTag, None);
                }
                self.stream.advance_until_char(_RAN); // >
                return self.finish_token(offset, TokenType::Doctype, None);
            }

            ScannerState::WithinContent => {
                if self.stream.advance_if_char(_LAN) {
                    // <
                    if !self.stream.eos() && self.stream.peek_char(0) == _BNG {
                        // !
                        if self.stream.advance_if_chars(vec![_BNG, _MIN, _MIN]) {
                            // <!--
                            self.state = ScannerState::WithinComment;
                            return self.finish_token(offset, TokenType::StartCommentTag, None);
                        }
                        if self
                            .stream
                            .advance_if_regexp(Regex::new(r"^!doctype").unwrap())
                            != ""
                        {
                            self.state = ScannerState::WithinDoctype;
                            return self.finish_token(offset, TokenType::StartDoctypeTag, None);
                        }
                    }
                    if self.stream.advance_if_char(_FSL) {
                        // /
                        self.state = ScannerState::AfterOpeningEndTag;
                        return self.finish_token(offset, TokenType::EndTagOpen, None);
                    }
                    self.state = ScannerState::AfterOpeningStartTag;
                    return self.finish_token(offset, TokenType::StartTagOpen, None);
                }
                self.stream.advance_until_char(_LAN);
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
                self.stream.advance_until_char(_RAN);
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
                if self.stream.advance_if_char(_RAN) {
                    // >
                    self.state = ScannerState::WithinContent;
                    return self.finish_token(offset, TokenType::EndTagClose, None);
                }
                if self.emit_pseudo_close_tags && self.stream.peek_char(0) == _LAN {
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
                self.stream.advance_until_char(_RAN);
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
                }
                if self.stream.advance_if_chars(vec![_FSL, _RAN]) {
                    // />
                    self.state = ScannerState::WithinContent;
                    return self.finish_token(offset, TokenType::StartTagSelfClose, None);
                }
                if self.stream.advance_if_char(_RAN) {
                    // >
                    if self.last_tag == Some("script".to_string()) {
                        if self.last_type_value.is_some() {
                            // stay in html
                            self.state = ScannerState::WithinContent;
                        } else {
                            self.state = ScannerState::WithinScriptContent;
                        }
                    } else if self.last_tag == Some("style".to_string()) {
                        self.state = ScannerState::WithinStyleContent;
                    } else {
                        self.state = ScannerState::WithinContent;
                    }
                    return self.finish_token(offset, TokenType::StartTagClose, None);
                }
                if self.emit_pseudo_close_tags && self.stream.peek_char(0) == _LAN {
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

                if self.stream.advance_if_char(_EQS) {
                    self.state = ScannerState::BeforeAttributeValue;
                    return self.finish_token(offset, TokenType::DelimiterAssign, None);
                }
                self.state = ScannerState::WithinTag;
                return self.internal_scan(); // no advance yet - jump to WithinTag
            }

            ScannerState::BeforeAttributeValue => {
                if self.stream.skip_whitespace() {
                    return self.finish_token(offset, TokenType::Whitespace, None);
                }
                let cur_char = self.stream.peek_char(0);
                let prev_char = self.stream.peek_char(-1);
                let mut attribute_value = self
                    .stream
                    .advance_if_regexp(Regex::new(r#"^[^\s"'`=<>]+"#).unwrap());
                if attribute_value.len() > 0 {
                    let mut is_go_back = false;
                    if cur_char == _RAN && prev_char == _FSL {
                        // <foo bar=http://foo/>
                        is_go_back = true;
                        attribute_value = &attribute_value[..attribute_value.len() - 1];
                    }
                    if self.last_attribute_name == Some("type".to_string()) {
                        let s = attribute_value.to_string();
                        self.last_type_value = if s.len() != 0 { Some(s) } else { None };
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
                if ch == _SQO || ch == _DQO {
                    self.stream.advance(1); // consume quote
                    if self.stream.advance_until_char(ch) {
                        self.stream.advance(1); // consume quote
                    }
                    if self.last_attribute_name == Some("type".to_string()) {
                        let s = self.stream.get_source()[if offset + 1 > self.stream.pos() - 1 {
                            self.stream.pos() - 1..offset + 1
                        } else {
                            offset + 1..self.stream.pos() - 1
                        }]
                        .to_string();
                        self.last_type_value = if s.len() != 0 { Some(s) } else { None }
                    }
                    self.state = ScannerState::WithinTag;
                    self.has_space_after_tag = false;
                    return self.finish_token(offset, TokenType::AttributeValue, None);
                }
                self.state = ScannerState::WithinTag;
                self.has_space_after_tag = false;
                return self.internal_scan(); // no advance yet - jump to WithinTag
            }

            ScannerState::WithinScriptContent => {
                // see http://stackoverflow.com/questions/14574471/how-do-browsers-parse-a-script-tag-exactly
                let mut script_state: u8 = 1;
                while !self.stream.eos() {
                    let m = self
                        .stream
                        .advance_if_regexp(Regex::new(r"<!--|-->|<\/?script\s*\/?>?").unwrap());
                    if m.len() == 0 {
                        self.stream.go_to_end();
                        return self.finish_token(offset, TokenType::Script, None);
                    } else if m == "<!--" {
                        if script_state == 1 {
                            script_state = 2;
                        }
                    } else if m == "-->" {
                        script_state = 1;
                    } else if &m[1..2] != "/" {
                        // <script
                        if script_state == 2 {
                            script_state = 3;
                        }
                    } else {
                        // </script
                        if script_state == 3 {
                            script_state = 2;
                        } else {
                            let length = m.len();
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
                self.stream
                    .advance_until_regexp(Regex::new(r"<\/style").unwrap());
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

    fn next_element_name(&mut self) -> Option<String> {
        let s = self
            .stream
            .advance_if_regexp(Regex::new(r"^[_:\w][_:\w\-.\d]*").unwrap())
            .to_lowercase();
        if s.len() != 0 {
            Some(s)
        } else {
            None
        }
    }

    fn next_attribute_name(&mut self) -> Option<String> {
        let s = self
            .stream
            .advance_if_regexp(Regex::new(r#"^[^\s"'></=\x00-\x0F\x7F\x80-\x9F]*"#).unwrap())
            .to_lowercase();
        if s.len() != 0 {
            Some(s)
        } else {
            None
        }
    }
}

struct MultiLineStream<'a> {
    source: &'a str,
    len: usize,
    position: usize,
}

impl MultiLineStream<'_> {
    pub fn new<'a>(source: &'a str, position: usize) -> MultiLineStream<'a> {
        MultiLineStream {
            source,
            len: source.len(),
            position,
        }
    }

    pub fn eos(&self) -> bool {
        self.len <= self.position
    }

    pub fn get_source(&self) -> &str {
        self.source
    }

    pub fn pos(&self) -> usize {
        self.position
    }

    pub fn _go_back_to(&mut self, pos: usize) {
        self.position = pos;
    }

    pub fn go_back(&mut self, n: usize) {
        self.position -= n;
    }

    pub fn advance(&mut self, n: usize) {
        self.position += n;
    }

    pub fn go_to_end(&mut self) {
        self.position = self.source.len();
    }

    pub fn _next_char(&mut self) -> u8 {
        if let Some(char) = self.source.bytes().nth(self.position) {
            self.position += 1;
            char
        } else {
            0
        }
    }

    pub fn peek_char(&self, n: isize) -> u8 {
        let index = if n >= 0 {
            self.position + n as usize
        } else {
            self.position - (-n) as usize
        };
        if let Some(char) = self.source.bytes().nth(index) {
            char
        } else {
            0
        }
    }

    pub fn advance_if_char(&mut self, ch: u8) -> bool {
        if let Some(char) = self.source.bytes().nth(self.position) {
            if char == ch {
                self.position += 1;
                return true;
            }
        }
        false
    }

    pub fn advance_if_chars(&mut self, ch: Vec<u8>) -> bool {
        if self.position + ch.len() > self.source.len() {
            return false;
        }

        for i in 0..ch.len() {
            if self.source.bytes().nth(self.position + i).unwrap() != ch[i] {
                return false;
            }
        }

        self.advance(ch.len());
        true
    }

    pub fn advance_if_regexp(&mut self, regexp: Regex) -> &str {
        let haystack = &self.source[self.position..];
        if let Some(captures) = regexp.captures(haystack) {
            let m = captures.get(0).unwrap();
            self.position += m.end();
            m.as_str()
        } else {
            ""
        }
    }

    pub fn advance_until_regexp(&mut self, regexp: Regex) -> &str {
        let haystack = &self.source[self.position..];
        if let Some(captures) = regexp.captures(haystack) {
            let m = captures.get(0).unwrap();
            self.position += m.start();
            m.as_str()
        } else {
            self.go_to_end();
            ""
        }
    }

    pub fn advance_until_char(&mut self, ch: u8) -> bool {
        while self.position < self.source.len() {
            if self.source.bytes().nth(self.position).unwrap() == ch {
                return true;
            }
            self.advance(1);
        }
        false
    }

    pub fn advance_until_chars(&mut self, ch: Vec<u8>) -> bool {
        while self.position + ch.len() < self.source.len() {
            let mut same = true;
            for i in 0..ch.len() {
                if ch[i] != self.source.bytes().nth(self.position + i).unwrap() {
                    same = false;
                    break;
                }
            }
            if same {
                return true;
            }
            self.advance(1);
        }
        self.go_to_end();
        false
    }

    pub fn skip_whitespace(&mut self) -> bool {
        let n = self.advance_while_char(|ch| vec![_WSP, _TAB, _NWL, _LFD, _CAR].contains(&ch));
        n > 0
    }

    pub fn advance_while_char<F>(&mut self, condition: F) -> usize
    where
        F: Fn(u8) -> bool,
    {
        let pos_now = self.position;
        while self.position < self.source.len()
            && condition(self.source.bytes().nth(self.position).unwrap())
        {
            self.advance(1);
        }
        self.position - pos_now
    }
}

const _BNG: u8 = b'!';
const _MIN: u8 = b'-';
const _LAN: u8 = b'<';
const _RAN: u8 = b'>';
const _FSL: u8 = b'/';
const _EQS: u8 = b'=';
const _DQO: u8 = b'"';
const _SQO: u8 = b'\'';
const _NWL: u8 = b'\n';
const _CAR: u8 = b'\r';
const _LFD: u8 = 12;
const _WSP: u8 = b' ';
const _TAB: u8 = b'\t';

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
            let mut scanner = Scanner::new(&t.input, 0, scanner_state, false);
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
