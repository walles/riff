use std::cmp::{max, min};

use crate::tokenizer;
use crate::{
    constants::*,
    token_collector::{Style, StyledToken, TokenCollector},
};
use diffus::{
    edit::{self, collection},
    Diffable,
};

/// If more than this percentage of either adds or moves is highlighted, we
/// consider it to be a replacement rather than a move, and skip highlighting
/// it.
const MAX_HIGHLIGHT_PERCENTAGE: usize = 30;

const LARGE_COUNT_CHANGE_PERCENT: usize = 100;
const SMALL_COUNT_CHANGE: usize = 10;

/// Like format!(), but faster for our special case
fn format_simple_line(old_new: &str, plus_minus: char, contents: &str) -> String {
    let mut line = String::with_capacity(old_new.len() + 1 + contents.len() + NORMAL.len());
    line.push_str(old_new);
    line.push(plus_minus);
    line.push_str(contents);
    line.push_str(NORMAL);
    return line;
}

/// Format old and new lines in OLD and NEW colors.
///
/// No intra-line refinement.
///
/// Returns one old and one new line array.
#[must_use]
fn simple_format(old_text: &str, new_text: &str) -> (Vec<String>, Vec<String>) {
    let mut old_lines: Vec<String> = Vec::new();
    let mut new_lines: Vec<String> = Vec::new();

    for old_line in old_text.lines() {
        // Use a specialized line formatter since this code is in a hot path
        old_lines.push(format_simple_line(OLD, '-', old_line));
    }
    if (!old_text.is_empty()) && !old_text.ends_with('\n') {
        old_lines.push(format!(
            "{}{}{}",
            NO_EOF_NEWLINE_COLOR, NO_EOF_NEWLINE_MARKER, NORMAL
        ));
    }

    for add_line in new_text.lines() {
        // Use a specialized line formatter since this code is in a hot path
        new_lines.push(format_simple_line(NEW, '+', add_line));
    }
    if (!new_text.is_empty()) && !new_text.ends_with('\n') {
        new_lines.push(format!(
            "{}{}{}",
            NO_EOF_NEWLINE_COLOR, NO_EOF_NEWLINE_MARKER, NORMAL
        ));
    }

    return (old_lines, new_lines);
}

/// Returns the last byte index of the nth line of the given string.
fn last_byte_index_of_nth_line(text: &str, line_count: usize) -> usize {
    let mut newlines_found: usize = 0;
    for (byte_index, c) in text.char_indices() {
        if c != '\n' {
            continue;
        }

        // Newline found
        newlines_found += 1;
        if line_count == newlines_found {
            return byte_index;
        }
    }

    panic!("Line {} not found in \n{}", line_count, text);
}

#[must_use]
fn extract_initial_lines(count: usize, text: &str) -> &str {
    let initial_lines_last_offset = last_byte_index_of_nth_line(text, count);
    let remaining_lines_first_offset = initial_lines_last_offset + 1;
    let initial_lines = &text[0..remaining_lines_first_offset];
    return initial_lines;
}

#[must_use]
fn extract_trailing_lines(count: usize, text: &str) -> &str {
    let newline_count: usize = text.matches('\n').count();
    let count_from_start = newline_count - count; // FIXME: Validate this maths with a test

    let initial_lines_last_offset = last_byte_index_of_nth_line(text, count_from_start);
    let trailing_lines_first_offset = initial_lines_last_offset + 1;
    let trailing_lines = &text[trailing_lines_first_offset..];
    return trailing_lines;
}

/// If old and new have different amounts of lines, try matching the shorter one
/// vs the start and end of the longer one and use highlights from one of those.
///
/// Test case: testdata/partial-refine.diff
#[must_use]
fn partial_format(old_text: &str, new_text: &str) -> (Vec<String>, Vec<String>) {
    let old_linecount = old_text.lines().count();
    let new_linecount = new_text.lines().count();

    if !(old_text.ends_with('\n') && new_text.ends_with('\n')) {
        // FIXME: Write tests for and handle these cases rather than just
        // bailing here by doing simple_format()
        return simple_format(old_text, new_text);
    }

    if new_linecount > old_linecount {
        // First, try old text vs start of new text
        let initial_new_text = extract_initial_lines(old_linecount, new_text);
        if let Some((formatted_old_lines, mut formatted_initial_new_lines)) =
            format_split(old_text, initial_new_text)
        {
            // Got a refinement of the old text vs start of new_text

            // Extract the remaining lines from new_text
            let remaining_new_text = &new_text[initial_new_text.len()..];
            let (_, mut formatted_remaining_new_lines) = simple_format("", remaining_new_text);

            let mut new_lines: Vec<String> = Vec::new();
            new_lines.append(&mut formatted_initial_new_lines);
            new_lines.append(&mut formatted_remaining_new_lines);

            return (formatted_old_lines, new_lines);
        }

        // Try old text vs end of new text
        let trailing_new_text: &str = extract_trailing_lines(old_linecount, new_text);
        if let Some((formatted_old_lines, mut formatted_trailing_new_lines)) =
            format_split(old_text, trailing_new_text)
        {
            // Got a refinement of the old text vs end of new_text

            // Extract the remaining lines from new_text
            let initial_new_text: &str = &new_text[0..(new_text.len() - trailing_new_text.len())];
            let (_, mut formatted_initial_new_lines) = simple_format("", initial_new_text);

            let mut new_lines: Vec<String> = Vec::new();
            new_lines.append(&mut formatted_initial_new_lines);
            new_lines.append(&mut formatted_trailing_new_lines);

            return (formatted_old_lines, new_lines);
        }
    }

    // FIXME: Handle the case where the old text is longer

    // All partial attempts failed
    return simple_format(old_text, new_text);
}

/// Returns a vector of ANSI highlighted lines
#[must_use]
pub fn format(old_text: &str, new_text: &str) -> Vec<String> {
    // FIXME: The two code blocks contain almost the same thing, refactor that somehow!
    match format_split(old_text, new_text) {
        None => {
            let (mut old_lines, mut new_lines) = partial_format(old_text, new_text);
            let mut merged: Vec<String> = Vec::new();
            merged.append(&mut old_lines);
            merged.append(&mut new_lines);
            return merged;
        }

        Some((mut old_lines, mut new_lines)) => {
            let mut merged: Vec<String> = Vec::new();
            merged.append(&mut old_lines);
            merged.append(&mut new_lines);
            return merged;
        }
    }
}

/// Returns two vectors of ANSI highlighted lines, the old lines and the new
/// lines.
///
/// A return value of None means you should try partial_format() instead.
#[must_use]
fn format_split(old_text: &str, new_text: &str) -> Option<(Vec<String>, Vec<String>)> {
    if old_text.is_empty() || new_text.is_empty() {
        return Some(simple_format(old_text, new_text));
    }

    // These checks make us faster, please use the benchmark.py script before
    // and after if you change this.
    if is_large_byte_count_change(old_text, new_text) {
        return None;
    }
    if is_large_newline_count_change(old_text, new_text) {
        return None;
    }

    // Find diffs between adds and removals
    let mut old_collector = TokenCollector::create(StyledToken::new("-".to_string(), Style::Old));
    let mut new_collector = TokenCollector::create(StyledToken::new("+".to_string(), Style::New));

    // Tokenize adds and removes before diffing them
    let tokenized_old = tokenizer::tokenize(old_text);
    let tokenized_new = tokenizer::tokenize(new_text);

    let diff = tokenized_old.diff(&tokenized_new);
    match diff {
        edit::Edit::Copy(unchanged) => {
            for token in unchanged {
                old_collector.push(StyledToken::new(token.to_string(), Style::Old));
                new_collector.push(StyledToken::new(token.to_string(), Style::New));
            }
        }
        edit::Edit::Change(diff) => {
            diff.into_iter()
                .map(|edit| {
                    match edit {
                        collection::Edit::Copy(token) => {
                            old_collector.push(StyledToken::new(token.to_string(), Style::Old));
                            new_collector.push(StyledToken::new(token.to_string(), Style::New));
                        }
                        collection::Edit::Insert(token) => {
                            if *token == "\n" {
                                // Make sure the highlighted linefeed is visible
                                new_collector
                                    .push(StyledToken::new("⏎".to_string(), Style::NewInverse));
                            }
                            new_collector
                                .push(StyledToken::new(token.to_string(), Style::NewInverse));
                        }
                        collection::Edit::Remove(token) => {
                            if *token == "\n" {
                                // Make sure the highlighted linefeed is visible
                                old_collector
                                    .push(StyledToken::new("⏎".to_string(), Style::OldInverse));
                            }
                            old_collector
                                .push(StyledToken::new(token.to_string(), Style::OldInverse));
                        }
                        collection::Edit::Change(_) => unimplemented!("Not implemented, help!"),
                    };
                })
                .for_each(drop);
        }
    }

    let highlighted_old_text = old_collector.render();
    let highlighted_new_text = new_collector.render();

    let highlighted_bytes_count =
        old_collector.highlighted_chars_count() + new_collector.highlighted_chars_count();
    let bytes_count = old_collector.chars_count() + new_collector.chars_count();

    // Don't highlight too much
    if (100 * highlighted_bytes_count) / bytes_count > MAX_HIGHLIGHT_PERCENTAGE {
        return None;
    }

    return Some(to_lines(&highlighted_old_text, &highlighted_new_text));
}

#[must_use]
fn is_large_count_change(count1: usize, count2: usize) -> bool {
    // This check makes us ignore some changes, thus making us faster. Please
    // use the benchmark.py script before and after if you touch this code.

    let high_count = max(count1, count2);
    let low_count = min(count1, count2);

    if high_count - low_count <= SMALL_COUNT_CHANGE {
        return false;
    }

    // "+ 99" makes the result round up, so 0->0, 1->2.
    let low_count_plus_percentage = (low_count * (LARGE_COUNT_CHANGE_PERCENT + 100) + 99) / 100;

    return high_count >= low_count_plus_percentage;
}

#[must_use]
fn is_large_byte_count_change(old_text: &str, new_text: &str) -> bool {
    return is_large_count_change(old_text.len(), new_text.len());
}

#[must_use]
fn is_large_newline_count_change(old_text: &str, new_text: &str) -> bool {
    let old_newline_count = bytecount::count(old_text.as_bytes(), b'\n');
    let new_newline_count = bytecount::count(new_text.as_bytes(), b'\n');
    return is_large_count_change(old_newline_count, new_newline_count);
}

#[must_use]
fn to_lines(old: &str, new: &str) -> (Vec<String>, Vec<String>) {
    let mut old_lines: Vec<String> = Vec::new();
    for highlighted_old_line in old.lines() {
        old_lines.push(highlighted_old_line.to_string());
    }
    if (!old.is_empty()) && !old.ends_with('\n') {
        old_lines.push(format!(
            "{}{}{}",
            NO_EOF_NEWLINE_COLOR, NO_EOF_NEWLINE_MARKER, NORMAL
        ));
    }

    let mut new_lines: Vec<String> = Vec::new();
    for highlighted_new_line in new.lines() {
        new_lines.push(highlighted_new_line.to_string());
    }
    if (!new.is_empty()) && !new.ends_with('\n') {
        new_lines.push(format!(
            "{}{}{}",
            NO_EOF_NEWLINE_COLOR, NO_EOF_NEWLINE_MARKER, NORMAL
        ));
    }

    return (old_lines, new_lines);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    use pretty_assertions::assert_eq;

    fn simple_format_merged(old_text: &str, new_text: &str) -> Vec<String> {
        let (mut old_lines, mut new_lines) = simple_format(old_text, new_text);

        let mut merged: Vec<String> = Vec::new();
        merged.append(&mut old_lines);
        merged.append(&mut new_lines);
        return merged;
    }

    #[test]
    fn test_simple_format_adds_and_removes() {
        let empty: Vec<String> = Vec::new();
        assert_eq!(
            simple_format_merged(&"".to_string(), &"".to_string()),
            empty
        );

        // Test adds-only
        assert_eq!(
            simple_format_merged(&"".to_string(), &"a\n".to_string()),
            ["".to_string() + NEW + "+a" + NORMAL]
        );
        assert_eq!(
            simple_format_merged(&"".to_string(), &"a\nb\n".to_string()),
            [
                "".to_string() + NEW + "+a" + NORMAL,
                "".to_string() + NEW + "+b" + NORMAL,
            ]
        );

        // Test removes-only
        assert_eq!(
            simple_format_merged(&"a\n".to_string(), &"".to_string()),
            ["".to_string() + OLD + "-a" + NORMAL]
        );
        assert_eq!(
            simple_format_merged(&"a\nb\n".to_string(), &"".to_string()),
            [
                "".to_string() + OLD + "-a" + NORMAL,
                "".to_string() + OLD + "-b" + NORMAL,
            ]
        );
    }

    #[test]
    fn test_quote_change() {
        let result = format(&"<quotes>\n".to_string(), &"[quotes]\n".to_string());
        assert_eq!(
            result,
            [
                format!(
                    "{}-{}<{}quotes{}>{}",
                    OLD, INVERSE_VIDEO, NOT_INVERSE_VIDEO, INVERSE_VIDEO, NORMAL
                ),
                format!(
                    "{}+{}[{}quotes{}]{}",
                    NEW, INVERSE_VIDEO, NOT_INVERSE_VIDEO, INVERSE_VIDEO, NORMAL
                ),
            ]
        )
    }

    #[test]
    fn test_almost_empty_changes() {
        let result = format(&"x\n".to_string(), &"".to_string());
        assert_eq!(result, [format!("{}-x{}", OLD, NORMAL),]);

        let result = format(&"".to_string(), &"x\n".to_string());
        assert_eq!(result, [format!("{}+x{}", NEW, NORMAL),]);
    }

    #[test]
    fn test_is_large_byte_count_change() {
        assert_eq!(is_large_byte_count_change("", ""), false);

        assert_eq!(
            is_large_byte_count_change("", &"x".repeat(SMALL_COUNT_CHANGE)),
            false
        );
        assert_eq!(
            is_large_byte_count_change("", &"x".repeat(SMALL_COUNT_CHANGE + 1)),
            true
        );

        // Verify that doubling the length counts as large
        let base_len = SMALL_COUNT_CHANGE * 2;
        let double_len = base_len * 2;
        let almost_double_len = double_len - 1;
        assert_eq!(
            is_large_byte_count_change(&"x".repeat(base_len), &"y".repeat(almost_double_len)),
            false
        );
        assert_eq!(
            is_large_byte_count_change(&"x".repeat(base_len), &"y".repeat(double_len)),
            true
        );
    }
}
