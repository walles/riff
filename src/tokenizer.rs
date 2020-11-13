// FIXME: The doctest doesn't run on `cargo test`, why?
/// Splits string into a vector of words. A word is any sequence of alphanumeric
/// characters. Non-words get into the vector one and one.
///
/// ```rust
/// assert_eq!(tokenize("Adam, Bea"), ["Adam", ", ", " ", "Bea"]);
/// ```
pub fn tokenize(input: &str) -> Vec<String> {
    let mut first_alphanumeric_index = 0;
    let mut last_was_alphanumeric = false;
    let mut result: Vec<String> = Vec::new();
    for (index, character) in input.chars().enumerate() {
        let current_is_alphanumeric = character.is_alphanumeric();
        if current_is_alphanumeric {
            if last_was_alphanumeric {
                // Do nothing, keep working on the current word.
            } else {
                // Last was something else, we just started a new word.
                first_alphanumeric_index = index;
            }
        } else {
            // Current is something else
            if last_was_alphanumeric {
                // Push the word that just ended
                result.push(input[first_alphanumeric_index..index].to_string());
            }

            // Push current char
            result.push(character.to_string());
        }

        last_was_alphanumeric = current_is_alphanumeric;
    }

    if last_was_alphanumeric {
        // Push the ending word
        result.push(input[first_alphanumeric_index..].to_string());
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
        let no_strings: Vec<String> = Vec::new();
        assert_eq!(tokenize(""), no_strings);
    }

    #[test]
    fn test_words() {
        assert_eq!(tokenize("word"), ["word"]);
        assert_eq!(tokenize("Adam Bea"), ["Adam", " ", "Bea"]);
    }

    #[test]
    fn test_numbers() {
        assert_eq!(tokenize("123"), ["123"]);
        assert_eq!(tokenize("123 456"), ["123", " ", "456"]);
    }

    #[test]
    fn test_alphanumeric() {
        assert_eq!(tokenize("0xC0deCafe"), ["0xC0deCafe"]);
    }

    #[test]
    fn test_others() {
        assert_eq!(tokenize("+!,"), ["+", "!", ","]);
    }

    #[test]
    fn test_non_breaking_space() {
        assert_eq!(tokenize("\u{00a0}"), ["\u{00a0}"]);
    }
}
