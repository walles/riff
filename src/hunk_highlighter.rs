use std::iter;

use threadpool::ThreadPool;

use crate::hunk_header::HunkHeader;
use crate::lines_highlighter::{LineAcceptance, LinesHighlighter, Response};
use crate::refiner;
use crate::string_future::StringFuture;

pub(crate) struct HunkLinesHighlighter {
    // This will have to be rendered at the top of our returned result.
    hunk_header: Option<String>,

    /// Calculated by HunkHeader::parse(). We'll count this value down as we consume lines.
    expected_line_counts: Vec<usize>,

    /// The different flavors of some text. All but the last are `-` sections,
    /// and the last one is a `+` section.
    flavors: Vec<String>,

    /// Line prefixes for each flavor. Usually `-` or `+`, but could be things
    /// line ` -` or `+++` if this is a merge diff.
    prefixes: Vec<String>,
}

impl LinesHighlighter for HunkLinesHighlighter {
    fn from_line(line: &str) -> Option<Self>
    where
        Self: Sized,
    {
        if let Some(hunk_header) = HunkHeader::parse(line) {
            let expected_line_counts = hunk_header.linecounts;
            let mut flavors = Vec::new();
            flavors.resize(expected_line_counts.len(), String::new());
            let mut prefixes = Vec::new();
            prefixes.resize(expected_line_counts.len(), String::new());

            return Some(HunkLinesHighlighter {
                hunk_header: Some(hunk_header.render()),
                expected_line_counts,
                flavors,
                prefixes,
            });
        }

        return None;
    }

    fn consume_line(&mut self, line: &str, thread_pool: &ThreadPool) -> Result<Response, String> {
        let mut return_me = vec![];

        // Always start by rendering the hunk header
        if let Some(hunk_header) = &self.hunk_header {
            return_me.push(StringFuture::from_string(hunk_header.to_string() + "\n"));
            self.hunk_header = None;
        }

        // "\ No newline at end of file"
        if line.starts_with('\\') {
            // "\ No newline at end of file"
            todo!("Remove trailing newlines from whatever section(s) we are in");
        }

        if !self.more_lines_expected() {
            return Ok(Response {
                line_accepted: LineAcceptance::RejectedDone,
                highlighted: return_me,
            });
        }

        // Context lines
        let spaces_only = iter::repeat(' ')
            .take(self.expected_line_counts.len())
            .collect::<String>();
        if line.starts_with(&spaces_only) || line.is_empty() {
            return_me.append(&mut self.drain(thread_pool));

            self.expected_line_counts
                .iter_mut()
                .for_each(|expected_line_count| {
                    *expected_line_count -= 1;
                });

            // FIXME: Consider whether we should be coalescing the plain lines?
            // Maybe that would improve performance? Measure and find out!
            return_me.push(StringFuture::from_string(line.to_string() + "\n"));

            let acceptance = if !self.more_lines_expected() {
                LineAcceptance::AcceptedDone
            } else {
                LineAcceptance::AcceptedWantMore
            };

            return Ok(Response {
                line_accepted: acceptance,
                highlighted: return_me,
            });
        }

        if let Some(minus_line) = line.strip_prefix('-') {
            self.expected_old_lines -= 1;
            self.old_text.push_str(minus_line);
            self.old_text.push('\n');

            return Ok(Response {
                // Note that even if we don't "expect" any more lines, we could
                // still get "\ No newline at end of file" lines, so we have to
                // ask for more here, and we can't drain_old_new() just yet.
                line_accepted: LineAcceptance::AcceptedWantMore,
                highlighted: return_me,
            });
        }

        if let Some(plus_line) = line.strip_prefix('+') {
            self.expected_new_lines -= 1;
            self.new_text.push_str(plus_line);
            self.new_text.push('\n');

            return Ok(Response {
                // Note that even if we don't "expect" any more lines, we could
                // still get "\ No newline at end of file" lines, so we have to
                // ask for more here, and we can't drain_old_new() just yet.
                line_accepted: LineAcceptance::AcceptedWantMore,
                highlighted: return_me,
            });
        }

        return Err("Hunk line must start with '-' or '+'".to_string());
    }

    fn consume_eof(&mut self, thread_pool: &ThreadPool) -> Result<Vec<StringFuture>, String> {
        if self.more_lines_expected() {
            return Err(format!(
                "Still expecting more lines, but got EOF: {:?}",
                self.expected_line_counts
            ));
        }

        return Ok(self.drain(thread_pool));
    }
}

impl HunkLinesHighlighter {
    fn drain(&mut self, thread_pool: &ThreadPool) -> Vec<StringFuture> {
        // Return nothing if all flavors are empty
        if self.flavors.iter().all(|flavor| flavor.is_empty()) {
            return vec![];
        }

        let flavors = self.flavors.clone();
        let prefixes = self.prefixes.clone();

        self.flavors.iter_mut().for_each(|flavor| flavor.clear());
        self.prefixes.iter_mut().for_each(|prefix| prefix.clear());

        let return_me = StringFuture::from_function(
            move || {
                let mut result = String::new();
                for line in refiner::format(&flavors, &prefixes) {
                    result.push_str(&line);
                    result.push('\n');
                }

                result
            },
            thread_pool,
        );

        return vec![return_me];
    }

    fn more_lines_expected(&self) -> bool {
        for expected_line_count in &self.expected_line_counts {
            if *expected_line_count != 0 {
                return true;
            }
        }
        return false;
    }
}

#[cfg(test)]
mod tests {
    use crate::lines_highlighter::LineAcceptance;

    use super::*;

    /// Based on `testdata/adds-only.diff`
    #[test]
    fn test_happy_path() {
        let thread_pool = ThreadPool::new(1);

        let mut test_me = HunkLinesHighlighter::from_line("@@ -1,2 +1,2 @@").unwrap();

        // First call to consume_line() should get us the hunk header
        let mut result = test_me
            .consume_line("-Hello, my name is Johan", &thread_pool)
            .unwrap();

        // Expect to get the hunk header back immediately, no matter what else
        // we got.
        assert_eq!(result.line_accepted, LineAcceptance::AcceptedWantMore);
        assert_eq!(result.highlighted.len(), 1);
        assert_eq!(
            result.highlighted[0].get(),
            "\u{1b}[36m@@ -1,2 +1,2 @@\u{1b}[0m\n"
        );

        let result = test_me
            .consume_line("+Hello, my first name is Johan", &thread_pool)
            .unwrap();
        assert_eq!(result.line_accepted, LineAcceptance::AcceptedWantMore);
        assert_eq!(result.highlighted.len(), 0);

        let mut result = test_me.consume_line(" I like pie.", &thread_pool).unwrap();
        assert_eq!(result.line_accepted, LineAcceptance::AcceptedDone);

        assert_eq!(result.highlighted.len(), 2);
        assert_eq!(
            result.highlighted[0].get(),
            concat!(
                "\u{1b}[2m\u{1b}[31m-Hello, my name is Johan\u{1b}[0m\n",
                "\u{1b}[2m\u{1b}[32m+\u{1b}[0mHello, my \u{1b}[7m\u{1b}[32mfirst \u{1b}[0mname is Johan\n"
            )
        );
        assert_eq!(result.highlighted[1].get(), " I like pie.\n");
    }
}
