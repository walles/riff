use std::cmp;

use crate::ansi::AnsiStyle;
use crate::ansi::Color::Green;
use crate::ansi::Color::Red;
use crate::ansi::Color::Yellow;
use crate::ansi::Weight;
use crate::ansi::ANSI_STYLE_NORMAL;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum Style {
    Lowlighted,                  // Faint
    Context,                     // Default
    Bright,                      // Bold
    DiffPartUnchanged,           // Yellow
    DiffPartUnchangedUnderlined, // Yellow and underlined
    DiffPartMidlighted,          // Red or Green
    DiffPartHighlighted,         // Inverse Red or Green
    Error,                       // Inverse Red
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StyledToken {
    pub(crate) token: String,
    pub(crate) style: Style,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct LineStyle {
    prefix_style: AnsiStyle,
    unchanged_style: AnsiStyle,
    unchanged_with_underline_style: AnsiStyle,
    midlighted_style: AnsiStyle,
    highlighted_style: AnsiStyle,
}

pub(crate) const LINE_STYLE_OLD: LineStyle = {
    LineStyle {
        prefix_style: ANSI_STYLE_NORMAL.with_color(Red),
        unchanged_style: ANSI_STYLE_NORMAL.with_color(Yellow),
        unchanged_with_underline_style: ANSI_STYLE_NORMAL.with_color(Yellow).with_underline(true),
        midlighted_style: ANSI_STYLE_NORMAL.with_color(Red),
        highlighted_style: ANSI_STYLE_NORMAL.with_color(Red).with_inverse(true),
    }
};

pub(crate) const LINE_STYLE_NEW: LineStyle = {
    LineStyle {
        prefix_style: ANSI_STYLE_NORMAL.with_color(Green),
        unchanged_style: ANSI_STYLE_NORMAL.with_color(Yellow),
        unchanged_with_underline_style: ANSI_STYLE_NORMAL.with_color(Yellow).with_underline(true),
        midlighted_style: ANSI_STYLE_NORMAL.with_color(Green),
        highlighted_style: ANSI_STYLE_NORMAL.with_color(Green).with_inverse(true),
    }
};

pub(crate) const LINE_STYLE_CONFLICT_BASE: LineStyle = {
    LineStyle {
        prefix_style: ANSI_STYLE_NORMAL.with_inverse(true),
        unchanged_style: ANSI_STYLE_NORMAL,
        unchanged_with_underline_style: ANSI_STYLE_NORMAL,
        midlighted_style: ANSI_STYLE_NORMAL.with_color(Red),
        highlighted_style: ANSI_STYLE_NORMAL.with_color(Red).with_inverse(true),
    }
};

pub(crate) const LINE_STYLE_CONFLICT_OLD: LineStyle = {
    LineStyle {
        prefix_style: ANSI_STYLE_NORMAL.with_inverse(true),
        unchanged_style: ANSI_STYLE_NORMAL,
        unchanged_with_underline_style: ANSI_STYLE_NORMAL,
        midlighted_style: ANSI_STYLE_NORMAL.with_color(Red),
        highlighted_style: ANSI_STYLE_NORMAL.with_color(Red).with_inverse(true),
    }
};

pub(crate) const LINE_STYLE_CONFLICT_NEW: LineStyle = {
    LineStyle {
        prefix_style: ANSI_STYLE_NORMAL.with_inverse(true),
        unchanged_style: ANSI_STYLE_NORMAL,
        unchanged_with_underline_style: ANSI_STYLE_NORMAL,
        midlighted_style: ANSI_STYLE_NORMAL.with_color(Green),
        highlighted_style: ANSI_STYLE_NORMAL.with_color(Green).with_inverse(true),
    }
};

pub(crate) const LINE_STYLE_OLD_FILENAME: LineStyle = {
    LineStyle {
        prefix_style: ANSI_STYLE_NORMAL.with_weight(Weight::Bold),
        unchanged_style: ANSI_STYLE_NORMAL,
        unchanged_with_underline_style: ANSI_STYLE_NORMAL,
        midlighted_style: ANSI_STYLE_NORMAL.with_color(Red),
        highlighted_style: ANSI_STYLE_NORMAL.with_color(Red).with_inverse(true),
    }
};

pub(crate) const LINE_STYLE_NEW_FILENAME: LineStyle = {
    LineStyle {
        prefix_style: ANSI_STYLE_NORMAL.with_weight(Weight::Bold),
        unchanged_style: ANSI_STYLE_NORMAL,
        unchanged_with_underline_style: ANSI_STYLE_NORMAL,
        midlighted_style: ANSI_STYLE_NORMAL.with_color(Green),
        highlighted_style: ANSI_STYLE_NORMAL.with_color(Green).with_inverse(true),
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
            Style::Context => ANSI_STYLE_NORMAL,
            Style::Lowlighted => ANSI_STYLE_NORMAL.with_weight(Weight::Faint),
            Style::Bright => ANSI_STYLE_NORMAL.with_weight(Weight::Bold),
            Style::DiffPartUnchanged => line_style.unchanged_style,
            Style::DiffPartUnchangedUnderlined => line_style.unchanged_with_underline_style,
            Style::DiffPartMidlighted => line_style.midlighted_style,
            Style::DiffPartHighlighted => line_style.highlighted_style,
            Style::Error => AnsiStyle {
                color: Red,
                weight: Weight::Normal,
                underline: false,
                inverse: true,
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

/// Highlight single space between two highlighted tokens
pub fn bridge_consecutive_highlighted_tokens(tokens: &mut [StyledToken]) {
    fn bridgable(candidate: &StyledToken) -> bool {
        if candidate.style as u8 > Style::DiffPartUnchanged as u8 {
            return false;
        }
        if candidate.token.len() != 1 {
            return false;
        }

        let rune = candidate.token.chars().next().unwrap();
        return rune == ' ' || rune.is_ascii_punctuation();
    }

    for i in 1..(tokens.len() - 1) {
        if tokens[i - 1].style != Style::DiffPartHighlighted
            || tokens[i + 1].style != Style::DiffPartHighlighted
        {
            continue;
        }
        if bridgable(&tokens[i - 1]) || !bridgable(&tokens[i]) || bridgable(&tokens[i + 1]) {
            continue;
        }

        tokens[i].style = Style::DiffPartHighlighted;
    }
}

/// Requires at least one token
fn find_last_line_start(tokens: &[StyledToken]) -> usize {
    let ends_in_newline = tokens.last().unwrap().token == "\n";
    let without_ending_newline = if ends_in_newline {
        &tokens[..tokens.len() - 1]
    } else {
        tokens
    };

    let last_newline_index = without_ending_newline
        .iter()
        .rposition(|token| token.token == "\n");

    return last_newline_index.map_or(0, |index| index + 1);
}

pub fn underline_last_line(tokens: &mut [StyledToken]) {
    if tokens.is_empty() {
        return;
    }

    let last_line_start = find_last_line_start(tokens);
    for token in tokens.iter_mut().skip(last_line_start) {
        if token.style == Style::DiffPartUnchanged {
            token.style = Style::DiffPartUnchangedUnderlined;
        }
    }
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
        if token.style == Style::DiffPartHighlighted {
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
            StyledToken::new("\t".to_string(), Style::DiffPartUnchanged),
            StyledToken::new("\t".to_string(), Style::DiffPartUnchanged),
            StyledToken::new("one".to_string(), Style::DiffPartUnchanged),
            StyledToken::new("\n".to_string(), Style::DiffPartUnchanged),
            StyledToken::new("two".to_string(), Style::DiffPartUnchanged),
            StyledToken::new("\t".to_string(), Style::DiffPartUnchanged),
        ];
        errorlight_nonleading_tabs(&mut tokens);

        assert_eq!(
            tokens,
            vec![
                StyledToken::new("\t".to_string(), Style::DiffPartUnchanged),
                StyledToken::new("\t".to_string(), Style::DiffPartUnchanged),
                StyledToken::new("one".to_string(), Style::DiffPartUnchanged),
                StyledToken::new("\n".to_string(), Style::DiffPartUnchanged),
                StyledToken::new("two".to_string(), Style::DiffPartUnchanged),
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
                    style: Style::DiffPartMidlighted,
                },
                StyledToken {
                    token: "\n".to_string(),
                    style: Style::DiffPartMidlighted,
                },
            ],
        );
        assert_eq!(rendered, format!("{NEW}+hej{NORMAL}\n"));
    }

    #[test]
    fn test_errorlight_trailing_whitespace() {
        // Just a whitespace
        let mut row = [StyledToken::new(" ".to_string(), Style::DiffPartUnchanged)];
        errorlight_trailing_whitespace(&mut row);
        assert_eq!(row, [StyledToken::new(" ".to_string(), Style::Error)]);

        // Trailing whitespace
        let mut row = [
            StyledToken::new("x".to_string(), Style::DiffPartUnchanged),
            StyledToken::new(" ".to_string(), Style::DiffPartUnchanged),
        ];
        errorlight_trailing_whitespace(&mut row);
        assert_eq!(
            row,
            [
                StyledToken::new("x".to_string(), Style::DiffPartUnchanged),
                StyledToken::new(" ".to_string(), Style::Error),
            ]
        );

        // Leading whitespace
        let mut row = [
            StyledToken::new(" ".to_string(), Style::DiffPartUnchanged),
            StyledToken::new("x".to_string(), Style::DiffPartUnchanged),
        ];
        errorlight_trailing_whitespace(&mut row);
        assert_eq!(
            row,
            [
                StyledToken::new(" ".to_string(), Style::DiffPartUnchanged),
                StyledToken::new("x".to_string(), Style::DiffPartUnchanged),
            ]
        );
    }

    #[test]
    fn test_removed_trailing_whitespace() {
        // It shouldn't be highlighted, just added ones should
        let actual = render(
            &LINE_STYLE_OLD,
            "-",
            &[StyledToken::new(" ".to_string(), Style::DiffPartMidlighted)],
        );

        assert_eq!(actual, format!("{OLD}- {NORMAL}"));
    }

    #[test]
    fn test_add_nonleading_tab() {
        // Trailing TAB
        let mut row = [
            StyledToken::new("x".to_string(), Style::DiffPartUnchanged),
            StyledToken::new("\t".to_string(), Style::DiffPartUnchanged),
        ];
        errorlight_nonleading_tabs(&mut row);
        assert_eq!(
            row,
            [
                StyledToken::new("x".to_string(), Style::DiffPartUnchanged),
                StyledToken::new("\t".to_string(), Style::Error),
            ]
        );

        // Middle TAB
        let mut row = [
            StyledToken::new("x".to_string(), Style::DiffPartUnchanged),
            StyledToken::new("\t".to_string(), Style::DiffPartUnchanged),
            StyledToken::new("y".to_string(), Style::DiffPartUnchanged),
        ];
        errorlight_nonleading_tabs(&mut row);
        assert_eq!(
            row,
            [
                StyledToken::new("x".to_string(), Style::DiffPartUnchanged),
                StyledToken::new("\t".to_string(), Style::Error),
                StyledToken::new("y".to_string(), Style::DiffPartUnchanged),
            ]
        );

        // Leading TAB (don't highlight)
        let mut row = [
            StyledToken::new("\t".to_string(), Style::DiffPartUnchanged),
            StyledToken::new("x".to_string(), Style::DiffPartUnchanged),
        ];
        errorlight_nonleading_tabs(&mut row);
        assert_eq!(
            row,
            [
                StyledToken::new("\t".to_string(), Style::DiffPartUnchanged),
                StyledToken::new("x".to_string(), Style::DiffPartUnchanged),
            ]
        );

        // Single TAB (don't highlight because it is leading)
        let mut row = [StyledToken::new("\t".to_string(), Style::DiffPartUnchanged)];
        errorlight_nonleading_tabs(&mut row);
        assert_eq!(
            row,
            [StyledToken::new("\t".to_string(), Style::DiffPartUnchanged),]
        );
    }

    #[test]
    fn test_removed_nonleading_tab() {
        // It shouldn't be highlighted, just added ones should
        let actual = render(
            &LINE_STYLE_OLD,
            "-",
            &[
                StyledToken::new("x".to_string(), Style::DiffPartMidlighted),
                StyledToken::new("\t".to_string(), Style::DiffPartMidlighted),
            ],
        );

        assert_eq!(actual, format!("{OLD}-x\t{NORMAL}"));
    }

    fn is_char_bridged(before: char, victim: char, after: char) -> bool {
        let mut row = [
            StyledToken::new(before.to_string(), Style::DiffPartHighlighted),
            StyledToken::new(victim.to_string(), Style::DiffPartUnchanged),
            StyledToken::new(after.to_string(), Style::DiffPartHighlighted),
        ];

        bridge_consecutive_highlighted_tokens(&mut row);

        return row[1].style == Style::DiffPartHighlighted;
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
            StyledToken::new("\n".to_string(), Style::DiffPartHighlighted),
            StyledToken::new("*".to_string(), Style::DiffPartHighlighted),
            StyledToken::new(" ".to_string(), Style::DiffPartUnchanged),
            StyledToken::new("Hello".to_string(), Style::DiffPartHighlighted),
        ];

        bridge_consecutive_highlighted_tokens(&mut row);

        assert_eq!(
            row,
            [
                StyledToken::new("\n".to_string(), Style::DiffPartHighlighted),
                StyledToken::new("*".to_string(), Style::DiffPartHighlighted),
                StyledToken::new(" ".to_string(), Style::DiffPartHighlighted),
                StyledToken::new("Hello".to_string(), Style::DiffPartHighlighted),
            ]
        );
    }
}
