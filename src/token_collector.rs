use std::cmp;

use crate::ansi::AnsiStyle;
use crate::ansi::Color::Green;
use crate::ansi::Color::Red;
use crate::ansi::Weight;
use crate::ansi::ANSI_STYLE_NORMAL;

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LineStyle {
    pub(crate) prefix_style: AnsiStyle,
    pub(crate) unchanged_style: AnsiStyle,
    pub(crate) midlighted_style: AnsiStyle,
    pub(crate) highlighted_style: AnsiStyle,
}

// The base line styles live in refiner.rs

pub(crate) const LINE_STYLE_CONFLICT_BASE: LineStyle = {
    LineStyle {
        prefix_style: ANSI_STYLE_NORMAL.with_inverse(true),
        unchanged_style: ANSI_STYLE_NORMAL,
        midlighted_style: ANSI_STYLE_NORMAL.with_color(Red),
        highlighted_style: ANSI_STYLE_NORMAL.with_color(Red).with_inverse(true),
    }
};

pub(crate) const LINE_STYLE_CONFLICT_OLD: LineStyle = {
    LineStyle {
        prefix_style: ANSI_STYLE_NORMAL.with_inverse(true),
        unchanged_style: ANSI_STYLE_NORMAL,
        midlighted_style: ANSI_STYLE_NORMAL.with_color(Red),
        highlighted_style: ANSI_STYLE_NORMAL.with_color(Red).with_inverse(true),
    }
};

pub(crate) const LINE_STYLE_CONFLICT_NEW: LineStyle = {
    LineStyle {
        prefix_style: ANSI_STYLE_NORMAL.with_inverse(true),
        unchanged_style: ANSI_STYLE_NORMAL,
        midlighted_style: ANSI_STYLE_NORMAL.with_color(Green),
        highlighted_style: ANSI_STYLE_NORMAL.with_color(Green).with_inverse(true),
    }
};

pub(crate) const LINE_STYLE_OLD_FILENAME: LineStyle = {
    LineStyle {
        prefix_style: ANSI_STYLE_NORMAL.with_weight(Weight::Bold),
        unchanged_style: ANSI_STYLE_NORMAL,
        midlighted_style: ANSI_STYLE_NORMAL.with_color(Red),
        highlighted_style: ANSI_STYLE_NORMAL.with_color(Red).with_inverse(true),
    }
};

pub(crate) const LINE_STYLE_NEW_FILENAME: LineStyle = {
    LineStyle {
        prefix_style: ANSI_STYLE_NORMAL.with_weight(Weight::Bold),
        unchanged_style: ANSI_STYLE_NORMAL,
        midlighted_style: ANSI_STYLE_NORMAL.with_color(Green),
        highlighted_style: ANSI_STYLE_NORMAL.with_color(Green).with_inverse(true),
    }
};

impl StyledToken {
    pub fn new(token: String, style: Style) -> StyledToken {
        if token.len() != 1 {
            return StyledToken { token, style };
        }

        let character = token.chars().next().unwrap();
        if character >= ' ' || character == '\x09' || character == '\x0a' {
            return StyledToken { token, style };
        }

        // This is a special character, let's replace it with its Unicode symbol
        let symbol = char::from_u32((character as u32) + 0x2400).unwrap_or(character);
        return StyledToken {
            token: symbol.to_string(),
            style,
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
    current_style = line_style.prefix_style;
    rendered.push_str(prefix);

    // Render tokens
    for token in row {
        let mut new_style = match token.style {
            Style::Context => ANSI_STYLE_NORMAL,
            Style::Lowlighted => ANSI_STYLE_NORMAL.with_weight(Weight::Faint),
            Style::Bright => ANSI_STYLE_NORMAL.with_weight(Weight::Bold),
            Style::DiffPartUnchanged => line_style.unchanged_style,
            Style::DiffPartMidlighted => line_style.midlighted_style,
            Style::DiffPartHighlighted => line_style.highlighted_style,
            Style::Error => ANSI_STYLE_NORMAL.with_color(Red).with_inverse(true),
        };

        if force_faint {
            new_style = new_style.with_weight(Weight::Faint);
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

/// Splits a row into (filename, timestamp) slices.
///
/// Response slices may be empty.
pub fn split_filename_and_timestamp(
    row: &mut [StyledToken],
) -> (&mut [StyledToken], &mut [StyledToken]) {
    let mut split_index = None;
    for (i, token) in row.iter().enumerate() {
        if token.token == "\t" {
            split_index = Some(i + 1);
            break;
        }
        let is_multispace = token.token.len() > 1 && token.token.chars().all(|c| c == ' ');
        if is_multispace {
            split_index = Some(i + 1);
            break;
        }
    }

    if let Some(idx) = split_index {
        let (filename, timestamp) = row.split_at_mut(idx);
        return (filename, timestamp);
    }

    (row, &mut [])
}

/// File timestamps are found after either a tab character or a double space
pub fn lowlight_timestamp(row: &mut [StyledToken]) {
    let (_filename, timestamp) = split_filename_and_timestamp(row);
    for token in timestamp {
        token.style = Style::Lowlighted;
    }
}

/// Unhighlight leading 'a/' or 'b/' in git diff file names.
///
/// They are just placeholders that do not indicate any changes introduced by
/// the user.
pub fn lowlight_git_prefix(row: &mut [StyledToken]) {
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

/// Note that the file name might end with one or more tabs followed by a
/// timestamp, so it's not really just a file name.
pub fn hyperlink_filename(row: &mut [StyledToken]) {
    let (filename_tokens, _timestamp) = split_filename_and_timestamp(row);

    // Convert filename_tokens into a String
    let mut filename = String::new();
    for token in filename_tokens.iter() {
        filename.push_str(&token.token);
    }
    let mut path = std::path::PathBuf::from(filename);

    if !path.is_absolute() {
        // FIXME: Log or ignore if current_dir is not available?
        let current_dir = std::env::current_dir().unwrap();
        path = current_dir.join(path);
    }

    if !path.exists() {
        return;
    }

    let url_string = url::Url::from_file_path(&path).ok().map(|u| u.to_string());
    if url_string.is_none() {
        return;
    }

    let url_string = url_string.unwrap();

    FIXME: Hyperlink the tokens!
}

pub fn lowlight_dev_null(row: &mut [StyledToken]) {
    if row.len() < 4 {
        return;
    }
    if row[0].token == "/" && row[1].token == "dev" && row[2].token == "/" && row[3].token == "null"
    {
        for token in row {
            token.style = Style::Lowlighted;
        }
    }
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

    const LINE_STYLE_OLD: LineStyle = {
        LineStyle {
            prefix_style: ANSI_STYLE_NORMAL.with_color(Red),
            unchanged_style: ANSI_STYLE_NORMAL.with_color(Yellow),
            midlighted_style: ANSI_STYLE_NORMAL.with_color(Red),
            highlighted_style: ANSI_STYLE_NORMAL.with_color(Red).with_inverse(true),
        }
    };

    const LINE_STYLE_NEW: LineStyle = {
        LineStyle {
            prefix_style: ANSI_STYLE_NORMAL.with_color(Green),
            unchanged_style: ANSI_STYLE_NORMAL.with_color(Yellow),
            midlighted_style: ANSI_STYLE_NORMAL.with_color(Green),
            highlighted_style: ANSI_STYLE_NORMAL.with_color(Green).with_inverse(true),
        }
    };

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
        assert_eq!(rendered, format!("{GREEN}+hej{NORMAL}\n"));
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
