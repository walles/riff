use std::cmp::{self};

use threadpool::ThreadPool;

use crate::lines_highlighter::LineAcceptance;
use crate::lines_highlighter::{LinesHighlighter, Response};
use crate::refiner::diff;
use crate::string_future::StringFuture;
use crate::token_collector::{
    render, Style, StyledToken, LINE_STYLE_NEW_FILENAME, LINE_STYLE_OLD_FILENAME,
};

use crate::hunk_header::HunkHeader;
use crate::hunk_highlighter::HunkLinesHighlighter;
use crate::refiner::Formatter;

/// Old / new path prefixes we lowlight in file headers, and strip when looking
/// for files to hyperlink.
///
/// - `a` / `b` are from `git diff`
/// - `left` / `right` are from `jj diff --tool riff`
const PATH_PREFIX_PAIRS: &[(&str, &str)] = &[("a", "b"), ("left", "right")];

pub(crate) struct FileHighlighter {
    /// May or may not end with one or more tabs + a timestamp string.
    old_name: String,

    /// May or may not end with one or more tabs + a timestamp string.
    new_name: String,

    /// Used when spawning hunk highlighters
    formatter: Formatter,

    /// Current hunk highlighter, if any
    sub_highlighter: Option<Box<dyn LinesHighlighter>>,

    /// URL to the file we're currently highlighting, if any
    url: Option<url::Url>,

    /// Set once the `--- ` / `+++ ` header has been emitted.
    ///
    /// Emitting the header is deferred by one line so that, if that next
    /// line is a hunk header, the header's own hyperlink can point at the
    /// same line as the hunk header's hyperlink.
    header_rendered: bool,
}

/// Remove trailing diff timestamp from a string, retaining only the filename
/// portion. Separator is either a tab or two spaces.
fn without_timestamp(name: &str) -> &str {
    if let Some(idx) = name.rfind('\t').or_else(|| name.rfind("  ")) {
        return &name[..idx];
    } else {
        return name;
    }
}

impl LinesHighlighter for FileHighlighter {
    fn consume_line(&mut self, line: &str, thread_pool: &ThreadPool) -> Result<Response, String> {
        assert!(!self.old_name.is_empty());

        if self.new_name.is_empty() {
            // Header phase: waiting for +++
            if let Some(new_name) = line.strip_prefix("+++ ") {
                self.new_name.push_str(new_name);
                self.url = hyperlink_filename(without_timestamp(new_name));
                return Ok(Response {
                    line_accepted: LineAcceptance::AcceptedWantMore,
                    // The header itself is rendered from the next line, once
                    // we know whether it's a hunk header to link to.
                    highlighted: vec![],
                });
            }
            return Err("--- was not followed by +++".to_string());
        }

        // We are now past the --- / +++ headers and are dealing with the body
        let mut highlights: Vec<StringFuture> = Vec::new();

        if !self.header_rendered {
            // This is the first body line. Parse it as a hunk header once,
            // both to pick the header's link line and, if it is one, to
            // become the hunk sub-highlighter below (avoiding parsing it
            // twice).
            let parsed_hunk_header = HunkHeader::parse(line);
            let first_hunk_line = match &parsed_hunk_header {
                Some(hunk_header) => Some(hunk_header.first_modified_line()?),
                None => None,
            };
            highlights.push(StringFuture::from_string(self.highlighted(first_hunk_line)));
            self.header_rendered = true;

            return match parsed_hunk_header {
                Some(hunk_header) => {
                    self.sub_highlighter = Some(Box::new(HunkLinesHighlighter::from_parsed(
                        hunk_header,
                        self.formatter.clone(),
                        &self.url,
                    )?));
                    Ok(Response {
                        line_accepted: LineAcceptance::AcceptedWantMore,
                        highlighted: highlights,
                    })
                }
                None => Ok(Response {
                    line_accepted: LineAcceptance::RejectedDone,
                    highlighted: highlights,
                }),
            };
        }

        if let Some(ref mut highlighter) = self.sub_highlighter {
            let resp = highlighter.consume_line(line, thread_pool)?;
            highlights.extend(resp.highlighted);
            match resp.line_accepted {
                LineAcceptance::AcceptedWantMore => {
                    return Ok(Response {
                        line_accepted: LineAcceptance::AcceptedWantMore,
                        highlighted: highlights,
                    });
                }
                LineAcceptance::AcceptedDone => {
                    self.sub_highlighter = None;
                    return Ok(Response {
                        line_accepted: LineAcceptance::AcceptedWantMore,
                        highlighted: highlights,
                    });
                }
                LineAcceptance::RejectedDone => {
                    self.sub_highlighter = None;

                    // Fall through here and let the outer logic do its thing
                }
            }
        }

        // Not in a sub-highlighter: look for hunk header
        if let Some(hunk_highlighter) =
            HunkLinesHighlighter::from_line(line, self.formatter.clone(), &self.url)?
        {
            self.sub_highlighter = Some(Box::new(hunk_highlighter));
            return Ok(Response {
                line_accepted: LineAcceptance::AcceptedWantMore,
                highlighted: highlights,
            });
        }

        // Otherwise we're done
        return Ok(Response {
            line_accepted: LineAcceptance::RejectedDone,
            highlighted: highlights,
        });
    }

    fn consume_eof(&mut self, thread_pool: &ThreadPool) -> Result<Vec<StringFuture>, String> {
        if self.new_name.is_empty() {
            return Err("Input ended early, --- should have been followed by +++".to_string());
        }

        let mut return_me = vec![];
        if !self.header_rendered {
            // No hunk ever showed up, e.g. an empty file being added/deleted,
            // or a truncated diff. Fall back to a plain, line-less link.
            return_me.push(StringFuture::from_string(self.highlighted(None)));
            self.header_rendered = true;
        }

        if let Some(ref mut sub) = self.sub_highlighter {
            return_me.extend(sub.consume_eof(thread_pool)?);
        }

        Ok(return_me)
    }
}

impl FileHighlighter {
    /// Create a new LinesHighlighter from a line of input.
    ///
    /// Returns None if this line doesn't start a new LinesHighlighter.
    pub(crate) fn from_line(line: &str, formatter: Formatter) -> Option<Self>
    where
        Self: Sized,
    {
        if !line.starts_with("--- ") {
            return None;
        }

        let highlighter = FileHighlighter {
            old_name: line.strip_prefix("--- ").unwrap().to_string(),
            new_name: String::new(),
            formatter,
            sub_highlighter: None,
            url: None, // Will be set in consume_line() based on the +++ line
            header_rendered: false,
        };

        return Some(highlighter);
    }

    fn highlighted(&self, first_hunk_line: Option<usize>) -> String {
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

        decorate_paths(&mut old_tokens, &mut new_tokens, first_hunk_line);

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

/// Directories to resolve relative paths against, in order of preference.
///
/// `$PWD` goes first because `jj diff --tool riff` runs us with cwd in a
/// temporary directory that jj deletes when we exit, while leaving `$PWD`
/// pointing at the user's directory. Linking into the temporary directory
/// would give dead links.
///
/// The cwd is the fallback for when `$PWD` is unset, or has gone stale for some
/// other reason. `cargo test` from a subdirectory does that for example.
fn base_dirs() -> Vec<std::path::PathBuf> {
    let mut base_dirs = Vec::new();

    if let Some(pwd) = std::env::var_os("PWD") {
        let pwd = std::path::PathBuf::from(pwd);
        if pwd.is_absolute() {
            base_dirs.push(pwd);
        }
    }

    // Empty means relative to the cwd, even if we can't tell what it is
    base_dirs.push(std::env::current_dir().unwrap_or_default());

    return base_dirs;
}

/// This function will try to find the file, either by the passed-in name or by
/// stripping any of the `PATH_PREFIX_PAIRS` prefixes.
///
/// Relative paths are looked up in each of the `base_dirs()`, trying all
/// candidates in one directory before moving on to the next.
fn hyperlink_filename(filename: &str) -> Option<url::Url> {
    let mut raw_candidates: Vec<&str> = Vec::new();
    raw_candidates.push(filename);
    for prefix in PATH_PREFIX_PAIRS.iter().flat_map(|(old, new)| [old, new]) {
        if let Some(without_prefix) = filename
            .strip_prefix(prefix)
            .and_then(|rest| rest.strip_prefix('/'))
        {
            raw_candidates.push(without_prefix);
        }
    }

    let base_dirs = base_dirs();
    let paths = base_dirs.iter().flat_map(|base_dir| {
        raw_candidates
            .iter()
            .filter(|candidate| **candidate != "/dev/null")
            // Joining an absolute path just returns that absolute path
            .map(move |candidate| base_dir.join(candidate))
    });

    for path in paths {
        if !path.exists() {
            continue;
        }

        // Try canonicalize for nicer / realpath output, fall back if it fails.
        let canonical = path.canonicalize().unwrap_or(path);
        if canonical == std::path::Path::new("/dev/null") {
            continue;
        }

        if let Ok(url) = url::Url::from_file_path(&canonical) {
            return Some(url);
        }
    }

    return None;
}

fn hyperlink_tokenized(
    just_path: &mut [StyledToken],
    just_filename: &mut [StyledToken],
    fragment_line: Option<usize>,
) {
    // Convert filename_tokens into a String
    let mut filename = String::new();
    for token in just_path.iter() {
        filename.push_str(&token.token);
    }
    for token in just_filename.iter() {
        filename.push_str(&token.token);
    }

    if let Some(mut url) = hyperlink_filename(&filename) {
        if let Some(fragment_line) = fragment_line {
            url.set_fragment(Some(&fragment_line.to_string()));
        }

        // Actually link the tokens
        for token in just_path.iter_mut() {
            token.url = Some(url.clone());
        }
        for token in just_filename.iter_mut() {
            token.url = Some(url.clone());
        }
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
/// Let's say we have a row like this and `look_for_path_prefixes` is true:
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
fn split_row<'a>(look_for_path_prefixes: bool, row: &'a mut [StyledToken]) -> SplitRow<'a> {
    let is_any_prefix = |tokens: &[StyledToken]| {
        PATH_PREFIX_PAIRS
            .iter()
            .any(|(old, new)| starts_with_prefix(tokens, old) || starts_with_prefix(tokens, new))
    };
    let path_start = if look_for_path_prefixes && is_any_prefix(row) {
        // If we have a path prefix, the path starts after the first two tokens
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

fn is_separator(token: &StyledToken) -> bool {
    return token.token == "/" || token.token == std::path::MAIN_SEPARATOR.to_string();
}

/// True if `tokens` start with `prefix` followed by a path separator
fn starts_with_prefix(tokens: &[StyledToken], prefix: &str) -> bool {
    return tokens.len() >= 2 && tokens[0].token == prefix && is_separator(&tokens[1]);
}

/// True if both paths have matching prefixes from `PATH_PREFIX_PAIRS`, like
/// "a/..." and "b/...". An absolute path, like "/dev/null", matches any
/// prefix.
fn have_path_prefixes(old_tokens: &[StyledToken], new_tokens: &[StyledToken]) -> bool {
    let old_is_absolute = old_tokens.first().is_some_and(is_separator);
    let new_is_absolute = new_tokens.first().is_some_and(is_separator);

    return PATH_PREFIX_PAIRS.iter().any(|(old_prefix, new_prefix)| {
        (old_is_absolute || starts_with_prefix(old_tokens, old_prefix))
            && (new_is_absolute || starts_with_prefix(new_tokens, new_prefix))
    });
}

pub(crate) fn decorate_paths(
    old_tokens: &mut [StyledToken],
    new_tokens: &mut [StyledToken],
    fragment_line: Option<usize>,
) {
    let look_for_path_prefixes = have_path_prefixes(old_tokens, new_tokens);

    let old_split = split_row(look_for_path_prefixes, old_tokens);
    let new_split = split_row(look_for_path_prefixes, new_tokens);

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
        hyperlink_tokenized(old_split.just_path, old_split.just_filename, fragment_line);
    }
    hyperlink_tokenized(new_split.just_path, new_split.just_filename, fragment_line);

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

    // Lowlight path prefixes
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

    use crate::refiner::tests::FORMATTER;
    fn highlight_header_lines(old_line: &str, new_line: &str) -> String {
        let thread_pool = ThreadPool::new(1);
        let mut test_me = FileHighlighter::from_line(old_line, FORMATTER.clone()).unwrap();
        let response = test_me.consume_line(new_line, &thread_pool).unwrap();
        assert_eq!(LineAcceptance::AcceptedWantMore, response.line_accepted);
        assert_eq!(0, response.highlighted.len());

        // The header is rendered once we know whether the next line is a
        // hunk header. These tests have no next line, so EOF flushes it.
        let eof_highlighted = test_me.consume_eof(&thread_pool).unwrap();
        assert_eq!(1, eof_highlighted.len());

        let highlighted = eof_highlighted
            .into_iter()
            .next()
            .unwrap()
            .get()
            .to_string();
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
    fn test_header_with_timestamp_should_hyperlink() {
        // Desired behavior: timestamps in --- / +++ lines should NOT block
        // hyperlink creation. Currently this will FAIL because url stays None.
        let mut test_me =
            FileHighlighter::from_line("--- README.md\t2024-01-01 12:00:00", FORMATTER.clone())
                .unwrap();
        let response = test_me
            .consume_line("+++ README.md\t2024-01-02 12:00:00", &ThreadPool::new(1))
            .unwrap();
        assert_eq!(LineAcceptance::AcceptedWantMore, response.line_accepted);

        let path = test_me
            .url
            .unwrap()
            .to_file_path()
            .expect("Hyperlink should be a file path");
        let canonical = std::fs::canonicalize(&path).expect("Path should canonicalize");
        let expected = std::fs::canonicalize("README.md").expect("README.md should exist");
        assert_eq!(canonical, expected, "Hyperlink should point to README.md");
    }

    /// Clicking the `--- ` / `+++ ` header should land in the same place as
    /// clicking the first section header below it.
    #[test]
    fn test_header_links_to_first_hunk_line() {
        let thread_pool = ThreadPool::new(1);
        let mut test_me = FileHighlighter::from_line("--- README.md", FORMATTER.clone()).unwrap();

        test_me.consume_line("+++ README.md", &thread_pool).unwrap();
        let response = test_me
            .consume_line("@@ -10,3 +10,3 @@ Some section", &thread_pool)
            .unwrap();

        let mut highlighted = String::new();
        for mut future in response.highlighted {
            highlighted.push_str(future.get());
        }

        // The section header links to the start line plus three context lines,
        // and the file header should agree with it.
        assert!(
            highlighted.contains("README.md#13"),
            "Expected a hyperlink to line 13 of README.md, got: {:?}",
            highlighted
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
    fn test_brighten_filename_left_right_prefixes() {
        // jj uses left/ and right/ when running riff as its diff tool:
        // https://github.com/walles/riff/issues/82
        let highlighted = highlight_header_lines("--- left/x/y/z.txt", "+++ right/x/y/z.txt");
        assert_eq!(
            format!(
                "\
                {BOLD}--- {NORMAL_INTENSITY}{FAINT}left/{NORMAL}x/y/{BOLD}z.txt{NORMAL}\n\
                {BOLD}+++ {NORMAL_INTENSITY}{FAINT}right/{NORMAL}x/y/{BOLD}z.txt{NORMAL}\n"
            ),
            highlighted
        );
    }

    #[test]
    fn test_mismatched_prefixes_are_highlighted() {
        // a/ only goes with b/, and left/ only goes with right/
        let highlighted = highlight_header_lines("--- a/z.txt", "+++ right/z.txt");
        assert_eq!(
            format!(
                "\
                {BOLD}--- {INVERSE_VIDEO}{NORMAL_INTENSITY}{OLD}a{NORMAL}/{BOLD}z.txt{NORMAL}\n\
                {BOLD}+++ {INVERSE_VIDEO}{NORMAL_INTENSITY}{GREEN}right{NORMAL}/{BOLD}z.txt{NORMAL}\n"
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
        hyperlink_tokenized(&mut [], &mut row, None);

        // Assert: the file:/// URL points to our README.md file
        let url = row[0].url.as_ref().expect("Token should have a URL");
        let url_path = url.to_file_path().expect("URL should be a file path");
        let url_canon = std::fs::canonicalize(&url_path).expect("URL file should exist");
        let readme_canon = std::fs::canonicalize("README.md").expect("README.md should exist");
        assert_eq!(url_canon, readme_canon, "Canonicalized paths should match");
    }

    #[test]
    fn test_hyperlink_filename_happy_path() {
        // Arrange: ensure the relative path exists (defensive check)
        assert!(
            std::path::Path::new("README.md").exists(),
            "README.md should exist in crate root"
        );

        // Act: call the function directly
        let url_opt = super::hyperlink_filename("README.md");

        // Assert: we got Some(url) and it resolves back to the same file
        assert!(
            url_opt.is_some(),
            "Expected Some(url::Url) for existing relative path"
        );
        let url = url_opt.unwrap();
        assert_eq!(url.scheme(), "file", "Scheme should be file");
        let path_from_url = url.to_file_path().expect("Should convert URL back to path");
        assert!(path_from_url.exists(), "Path from URL should exist");

        let expected = std::fs::canonicalize("README.md").expect("Canonicalize README.md");
        let actual = std::fs::canonicalize(&path_from_url).expect("Canonicalize URL path");
        assert_eq!(actual, expected, "Canonical paths must match");
    }

    #[test]
    fn test_hyperlink_filename_missing_file() {
        // Act: call the function directly
        let url_opt = super::hyperlink_filename("does-not-exist.txt");

        // Assert: we got Some(url) and it resolves back to the same file
        assert!(
            url_opt.is_none(),
            "Expected None for non-existing relative path"
        );
    }

    #[test]
    fn test_hyperlink_filename_git_prefix() {
        // Arrange: ensure the relative path exists (defensive check)
        assert!(
            std::path::Path::new("README.md").exists(),
            "README.md should exist in crate root"
        );

        // Act: call the function with git-style prefix
        let url_opt = super::hyperlink_filename("a/README.md");

        // Assert: we got Some(url) and it resolves back to the same file
        assert!(
            url_opt.is_some(),
            "Expected Some(url::Url) for git-style prefixed path"
        );
        let url = url_opt.unwrap();
        assert_eq!(url.scheme(), "file", "Scheme should be file");
        let path_from_url = url.to_file_path().expect("Should convert URL back to path");
        assert!(path_from_url.exists(), "Path from URL should exist");

        let expected = std::fs::canonicalize("README.md").expect("Canonicalize README.md");
        let actual = std::fs::canonicalize(&path_from_url).expect("Canonicalize URL path");
        assert_eq!(actual, expected, "Canonical paths must match");
    }

    #[test]
    fn test_hyperlink_filename_right_prefix() {
        let url_opt = super::hyperlink_filename("right/README.md");

        let path_from_url = url_opt
            .expect("Expected Some(url::Url) for right/ prefixed path")
            .to_file_path()
            .expect("Should convert URL back to path");
        let expected = std::fs::canonicalize("README.md").expect("Canonicalize README.md");
        let actual = std::fs::canonicalize(&path_from_url).expect("Canonicalize URL path");
        assert_eq!(actual, expected, "Canonical paths must match");
    }

    #[test]
    fn test_split_row_with_multi_character_prefix() {
        let mut tokens: Vec<StyledToken> = ["left", "/", "doc", "/", "c.txt"]
            .iter()
            .map(|s| StyledToken::new(s.to_string(), Style::Context))
            .collect();
        let split = split_row(true, &mut tokens);
        assert_eq!(
            split.prefix.iter().map(|t| &t.token).collect::<Vec<_>>(),
            ["left", "/"]
        );
        assert_eq!(
            split.just_path.iter().map(|t| &t.token).collect::<Vec<_>>(),
            ["doc", "/"]
        );
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
