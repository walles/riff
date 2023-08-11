use crate::ansi::AnsiStyle;
use crate::ansi::Color::Default;
use crate::ansi::Color::Green;
use crate::ansi::Color::Red;
use crate::constants::*;
use crate::token_collector::Style::Highlighted;
use crate::token_collector::Style::Plain;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Style {
    Plain,
    Highlighted,
    Error,
}

#[derive(Debug, PartialEq, Eq)]
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

const LINE_STYLE_OLD: LineStyle = {
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

const LINE_STYLE_NEW: LineStyle = {
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

    unhighlight_noisy_rows(row);

    if line_style == &LINE_STYLE_NEW {
        highlight_trailing_whitespace(row);
        highlight_nonleading_tab(row);
    }

    let mut current_style = AnsiStyle {
        inverse: false,
        color: Default,
    };

    // Render prefix
    rendered.push_str(line_style.prefix_style.from(&current_style));
    current_style = line_style.prefix_style;
    rendered.push_str(line_style.prefix);

    // Render tokens
    for token in row {
        let new_style = match token.style {
            Plain => line_style.plain_style,
            Highlighted => line_style.highlighted_style,
            Error => AnsiStyle {
                inverse: true,
                color: Red,
            },
        };

        rendered.push_str(new_style.from(&current_style));
        current_style = new_style;
        rendered.push_str(&token.token);
    }

    // Reset formatting at the end of the line
    rendered.push_str(
        AnsiStyle {
            inverse: false,
            color: Default,
        }
        .from(&current_style),
    );

    return rendered;
}

/// Render all the tokens into a (most of the time multiline) string
#[must_use]
pub fn render(&mut self) -> String {
    assert!(!self.rendered);
    let mut current_row: Vec<StyledToken> = Vec::new();
    let mut rendered = String::new();

    let mut tokens = std::mem::take(&mut self.tokens);

    bridge_consecutive_highlighted_tokens(&mut tokens);

    for token in tokens {
        if token.token == "\n" {
            let rendered_row = &self.render_row(&mut current_row);
            rendered.push_str(rendered_row);
            rendered.push('\n');
            current_row.clear();
            continue;
        }

        current_row.push(token);
    }

    if !current_row.is_empty() {
        let rendered_row = &self.render_row(&mut current_row);
        rendered.push_str(rendered_row);
    }

    self.rendered = true;
    return rendered;
}

/// Unhighlight everything if too much of the line is highlighted
fn unhighlight_noisy_rows(row: &mut [StyledToken]) {
    let mut highlighted_tokens_count = 0;

    for token in row.iter_mut().rev() {
        if token.style == Highlighted {
            highlighted_tokens_count += 1;
        }
    }

    if !row.is_empty() {
        let highlighted_percentage = (100 * highlighted_tokens_count) / row.len();
        if highlighted_percentage <= 70 {
            // Little enough of the line highlighted, let it be
            return;
        }
    }

    // Line too noisy, unhighlight!
    for token in row.iter_mut() {
        token.style = Plain;
    }
}

fn highlight_trailing_whitespace(row: &mut [StyledToken]) {
    for token in row.iter_mut().rev() {
        if !token.is_whitespace() {
            return;
        }

        token.style = Style::Error;
    }
}

fn highlight_nonleading_tab(row: &mut [StyledToken]) {
    let mut token_iter = row.iter_mut();

    // Skip leading TABs
    loop {
        let next = token_iter.next();
        if next.is_none() {
            // Done!
            return;
        }

        let token = next.unwrap();
        if token.token != "\t" {
            // Not a TAB, this means we're out of skipping the leading TABs
            break;
        }
    }

    // Scan the rest of the line for non-leading TABs
    for token in token_iter {
        if token.token != "\t" {
            // Not a TAB, never mind
            continue;
        }

        // Non-leading TAB, mark it!
        token.style = Style::Error;
    }
}

/// Highlight single space between two highlighted tokens
fn bridge_consecutive_highlighted_tokens(row: &mut [StyledToken]) {
    enum FoundState {
        Nothing,
        HighlightedWord,
        WordSpace,
    }

    let mut found_state = FoundState::Nothing;
    let mut previous_token: Option<&mut StyledToken> = None;
    for token in row.iter_mut() {
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
                    if let Some(_previous_token) = previous_token {
                        _previous_token.style = Highlighted;
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

    #[cfg(test)]
    use pretty_assertions::assert_eq;

    #[test]
    fn test_basic() {
        let mut test_me = TokenCollector::create(StyledToken {
            token: "+".to_string(),
            style: Style::Plain,
        });

        test_me.push(StyledToken {
            token: "hej".to_string(),
            style: Style::Plain,
        });

        test_me.push(StyledToken {
            token: "\n".to_string(),
            style: Style::Plain,
        });

        let rendered = test_me.render();
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
        let mut test_me = TokenCollector::create(StyledToken::new("-".to_string(), Style::Plain));
        test_me.push(StyledToken::new(" ".to_string(), Style::Plain));
        let actual = test_me.render();

        assert_eq!(actual, format!("{OLD}- {NORMAL}"));
    }

    #[test]
    fn test_add_nonleading_tab() {
        // Trailing TAB
        let mut row = [
            StyledToken::new("x".to_string(), Style::Plain),
            StyledToken::new("\t".to_string(), Style::Plain),
        ];
        highlight_nonleading_tab(&mut row);
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
        highlight_nonleading_tab(&mut row);
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
        highlight_nonleading_tab(&mut row);
        assert_eq!(
            row,
            [
                StyledToken::new("\t".to_string(), Style::Plain),
                StyledToken::new("x".to_string(), Style::Plain),
            ]
        );

        // Single TAB (don't highlight because it is leading)
        let mut row = [StyledToken::new("\t".to_string(), Style::Plain)];
        highlight_nonleading_tab(&mut row);
        assert_eq!(row, [StyledToken::new("\t".to_string(), Style::Plain),]);
    }

    #[test]
    fn test_removed_nonleading_tab() {
        // It shouldn't be highlighted, just added ones should
        let mut test_me = TokenCollector::create(StyledToken::new("-".to_string(), Style::Plain));
        test_me.push(StyledToken::new("x".to_string(), Style::Plain));
        test_me.push(StyledToken::new("\t".to_string(), Style::Plain));
        let actual = test_me.render();

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
