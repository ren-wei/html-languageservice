pub fn is_letter_or_digit(text: &str, index: usize) -> bool {
    let c = text.chars().nth(index);
    if let Some(c) = c {
        c.is_ascii_lowercase() || c.is_ascii_uppercase() || c.is_ascii_digit()
    } else {
        false
    }
}
