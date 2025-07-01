use crate::ansi::AnsiStyle;
use crate::ansi::Color::Green;
use crate::ansi::Color::Red;
use crate::ansi::Weight;
use crate::ansi::ANSI_STYLE_NORMAL;
use once_cell::sync::Lazy;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Style {
    Lowlighted,          // Faint
    Context,             // Default
    Bright,              // Bold
    DiffPartUnchanged,   // Yellow
    DiffPartMidlighted,  // Red or Green
    DiffPartHighlighted, // Inverse Red or Green
    Error,               // Inverse Red
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StyledToken {
    pub(crate) token: String,
    pub(crate) style: Style,
    pub(crate) url: Option<url::Url>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LineStyle {
    pub(crate) prefix_style: AnsiStyle,
    pub(crate) unchanged_style: AnsiStyle,
    pub(crate) midlighted_style: AnsiStyle,
    pub(crate) highlighted_style: AnsiStyle,
}

// The base line styles live in refiner.rs

pub(crate) static LINE_STYLE_CONFLICT_BASE: Lazy<LineStyle> = Lazy::new(|| LineStyle {
    prefix_style: ANSI_STYLE_NORMAL.with_inverse(true),
    unchanged_style: ANSI_STYLE_NORMAL,
    midlighted_style: ANSI_STYLE_NORMAL.with_color(Red),
    highlighted_style: ANSI_STYLE_NORMAL.with_color(Red).with_inverse(true),
});

pub(crate) static LINE_STYLE_CONFLICT_OLD: Lazy<LineStyle> = Lazy::new(|| LineStyle {
    prefix_style: ANSI_STYLE_NORMAL.with_inverse(true),
    unchanged_style: ANSI_STYLE_NORMAL,
    midlighted_style: ANSI_STYLE_NORMAL.with_color(Red),
    highlighted_style: ANSI_STYLE_NORMAL.with_color(Red).with_inverse(true),
});

pub(crate) static LINE_STYLE_CONFLICT_NEW: Lazy<LineStyle> = Lazy::new(|| LineStyle {
    prefix_style: ANSI_STYLE_NORMAL.with_inverse(true),
    unchanged_style: ANSI_STYLE_NORMAL,
    midlighted_style: ANSI_STYLE_NORMAL.with_color(Green),
    highlighted_style: ANSI_STYLE_NORMAL.with_color(Green).with_inverse(true),
});

pub(crate) static LINE_STYLE_OLD_FILENAME: Lazy<LineStyle> = Lazy::new(|| LineStyle {
    prefix_style: ANSI_STYLE_NORMAL.with_weight(Weight::Bold),
    unchanged_style: ANSI_STYLE_NORMAL,
    midlighted_style: ANSI_STYLE_NORMAL.with_color(Red),
    highlighted_style: ANSI_STYLE_NORMAL.with_color(Red).with_inverse(true),
});

pub(crate) static LINE_STYLE_NEW_FILENAME: Lazy<LineStyle> = Lazy::new(|| LineStyle {
    prefix_style: ANSI_STYLE_NORMAL.with_weight(Weight::Bold),
    unchanged_style: ANSI_STYLE_NORMAL,
    midlighted_style: ANSI_STYLE_NORMAL.with_color(Green),
    highlighted_style: ANSI_STYLE_NORMAL.with_color(Green).with_inverse(true),
});

impl StyledToken {
    pub fn new(token: String, style: Style) -> StyledToken {
        if token.len() != 1 {
            return StyledToken {
                token,
                style,
                url: None,
            };
        }

        let character = token.chars().next().unwrap();
        if character >= ' ' || character == '\x09' || character == '\x0a' {
            return StyledToken {
                token,
                style,
                url: None,
            };
        }

        // This is a special character, let's replace it with its Unicode symbol
        let symbol = char::from_u32((character as u32) + 0x2400).unwrap_or(character);
        return StyledToken {
            token: symbol.to_string(),
            style,
            url: None,
        };
    }

    // Are all characters in this token whitespace?
    pub fn is_whitespace(&self) -> bool {
        return self.token.chars().all(|c| c.is_whitespace());
    }
}

#[must_use]
pub(crate) fn render_row(
    line_style: &LineStyle,
    prefix: &str,
    row: &[StyledToken],
    force_faint: bool,
) -> String {
    let mut rendered = String::new();

    let mut current_style = ANSI_STYLE_NORMAL;

    // Render prefix
    rendered.push_str(&line_style.prefix_style.from(&current_style));
    current_style = line_style.prefix_style.clone();
    rendered.push_str(prefix);

    // Render tokens
    for token in row {
        let mut new_style = match token.style {
            Style::Context => ANSI_STYLE_NORMAL,
            Style::Lowlighted => ANSI_STYLE_NORMAL.with_weight(Weight::Faint),
            Style::Bright => ANSI_STYLE_NORMAL.with_weight(Weight::Bold),
            Style::DiffPartUnchanged => line_style.unchanged_style.clone(),
            Style::DiffPartMidlighted => line_style.midlighted_style.clone(),
            Style::DiffPartHighlighted => line_style.highlighted_style.clone(),
            Style::Error => ANSI_STYLE_NORMAL.with_color(Red).with_inverse(true),
        };

        if force_faint {
            new_style = new_style.with_weight(Weight::Faint);
        }

        if let Some(url) = &token.url {
            new_style = new_style.with_url(url.clone());
        }

        rendered.push_str(&new_style.from(&current_style));
        current_style = new_style;
        rendered.push_str(&token.token);
    }

    // Reset formatting at the end of the line
    rendered.push_str(&ANSI_STYLE_NORMAL.from(&current_style));

    return rendered;
}

/// Render all the tokens into a (most of the time multiline) string
#[must_use]
pub fn render(line_style: &LineStyle, prefix: &str, tokens: &[StyledToken]) -> String {
    let mut rendered = String::new();

    let mut current_row_start = 0;
    for (i, token) in tokens.iter().enumerate() {
        if token.token == "\n" {
            let rendered_row =
                &render_row(line_style, prefix, &tokens[current_row_start..i], false);
            rendered.push_str(rendered_row);
            rendered.push('\n');
            current_row_start = i + 1;
            continue;
        }
    }

    if current_row_start < tokens.len() {
        let rendered_row = &render_row(line_style, prefix, &tokens[current_row_start..], false);
        rendered.push_str(rendered_row);
    }

    return rendered;
}

/// Render all the tokens into a (most of the time multiline) string. Each line
/// is prefixed by a prefix from `line_prefixes`.
#[must_use]
pub fn render_multiprefix(
    line_style: &LineStyle,
    line_prefixes: &[String],
    tokens: &[StyledToken],
) -> String {
    let mut rendered = String::new();

    let mut current_row_start = 0;
    let mut line_number_zero_based = 0;
    for (i, token) in tokens.iter().enumerate() {
        if token.token != "\n" {
            continue;
        }

        let prefix = &line_prefixes[line_number_zero_based];
        let force_faint = prefix.chars().any(|c| c == '-');

        let rendered_row = &render_row(
            line_style,
            prefix,
            &tokens[current_row_start..i],
            force_faint,
        );
        rendered.push_str(rendered_row);
        rendered.push('\n');
        current_row_start = i + 1;

        line_number_zero_based += 1;
    }

    if current_row_start < tokens.len() {
        // Render the last row
        let prefix = &line_prefixes[line_number_zero_based];
        let force_faint = prefix.chars().any(|c| c == '-');

        let rendered_row = &render_row(
            line_style,
            prefix,
            &tokens[current_row_start..],
            force_faint,
        );
        rendered.push_str(rendered_row);
    }

    return rendered;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ansi::Color::Yellow;
    use crate::constants::GREEN;
    use crate::constants::NORMAL;
    use crate::constants::OLD;

    #[cfg(test)]
    use pretty_assertions::assert_eq;

    static LINE_STYLE_OLD: Lazy<LineStyle> = Lazy::new(|| LineStyle {
        prefix_style: ANSI_STYLE_NORMAL.with_color(Red),
        unchanged_style: ANSI_STYLE_NORMAL.with_color(Yellow),
        midlighted_style: ANSI_STYLE_NORMAL.with_color(Red),
        highlighted_style: ANSI_STYLE_NORMAL.with_color(Red).with_inverse(true),
    });

    static LINE_STYLE_NEW: Lazy<LineStyle> = Lazy::new(|| LineStyle {
        prefix_style: ANSI_STYLE_NORMAL.with_color(Green),
        unchanged_style: ANSI_STYLE_NORMAL.with_color(Yellow),
        midlighted_style: ANSI_STYLE_NORMAL.with_color(Green),
        highlighted_style: ANSI_STYLE_NORMAL.with_color(Green).with_inverse(true),
    });

    #[test]
    fn test_basic() {
        let rendered = render(
            &*LINE_STYLE_NEW,
            "+",
            &[
                StyledToken {
                    token: "hej".to_string(),
                    style: Style::DiffPartMidlighted,
                    url: None,
                },
                StyledToken {
                    token: "\n".to_string(),
                    style: Style::DiffPartMidlighted,
                    url: None,
                },
            ],
        );
        assert_eq!(rendered, format!("{GREEN}+hej{NORMAL}\n"));
    }

    #[test]
    fn test_removed_trailing_whitespace() {
        // It shouldn't be highlighted, just added ones should
        let actual = render(
            &*LINE_STYLE_OLD,
            "-",
            &[StyledToken::new(" ".to_string(), Style::DiffPartMidlighted)],
        );

        assert_eq!(actual, format!("{OLD}- {NORMAL}"));
    }

    #[test]
    fn test_removed_nonleading_tab() {
        // It shouldn't be highlighted, just added ones should
        let actual = render(
            &*LINE_STYLE_OLD,
            "-",
            &[
                StyledToken::new("x".to_string(), Style::DiffPartMidlighted),
                StyledToken::new("\t".to_string(), Style::DiffPartMidlighted),
            ],
        );

        assert_eq!(actual, format!("{OLD}-x\t{NORMAL}"));
    }

    #[test]
    fn test_below_ascii_space() {
        assert_eq!(
            "␛",
            StyledToken::new("\x1b".to_string(), Style::Context).token
        );
        assert_eq!(
            "␇",
            StyledToken::new("\x07".to_string(), Style::Context).token
        );
    }
}
