#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Color {
    Default,
    Red,
    Green,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Weight {
    Normal,
    Bold,
    Faint,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AnsiStyle {
    pub inverse: bool,
    pub weight: Weight,
    pub color: Color,
}

pub const ANSI_STYLE_NORMAL: AnsiStyle = AnsiStyle {
    inverse: false,
    weight: Weight::Normal,
    color: Color::Default,
};

impl AnsiStyle {
    /// Renders a (possibly empty) ANSI escape sequence to switch to this style
    /// from the before style.
    pub fn from(&self, before: &AnsiStyle) -> String {
        if self == before {
            return String::from("");
        }

        if self == &ANSI_STYLE_NORMAL {
            // Special case for resetting to default style
            return String::from("\x1b[0m");
        }

        let mut return_me = String::new();

        if self.inverse && !before.inverse {
            // Inverse on
            return_me.push_str("\x1b[7m");
        }
        if !self.inverse && before.inverse {
            // Inverse off
            return_me.push_str("\x1b[27m");
        }

        if self.weight != before.weight {
            match self.weight {
                Weight::Normal => return_me.push_str("\x1b[22m"),
                Weight::Bold => return_me.push_str("\x1b[1m"),
                Weight::Faint => return_me.push_str("\x1b[2m"),
            }
        }

        if self.color != before.color {
            match self.color {
                Color::Default => return_me.push_str("\x1b[39m"),
                Color::Red => return_me.push_str("\x1b[31m"),
                Color::Green => return_me.push_str("\x1b[32m"),
            }
        }

        return return_me;
    }
}

pub fn without_ansi_escape_codes(input: &[u8]) -> Vec<u8> {
    enum State {
        Normal,
        Escape,
        EscapeBracket,
    }

    let mut return_me = Vec::with_capacity(input.len());
    let mut state = State::Normal;

    for byte in input {
        let byte = byte.to_owned();
        match state {
            State::Normal => {
                if byte == b'\x1b' {
                    state = State::Escape;
                } else {
                    return_me.push(byte);
                }
            }
            State::Escape => {
                if byte == b'[' {
                    state = State::EscapeBracket;
                } else {
                    // Not an ANSI sequence
                    state = State::Normal;

                    // Push the characters that we thought were the escape
                    // sequence's opening
                    return_me.push(b'\x1b');
                    return_me.push(byte);
                }
            }
            State::EscapeBracket => {
                if !byte.is_ascii_digit() && byte != b';' {
                    // Neither digit nor semicolon, this marks the end of the sequence
                    state = State::Normal;
                }
            }
        }
    }

    return return_me;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    use pretty_assertions::assert_eq;

    #[test]
    fn test_non_sgr() {
        assert_eq!(without_ansi_escape_codes(b"hel\x1b[0Klo"), b"hello");
    }

    #[test]
    fn test_sgr() {
        assert_eq!(without_ansi_escape_codes(b"hel\x1b[33mlo"), b"hello");
    }

    #[test]
    fn test_multi_sgr() {
        assert_eq!(without_ansi_escape_codes(b"hel\x1b[33;34mlo"), b"hello");
    }
}
