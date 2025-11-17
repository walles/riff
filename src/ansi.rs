use crate::constants::{
    BOLD, DEFAULT_COLOR, FAINT, GREEN, INVERSE_VIDEO, NORMAL, NORMAL_INTENSITY, NO_INVERSE_VIDEO,
    RED, YELLOW,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnsiStyle {
    pub(crate) color: Color,
    pub(crate) weight: Weight,
    pub(crate) inverse: bool,
    pub(crate) url: Option<url::Url>,
}

pub const ANSI_STYLE_NORMAL: AnsiStyle = AnsiStyle {
    color: Color::Default,
    weight: Weight::Normal,
    inverse: false,
    url: None,
};

impl AnsiStyle {
    fn should_add_url(new_url: &Option<url::Url>, old_url: &Option<url::Url>) -> bool {
        match (new_url, old_url) {
            (Some(_), None) => true,              // Adding a URL
            (None, Some(_)) => true,              // Removing a URL
            (Some(new), Some(old)) => new != old, // Changing the URL
            _ => false,                           // No change in URL
        }
    }

    fn normal_except_hyperlink(&self) -> bool {
        self.color == Color::Default && self.weight == Weight::Normal && !self.inverse
    }

    /// Renders a (possibly empty) ANSI escape sequence to switch to this style
    /// from the before style.
    pub fn from(&self, before: &AnsiStyle) -> String {
        if self == before {
            return String::from("");
        }

        let mut return_me = String::new();

        if AnsiStyle::should_add_url(&self.url, &before.url) {
            if let Some(url) = &self.url {
                // Write the updated URL
                return_me.push_str(&format!("\x1b]8;;{url}\x1b\\"));
            } else {
                // Remove the URL
                return_me.push_str("\x1b]8;;\x1b\\");
            }
        }

        if self.normal_except_hyperlink() && !before.normal_except_hyperlink() {
            // Special case for resetting to default style
            return_me.push_str(NORMAL);
            return return_me;
        }

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
                Color::Default => return_me.push_str(DEFAULT_COLOR),
                Color::Red => return_me.push_str(RED),
                Color::Green => return_me.push_str(GREEN),
                Color::Yellow => return_me.push_str(YELLOW),
            }
        }

        return return_me;
    }

    pub fn with_color(&self, color: Color) -> AnsiStyle {
        return AnsiStyle {
            color,
            weight: self.weight,
            inverse: self.inverse,
            url: self.url.clone(),
        };
    }

    pub fn with_inverse(&self, inverse: bool) -> AnsiStyle {
        return AnsiStyle {
            color: self.color,
            weight: self.weight,
            inverse,
            url: self.url.clone(),
        };
    }

    pub fn with_weight(&self, weight: Weight) -> AnsiStyle {
        return AnsiStyle {
            color: self.color,
            weight,
            inverse: self.inverse,
            url: self.url.clone(),
        };
    }

    pub fn with_url(&self, url: url::Url) -> AnsiStyle {
        return AnsiStyle {
            color: self.color,
            weight: self.weight,
            inverse: self.inverse,
            url: Some(url),
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
        Osc,
        OscSequence,
    }

    let mut state = State::Normal;
    let mut without_ansi = Vec::with_capacity(line.len());
    let mut i = 0;
    while i < line.len() {
        let byte = line[i];
        match state {
            State::Normal => {
                if byte == b'\x1b' {
                    state = State::Escape;
                } else {
                    without_ansi.push(byte);
                }
            }
            State::Escape => {
                if byte == b'[' {
                    state = State::EscapeBracket;
                } else if byte == b']' {
                    state = State::Osc;
                } else {
                    // Not an ANSI sequence
                    state = State::Normal;
                    without_ansi.push(b'\x1b');
                    without_ansi.push(byte);
                }
            }
            State::EscapeBracket => {
                if !byte.is_ascii_digit() && byte != b';' {
                    // End of SGR sequence
                    state = State::Normal;
                }
            }
            State::Osc => {
                // OSC sequence, look for the next '\x1b\\' (ST)
                // OSC 8 is hyperlink, but we strip all OSC for now
                // Find the next '\x1b' followed by '\\'
                // We are at the ']' after '\x1b', so skip until we see '\x1b\\'
                state = State::OscSequence;
            }
            State::OscSequence => {
                // Look for '\x1b\\' (ST)
                if byte == b'\x1b' && i + 1 < line.len() && line[i + 1] == b'\\' {
                    // End of OSC sequence
                    state = State::Normal;
                    i += 1; // Skip the '\\' as well
                }
                // Otherwise, keep skipping
            }
        }
        i += 1;
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

    #[test]
    fn test_strip_hyperlink() {
        // OSC 8 hyperlink: \x1b]8;;https://example.com\x1b\\
        let line = b"foo\x1b]8;;https://example.com\x1b\\bar".to_vec();
        // Expect "foobar" after removing OSC 8 hyperlink
        assert_eq!(without_ansi_escape_codes(&line), b"foobar");

        // OSC 8 hyperlink removal: \x1b]8;;\x1b\\
        let line2 = b"foo\x1b]8;;\x1b\\bar".to_vec();
        assert_eq!(without_ansi_escape_codes(&line2), b"foobar");
    }

    #[test]
    fn test_from_hyperlinked() {
        use url::Url;

        let url = Url::parse("https://example.com").unwrap();
        let hyperlink = AnsiStyle {
            color: Color::Default,
            weight: Weight::Normal,
            inverse: false,
            url: Some(url.clone()),
        };

        // Plain to hyperlink: should emit OSC 8 with URL. Note that the URL has
        // been normalized with a trailing slash, this is done by the URL
        // library.
        let seq1 = hyperlink.from(&ANSI_STYLE_NORMAL);
        assert_eq!(seq1, "\x1b]8;;https://example.com/\x1b\\");

        // Hyperlink to plain: should emit OSC 8 with empty URL (removes link)
        let seq2 = ANSI_STYLE_NORMAL.from(&hyperlink);
        assert_eq!(seq2, "\x1b]8;;\x1b\\");
    }

    #[test]
    fn test_to_normal_from_all_attributes_hyperlink_and_non_hyperlink() {
        use url::Url;

        let url = Url::parse("https://example.com").unwrap();

        // Color
        let red = AnsiStyle {
            color: Color::Red,
            weight: Weight::Normal,
            inverse: false,
            url: None,
        };
        let red_link = AnsiStyle {
            color: Color::Red,
            weight: Weight::Normal,
            inverse: false,
            url: Some(url.clone()),
        };
        assert_eq!(ANSI_STYLE_NORMAL.from(&red), "\x1b[0m");
        assert_eq!(ANSI_STYLE_NORMAL.from(&red_link), "\x1b]8;;\x1b\\\x1b[0m");

        // Weight
        let bold = AnsiStyle {
            color: Color::Default,
            weight: Weight::Bold,
            inverse: false,
            url: None,
        };
        let bold_link = AnsiStyle {
            color: Color::Default,
            weight: Weight::Bold,
            inverse: false,
            url: Some(url.clone()),
        };
        assert_eq!(ANSI_STYLE_NORMAL.from(&bold), "\x1b[0m");
        assert_eq!(ANSI_STYLE_NORMAL.from(&bold_link), "\x1b]8;;\x1b\\\x1b[0m");

        // Inverse
        let inverse = AnsiStyle {
            color: Color::Default,
            weight: Weight::Normal,
            inverse: true,
            url: None,
        };
        let inverse_link = AnsiStyle {
            color: Color::Default,
            weight: Weight::Normal,
            inverse: true,
            url: Some(url),
        };
        assert_eq!(ANSI_STYLE_NORMAL.from(&inverse), "\x1b[0m");
        assert_eq!(
            ANSI_STYLE_NORMAL.from(&inverse_link),
            "\x1b]8;;\x1b\\\x1b[0m"
        );
    }
}
