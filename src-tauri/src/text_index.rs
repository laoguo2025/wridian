use std::collections::{HashMap, HashSet};

pub(crate) fn tokenize_mixed(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let mut tokens = Vec::new();
    for token in lower.split(|ch: char| !ch.is_alphanumeric() && ch != '_') {
        if token.chars().count() > 1 {
            tokens.push(token.to_string());
        }
    }
    let cjk = text
        .chars()
        .filter(|ch| ('\u{4e00}'..='\u{9fff}').contains(ch))
        .collect::<Vec<_>>();
    for window in cjk.windows(2) {
        tokens.push(window.iter().collect());
    }
    tokens
}

pub(crate) fn tokenize_mixed_set(text: &str) -> HashSet<String> {
    tokenize_mixed(text).into_iter().collect()
}

pub(crate) fn term_frequency(tokens: Vec<String>) -> HashMap<String, usize> {
    let mut values = HashMap::new();
    for token in tokens {
        *values.entry(token).or_default() += 1;
    }
    values
}
