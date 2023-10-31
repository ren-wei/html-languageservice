pub fn is_letter_or_digit(text: &str, index: usize) -> bool {
    let c = text.as_bytes()[index];
    (b'a' <= c && c <= b'z') || (b'A' <= c && c <= b'Z') || (b'0' <= c && c <= b'9')
}
