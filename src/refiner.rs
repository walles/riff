use crate::tokenizer;
use crate::{
    constants::*,
    token_collector::{Style, StyledToken, TokenCollector},
};
use diffus::{
    edit::{self, collection},
    Diffable,
};

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

#[must_use]
fn concat(mut a: Vec<String>, mut b: Vec<String>) -> Vec<String> {
    let mut merged: Vec<String> = Vec::new();
    merged.append(&mut a);
    merged.append(&mut b);
    return merged;
}

/// Returns a vector of ANSI highlighted lines
#[must_use]
pub fn format(old_text: &str, new_text: &str) -> Vec<String> {
    match format_split(old_text, new_text) {
        None => {
            let (old_lines, new_lines) = simple_format(old_text, new_text);
            return concat(old_lines, new_lines);
        }

        Some((old_lines, new_lines)) => {
            return concat(old_lines, new_lines);
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

    // FIXME: LCS is O(m * n) complexity, consider returning None here if
    // len(old_text) * len(new_text) is too large.

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
                            new_collector
                                .push(StyledToken::new(token.to_string(), Style::NewInverse));
                        }
                        collection::Edit::Remove(token) => {
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

    return Some(to_lines(&highlighted_old_text, &highlighted_new_text));
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
        let (old_lines, new_lines) = simple_format(old_text, new_text);

        return concat(old_lines, new_lines);
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
}
