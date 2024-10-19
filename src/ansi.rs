use crate::constants::{
    BOLD, FAINT, GREEN, INVERSE_VIDEO, NORMAL, NORMAL_INTENSITY, NO_INVERSE_VIDEO, RED, YELLOW,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Color {
    Default,
    Red,
    Green,
    Yellow,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Weight {
    Normal,
    Bold,
    Faint,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AnsiStyle {
    pub color: Color,
    pub weight: Weight,
    pub inverse: bool,
}

pub const ANSI_STYLE_NORMAL: AnsiStyle = AnsiStyle {
    color: Color::Default,
    weight: Weight::Normal,
    inverse: false,
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
            return String::from(NORMAL);
        }

        let mut return_me = String::new();

        if self.inverse && !before.inverse {
            // Inverse on
            return_me.push_str(INVERSE_VIDEO);
        }
        if !self.inverse && before.inverse {
            // Inverse off
            return_me.push_str(NO_INVERSE_VIDEO);
        }

        if self.weight != before.weight {
            if before.weight != Weight::Normal {
                // Turn off bold or faint
                return_me.push_str(NORMAL_INTENSITY);
            }
            if self.weight == Weight::Faint {
                return_me.push_str(FAINT);
            }
            if self.weight == Weight::Bold {
                return_me.push_str(BOLD);
            }
        }

        if self.color != before.color {
            match self.color {
                Color::Default => return_me.push_str("\x1b[39m"),
                Color::Red => return_me.push_str(RED),
                Color::Green => return_me.push_str(GREEN),
                Color::Yellow => return_me.push_str(YELLOW),
            }
        }

        return return_me;
    }

    pub const fn with_color(&self, color: Color) -> AnsiStyle {
        return AnsiStyle {
            color,
            weight: self.weight,
            inverse: self.inverse,
        };
    }

    pub const fn with_inverse(&self, inverse: bool) -> AnsiStyle {
        return AnsiStyle {
            color: self.color,
            weight: self.weight,
            inverse,
        };
    }

    pub const fn with_weight(&self, weight: Weight) -> AnsiStyle {
        return AnsiStyle {
            color: self.color,
            weight,
            inverse: self.inverse,
        };
    }
}

// Modifies the input so that all ANSI escape codes are removed
#[must_use]
pub fn without_ansi_escape_codes(line: &[u8]) -> Vec<u8> {
    enum State {
        Normal,
        Escape,
        EscapeBracket,
    }

    let mut state = State::Normal;
    let mut without_ansi = Vec::with_capacity(line.len());

    for byte in line {
        match state {
            State::Normal => {
                if *byte == b'\x1b' {
                    state = State::Escape;
                } else {
                    without_ansi.push(*byte);
                }
            }
            State::Escape => {
                if *byte == b'[' {
                    state = State::EscapeBracket;
                } else {
                    // Not an ANSI sequence
                    state = State::Normal;

                    // Push the characters that we thought were the escape
                    // sequence's opening
                    without_ansi.push(b'\x1b');
                    without_ansi.push(*byte);
                }
            }
            State::EscapeBracket => {
                if !byte.is_ascii_digit() && *byte != b';' {
                    // Neither digit nor semicolon, this marks the end of the sequence
                    state = State::Normal;
                }
            }
        }
    }

    return without_ansi;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    use pretty_assertions::assert_eq;

    #[test]
    fn test_non_sgr() {
        let line = b"hel\x1b[0Klo".to_vec();
        assert_eq!(without_ansi_escape_codes(&line), b"hello");
    }

    #[test]
    fn test_sgr() {
        let line = b"hel\x1b[33mlo".to_vec();
        assert_eq!(without_ansi_escape_codes(&line), b"hello");
    }

    #[test]
    fn test_multi_sgr() {
        let line = b"hel\x1b[33;34mlo".to_vec();
        assert_eq!(without_ansi_escape_codes(&line), b"hello");
    }
}
