use std::cmp::Eq;
use std::cmp::PartialEq;
use std::fmt::Debug;

#[derive(Debug, PartialEq, Eq)]
enum Style {
    Plain,
    Add,
    AddInverse,
    Remove,
    RemoveInverse,
    Error,
}

#[derive(Debug, PartialEq, Eq)]
struct StyledToken {
    token: String,
    style: Style,
}

impl StyledToken {
    fn from_str(token: &str) -> Self {
        return StyledToken {
            token: token.to_string(),
            style: Style::Plain,
        };
    }

    fn from_char(token: char) -> Self {
        return StyledToken {
            token: token.to_string(),
            style: Style::Plain,
        };
    }
}

// FIXME: The doctest doesn't run on `cargo test`, why?
/// Splits string into a vector of words. A word is any sequence of alphanumeric
/// characters. Non-words get into the vector one and one.
///
/// ```rust
/// assert_eq!(tokenize("Adam, Bea"), ["Adam", ", ", " ", "Bea"]);
/// ```
pub fn tokenize(input: &str) -> Vec<StyledToken> {
    let mut first_alphanumeric_byte_index = 0;
    let mut last_was_alphanumeric = false;
    let mut result: Vec<StyledToken> = Vec::new();
    let mut byte_index = 0;
    for character in input.chars() {
        let current_is_alphanumeric = character.is_alphanumeric();
        if current_is_alphanumeric {
            if last_was_alphanumeric {
                // Do nothing, keep working on the current word.
            } else {
                // Last was something else, we just started a new word.
                first_alphanumeric_byte_index = byte_index;
            }
        } else {
            // Current is something else
            if last_was_alphanumeric {
                // Push the word that just ended
                let word = input[first_alphanumeric_byte_index..byte_index].to_string();
                result.push(StyledToken::from_str(&word));
            }

            // Push current char
            result.push(StyledToken::from_char(character));
        }

        last_was_alphanumeric = current_is_alphanumeric;
        byte_index += character.len_utf8();
    }

    if last_was_alphanumeric {
        // Push the ending word
        let word = input[first_alphanumeric_byte_index..].to_string();
        result.push(StyledToken::from_str(&word));
    }

    return result;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    use pretty_assertions::assert_eq;

    #[test]
    fn test_empty() {
        let no_strings: Vec<StyledToken> = Vec::new();
        assert_eq!(tokenize(""), no_strings);
    }

    #[test]
    fn test_words() {
        assert_eq!(tokenize("word"), [StyledToken::from_str("word")]);
        assert_eq!(
            tokenize("Adam Bea"),
            [
                StyledToken::from_str("Adam"),
                StyledToken::from_str(" "),
                StyledToken::from_str("Bea")
            ]
        );
    }

    #[test]
    fn test_numbers() {
        assert_eq!(tokenize("123"), [StyledToken::from_str("123")]);
        assert_eq!(
            tokenize("123 456"),
            [
                StyledToken::from_str("123"),
                StyledToken::from_str(" "),
                StyledToken::from_str("456")
            ]
        );
    }

    #[test]
    fn test_alphanumeric() {
        assert_eq!(
            tokenize("0xC0deCafe"),
            [StyledToken::from_str("0xC0deCafe")]
        );
    }

    #[test]
    fn test_others() {
        assert_eq!(
            tokenize("+!,"),
            [
                StyledToken::from_str("+"),
                StyledToken::from_str("!"),
                StyledToken::from_str(",")
            ]
        );
    }

    #[test]
    fn test_non_breaking_space() {
        assert_eq!(tokenize("\u{00a0}"), [StyledToken::from_str("\u{00a0}")]);
        assert_eq!(
            tokenize("\u{00a0}s"),
            [
                StyledToken::from_str("\u{00a0}"),
                StyledToken::from_str("s")
            ]
        );
    }
}
