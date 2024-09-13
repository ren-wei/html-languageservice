#[cfg(any(feature = "completion", feature = "hover"))]
pub fn is_letter_or_digit(text: &str, index: usize) -> bool {
    use regex::Regex;

    let c = text.get(index..index + 1);
    c.is_some_and(|c| Regex::new("^[A-Za-z0-9]+$").unwrap().is_match(c))
}
