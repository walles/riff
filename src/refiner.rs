use crate::constants::*;
use crate::tokenizer;
use diffus::{
    edit::{self, collection},
    Diffable,
};

#[derive(Clone)]
enum Style {
    Old,
    OldInverse,
    New,
    NewInverse,
}

#[derive(Clone)]
struct StyledToken {
    token: String,
    style: Style,
}

impl Style {
    pub fn is_inverse(&self) -> bool {
        match self {
            Style::OldInverse | Style::NewInverse => {
                return true;
            }
            _ => {
                return false;
            }
        }
    }

    pub fn color(&self) -> &str {
        match self {
            Style::Old => {
                return OLD;
            }
            Style::OldInverse => {
                return OLD;
            }
            Style::New => {
                return NEW;
            }
            Style::NewInverse => {
                return NEW;
            }
        }
    }
}

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
    let mut highlighted_old: Vec<StyledToken> = Vec::new();
    let mut highlighted_new: Vec<StyledToken> = Vec::new();
    let mut old_highlight_count = 0;
    let mut new_highlight_count = 0;

    // Tokenize adds and removes before diffing them
    let tokenized_old = tokenizer::tokenize(old_text);
    let tokenized_new = tokenizer::tokenize(new_text);

    let diff = tokenized_old.diff(&tokenized_new);
    match diff {
        edit::Edit::Copy(unchanged) => {
            for token in unchanged {
                highlighted_old.push(StyledToken {
                    token: token.clone(),
                    style: Style::Old,
                });
                highlighted_new.push(StyledToken {
                    token: token.to_string(),
                    style: Style::New,
                });
            }
        }
        edit::Edit::Change(diff) => {
            diff.into_iter()
                .map(|edit| {
                    match edit {
                        collection::Edit::Copy(token) => {
                            highlighted_old.push(StyledToken {
                                token: token.clone(),
                                style: Style::Old,
                            });
                            highlighted_new.push(StyledToken {
                                token: token.clone(),
                                style: Style::New,
                            });
                        }
                        collection::Edit::Insert(token) => {
                            new_highlight_count += 1;
                            if token == "\n" {
                                // Make sure the highlighted linefeed is visible
                                highlighted_new.push(StyledToken {
                                    token: "⏎".to_string(),
                                    style: Style::NewInverse,
                                });
                            }
                            highlighted_new.push(StyledToken {
                                token: token.clone(),
                                style: Style::NewInverse,
                            });
                        }
                        collection::Edit::Remove(token) => {
                            old_highlight_count += 1;

                            if token == "\n" {
                                // Make sure the highlighted linefeed is visible
                                highlighted_old.push(StyledToken {
                                    token: "⏎".to_string(),
                                    style: Style::OldInverse,
                                });
                            }
                            highlighted_old.push(StyledToken {
                                token: token.clone(),
                                style: Style::OldInverse,
                            });
                        }
                        collection::Edit::Change(_) => unimplemented!("Not implemented, help!"),
                    };
                })
                .for_each(drop);
        }
    }

    let highlight_count = old_highlight_count + new_highlight_count;
    let token_count = tokenized_old.len() + tokenized_new.len();

    // FIXME: Maybe for this check count how many characters were highlighted
    // rather than how many tokens? Heuristics are difficult...
    if highlight_count <= OK_HIGHLIGHT_COUNT {
        // Few enough highlights, Just do it (tm)
    } else if (100 * highlight_count) / token_count > MAX_HIGHLIGHT_PERCENTAGE {
        return simple_format(old_text, new_text);
    }

    let highlighted_old = with_line_prefixes(
        &highlighted_old,
        StyledToken {
            token: "-".to_string(),
            style: Style::Old,
        },
    );
    let highlighted_new = with_line_prefixes(
        &highlighted_new,
        StyledToken {
            token: "+".to_string(),
            style: Style::New,
        },
    );

    let highlighted_old_text = render(&highlighted_old);
    let highlighted_new_text = render(&highlighted_new);
    return to_lines(&highlighted_old_text, &highlighted_new_text);
}

/// Adds prefix in front of every new line
#[must_use]
fn with_line_prefixes(without_prefixes: &[StyledToken], prefix: StyledToken) -> Vec<StyledToken> {
    let mut want_prefix = true;
    let mut with_prefixes: Vec<StyledToken> = Vec::new();
    for token in without_prefixes {
        if want_prefix {
            with_prefixes.push(prefix.clone());
            want_prefix = false;
        }

        with_prefixes.push(token.clone());

        if token.token == "\n" {
            want_prefix = true;
        }
    }
    return with_prefixes;
}

#[must_use]
fn render(tokens: &[StyledToken]) -> String {
    let mut rendered = String::new();
    let mut is_inverse = false;
    let mut color = NORMAL;
    for token in tokens {
        if token.token == "\n" {
            rendered.push_str(NORMAL);
            rendered.push('\n');
            is_inverse = false;
            color = NORMAL;
            continue;
        }

        if token.style.is_inverse() && !is_inverse {
            rendered.push_str(INVERSE_VIDEO);
        }
        if is_inverse && !token.style.is_inverse() {
            rendered.push_str(NOT_INVERSE_VIDEO);
        }
        is_inverse = token.style.is_inverse();

        if token.style.color() != color {
            rendered.push_str(token.style.color());
            color = token.style.color();
        }

        rendered.push_str(&token.token);
    }

    if !rendered.ends_with(&"\n") {
        // Don't forget to reset even if we don't end in a newline
        rendered.push_str(NORMAL);
    }

    return rendered;
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
