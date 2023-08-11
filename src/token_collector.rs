use crate::constants::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Style {
    Old,
    OldInverse,
    New,
    NewInverse,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StyledToken {
    token: String,
    style: Style,
}

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

impl Style {
    #[must_use]
    pub fn is_inverse(&self) -> bool {
        match self {
            Style::OldInverse | Style::NewInverse | Style::Error => {
                return true;
            }
            _ => {
                return false;
            }
        }
    }

    pub fn inverted(&self) -> Style {
        return match self {
            Style::Old => Style::OldInverse,
            Style::New => Style::NewInverse,
            Style::OldInverse => Style::OldInverse,
            Style::NewInverse => Style::NewInverse,
            Style::Error => Style::Error,
        };
    }

    pub fn not_inverted(&self) -> Style {
        return match self {
            Style::Old => Style::Old,
            Style::New => Style::New,
            Style::OldInverse => Style::Old,
            Style::NewInverse => Style::New,
            Style::Error => Style::Error,
        };
    }

    #[must_use]
    pub fn color<'a>(&self) -> &'a str {
        match self {
            Style::Old => {
                return OLD;
            }
            Style::OldInverse => {
                return OLD;
            }
            Style::New => {
                return NEW;
            }
            Style::NewInverse => {
                return NEW;
            }
            Style::Error => {
                return ERROR;
            }
        }
    }
}

#[must_use]
fn render_row(line_prefix: &StyledToken, row: &mut [StyledToken]) -> String {
    let mut rendered = String::new();

    if line_prefix.style == Style::New {
        highlight_nonleading_tab(row);
    }

    // Set inverse from prefix
    let mut is_inverse = line_prefix.style.is_inverse();
    if is_inverse {
        rendered.push_str(INVERSE_VIDEO);
    }

    // Set line color from prefix
    let mut color = line_prefix.style.color();
    rendered.push_str(line_prefix.style.color());

    // Render prefix
    rendered.push_str(&line_prefix.token);

    for token in row {
        if token.style.is_inverse() && !is_inverse {
            rendered.push_str(INVERSE_VIDEO);
        }
        if is_inverse && !token.style.is_inverse() {
            rendered.push_str(NOT_INVERSE_VIDEO);
        }
        is_inverse = token.style.is_inverse();

        if token.style.color() != color {
            rendered.push_str(token.style.color());
            color = token.style.color();
        }

        rendered.push_str(&token.token);
    }

    rendered.push_str(NORMAL);

    return rendered;
}

/// Render all the tokens into a (most of the time multiline) string
#[must_use]
pub fn render(line_prefix: &StyledToken, tokens: &mut Vec<StyledToken>) -> String {
    let mut current_row: Vec<StyledToken> = Vec::new();
    let mut rendered = String::new();

    for token in tokens {
        if token.token == "\n" {
            let rendered_row = &render_row(line_prefix, &mut current_row);
            rendered.push_str(rendered_row);
            rendered.push('\n');
            current_row.clear();
            continue;
        }

        current_row.push(token.to_owned());
    }

    if !current_row.is_empty() {
        let rendered_row = render_row(line_prefix, &mut current_row);
        rendered.push_str(&rendered_row);
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
                        token.style = token.style.not_inverted();
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
        if token.style.is_inverse() {
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
                token.style = token.style.not_inverted();
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
                if token.style.is_inverse() {
                    // Found "Monkey"
                    found_state = FoundState::HighlightedWord;
                }
            }

            FoundState::HighlightedWord => {
                if token.token.len() == 1 {
                    // Found "Monkey " (note trailing space)
                    found_state = FoundState::WordSpace;
                } else if token.style.is_inverse() {
                    found_state = FoundState::HighlightedWord;
                } else {
                    found_state = FoundState::Nothing;
                }
            }

            FoundState::WordSpace => {
                if token.style.is_inverse() {
                    // Found "Monkey Dance"
                    if let Some(whitespace) = previous_token {
                        whitespace.style = whitespace.style.inverted();
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
        let line_prefix = StyledToken {
            token: "+".to_string(),
            style: Style::New,
        };
        let mut tokens = vec![
            StyledToken {
                token: "hej".to_string(),
                style: Style::New,
            },
            StyledToken {
                token: "\n".to_string(),
                style: Style::New,
            },
        ];

        let rendered = render(&line_prefix, &mut tokens);
        assert_eq!(rendered, format!("{NEW}+hej{NORMAL}\n"));
    }

    #[test]
    fn test_add_trailing_whitespace() {
        // Just a whitespace
        let mut row = [StyledToken::new(" ".to_string(), Style::New)];
        highlight_trailing_whitespace(&mut row);
        assert_eq!(row, [StyledToken::new(" ".to_string(), Style::Error)]);

        // Trailing whitespace
        let mut row = [
            StyledToken::new("x".to_string(), Style::New),
            StyledToken::new(" ".to_string(), Style::New),
        ];
        highlight_trailing_whitespace(&mut row);
        assert_eq!(
            row,
            [
                StyledToken::new("x".to_string(), Style::New),
                StyledToken::new(" ".to_string(), Style::Error),
            ]
        );

        // Leading whitespace
        let mut row = [
            StyledToken::new(" ".to_string(), Style::New),
            StyledToken::new("x".to_string(), Style::New),
        ];
        highlight_trailing_whitespace(&mut row);
        assert_eq!(
            row,
            [
                StyledToken::new(" ".to_string(), Style::New),
                StyledToken::new("x".to_string(), Style::New),
            ]
        );
    }

    #[test]
    fn test_removed_trailing_whitespace() {
        // It shouldn't be highlighted, just added ones should
        let mut tokens = vec![StyledToken::new(" ".to_string(), Style::Old)];
        let line_prefix = StyledToken::new("-".to_string(), Style::Old);
        let actual = render(&line_prefix, &mut tokens);

        assert_eq!(actual, format!("{OLD}- {NORMAL}"));
    }

    #[test]
    fn test_add_nonleading_tab() {
        // Trailing TAB
        let mut row = [
            StyledToken::new("x".to_string(), Style::New),
            StyledToken::new("\t".to_string(), Style::New),
        ];
        highlight_nonleading_tab(&mut row);
        assert_eq!(
            row,
            [
                StyledToken::new("x".to_string(), Style::New),
                StyledToken::new("\t".to_string(), Style::Error),
            ]
        );

        // Middle TAB
        let mut row = [
            StyledToken::new("x".to_string(), Style::New),
            StyledToken::new("\t".to_string(), Style::New),
            StyledToken::new("y".to_string(), Style::New),
        ];
        highlight_nonleading_tab(&mut row);
        assert_eq!(
            row,
            [
                StyledToken::new("x".to_string(), Style::New),
                StyledToken::new("\t".to_string(), Style::Error),
                StyledToken::new("y".to_string(), Style::New),
            ]
        );

        // Leading TAB (don't highlight)
        let mut row = [
            StyledToken::new("\t".to_string(), Style::New),
            StyledToken::new("x".to_string(), Style::New),
        ];
        highlight_nonleading_tab(&mut row);
        assert_eq!(
            row,
            [
                StyledToken::new("\t".to_string(), Style::New),
                StyledToken::new("x".to_string(), Style::New),
            ]
        );

        // Single TAB (don't highlight because it is leading)
        let mut row = [StyledToken::new("\t".to_string(), Style::New)];
        highlight_nonleading_tab(&mut row);
        assert_eq!(row, [StyledToken::new("\t".to_string(), Style::New),]);
    }

    #[test]
    fn test_removed_nonleading_tab() {
        // It shouldn't be highlighted, just added ones should
        let line_prefix = StyledToken::new("-".to_string(), Style::Old);
        let mut test_me = vec![
            StyledToken::new("x".to_string(), Style::Old),
            StyledToken::new("\t".to_string(), Style::Old),
        ];
        let actual = render(&line_prefix, &mut test_me);

        assert_eq!(actual, format!("{OLD}-x\t{NORMAL}"));
    }

    #[test]
    fn test_highlight_space_between_words() {
        let mut row = [
            StyledToken::new("Monkey".to_string(), Style::NewInverse),
            StyledToken::new(" ".to_string(), Style::New),
            StyledToken::new("Dance".to_string(), Style::NewInverse),
        ];

        bridge_consecutive_highlighted_tokens(&mut row);

        assert_eq!(
            row,
            [
                StyledToken::new("Monkey".to_string(), Style::NewInverse),
                StyledToken::new(" ".to_string(), Style::NewInverse),
                StyledToken::new("Dance".to_string(), Style::NewInverse),
            ]
        );
    }

    #[test]
    fn test_highlight_space_between_random_chars() {
        let mut row = [
            StyledToken::new(">".to_string(), Style::NewInverse),
            StyledToken::new(" ".to_string(), Style::New),
            StyledToken::new("5".to_string(), Style::NewInverse),
        ];

        bridge_consecutive_highlighted_tokens(&mut row);

        assert_eq!(
            row,
            [
                StyledToken::new(">".to_string(), Style::NewInverse),
                StyledToken::new(" ".to_string(), Style::NewInverse),
                StyledToken::new("5".to_string(), Style::NewInverse),
            ]
        );
    }
}
