use std::cmp;

use crate::ansi::AnsiStyle;
use crate::ansi::Color::Default;
use crate::ansi::Color::Green;
use crate::ansi::Color::Red;
use crate::ansi::Weight;
use crate::ansi::ANSI_STYLE_NORMAL;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Style {
    Lowlighted,
    Context,
    Plain,
    /// Brightened up, but not a highlighted difference
    Bright,
    /// A difference that should not be highlighted
    PlainChange,
    /// A difference to be highlighted
    HighlightedChange,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StyledToken {
    pub(crate) token: String,
    pub(crate) style: Style,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct LineStyle {
    prefix_style: AnsiStyle,
    plain_style: AnsiStyle,
    highlighted_style: AnsiStyle,
}

pub(crate) const LINE_STYLE_OLD: LineStyle = {
    LineStyle {
        prefix_style: AnsiStyle {
            inverse: false,
            weight: Weight::Normal,
            color: Red,
        },
        plain_style: AnsiStyle {
            inverse: false,
            weight: Weight::Normal,
            color: Red,
        },
        highlighted_style: AnsiStyle {
            inverse: true,
            weight: Weight::Normal,
            color: Red,
        },
    }
};

pub(crate) const LINE_STYLE_OLD_FAINT: LineStyle = {
    LineStyle {
        prefix_style: AnsiStyle {
            inverse: false,
            weight: Weight::Faint,
            color: Red,
        },
        plain_style: AnsiStyle {
            inverse: false,
            weight: Weight::Faint,
            color: Red,
        },
        highlighted_style: AnsiStyle {
            inverse: true,
            weight: Weight::Faint,
            color: Red,
        },
    }
};

pub(crate) const LINE_STYLE_NEW: LineStyle = {
    LineStyle {
        prefix_style: AnsiStyle {
            inverse: false,
            weight: Weight::Normal,
            color: Green,
        },
        plain_style: AnsiStyle {
            inverse: false,
            weight: Weight::Normal,
            color: Green,
        },
        highlighted_style: AnsiStyle {
            inverse: true,
            weight: Weight::Normal,
            color: Green,
        },
    }
};

pub(crate) const LINE_STYLE_ADDS_ONLY: LineStyle = {
    LineStyle {
        prefix_style: AnsiStyle {
            inverse: false,
            weight: Weight::Faint,
            color: Green,
        },
        plain_style: AnsiStyle {
            inverse: false,
            weight: Weight::Normal,
            color: Default,
        },
        highlighted_style: AnsiStyle {
            inverse: true,
            weight: Weight::Normal,
            color: Green,
        },
    }
};

pub(crate) const LINE_STYLE_CONFLICT_BASE: LineStyle = {
    LineStyle {
        prefix_style: AnsiStyle {
            inverse: false,
            weight: Weight::Normal,
            color: Green,
        },
        plain_style: AnsiStyle {
            inverse: false,
            weight: Weight::Normal,
            color: Red,
        },
        highlighted_style: AnsiStyle {
            inverse: true,
            weight: Weight::Normal,
            color: Red,
        },
    }
};

pub(crate) const LINE_STYLE_OLD_FILENAME: LineStyle = {
    LineStyle {
        prefix_style: AnsiStyle {
            inverse: false,
            weight: Weight::Bold,
            color: Default,
        },
        plain_style: ANSI_STYLE_NORMAL,
        highlighted_style: AnsiStyle {
            inverse: true,
            weight: Weight::Normal,
            color: Red,
        },
    }
};

pub(crate) const LINE_STYLE_NEW_FILENAME: LineStyle = {
    LineStyle {
        prefix_style: AnsiStyle {
            inverse: false,
            weight: Weight::Bold,
            color: Default,
        },
        plain_style: ANSI_STYLE_NORMAL,
        highlighted_style: AnsiStyle {
            inverse: true,
            weight: Weight::Normal,
            color: Green,
        },
    }
};

impl StyledToken {
    pub fn new(token: String, style: Style) -> StyledToken {
        return StyledToken { token, style };
    }

    // Are all characters in this token whitespace?
    pub fn is_whitespace(&self) -> bool {
        return self.token.chars().all(|c| c.is_whitespace());
    }
}

#[must_use]
fn render_row(line_style: &LineStyle, prefix: &str, row: &[StyledToken]) -> String {
    let mut rendered = String::new();

    let mut current_style = ANSI_STYLE_NORMAL;

    // Render prefix
    rendered.push_str(&line_style.prefix_style.from(&current_style));
    current_style = line_style.prefix_style;
    rendered.push_str(prefix);

    // Render tokens
    for token in row {
        let new_style = match token.style {
            Style::Context => AnsiStyle {
                inverse: false,
                weight: Weight::Normal,
                color: Default,
            },
            Style::Lowlighted => AnsiStyle {
                inverse: false,
                weight: Weight::Faint,
                color: Default,
            },
            Style::Bright => AnsiStyle {
                inverse: false,
                weight: Weight::Bold,
                color: Default,
            },
            Style::Plain => line_style.plain_style,
            Style::PlainChange => line_style.plain_style,
            Style::HighlightedChange => line_style.highlighted_style,
            Style::Error => AnsiStyle {
                inverse: true,
                weight: Weight::Normal,
                color: Red,
            },
        };

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
            let rendered_row = &render_row(line_style, prefix, &tokens[current_row_start..i]);
            rendered.push_str(rendered_row);
            rendered.push('\n');
            current_row_start = i + 1;
            continue;
        }
    }

    if current_row_start < tokens.len() {
        let rendered_row = &render_row(line_style, prefix, &tokens[current_row_start..]);
        rendered.push_str(rendered_row);
    }

    return rendered;
}

pub fn errorlight_trailing_whitespace(tokens: &mut [StyledToken]) {
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

pub fn errorlight_nonleading_tabs(tokens: &mut [StyledToken]) {
    let mut leading = true;
    for token in tokens.iter_mut() {
        if token.token == "\n" {
            leading = true;
            continue;
        }

        if token.token != "\t" {
            leading = false;
        }

        if token.token == "\t" && !leading {
            token.style = Style::Error;
        }
    }
}

pub(crate) fn align_tabs(old: &mut [StyledToken], new: &mut [StyledToken]) {
    let old_tab_index_token = old.iter().position(|token| token.token == "\t");
    if old_tab_index_token.is_none() {
        return;
    }
    let old_tab_index_token = old_tab_index_token.unwrap();
    let old_tab_index_char = old
        .iter()
        .take(old_tab_index_token)
        .map(|token| token.token.chars().count())
        .sum::<usize>();

    let new_tab_index_token = new.iter().position(|token| token.token == "\t");
    if new_tab_index_token.is_none() {
        return;
    }
    let new_tab_index_token = new_tab_index_token.unwrap();
    let new_tab_index_char = new
        .iter()
        .take(new_tab_index_token)
        .map(|token| token.token.chars().count())
        .sum::<usize>();

    let old_spaces =
        " ".repeat(2 + cmp::max(old_tab_index_char, new_tab_index_char) - old_tab_index_char);
    let new_spaces =
        " ".repeat(2 + cmp::max(old_tab_index_char, new_tab_index_char) - new_tab_index_char);

    old[old_tab_index_token].token = old_spaces;
    new[new_tab_index_token].token = new_spaces;
}

/// If a line contains only plain tokens, style all tokens on that line as
/// context. Lines are separated by newline tokens.
///
/// This can happen during conflicts highlighting.
pub fn contextualize_unhighlighted_lines(tokens: &mut [StyledToken]) {
    let mut line_start = 0;
    for i in 0..tokens.len() {
        if tokens[i].token != "\n" {
            continue;
        }

        // Line ended

        if tokens[line_start..(i + 1)]
            .iter()
            .all(|token| token.style == Style::Plain)
        {
            // Line contains only plain tokens
            for token in &mut tokens[line_start..i] {
                token.style = Style::Context;
            }
        }
        line_start = i + 1;
    }

    // Handle the last line
    if tokens[line_start..]
        .iter()
        .all(|token| token.style == Style::Plain)
    {
        for token in &mut tokens[line_start..] {
            token.style = Style::Context;
        }
    }
}

/// Highlight single space between two highlighted tokens
pub fn bridge_consecutive_highlighted_tokens(tokens: &mut [StyledToken]) {
    fn bridgable(candidate: &StyledToken) -> bool {
        if candidate.style != Style::Plain {
            return false;
        }
        if candidate.token.len() != 1 {
            return false;
        }

        let rune = candidate.token.chars().next().unwrap();
        return rune == ' ' || rune.is_ascii_punctuation();
    }

    for i in 1..(tokens.len() - 1) {
        if tokens[i - 1].style != Style::HighlightedChange
            || tokens[i + 1].style != Style::HighlightedChange
        {
            continue;
        }
        if bridgable(&tokens[i - 1]) || !bridgable(&tokens[i]) || bridgable(&tokens[i + 1]) {
            continue;
        }

        tokens[i].style = Style::HighlightedChange;
    }
}

pub fn count_lines(tokens: &[StyledToken]) -> usize {
    if tokens.is_empty() {
        return 0;
    }

    let mut lines = 0;
    let mut ends_with_newline = false;
    for token in tokens {
        if token.token == "\n" {
            lines += 1;
            ends_with_newline = true;
            continue;
        }
        ends_with_newline = false;
    }

    if !ends_with_newline {
        lines += 1;
    }

    return lines;
}

/// File timestamps are found after either a tab character or a double space
pub fn lowlight_timestamp(row: &mut [StyledToken]) {
    #[derive(PartialEq)]
    enum State {
        Initial,
        InTimestamp,
    }

    let mut state = State::Initial;
    for token in row.iter_mut() {
        match state {
            State::Initial => {
                let is_multispace = token.token.len() > 1 && token.token.chars().all(|c| c == ' ');
                if token.token == "\t" || is_multispace {
                    state = State::InTimestamp;
                }
            }
            State::InTimestamp => {
                // Intentionally left blank, no way out of this state
            }
        }

        if state == State::InTimestamp {
            token.style = Style::Lowlighted;
            continue;
        }
    }
}

/// Unhighlight leading 'a/' or 'b/' in git diff file names.
///
/// They are just placeholders that do not indicate any changes introduced by
/// the user.
pub fn unhighlight_git_prefix(row: &mut [StyledToken]) {
    if row.len() < 2 {
        return;
    }

    if (row[0].token == "a" || row[0].token == "b") && row[1].token == "/" {
        row[0].style = Style::Lowlighted;
        row[1].style = Style::Lowlighted;
    }
}

/// If we get "x/y/z.txt", make "z.txt" bright.
///
/// As an exception, if the file name is already highlighted, don't brighten it.
pub fn brighten_filename(row: &mut [StyledToken]) {
    let mut last_slash_index = None;
    for (i, token) in row.iter().enumerate() {
        if token.token == "/" {
            last_slash_index = Some(i);
        }
    }

    let to_brighten: &mut [StyledToken];
    if let Some(last_slash_index) = last_slash_index {
        to_brighten = &mut row[last_slash_index + 1..];
    } else {
        to_brighten = row;
    }

    for token in to_brighten {
        if token.style == Style::HighlightedChange {
            continue;
        }
        token.style = Style::Bright;
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
    fn test_errorlight_nonleading_tabs() {
        let mut tokens = vec![
            StyledToken::new("\t".to_string(), Style::Plain),
            StyledToken::new("\t".to_string(), Style::Plain),
            StyledToken::new("one".to_string(), Style::Plain),
            StyledToken::new("\n".to_string(), Style::Plain),
            StyledToken::new("two".to_string(), Style::Plain),
            StyledToken::new("\t".to_string(), Style::Plain),
        ];
        errorlight_nonleading_tabs(&mut tokens);

        assert_eq!(
            tokens,
            vec![
                StyledToken::new("\t".to_string(), Style::Plain),
                StyledToken::new("\t".to_string(), Style::Plain),
                StyledToken::new("one".to_string(), Style::Plain),
                StyledToken::new("\n".to_string(), Style::Plain),
                StyledToken::new("two".to_string(), Style::Plain),
                StyledToken::new("\t".to_string(), Style::Error),
            ]
        );
    }

    #[test]
    fn test_basic() {
        let rendered = render(
            &LINE_STYLE_NEW,
            "+",
            &[
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
    fn test_errorlight_trailing_whitespace() {
        // Just a whitespace
        let mut row = [StyledToken::new(" ".to_string(), Style::Plain)];
        errorlight_trailing_whitespace(&mut row);
        assert_eq!(row, [StyledToken::new(" ".to_string(), Style::Error)]);

        // Trailing whitespace
        let mut row = [
            StyledToken::new("x".to_string(), Style::Plain),
            StyledToken::new(" ".to_string(), Style::Plain),
        ];
        errorlight_trailing_whitespace(&mut row);
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
        errorlight_trailing_whitespace(&mut row);
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
            "-",
            &[StyledToken::new(" ".to_string(), Style::Plain)],
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
        errorlight_nonleading_tabs(&mut row);
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
        errorlight_nonleading_tabs(&mut row);
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
        errorlight_nonleading_tabs(&mut row);
        assert_eq!(
            row,
            [
                StyledToken::new("\t".to_string(), Style::Plain),
                StyledToken::new("x".to_string(), Style::Plain),
            ]
        );

        // Single TAB (don't highlight because it is leading)
        let mut row = [StyledToken::new("\t".to_string(), Style::Plain)];
        errorlight_nonleading_tabs(&mut row);
        assert_eq!(row, [StyledToken::new("\t".to_string(), Style::Plain),]);
    }

    #[test]
    fn test_removed_nonleading_tab() {
        // It shouldn't be highlighted, just added ones should
        let actual = render(
            &LINE_STYLE_OLD,
            "-",
            &[
                StyledToken::new("x".to_string(), Style::Plain),
                StyledToken::new("\t".to_string(), Style::Plain),
            ],
        );

        assert_eq!(actual, format!("{OLD}-x\t{NORMAL}"));
    }

    fn is_char_bridged(before: char, victim: char, after: char) -> bool {
        let mut row = [
            StyledToken::new(before.to_string(), Style::HighlightedChange),
            StyledToken::new(victim.to_string(), Style::Plain),
            StyledToken::new(after.to_string(), Style::HighlightedChange),
        ];

        bridge_consecutive_highlighted_tokens(&mut row);

        return row[1].style == Style::HighlightedChange;
    }

    #[test]
    fn test_bridge_consecutive_highlighted_tokens() {
        assert!(is_char_bridged('a', ' ', 'b'));
        assert!(is_char_bridged('>', ' ', '5'));
        assert!(is_char_bridged('a', ' ', ' ')); // Because the second space is highlighted
        assert!(!is_char_bridged('\'', '1', '\''));
        assert!(is_char_bridged('a', '.', 'b')); // Bridge separators
    }

    #[test]
    fn test_four_tokens_highlighting() {
        let mut row = [
            StyledToken::new("\n".to_string(), Style::HighlightedChange),
            StyledToken::new("*".to_string(), Style::HighlightedChange),
            StyledToken::new(" ".to_string(), Style::Plain),
            StyledToken::new("Hello".to_string(), Style::HighlightedChange),
        ];

        bridge_consecutive_highlighted_tokens(&mut row);

        assert_eq!(
            row,
            [
                StyledToken::new("\n".to_string(), Style::HighlightedChange),
                StyledToken::new("*".to_string(), Style::HighlightedChange),
                StyledToken::new(" ".to_string(), Style::HighlightedChange),
                StyledToken::new("Hello".to_string(), Style::HighlightedChange),
            ]
        );
    }
}
