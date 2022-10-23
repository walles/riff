use crate::constants::*;

pub fn format_commit_line(line: &str) -> String {
    let parts = line.split('(').collect::<Vec<_>>();
    if parts.len() == 1 {
        // Just "commit: 123abc", color it all yellow
        return format!("{}{}{}", COMMIT_YELLOW, line, NORMAL);
    }

    let commit_part = parts[0].trim();
    let without_trailing_parenthesis = parts[1].strip_suffix(')');
    if without_trailing_parenthesis == None {
        // No final parenthesis, this is weird, fall back to showing everything
        // in yellow
        return format!("{}{}{}", COMMIT_YELLOW, line, NORMAL);
    }

    let parenthesis_parts = without_trailing_parenthesis
        .unwrap()
        .split(", ")
        .collect::<Vec<_>>();

    // FIXME: Placeholder logic until we have all tests
    return format!(
        "{}{} ({}){}",
        COMMIT_YELLOW,
        commit_part,
        parenthesis_parts.join(", "),
        NORMAL
    );
}

#[cfg(test)]
mod tests {
    #[cfg(test)]
    use pretty_assertions::assert_eq;

    use crate::constants::*;

    use super::*;

    pub const COMMIT_BOLD_YELLOW: &str = "\x1b[1;33m";
    pub const COMMIT_CYAN: &str = "\x1b[1;36m";
    pub const COMMIT_GREEN: &str = "\x1b[1;33m";

    #[test]
    fn test_plain_commit() {
        assert_eq!(
            format_commit_line("commit abc123"),
            format!("{}commit abc123{}", COMMIT_YELLOW, NORMAL)
        )
    }

    #[test]
    fn test_commit_with_headmaster() {
        assert_eq!(
            format_commit_line("commit abc123 (HEAD -> master)"),
            format!(
                "{}commit abc123 ({}HEAD ->{}master{}{}){}",
                COMMIT_YELLOW, COMMIT_CYAN, COMMIT_GREEN, NORMAL, COMMIT_YELLOW, NORMAL
            )
        )
    }

    #[test]
    fn test_commit_with_tag() {
        assert_eq!(
            format_commit_line("commit abc123 (tag: 1.2.3)"),
            format!(
                "{}commit abc123 ({}tag: 1.2.3{}{}){}",
                COMMIT_YELLOW, COMMIT_BOLD_YELLOW, NORMAL, COMMIT_YELLOW, NORMAL
            )
        )
    }
}
