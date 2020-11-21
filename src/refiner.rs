use crate::constants::*;
use crate::tokenizer;
use diffus::{
    edit::{self, collection},
    Diffable,
};

/// If more than this part of either adds or moves is highlighted,
/// we consider it to be a replacement rather than a move, and skip
/// highlighting it.
const MAX_HIGHLIGHT_PERCENTAGE: usize = 30;

/// If it's only this few highlights, we'll just highligh anyway without
/// checking the `MAX_HIGHLIGHT_PERCENTAGE`.
const OK_HIGHLIGHT_COUNT: usize = 5;

/// Format old and new lines in OLD and NEW colors.
///
/// No intra-line refinement.
#[must_use]
fn simple_format(old_text: &str, new_text: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();

    for old_line in old_text.lines() {
        lines.push(format!("{}-{}{}", OLD, old_line, NORMAL));
    }
    if (!old_text.is_empty()) && !old_text.ends_with('\n') {
        lines.push(format!(
            "{}{}{}",
            NO_EOF_NEWLINE_COLOR, NO_EOF_NEWLINE_MARKER, NORMAL
        ));
    }

    for add_line in new_text.lines() {
        lines.push(format!("{}+{}{}", NEW, add_line, NORMAL))
    }
    if (!new_text.is_empty()) && !new_text.ends_with('\n') {
        lines.push(format!(
            "{}{}{}",
            NO_EOF_NEWLINE_COLOR, NO_EOF_NEWLINE_MARKER, NORMAL
        ));
    }

    return lines;
}

/// Returns a vector of ANSI highlighted lines
#[must_use]
pub fn format(old_text: &str, new_text: &str) -> Vec<String> {
    if new_text.is_empty() {
        return simple_format(old_text, new_text);
    }

    if old_text.is_empty() {
        return simple_format(old_text, new_text);
    }

    // Find diffs between adds and removals
    let mut highlighted_old_text = String::new();
    let mut highlighted_new_text = String::new();
    let mut old_is_inverse = false;
    let mut new_is_inverse = false;
    let mut old_highlight_count = 0;
    let mut new_highlight_count = 0;

    // Tokenize adds and removes before diffing them
    let tokenized_old = tokenizer::tokenize(old_text);
    let tokenized_new = tokenizer::tokenize(new_text);

    let diff = tokenized_old.diff(&tokenized_new);
    match diff {
        edit::Edit::Copy(unchanged) => {
            for token in unchanged {
                highlighted_new_text.push_str(token);
                highlighted_old_text.push_str(token);
            }
        }
        edit::Edit::Change(diff) => {
            diff.into_iter()
                .map(|edit| {
                    match edit {
                        collection::Edit::Copy(elem) => {
                            if new_is_inverse {
                                highlighted_new_text.push_str(NOT_INVERSE_VIDEO);
                            }
                            new_is_inverse = false;

                            if old_is_inverse {
                                highlighted_old_text.push_str(NOT_INVERSE_VIDEO);
                            }
                            old_is_inverse = false;

                            highlighted_new_text.push_str(elem);
                            highlighted_old_text.push_str(elem);
                        }
                        collection::Edit::Insert(elem) => {
                            new_highlight_count += 1;
                            if !new_is_inverse {
                                highlighted_new_text.push_str(INVERSE_VIDEO);
                            }
                            new_is_inverse = true;

                            if elem == "\n" {
                                // Make sure the highlighted linefeed is visible
                                highlighted_new_text.push('⏎');

                                // This will be reset by the linefeed, so we need to re-inverse on the next line
                                new_is_inverse = false;
                            }
                            highlighted_new_text.push_str(elem);
                        }
                        collection::Edit::Remove(elem) => {
                            old_highlight_count += 1;
                            if !old_is_inverse {
                                highlighted_old_text.push_str(INVERSE_VIDEO);
                            }
                            old_is_inverse = true;

                            if elem == "\n" {
                                // Make sure the highlighted linefeed is visible
                                highlighted_old_text.push('⏎');

                                // This will be reset by the linefeed, so we need to re-inverse on the next line
                                old_is_inverse = false;
                            }
                            highlighted_old_text.push_str(elem);
                        }
                        collection::Edit::Change(_) => panic!("Not implemented, help!"),
                    };
                })
                .for_each(drop);
        }
    }

    let highlight_count = old_highlight_count + new_highlight_count;
    let token_count = tokenized_old.len() + tokenized_new.len();

    // FIXME: Maybe for this check count how many runs of characters were
    // highlighted rather than how many tokens? Heuristics are difficult...
    if highlight_count <= OK_HIGHLIGHT_COUNT {
        // Few enough highlights, Just do it (tm)
    } else if (100 * highlight_count) / token_count > MAX_HIGHLIGHT_PERCENTAGE {
        return simple_format(old_text, new_text);
    }

    let mut lines: Vec<String> = Vec::new();
    for highlighted_old_line in highlighted_old_text.lines() {
        lines.push(format!("{}-{}{}", OLD, highlighted_old_line, NORMAL));
    }
    if (!old_text.is_empty()) && !old_text.ends_with('\n') {
        lines.push(format!(
            "{}{}{}",
            NO_EOF_NEWLINE_COLOR, NO_EOF_NEWLINE_MARKER, NORMAL
        ));
    }

    for highlighted_new_line in highlighted_new_text.lines() {
        lines.push(format!("{}+{}{}", NEW, highlighted_new_line, NORMAL));
    }
    if (!new_text.is_empty()) && !new_text.ends_with('\n') {
        lines.push(format!(
            "{}{}{}",
            NO_EOF_NEWLINE_COLOR, NO_EOF_NEWLINE_MARKER, NORMAL
        ));
    }

    return lines;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    use pretty_assertions::assert_eq;

    #[test]
    fn test_simple_format_adds_and_removes() {
        let empty: Vec<String> = Vec::new();
        assert_eq!(simple_format(&"".to_string(), &"".to_string()), empty);

        // Test adds-only
        assert_eq!(
            simple_format(&"".to_string(), &"a\n".to_string()),
            ["".to_string() + NEW + "+a" + NORMAL]
        );
        assert_eq!(
            simple_format(&"".to_string(), &"a\nb\n".to_string()),
            [
                "".to_string() + NEW + "+a" + NORMAL,
                "".to_string() + NEW + "+b" + NORMAL,
            ]
        );

        // Test removes-only
        assert_eq!(
            simple_format(&"a\n".to_string(), &"".to_string()),
            ["".to_string() + OLD + "-a" + NORMAL]
        );
        assert_eq!(
            simple_format(&"a\nb\n".to_string(), &"".to_string()),
            [
                "".to_string() + OLD + "-a" + NORMAL,
                "".to_string() + OLD + "-b" + NORMAL,
            ]
        );
    }

    #[test]
    fn test_quote_change() {
        assert_eq!(
            format(&"<quotes>\n".to_string(), &"[quotes]\n".to_string()),
            [
                format!(
                    "{}-{}<{}quotes{}>{}{}",
                    OLD, INVERSE_VIDEO, NOT_INVERSE_VIDEO, INVERSE_VIDEO, NOT_INVERSE_VIDEO, NORMAL
                ),
                format!(
                    "{}+{}[{}quotes{}]{}{}",
                    NEW, INVERSE_VIDEO, NOT_INVERSE_VIDEO, INVERSE_VIDEO, NOT_INVERSE_VIDEO, NORMAL
                ),
            ]
        )
    }
}
