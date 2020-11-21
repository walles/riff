use crate::constants::*;
use crate::tokenizer;
use crate::tokenizer::Style;
use crate::tokenizer::StyledToken;
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

/// Returns a vector of ANSI highlighted lines
#[must_use]
fn refine<'a>(
    old: &'a [StyledToken],
    new: &'a [StyledToken],
) -> (Vec<StyledToken>, Vec<StyledToken>) {
    if old.is_empty() {
        return (old.to_owned(), new.to_owned());
    }

    if new.is_empty() {
        return (old.to_owned(), new.to_owned());
    }

    // Find diffs between adds and removals
    let mut highlighted_old: Vec<StyledToken> = Vec::new();
    let mut highlighted_new: Vec<StyledToken> = Vec::new();
    let mut old_highlight_count = 0;
    let mut new_highlight_count = 0;

    let old_vec = old.to_vec();
    let new_vec = new.to_vec();
    let diff = old_vec.diff(&new_vec);
    match diff {
        edit::Edit::Copy(unchanged) => {
            for token in unchanged {
                highlighted_old.push(token.clone());
                highlighted_new.push(token.clone());
            }
        }
        edit::Edit::Change(diff) => {
            diff.into_iter()
                .map(|edit| {
                    match edit {
                        collection::Edit::Copy(copied) => {
                            highlighted_new.push(copied.clone());
                            highlighted_old.push(copied.clone());
                        }
                        collection::Edit::Insert(inserted) => {
                            new_highlight_count += 1;

                            if inserted.token() == "\n" {
                                // Make sure the highlighted linefeed is visible
                                let styled_newline = StyledToken::styled_newline(Style::AddInverse);
                                highlighted_new.push(styled_newline);
                            }
                            let mut highlighted_token = inserted.clone();
                            highlighted_token.set_style(Style::AddInverse);
                            highlighted_new.push(highlighted_token);
                        }
                        collection::Edit::Remove(removed) => {
                            old_highlight_count += 1;

                            if removed.token() == "\n" {
                                // Make sure the highlighted linefeed is visible
                                highlighted_old
                                    .push(StyledToken::styled_newline(Style::RemoveInverse));
                            }
                            let mut highlighted_token = removed.clone();
                            highlighted_token.set_style(Style::AddInverse);
                            highlighted_old.push(highlighted_token);
                        }
                        collection::Edit::Change(_) => panic!("Not implemented, help!"),
                    };
                })
                .for_each(drop);
        }
    }

    let highlight_count = old_highlight_count + new_highlight_count;
    let token_count = old.len() + new.len();

    // FIXME: Maybe for this check count how many characters were highlighted
    // rather than how many tokens? Heuristics are difficult...
    if highlight_count <= OK_HIGHLIGHT_COUNT {
        // Few enough highlights, Just do it (tm)
    } else if (100 * highlight_count) / token_count > MAX_HIGHLIGHT_PERCENTAGE {
        return (old.to_owned(), new.to_owned());
    }

    return (highlighted_old, highlighted_new);
}

/// Returns a multi lined string, each line prefixed with `-` or `+`
///
/// The returned string is guaranteed to end in a newline.
#[must_use]
fn render(old: &[StyledToken], new: &[StyledToken]) -> String {
    let mut return_me =
        tokenizer::to_string_with_line_prefix(&StyledToken::styled_str(&"-", Style::Remove), old);

    if (!old.is_empty()) && old.last().unwrap().token() != "\n" {
        // Last old token is not a newline, add no-newline-at-end-of-file text
        return_me += &format!(
            "\n{}{}{}\n",
            NO_EOF_NEWLINE_COLOR, NO_EOF_NEWLINE_MARKER, NORMAL
        );
    }

    return_me +=
        &tokenizer::to_string_with_line_prefix(&StyledToken::styled_str(&"+", Style::Add), new);
    if (!new.is_empty()) && new.last().unwrap().token() != "\n" {
        // Last new token is not a newline, add no-newline-at-end-of-file text
        return_me += &format!(
            "\n{}{}{}\n",
            NO_EOF_NEWLINE_COLOR, NO_EOF_NEWLINE_MARKER, NORMAL
        );
    }

    return return_me;
}

fn style_tokens(tokens: &mut [StyledToken], style: Style) {
    for token in tokens {
        token.set_style(style.clone());
    }
}

#[must_use]
pub fn format(old: &str, new: &str) -> String {
    let mut old = tokenizer::tokenize(old);
    let mut new = tokenizer::tokenize(new);

    // Color the tokens
    style_tokens(&mut old, Style::Remove);
    style_tokens(&mut new, Style::Add);

    // FIXME: Re-style any trailing whitespace tokens among the adds to inverse red

    // FIXME: Re-style any non-leading tab tokens among the adds to inverse red

    // Highlight what actually changed between old and new
    let (old, new) = refine(&old, &new);

    // Render adds + removes into an array of ANSI styled lines
    return render(&old, &new);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    use pretty_assertions::assert_eq;

    #[test]
    fn test_quote_change() {
        let formatted_lines = format("<quotes>\n", "[quotes]\n");
        let formatted_lines: Vec<&str> = formatted_lines.lines().collect();
        assert_eq!(
            formatted_lines,
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

    #[test]
    fn test_style_tokens() {
        let mut tokens = [StyledToken::from_str("token")];
        assert_eq!(tokens[0].get_style(), &Style::Plain);
        style_tokens(&mut tokens, Style::Add);
        assert_eq!(tokens[0].get_style(), &Style::Add);
    }
}
