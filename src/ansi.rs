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

// Modifies the input so that all ANSI escape codes are removed
pub fn remove_ansi_escape_codes(line: &mut Vec<u8>) {
    enum State {
        Normal,
        Escape,
        EscapeBracket,
    }

    let mut state = State::Normal;
    let mut next_index_without_ansi = 0usize;

    for index in 0..line.len() {
        let byte = line[index];
        match state {
            State::Normal => {
                if byte == b'\x1b' {
                    state = State::Escape;
                } else {
                    line[next_index_without_ansi] = byte;
                    next_index_without_ansi += 1;
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
                    line[next_index_without_ansi] = b'\x1b';
                    next_index_without_ansi += 1;
                    line[next_index_without_ansi] = byte;
                    next_index_without_ansi += 1;
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

    line.truncate(next_index_without_ansi);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    use pretty_assertions::assert_eq;

    #[test]
    fn test_non_sgr() {
        let mut line = b"hel\x1b[0Klo".to_vec();
        remove_ansi_escape_codes(&mut line);
        assert_eq!(line, b"hello");
    }

    #[test]
    fn test_sgr() {
        let mut line = b"hel\x1b[33mlo".to_vec();
        remove_ansi_escape_codes(&mut line);
        assert_eq!(line, b"hello");
    }

    #[test]
    fn test_multi_sgr() {
        let mut line = b"hel\x1b[33;34mlo".to_vec();
        remove_ansi_escape_codes(&mut line);
        assert_eq!(line, b"hello");
    }
}
