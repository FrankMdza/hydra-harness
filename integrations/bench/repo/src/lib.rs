//! Count words and lines in a string.

/// Number of whitespace-separated words.
pub fn words(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Number of lines. An empty string is zero lines.
pub fn lines(text: &str) -> usize {
    if text.is_empty() {
        0
    } else {
        text.lines().count()
    }
}

/// Add one, wrapping at 255.
pub fn bump(n: u8) -> u8 {
    n.wrapping_add(1)
}

#[cfg(test)]
mod tests {
    use super::{bump, lines, words};

    #[test]
    fn words_counts_split_tokens() {
        assert_eq!(words(""), 0);
        assert_eq!(words("  "), 0);
        assert_eq!(words("one"), 1);
    }

    #[test]
    fn lines_treats_empty_as_zero() {
        assert_eq!(lines(""), 0);
        assert_eq!(lines("a\nb"), 2);
    }

    #[test]
    fn bump_adds_one() {
        assert_eq!(bump(0), 1);
        assert_eq!(bump(1), 2);
    }
}
