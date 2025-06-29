use std::cmp;

use threadpool::ThreadPool;

use crate::lines_highlighter::LineAcceptance;
use crate::lines_highlighter::{LinesHighlighter, Response};
use crate::refiner::diff;
use crate::string_future::StringFuture;
use crate::token_collector::{
    render, Style, StyledToken, LINE_STYLE_NEW_FILENAME, LINE_STYLE_OLD_FILENAME,
};

pub(crate) struct PlusMinusHeaderHighlighter {
    /// May or may not end with one or more tabs + a timestamp string.
    old_name: String,

    /// May or may not end with one or more tabs + a timestamp string.
    new_name: String,
}

impl LinesHighlighter for PlusMinusHeaderHighlighter {
    fn consume_line(&mut self, line: &str, _thread_pool: &ThreadPool) -> Result<Response, String> {
        assert!(!self.old_name.is_empty());
        assert!(self.new_name.is_empty());

        if let Some(new_name) = line.strip_prefix("+++ ") {
            self.new_name.push_str(new_name);
            return Ok(Response {
                line_accepted: LineAcceptance::AcceptedDone,
                highlighted: vec![StringFuture::from_string(self.highlighted())],
            });
        }

        return Err("--- was not followed by +++".to_string());
    }

    fn consume_eof(&mut self, _thread_pool: &ThreadPool) -> Result<Vec<StringFuture>, String> {
        return Err("Input ended early, --- should have been followed by +++".to_string());
    }
}

impl PlusMinusHeaderHighlighter {
    /// Create a new LinesHighlighter from a line of input.
    ///
    /// Returns None if this line doesn't start a new LinesHighlighter.
    pub(crate) fn from_line(line: &str) -> Option<Self>
    where
        Self: Sized,
    {
        if !line.starts_with("--- ") {
            return None;
        }

        let highlighter = PlusMinusHeaderHighlighter {
            old_name: line.strip_prefix("--- ").unwrap().to_string(),
            new_name: String::new(),
        };

        return Some(highlighter);
    }

    fn highlighted(&self) -> String {
        let (mut old_tokens, mut new_tokens) = diff(&self.old_name, &self.new_name);

        // New file
        if self.old_name == "/dev/null" {
            // Don't diff highlight vs "/dev/null"
            for token in &mut new_tokens {
                token.style = Style::Context;
            }
        }
        let new_prefix = if self.old_name == "/dev/null" {
            Some(StyledToken::new("NEW ".to_string(), Style::Bright))
        } else {
            None
        };

        // Deleted file
        if self.new_name == "/dev/null" {
            // Don't diff highlight vs "/dev/null"
            for token in &mut old_tokens {
                token.style = Style::Context;
            }
        }
        let old_prefix = if self.new_name == "/dev/null" {
            Some(StyledToken::new("DELETED ".to_string(), Style::Bright))
        } else {
            None
        };

        brighten_filename(&mut old_tokens);
        brighten_filename(&mut new_tokens);

        // If the old filename is not the same as the new, it means it's gone,
        // and any file we link to is likely to be the wrong one. So we only
        // hyperlink the old name if it is the same as the new name.
        if old_tokens == new_tokens {
            hyperlink_filename(&mut old_tokens);
        }
        hyperlink_filename(&mut new_tokens);

        lowlight_dev_null(&mut old_tokens);
        lowlight_dev_null(&mut new_tokens);

        lowlight_timestamp(&mut old_tokens);
        lowlight_timestamp(&mut new_tokens);

        align_tabs(&mut old_tokens, &mut new_tokens);

        lowlight_git_prefix(&mut old_tokens);
        lowlight_git_prefix(&mut new_tokens);

        if let Some(prefix) = new_prefix {
            new_tokens.insert(0, prefix);
        }

        if let Some(prefix) = old_prefix {
            old_tokens.insert(0, prefix);
        }

        let old_filename = render(&LINE_STYLE_OLD_FILENAME, "--- ", &old_tokens);
        let new_filename = render(&LINE_STYLE_NEW_FILENAME, "+++ ", &new_tokens);

        let mut highlighted = String::new();
        highlighted.push_str(&old_filename);
        highlighted.push('\n');
        highlighted.push_str(&new_filename);
        highlighted.push('\n');

        return highlighted;
    }
}

/// If we get "x/y/z.txt", make "z.txt" bright.
///
/// As an exception, if the file name is already highlighted, don't brighten it.
fn brighten_filename(row: &mut [StyledToken]) {
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
fn hyperlink_filename(row: &mut [StyledToken]) {
    let (filename_tokens, _timestamp) = split_filename_and_timestamp(row);

    // Convert filename_tokens into a String
    let mut filename = String::new();
    for token in filename_tokens.iter() {
        filename.push_str(&token.token);
    }

    if filename == "/dev/null" {
        // Hyperlinking /dev/null would just be confusing
        return;
    }

    let mut path = std::path::PathBuf::from(filename);

    // from_file_path() below requires an absolute path
    if !path.is_absolute() {
        // FIXME: Log or ignore if current_dir is not available?
        let current_dir = std::env::current_dir().unwrap();
        path = current_dir.join(path);
    }

    if !path.exists() {
        return;
    }

    let url = url::Url::from_file_path(&path).ok();
    if url.is_none() {
        // If we get here, maybe the absolutization under path.isAbsolute() a
        // few lines up failed?
        return;
    }

    let url = url.unwrap();

    // Actually link the tokens
    for token in filename_tokens.iter_mut() {
        token.url = Some(url.clone());
    }
}

fn lowlight_dev_null(row: &mut [StyledToken]) {
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

/// Splits a row into (filename, timestamp) slices.
///
/// Response slices may be empty.
fn split_filename_and_timestamp(
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
fn lowlight_timestamp(row: &mut [StyledToken]) {
    let (_filename, timestamp) = split_filename_and_timestamp(row);
    for token in timestamp {
        token.style = Style::Lowlighted;
    }
}

fn align_tabs(old: &mut [StyledToken], new: &mut [StyledToken]) {
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

/// Unhighlight leading 'a/' or 'b/' in git diff file names.
///
/// They are just placeholders that do not indicate any changes introduced by
/// the user.
fn lowlight_git_prefix(row: &mut [StyledToken]) {
    if row.len() < 2 {
        return;
    }

    if (row[0].token == "a" || row[0].token == "b") && row[1].token == "/" {
        row[0].style = Style::Lowlighted;
        row[1].style = Style::Lowlighted;
    }
}

#[cfg(test)]
mod tests {
    use crate::ansi::without_ansi_escape_codes;
    use crate::constants::*;

    use super::*;

    const NOT_INVERSE_VIDEO: &str = "\x1b[27m";
    const DEFAULT_COLOR: &str = "\x1b[39m";

    fn highlight_header_lines(old_line: &str, new_line: &str) -> String {
        let mut test_me = PlusMinusHeaderHighlighter::from_line(old_line).unwrap();
        let mut response = test_me.consume_line(new_line, &ThreadPool::new(1)).unwrap();
        assert_eq!(LineAcceptance::AcceptedDone, response.line_accepted);
        assert_eq!(1, response.highlighted.len());

        let highlighted = response.highlighted[0].get().to_string();
        return highlighted;
    }

    #[test]
    fn test_align_timestamps() {
        let highlighted = highlight_header_lines(
            "--- x.txt\t2023-12-15 15:43:29",
            "+++ /Users/johan/src/riff/README.md\t2024-01-29 14:56:40",
        );
        let highlighted_bytes = highlighted.clone().into_bytes();
        let plain = String::from_utf8(without_ansi_escape_codes(&highlighted_bytes)).unwrap();

        assert_eq!(
            "--- x.txt                            2023-12-15 15:43:29\n\
            +++ /Users/johan/src/riff/README.md  2024-01-29 14:56:40\n",
            plain.as_str()
        );
    }

    #[test]
    fn test_brighten_filename() {
        let highlighted = highlight_header_lines("--- a/x/y/z.txt", "+++ b/x/y/z.txt");
        assert_eq!(
            format!(
                "\
                {BOLD}--- {NORMAL_INTENSITY}{FAINT}a/{NORMAL}x/y/{BOLD}z.txt{NORMAL}\n\
                {BOLD}+++ {NORMAL_INTENSITY}{FAINT}b/{NORMAL}x/y/{BOLD}z.txt{NORMAL}\n"
            ),
            highlighted
        );
    }

    #[test]
    fn test_brighten_filename_without_path() {
        let highlighted = highlight_header_lines("--- z.txt", "+++ z.txt");
        assert_eq!(
            format!(
                "\
                {BOLD}--- z.txt{NORMAL}\n\
                {BOLD}+++ z.txt{NORMAL}\n"
            ),
            highlighted
        );
    }

    #[test]
    fn test_brighten_file_rename() {
        let highlighted = highlight_header_lines("--- x.txt", "+++ y.txt");
        assert_eq!(
            format!(
                "\
                {BOLD}--- {INVERSE_VIDEO}{NORMAL_INTENSITY}{OLD}x{NOT_INVERSE_VIDEO}{BOLD}{DEFAULT_COLOR}.txt{NORMAL}\n\
                {BOLD}+++ {INVERSE_VIDEO}{NORMAL_INTENSITY}{GREEN}y{NOT_INVERSE_VIDEO}{BOLD}{DEFAULT_COLOR}.txt{NORMAL}\n"
            ),
            highlighted
        );
    }

    #[test]
    fn test_new_file_header() {
        let highlighted = highlight_header_lines("--- /dev/null", "+++ b/newfile.txt");
        assert_eq!(
            format!(
                "\
                {BOLD}--- {NORMAL_INTENSITY}{FAINT}/dev/null{NORMAL}\n\
                {BOLD}+++ NEW {NORMAL_INTENSITY}{FAINT}b/{NORMAL_INTENSITY}{BOLD}newfile.txt{NORMAL}\n"
            ),
            highlighted
        );
    }

    #[test]
    fn test_deleted_file_header() {
        let highlighted = highlight_header_lines("--- a/oldfile.txt", "+++ /dev/null");
        assert_eq!(
            format!(
                "\
                {BOLD}--- DELETED {NORMAL_INTENSITY}{FAINT}a/{NORMAL_INTENSITY}{BOLD}oldfile.txt{NORMAL}\n\
                {BOLD}+++ {NORMAL_INTENSITY}{FAINT}/dev/null{NORMAL}\n"
            ),
            highlighted
        );
    }

    #[test]
    fn test_hyperlink_filename_relative_path() {
        // Arrange: create a row representing "README.md"
        let mut row = vec![StyledToken::new("README.md".to_string(), Style::Context)];

        // Act: call the function
        hyperlink_filename(&mut row);

        // Assert: the file:/// URL points to our README.md file
        let url = row[0].url.as_ref().expect("Token should have a URL");
        let url_path = url.to_file_path().expect("URL should be a file path");
        let url_canon = std::fs::canonicalize(&url_path).expect("URL file should exist");
        let readme_canon = std::fs::canonicalize("README.md").expect("README.md should exist");
        assert_eq!(url_canon, readme_canon, "Canonicalized paths should match");
    }
}
