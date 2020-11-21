use crate::constants;
use diffus::{edit, Diffable, Same};
use std::cmp::Eq;
use std::cmp::PartialEq;
use std::fmt::Debug;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Style {
    Plain,
    Add,
    AddInverse,
    Remove,
    RemoveInverse,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StyledToken {
    token: String,
    style: Style,
}

impl StyledToken {
    pub fn styled_str(token: &str, style: Style) -> Self {
        return StyledToken {
            token: token.to_string(),
            style,
        };
    }

    pub fn from_str(token: &str) -> Self {
        return StyledToken {
            token: token.to_string(),
            style: Style::Plain,
        };
    }

    pub fn from_char(token: char) -> Self {
        return StyledToken {
            token: token.to_string(),
            style: Style::Plain,
        };
    }

    pub fn styled_newline(style: Style) -> Self {
        return StyledToken {
            token: "⏎".to_string(),
            style,
        };
    }

    pub fn set_style(self: &mut StyledToken, style: Style) {
        self.style = style;
    }

    #[cfg(test)]
    pub fn get_style(self: &StyledToken) -> &Style {
        return &self.style;
    }

    pub fn token(self: &StyledToken) -> &str {
        return &self.token;
    }
}

impl<'a> Diffable<'a> for StyledToken {
    type Diff = <str as Diffable<'a>>::Diff;

    fn diff(&'a self, other: &'a Self) -> edit::Edit<Self> {
        match self.token.diff(&other.token) {
            edit::Edit::Change(diff) => edit::Edit::Change(diff),
            edit::Edit::Copy(_) => edit::Edit::Copy(self),
        }
    }
}

// FIXME: Can we use diffus::same::same_for_eq!() instead of this?
impl Same for StyledToken {
    fn same(&self, other: &Self) -> bool {
        return self == other;
    }
}

impl Style {
    fn is_inverse(&self) -> bool {
        if self == &Style::AddInverse {
            return true;
        }
        if self == &Style::RemoveInverse {
            return true;
        }
        if self == &Style::Error {
            return true;
        }
        return false;
    }

    fn color_code(&self) -> &str {
        match self {
            Style::Plain => {
                return constants::NORMAL;
            }
            Style::Add => {
                return constants::NEW;
            }
            Style::AddInverse => {
                return constants::NEW;
            }
            Style::Remove => {
                return constants::OLD;
            }
            Style::RemoveInverse => {
                return constants::OLD;
            }
            Style::Error => {
                return constants::ERROR;
            }
        }
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
                let styled_token = StyledToken::from_str(&word);
                result.push(styled_token);
            }

            // Push current char
            let styled_token = StyledToken::from_char(character);
            result.push(styled_token);
        }

        last_was_alphanumeric = current_is_alphanumeric;
        byte_index += character.len_utf8();
    }

    if last_was_alphanumeric {
        // Push the ending word
        let word = input[first_alphanumeric_byte_index..].to_string();
        let styled_token = StyledToken::from_str(&word);
        result.push(styled_token);
    }

    return result;
}

pub fn to_string_with_line_prefix(line_prefix: &StyledToken, tokens: &[StyledToken]) -> String {
    let mut return_me = String::new();
    let mut is_inverse = false;
    let mut color_code = constants::NORMAL;
    let mut want_prefix = true;
    for token in tokens {
        if want_prefix {
            return_me += line_prefix.style.color_code();
            return_me += &line_prefix.token;
            color_code = line_prefix.style.color_code();
            want_prefix = false;
        }

        if token.token() == "\n" {
            return_me += constants::NORMAL;
            return_me += "\n";
            is_inverse = false;
            color_code = constants::NORMAL;
            want_prefix = true;
            continue;
        }

        if token.style.color_code() != color_code {
            return_me += token.style.color_code();
            color_code = token.style.color_code();
        }

        if token.style.is_inverse() && !is_inverse {
            return_me += constants::INVERSE_VIDEO;
            is_inverse = true;
        }

        if is_inverse && !token.style.is_inverse() {
            return_me += constants::NOT_INVERSE_VIDEO;
            is_inverse = false;
        }

        return_me += &token.token;
    }

    return return_me;
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
        assert_eq!(tokenize("word"), &[StyledToken::from_str("word")]);
        assert_eq!(
            tokenize("Adam Bea"),
            &[
                StyledToken::from_str("Adam"),
                StyledToken::from_str(" "),
                StyledToken::from_str("Bea")
            ]
        );
    }

    #[test]
    fn test_numbers() {
        assert_eq!(tokenize("123"), &[StyledToken::from_str("123")]);
        assert_eq!(
            tokenize("123 456"),
            &[
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
            &[StyledToken::from_str("0xC0deCafe")]
        );
    }

    #[test]
    fn test_others() {
        assert_eq!(
            tokenize("+!,"),
            &[
                StyledToken::from_str("+"),
                StyledToken::from_str("!"),
                StyledToken::from_str(",")
            ]
        );
    }

    #[test]
    fn test_non_breaking_space() {
        assert_eq!(tokenize("\u{00a0}"), &[StyledToken::from_str("\u{00a0}")]);
        assert_eq!(
            tokenize("\u{00a0}s"),
            &[
                StyledToken::from_str("\u{00a0}"),
                StyledToken::from_str("s")
            ]
        );
    }

    #[test]
    fn test_to_string_with_line_prefix() {
        assert_eq!(
            to_string_with_line_prefix(
                &StyledToken::styled_str("+", Style::Add),
                &[
                    StyledToken::styled_str("normal", Style::Add),
                    StyledToken::styled_str("highlighted", Style::AddInverse),
                    StyledToken::styled_str("error", Style::Error),
                    StyledToken::styled_str("\n", Style::Add),
                ]
            ),
            format!(
                "{}+normal{}highlighted{}error{}\n",
                constants::NEW,
                constants::INVERSE_VIDEO,
                constants::ERROR,
                constants::NORMAL
            )
        );
    }
}
