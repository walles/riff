use std::cmp::{self};

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

        decorate_paths(&mut old_tokens, &mut new_tokens);

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

fn hyperlink_filename(just_path: &mut [StyledToken], just_filename: &mut [StyledToken]) {
    // Convert filename_tokens into a String
    let mut filename = String::new();
    for token in just_path.iter() {
        filename.push_str(&token.token);
    }
    for token in just_filename.iter() {
        filename.push_str(&token.token);
    }

    if filename == "/dev/null" {
        // Hyperlinking /dev/null would just be confusing
        return;
    }

    let mut path = std::path::PathBuf::from(filename);

    // from_file_path() below requires an absolute path
    if !path.is_absolute() {
        let maybe_current_dir = std::env::current_dir();
        if let Ok(current_dir) = maybe_current_dir {
            path = current_dir.join(path);
        } else {
            // Getting the current directory failed, we can't hyperlink
            return;
        }
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
    for token in just_path.iter_mut() {
        token.url = Some(url.clone());
    }
    for token in just_filename.iter_mut() {
        token.url = Some(url.clone());
    }
}

fn lowlight_dev_null(just_path: &mut [StyledToken], just_filename: &mut [StyledToken]) {
    if just_path.len() < 3 {
        // Expected "/dev/"
        return;
    }
    if just_filename.len() != 1 {
        // Expected "null"
        return;
    }

    if just_path[0].token == "/"
        && just_path[1].token == "dev"
        && just_path[2].token == "/"
        && just_filename[0].token == "null"
    {
        for token in just_path {
            token.style = Style::Lowlighted;
        }
        for token in just_filename {
            token.style = Style::Lowlighted;
        }
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

struct SplitRow<'a> {
    prefix: &'a mut [StyledToken],
    just_path: &'a mut [StyledToken],
    just_filename: &'a mut [StyledToken],
    time_space: &'a mut [StyledToken],
    timestamp: &'a mut [StyledToken],
}

/// Splits a row into (prefix, just_path, full_path, just_filename, timestamp) slices.
///
/// Let's say we have a row like this and `look_for_git_prefixes` is true:
/// ```
/// a/doc/c.txt\t2023-12-15 15:43:29
/// ```
///
/// Then the result will be:
/// - `prefix`: `a/`
/// - `just_path`: `doc/`
/// - `just_filename`: `c.txt`
/// - `time_space`: `\t`
/// - `timestamp`: `2023-12-15 15:43:29`
fn split_row<'a>(look_for_git_prefixes: bool, row: &'a mut [StyledToken]) -> SplitRow<'a> {
    let path_start = if look_for_git_prefixes
        && row.len() >= 2
        && row[0].token.len() == 1 // "a" or "b"
        && (row[1].token == "/" || row[1].token == std::path::MAIN_SEPARATOR.to_string())
    {
        // If we have a git prefix, the path starts after the first two tokens
        2
    } else {
        // Otherwise, it starts at the first token
        0
    };

    // Path ends where the timestamp starts, or at the end of the row if there is no timestamp
    let path_end = row
        .iter()
        .position(|token| token.token == "\t" || token.token.chars().all(char::is_whitespace))
        .unwrap_or(row.len());

    let timestamp_start = cmp::min(path_end + 1, row.len());

    let last_file_separator_index_from_path_start =
        row[path_start..path_end].iter().rposition(|token| {
            token.token == "/" || token.token == std::path::MAIN_SEPARATOR.to_string()
        });

    // To avoid multiple mutable borrows, split step by step
    let (row, timestamp) = row.split_at_mut(timestamp_start);
    let (prefix, rest) = row.split_at_mut(path_start);
    let (path_and_filename, space_plus_timestamp) = rest.split_at_mut(path_end - path_start);
    let time_space = if space_plus_timestamp.is_empty() {
        &mut []
    } else {
        // If there is a space + timestamp, it should be the last token
        &mut space_plus_timestamp[..1]
    };
    let (just_path, just_filename) = if let Some(last_file_separator_index_from_path_start) =
        last_file_separator_index_from_path_start
    {
        path_and_filename.split_at_mut(last_file_separator_index_from_path_start + 1)
    } else {
        path_and_filename.split_at_mut(0)
    };

    return SplitRow {
        prefix,
        just_path,
        just_filename,
        time_space,
        timestamp,
    };
}

fn have_git_prefixes(old_tokens: &[StyledToken], new_tokens: &[StyledToken]) -> bool {
    // Both "a/..." and "/dev/null" count as having a git prefix.

    let old_has_git_prefix = old_tokens.len() >= 2
        && old_tokens[0].token == "a"
        && (old_tokens[1].token == "/"
            || old_tokens[1].token == std::path::MAIN_SEPARATOR.to_string());
    let old_is_absolute = !old_tokens.is_empty() && old_tokens[0].token == "/"
        || old_tokens[0].token == std::path::MAIN_SEPARATOR.to_string();

    let new_has_git_prefix = new_tokens.len() >= 2
        && new_tokens[0].token == "b"
        && (new_tokens[1].token == "/"
            || new_tokens[1].token == std::path::MAIN_SEPARATOR.to_string());
    let new_is_absolute = !new_tokens.is_empty() && new_tokens[0].token == "/"
        || new_tokens[0].token == std::path::MAIN_SEPARATOR.to_string();

    return (old_has_git_prefix || old_is_absolute) && (new_has_git_prefix || new_is_absolute);
}

fn decorate_paths(old_tokens: &mut [StyledToken], new_tokens: &mut [StyledToken]) {
    let look_for_git_prefixes = have_git_prefixes(old_tokens, new_tokens);

    let old_split = split_row(look_for_git_prefixes, old_tokens);
    let new_split = split_row(look_for_git_prefixes, new_tokens);

    // Brighten file names
    old_split.just_filename.iter_mut().for_each(|token| {
        if token.style != Style::DiffPartHighlighted {
            token.style = Style::Bright;
        }
    });
    new_split.just_filename.iter_mut().for_each(|token| {
        if token.style != Style::DiffPartHighlighted {
            token.style = Style::Bright;
        }
    });

    // If the old filename is not the same as the new, it means it's gone,
    // and any file we link to is likely to be the wrong one. So we only
    // hyperlink the old name if it is the same as the new name.
    if old_split.just_path == new_split.just_path
        && old_split.just_filename == new_split.just_filename
    {
        hyperlink_filename(old_split.just_path, old_split.just_filename);
    }
    hyperlink_filename(new_split.just_path, new_split.just_filename);

    lowlight_dev_null(old_split.just_path, old_split.just_filename);
    lowlight_dev_null(new_split.just_path, new_split.just_filename);

    // Plain the spaces before the time stamps
    old_split.time_space.iter_mut().for_each(|token| {
        token.style = Style::Context;
    });
    new_split.time_space.iter_mut().for_each(|token| {
        token.style = Style::Context;
    });

    // Lowlight time stamps
    old_split.timestamp.iter_mut().for_each(|token| {
        token.style = Style::Lowlighted;
    });
    new_split.timestamp.iter_mut().for_each(|token| {
        token.style = Style::Lowlighted;
    });

    // Lowlight git prefixes
    old_split.prefix.iter_mut().for_each(|token| {
        token.style = Style::Lowlighted;
    });
    new_split.prefix.iter_mut().for_each(|token| {
        token.style = Style::Lowlighted;
    });

    align_tabs(old_tokens, new_tokens);
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
            "+++ /Users/johan/xsrc/riff/README.md\t2024-01-29 14:56:40",
        );
        let highlighted_bytes = highlighted.clone().into_bytes();
        let plain = String::from_utf8(without_ansi_escape_codes(&highlighted_bytes)).unwrap();

        assert_eq!(
            "--- x.txt                             2023-12-15 15:43:29\n\
             +++ /Users/johan/xsrc/riff/README.md  2024-01-29 14:56:40\n",
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
        hyperlink_filename(&mut [], &mut row);

        // Assert: the file:/// URL points to our README.md file
        let url = row[0].url.as_ref().expect("Token should have a URL");
        let url_path = url.to_file_path().expect("URL should be a file path");
        let url_canon = std::fs::canonicalize(&url_path).expect("URL file should exist");
        let readme_canon = std::fs::canonicalize("README.md").expect("README.md should exist");
        assert_eq!(url_canon, readme_canon, "Canonicalized paths should match");
    }

    #[test]
    fn test_split_row_with_slash_separator() {
        let mut tokens: Vec<StyledToken> = ["doc", "/", "c.txt", "\t", "2023-12-15 15:43:29"]
            .iter()
            .map(|s| StyledToken::new(s.to_string(), Style::Context))
            .collect();
        let split = split_row(false, &mut tokens);
        assert_eq!(split.prefix.len(), 0);
        assert_eq!(
            split.just_path.iter().map(|t| &t.token).collect::<Vec<_>>(),
            ["doc", "/"]
        );
        assert_eq!(
            split
                .just_filename
                .iter()
                .map(|t| &t.token)
                .collect::<Vec<_>>(),
            ["c.txt"]
        );
        assert_eq!(
            split.time_space,
            [StyledToken::new("\t".to_string(), Style::Context)],
        );
        assert_eq!(
            split.timestamp.iter().map(|t| &t.token).collect::<Vec<_>>(),
            ["2023-12-15 15:43:29"]
        );
    }

    #[test]
    fn test_split_row_with_os_separator() {
        let sep = std::path::MAIN_SEPARATOR.to_string();
        let mut tokens: Vec<StyledToken> = [
            "a",
            &sep,
            "doc",
            &sep,
            "c.txt",
            // Time separator can be either a tab or two or more spaces
            "  ",
            "2023-12-15 15:43:29",
        ]
        .iter()
        .map(|s| StyledToken::new(s.to_string(), Style::Context))
        .collect();
        let split = split_row(true, &mut tokens);
        assert_eq!(
            split.prefix.iter().map(|t| &t.token).collect::<Vec<_>>(),
            ["a", &sep]
        );
        assert_eq!(
            split.just_path.iter().map(|t| &t.token).collect::<Vec<_>>(),
            ["doc", &sep]
        );
        assert_eq!(
            split
                .just_filename
                .iter()
                .map(|t| &t.token)
                .collect::<Vec<_>>(),
            ["c.txt"]
        );
        assert_eq!(
            split.time_space,
            [StyledToken::new("  ".to_string(), Style::Context)],
        );
        assert_eq!(
            split.timestamp.iter().map(|t| &t.token).collect::<Vec<_>>(),
            ["2023-12-15 15:43:29"]
        );
    }

    #[test]
    fn test_split_row_timeless() {
        let mut tokens: Vec<StyledToken> = ["doc", "/", "c.txt"]
            .iter()
            .map(|s| StyledToken::new(s.to_string(), Style::Context))
            .collect();
        let split = split_row(false, &mut tokens);
        assert_eq!(split.prefix.len(), 0);
        assert_eq!(
            split.just_path.iter().map(|t| &t.token).collect::<Vec<_>>(),
            ["doc", "/"]
        );
        assert_eq!(
            split
                .just_filename
                .iter()
                .map(|t| &t.token)
                .collect::<Vec<_>>(),
            ["c.txt"]
        );
        assert_eq!(split.time_space, []);
        assert_eq!(split.timestamp, []);
    }

    #[test]
    fn test_split_row_pathless() {
        let mut tokens: Vec<StyledToken> = ["README.md", "\t", "2023-12-15 15:43:29"]
            .iter()
            .map(|s| StyledToken::new(s.to_string(), Style::Context))
            .collect();
        let split = split_row(false, &mut tokens);
        assert_eq!(split.prefix.len(), 0);
        assert_eq!(split.just_path.len(), 0);
        assert_eq!(
            split
                .just_filename
                .iter()
                .map(|t| &t.token)
                .collect::<Vec<_>>(),
            ["README.md"]
        );
        assert_eq!(
            split.time_space,
            [StyledToken::new("\t".to_string(), Style::Context)],
        );
        assert_eq!(
            split.timestamp.iter().map(|t| &t.token).collect::<Vec<_>>(),
            ["2023-12-15 15:43:29"]
        );
    }

    #[test]
    fn test_split_row_separator_without_time() {
        let mut tokens: Vec<StyledToken> = ["README.md", "\t"]
            .iter()
            .map(|s| StyledToken::new(s.to_string(), Style::Context))
            .collect();
        let split = split_row(false, &mut tokens);
        assert_eq!(split.prefix.len(), 0);
        assert_eq!(split.just_path.len(), 0);
        assert_eq!(
            split
                .just_filename
                .iter()
                .map(|t| &t.token)
                .collect::<Vec<_>>(),
            ["README.md"]
        );
        assert_eq!(
            split.time_space,
            [StyledToken::new("\t".to_string(), Style::Context)],
        );
        assert_eq!(split.timestamp.len(), 0);
    }
}
