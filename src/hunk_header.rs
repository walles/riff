use crate::constants::*;

/// Result of parsing a hunk header: <https://en.wikipedia.org/wiki/Diff#Unified_format>
///
/// Example hunk header: `@@ -1,2 +1,2 @@ Initial commit`
///
/// This would mean "old line numbers are 1-2, and new line numbers are 1-2",
/// making the line counts 2 for both.
#[derive(Debug, PartialEq)]
pub(crate) struct HunkHeader {
    /// "@@" with the right number of @ chars, usually two.
    ats: String,

    /// One-based start lines of one or more old sections + one new section.
    /// This vector will always have at least two entries, and mostly it will be
    /// exactly two.
    starts: Vec<usize>,

    /// Number of lines in one or more old sections + one new section. This
    /// vector will always have at least two entries, and mostly it will be
    /// exactly two.
    pub(crate) linecounts: Vec<usize>,

    pub title: Option<String>,
}

pub(crate) const HUNK_HEADER: &str = "\x1b[36m"; // Cyan

impl HunkHeader {
    /// Parse a hunk header from a line of text.
    ///
    /// Returns `None` if the line is not a valid hunk header.
    pub fn parse(line: &str) -> Option<Self> {
        // Count the number of @ chars at the start of the line, followed by a space
        let mut initial_at_count = 0;
        for c in line.chars() {
            if c == '@' {
                initial_at_count += 1;
                continue;
            }

            if c == ' ' {
                // We found the end of the @ chars
                break;
            }

            // Expected only @ chars followed by a space, this is not it
            return None;
        }

        if initial_at_count < 2 {
            // Expected at least two @ chars, this is not it
            return None;
        }

        let mut parts = line.splitn(3 + initial_at_count, ' ');
        let initial_ats = parts.next().unwrap();

        let expected_count_parts = initial_at_count;
        let mut expected_start_char = '-';
        let mut starts = Vec::new();
        let mut linecounts = Vec::new();
        loop {
            // Example: "-1,2", or just "-55"
            let counts_part = parts.next()?;

            // Parse the old line count
            let numbers = counts_part
                .trim_start_matches(expected_start_char)
                .split(',')
                .collect::<Vec<_>>();
            if numbers.is_empty() || numbers.len() > 2 {
                return None;
            }

            let start = numbers[0].parse::<usize>().ok()?;
            let linecount = if numbers.len() == 2 {
                numbers[1].parse::<usize>().ok()?
            } else {
                1
            };

            starts.push(start);
            linecounts.push(linecount);
            if starts.len() == expected_count_parts - 1 {
                // We are done with all the `-` parts, let's go for the final `+` part
                expected_start_char = '+';
            }

            if starts.len() == expected_count_parts {
                // We are done with all the parts
                break;
            }
        }

        if parts.next()? != initial_ats {
            // Not a hunk header, it wasn't finalized by @@ at the end
            return None;
        }

        // Example: "Initial commit"
        let title = parts.next().map(str::to_string);

        Some(HunkHeader {
            ats: initial_ats.to_string(),
            starts,
            linecounts,
            title,
        })
    }

    /// Render into an ANSI highlighted string, not ending in a newline.
    pub fn render(&self) -> String {
        let mut rendered = String::new();
        rendered.push_str(HUNK_HEADER);
        rendered.push_str(&self.ats);
        rendered.push(' ');

        for i in 0..self.starts.len() {
            if i == self.starts.len() - 1 {
                rendered.push('+');
            } else {
                rendered.push('-');
            }

            rendered.push_str(&self.starts[i].to_string());
            rendered.push(',');
            rendered.push_str(&self.linecounts[i].to_string());
            rendered.push(' ');
        }

        rendered.push_str(HUNK_HEADER);
        if let Some(title) = &self.title {
            rendered.push(' ');
            rendered.push_str(BOLD);
            rendered.push_str(title);
        }

        rendered.push_str(NORMAL);

        return rendered;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_non_hunk_header() {
        assert_eq!(None, HunkHeader::parse("This is not a hunk header"));
        assert_eq!(None, HunkHeader::parse(""));
    }

    #[test]
    fn test_simple_hunk_header() {
        assert_eq!(
            Some(HunkHeader {
                ats: "@@".to_string(),
                starts: vec![1, 1],
                linecounts: vec![2, 2],
                title: None,
            }),
            HunkHeader::parse("@@ -1,2 +1,2 @@")
        );
    }

    #[test]
    fn test_hunk_header_with_title() {
        assert_eq!(
            Some(HunkHeader {
                ats: "@@".to_string(),
                starts: vec![1, 1],
                linecounts: vec![2, 2],
                title: Some("Hello there".to_string()),
            }),
            HunkHeader::parse("@@ -1,2 +1,2 @@ Hello there")
        );
    }

    #[test]
    fn test_hunk_header_with_spaced_title() {
        assert_eq!(
            Some(HunkHeader {
                ats: "@@".to_string(),
                starts: vec![1, 1],
                linecounts: vec![2, 2],
                title: Some("Hello  there".to_string()),
            }),
            HunkHeader::parse("@@ -1,2 +1,2 @@ Hello  there")
        );
    }

    #[test]
    fn test_hunk_header_with_default_linecounts() {
        assert_eq!(
            Some(HunkHeader {
                ats: "@@".to_string(),
                starts: vec![5, 6],
                linecounts: vec![1, 1],
                title: None,
            }),
            HunkHeader::parse("@@ -5 +6 @@")
        );
    }

    #[test]
    fn test_hunk_header_with_multiple_olds() {
        assert_eq!(
            Some(HunkHeader {
                ats: "@@@".to_string(),
                starts: vec![1, 3, 5],
                linecounts: vec![2, 4, 6],
                title: None,
            }),
            HunkHeader::parse("@@@ -1,2 -3,4 +5,6 @@@")
        );
    }
}
