// FIXME: The doctest doesn't run on `cargo test`, why?
/// Splits string into a vector of words. A word is any sequence of alphanumeric
/// characters. Non-words get into the vector one and one.
///
/// ```rust
/// assert_eq!(tokenize("Adam, Bea"), ["Adam", ",", " ", "Bea"]);
/// ```
pub fn tokenize(input: &str) -> Vec<&str> {
    let mut run_start_byte_index: Option<usize> = None;
    let mut run_start_char: Option<char> = None;
    let mut result: Vec<&str> = Vec::with_capacity(input.len());
    let mut byte_index = 0;
    for character in input.chars() {
        let mut still_in_run = false;
        if let (Some(start_index), Some(start_char)) = (run_start_byte_index, run_start_char) {
            let current_is_word_part = (character == '_') || character.is_alphanumeric();
            let last_was_word_part = (start_char == '_') || start_char.is_alphanumeric();
            if current_is_word_part && last_was_word_part {
                still_in_run = true;
            } else if start_char.is_whitespace() && start_char != '\n' && character == start_char {
                still_in_run = true;
            } else {
                run_start_byte_index = None;
                run_start_char = None;

                // Push the token that just ended
                result.push(&input[start_index..byte_index]);
            }
        }

        if !still_in_run {
            // Start a new run?
            if (character == '_')
                || character.is_alphanumeric()
                || (character.is_whitespace() && character != '\n')
            {
                run_start_byte_index = Some(byte_index);
                run_start_char = Some(character);
                still_in_run = true;
            }
        }

        if !still_in_run {
            // Push the current character
            result.push(&input[byte_index..(byte_index + character.len_utf8())]);
        }

        byte_index += character.len_utf8();
    }

    if let Some(run_start) = run_start_byte_index {
        // Push the ending token
        result.push(&input[run_start..]);
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
        assert_eq!(tokenize("\u{00a0}s"), ["\u{00a0}", "s"]);
    }

    #[test]
    fn test_leading_whitespace() {
        assert_eq!(tokenize("  word"), ["  ", "word"]);
        assert_eq!(tokenize("  \t  word"), ["  ", "\t", "  ", "word"]);
        assert_eq!(tokenize("\t\t\tword"), ["\t\t\t", "word"]);
    }

    #[test]
    fn test_consecutive_newlines() {
        assert_eq!(tokenize("\n\n\n"), ["\n", "\n", "\n"]);
    }
}
