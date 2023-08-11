use crate::ansi::AnsiStyle;
use crate::ansi::Color::Default;
use crate::ansi::Color::Green;
use crate::ansi::Color::Red;
use crate::token_collector::Style::Highlighted;
use crate::token_collector::Style::Plain;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Style {
    Plain,
    Highlighted,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StyledToken {
    token: String,
    style: Style,
}

#[derive(Debug, PartialEq, Eq)]
pub struct LineStyle<'a> {
    prefix: &'a str,
    prefix_style: AnsiStyle,
    plain_style: AnsiStyle,
    highlighted_style: AnsiStyle,
}

pub const LINE_STYLE_OLD: LineStyle = {
    LineStyle {
        prefix: "-",
        prefix_style: AnsiStyle {
            inverse: false,
            color: Red,
        },
        plain_style: AnsiStyle {
            inverse: false,
            color: Red,
        },
        highlighted_style: AnsiStyle {
            inverse: true,
            color: Red,
        },
    }
};

pub const LINE_STYLE_NEW: LineStyle = {
    LineStyle {
        prefix: "+",
        prefix_style: AnsiStyle {
            inverse: false,
            color: Green,
        },
        plain_style: AnsiStyle {
            inverse: false,
            color: Green,
        },
        highlighted_style: AnsiStyle {
            inverse: true,
            color: Green,
        },
    }
};

impl StyledToken {
    pub fn new(token: String, style: Style) -> StyledToken {
        return StyledToken { token, style };
    }

    pub fn is_whitespace(&self) -> bool {
        let mut chars_iterator = self.token.chars();
        let first_char = chars_iterator.next().unwrap();
        if chars_iterator.next().is_some() {
            // Multiple chars found in this token, but whitespace will only be
            // one per token.
            return false;
        }

        return first_char.is_whitespace();
    }
}

#[must_use]
fn render_row(line_style: &LineStyle, row: &mut [StyledToken]) -> String {
    let mut rendered = String::new();

    let mut current_style = AnsiStyle {
        inverse: false,
        color: Default,
    };

    // Render prefix
    rendered.push_str(&line_style.prefix_style.from(&current_style));
    current_style = line_style.prefix_style;
    rendered.push_str(line_style.prefix);

    // Render tokens
    for token in row {
        let new_style = match token.style {
            Plain => line_style.plain_style,
            Highlighted => line_style.highlighted_style,
            Style::Error => AnsiStyle {
                inverse: true,
                color: Red,
            },
        };

        rendered.push_str(&new_style.from(&current_style));
        current_style = new_style;
        rendered.push_str(&token.token);
    }

    // Reset formatting at the end of the line
    rendered.push_str(
        &AnsiStyle {
            inverse: false,
            color: Default,
        }
        .from(&current_style),
    );

    return rendered;
}

/// Render all the tokens into a (most of the time multiline) string
#[must_use]
pub fn render(line_style: &LineStyle, tokens: Vec<StyledToken>) -> String {
    let mut current_row: Vec<StyledToken> = Vec::new();
    let mut rendered = String::new();

    for token in tokens {
        if token.token == "\n" {
            let rendered_row = &render_row(line_style, &mut current_row);
            rendered.push_str(rendered_row);
            rendered.push('\n');
            current_row.clear();
            continue;
        }

        current_row.push(token.clone());
    }

    if !current_row.is_empty() {
        let rendered_row = &render_row(line_style, &mut current_row);
        rendered.push_str(rendered_row);
    }

    return rendered;
}

/// Unhighlight rows that have too much highlighting.
///
/// Returns true if something was unhighlighted, false otherwise.
pub fn unhighlight_noisy_rows(tokens: &mut [StyledToken]) -> bool {
    let mut highlighted_tokens_count = 0;
    let mut total_tokens_count: usize = 0;
    let mut line_start_index = 0;
    let mut changed = false;

    for i in 0..tokens.len() {
        let token = &tokens[i];
        if token.token == "\n" {
            // End of line, evaluate!
            if total_tokens_count > 0 {
                let highlighted_percentage = (100 * highlighted_tokens_count) / total_tokens_count;
                if highlighted_percentage > 70 {
                    // Unhighlight the current row
                    changed = true;
                    for token in tokens[line_start_index..i].iter_mut() {
                        token.style = Plain;
                    }
                }
            }

            // Reset for the next row
            line_start_index = i + 1;
            highlighted_tokens_count = 0;
            total_tokens_count = 0;
            continue;
        }

        total_tokens_count += 1;
        if token.style == Highlighted {
            highlighted_tokens_count += 1;
        }
    }

    // Handle the last row
    if total_tokens_count > 0 {
        let highlighted_percentage = (100 * highlighted_tokens_count) / total_tokens_count;
        if highlighted_percentage > 70 {
            // Unhighlight the current row
            changed = true;
            for token in tokens[line_start_index..].iter_mut() {
                token.style = Plain;
            }
        }
    }

    return changed;
}

pub fn highlight_trailing_whitespace(tokens: &mut [StyledToken]) {
    let mut in_trailer = true;
    for token in tokens.iter_mut().rev() {
        if token.token == "\n" {
            in_trailer = true;
            continue;
        }

        if in_trailer && token.is_whitespace() {
            token.style = Style::Error;
            continue;
        }

        in_trailer = false;
    }
}

pub fn highlight_nonleading_tabs(tokens: &mut [StyledToken]) {
    let mut leading = true;
    for token in tokens.iter_mut() {
        if token.token == "\n" {
            leading = true;
            continue;
        }

        if !leading && token.token == "\t" {
            token.style = Style::Error;
            continue;
        }

        leading = false;
    }
}

/// Highlight single space between two highlighted tokens
pub fn bridge_consecutive_highlighted_tokens(tokens: &mut [StyledToken]) {
    enum FoundState {
        Nothing,
        HighlightedWord,
        WordSpace,
    }

    let mut found_state = FoundState::Nothing;
    let mut previous_token: Option<&mut StyledToken> = None;
    for token in tokens.iter_mut() {
        match found_state {
            FoundState::Nothing => {
                if token.style == Highlighted {
                    // Found "Monkey"
                    found_state = FoundState::HighlightedWord;
                }
            }

            FoundState::HighlightedWord => {
                if token.token.len() == 1 {
                    // Found "Monkey " (note trailing space)
                    found_state = FoundState::WordSpace;
                } else if token.style == Highlighted {
                    found_state = FoundState::HighlightedWord;
                } else {
                    found_state = FoundState::Nothing;
                }
            }

            FoundState::WordSpace => {
                if token.style == Highlighted {
                    // Found "Monkey Dance"
                    if let Some(whitespace) = previous_token {
                        whitespace.style = Highlighted;
                    }

                    found_state = FoundState::HighlightedWord;
                } else {
                    found_state = FoundState::Nothing;
                }
            }
        }

        previous_token = Some(token);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::NEW;
    use crate::constants::NORMAL;
    use crate::constants::OLD;

    #[cfg(test)]
    use pretty_assertions::assert_eq;

    #[test]
    fn test_basic() {
        let rendered = render(
            &LINE_STYLE_NEW,
            vec![
                StyledToken {
                    token: "hej".to_string(),
                    style: Style::Plain,
                },
                StyledToken {
                    token: "\n".to_string(),
                    style: Style::Plain,
                },
            ],
        );
        assert_eq!(rendered, format!("{NEW}+hej{NORMAL}\n"));
    }

    #[test]
    fn test_add_trailing_whitespace() {
        // Just a whitespace
        let mut row = [StyledToken::new(" ".to_string(), Style::Plain)];
        highlight_trailing_whitespace(&mut row);
        assert_eq!(row, [StyledToken::new(" ".to_string(), Style::Error)]);

        // Trailing whitespace
        let mut row = [
            StyledToken::new("x".to_string(), Style::Plain),
            StyledToken::new(" ".to_string(), Style::Plain),
        ];
        highlight_trailing_whitespace(&mut row);
        assert_eq!(
            row,
            [
                StyledToken::new("x".to_string(), Style::Plain),
                StyledToken::new(" ".to_string(), Style::Error),
            ]
        );

        // Leading whitespace
        let mut row = [
            StyledToken::new(" ".to_string(), Style::Plain),
            StyledToken::new("x".to_string(), Style::Plain),
        ];
        highlight_trailing_whitespace(&mut row);
        assert_eq!(
            row,
            [
                StyledToken::new(" ".to_string(), Style::Plain),
                StyledToken::new("x".to_string(), Style::Plain),
            ]
        );
    }

    #[test]
    fn test_removed_trailing_whitespace() {
        // It shouldn't be highlighted, just added ones should
        let actual = render(
            &LINE_STYLE_OLD,
            vec![StyledToken::new(" ".to_string(), Style::Plain)],
        );

        assert_eq!(actual, format!("{OLD}- {NORMAL}"));
    }

    #[test]
    fn test_add_nonleading_tab() {
        // Trailing TAB
        let mut row = [
            StyledToken::new("x".to_string(), Style::Plain),
            StyledToken::new("\t".to_string(), Style::Plain),
        ];
        highlight_nonleading_tabs(&mut row);
        assert_eq!(
            row,
            [
                StyledToken::new("x".to_string(), Style::Plain),
                StyledToken::new("\t".to_string(), Style::Error),
            ]
        );

        // Middle TAB
        let mut row = [
            StyledToken::new("x".to_string(), Style::Plain),
            StyledToken::new("\t".to_string(), Style::Plain),
            StyledToken::new("y".to_string(), Style::Plain),
        ];
        highlight_nonleading_tabs(&mut row);
        assert_eq!(
            row,
            [
                StyledToken::new("x".to_string(), Style::Plain),
                StyledToken::new("\t".to_string(), Style::Error),
                StyledToken::new("y".to_string(), Style::Plain),
            ]
        );

        // Leading TAB (don't highlight)
        let mut row = [
            StyledToken::new("\t".to_string(), Style::Plain),
            StyledToken::new("x".to_string(), Style::Plain),
        ];
        highlight_nonleading_tabs(&mut row);
        assert_eq!(
            row,
            [
                StyledToken::new("\t".to_string(), Style::Plain),
                StyledToken::new("x".to_string(), Style::Plain),
            ]
        );

        // Single TAB (don't highlight because it is leading)
        let mut row = [StyledToken::new("\t".to_string(), Style::Plain)];
        highlight_nonleading_tabs(&mut row);
        assert_eq!(row, [StyledToken::new("\t".to_string(), Style::Plain),]);
    }

    #[test]
    fn test_removed_nonleading_tab() {
        // It shouldn't be highlighted, just added ones should
        let actual = render(
            &LINE_STYLE_OLD,
            vec![
                StyledToken::new("x".to_string(), Style::Plain),
                StyledToken::new("\t".to_string(), Style::Plain),
            ],
        );

        assert_eq!(actual, format!("{OLD}-x\t{NORMAL}"));
    }

    #[test]
    fn test_highlight_space_between_words() {
        let mut row = [
            StyledToken::new("Monkey".to_string(), Style::Highlighted),
            StyledToken::new(" ".to_string(), Style::Plain),
            StyledToken::new("Dance".to_string(), Style::Highlighted),
        ];

        bridge_consecutive_highlighted_tokens(&mut row);

        assert_eq!(
            row,
            [
                StyledToken::new("Monkey".to_string(), Style::Highlighted),
                StyledToken::new(" ".to_string(), Style::Highlighted),
                StyledToken::new("Dance".to_string(), Style::Highlighted),
            ]
        );
    }

    #[test]
    fn test_highlight_space_between_random_chars() {
        let mut row = [
            StyledToken::new(">".to_string(), Style::Highlighted),
            StyledToken::new(" ".to_string(), Style::Plain),
            StyledToken::new("5".to_string(), Style::Highlighted),
        ];

        bridge_consecutive_highlighted_tokens(&mut row);

        assert_eq!(
            row,
            [
                StyledToken::new(">".to_string(), Style::Highlighted),
                StyledToken::new(" ".to_string(), Style::Highlighted),
                StyledToken::new("5".to_string(), Style::Highlighted),
            ]
        );
    }
}
