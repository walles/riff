use crate::constants::*;

pub fn format_commit_line(line: &str) -> String {
    if !line.contains('(') {
        // Just "commit: 123abc", color it all yellow
        return format!("{}{}{}", YELLOW, line, NORMAL);
    }

    // FIXME: Parse it as "commit AAA (BBB)", where AAA is the SHA and BBB is a
    // comma separated list of SHA tidbits

    return String::from("FIXME: This was too hard");
}

#[cfg(test)]
mod tests {
    #[cfg(test)]
    use pretty_assertions::assert_eq;

    use crate::constants::YELLOW;

    use super::*;

    #[test]
    fn test_plain_commit() {
        assert_eq!(
            format_commit_line("commit abc123"),
            format!("{}{}{}", YELLOW, "commit abc123", NORMAL)
        )
    }
}
