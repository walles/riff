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

/// If it's only this few highlighted chars, we'll just highligh anyway without
/// checking the `MAX_HIGHLIGHT_PERCENTAGE`.
const OK_HIGHLIGHT_COUNT: usize = 10;

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

    // These two conditions make us about 30% faster. The idea is that if the
    // texts are too unequal in length, we're unlikely to highlight just small
    // parts of them.
    if (100 * old_text.len()) / (old_text.len() + new_text.len()) < MAX_HIGHLIGHT_PERCENTAGE {
        return simple_format(old_text, new_text);
    }
    if (100 * new_text.len()) / (old_text.len() + new_text.len()) < MAX_HIGHLIGHT_PERCENTAGE {
        return simple_format(old_text, new_text);
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
                old_collector.push(StyledToken::new(token.clone(), Style::Old));
                new_collector.push(StyledToken::new(token.clone(), Style::New));
            }
        }
        edit::Edit::Change(diff) => {
            diff.into_iter()
                .map(|edit| {
                    match edit {
                        collection::Edit::Copy(token) => {
                            old_collector.push(StyledToken::new(token.clone(), Style::Old));
                            new_collector.push(StyledToken::new(token.clone(), Style::New));
                        }
                        collection::Edit::Insert(token) => {
                            if token == "\n" {
                                // Make sure the highlighted linefeed is visible
                                new_collector
                                    .push(StyledToken::new("⏎".to_string(), Style::NewInverse));
                            }
                            new_collector.push(StyledToken::new(token.clone(), Style::NewInverse));
                        }
                        collection::Edit::Remove(token) => {
                            if token == "\n" {
                                // Make sure the highlighted linefeed is visible
                                old_collector
                                    .push(StyledToken::new("⏎".to_string(), Style::OldInverse));
                            }
                            old_collector.push(StyledToken::new(token.clone(), Style::OldInverse));
                        }
                        collection::Edit::Change(_) => unimplemented!("Not implemented, help!"),
                    };
                })
                .for_each(drop);
        }
    }

    let highlighted_bytes_count =
        old_collector.highlighted_chars_count() + new_collector.highlighted_chars_count();
    let bytes_count = old_collector.chars_count() + new_collector.chars_count();

    // Don't highlight too much
    if highlighted_bytes_count <= OK_HIGHLIGHT_COUNT {
        // Few enough highlights, Just do it (tm)
    } else if (100 * highlighted_bytes_count) / bytes_count > MAX_HIGHLIGHT_PERCENTAGE {
        return simple_format(old_text, new_text);
    }

    let highlighted_old_text = old_collector.render();
    let highlighted_new_text = new_collector.render();
    return to_lines(&highlighted_old_text, &highlighted_new_text);
}

#[must_use]
fn to_lines(old: &str, new: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for highlighted_old_line in old.lines() {
        lines.push(highlighted_old_line.to_string());
    }
    if (!old.is_empty()) && !old.ends_with('\n') {
        lines.push(format!(
            "{}{}{}",
            NO_EOF_NEWLINE_COLOR, NO_EOF_NEWLINE_MARKER, NORMAL
        ));
    }

    for highlighted_new_line in new.lines() {
        lines.push(highlighted_new_line.to_string());
    }
    if (!new.is_empty()) && !new.ends_with('\n') {
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
}
