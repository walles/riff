use crate::commit_line::format_commit_line;
    static ref STATIC_HEADER_PREFIXES: Vec<(&'static str, &'static str)> = vec![
        ("diff ", FAINT),
        ("index ", FAINT),
        ("Binary files ", BOLD),
        ("copy from ", FAINT),
        ("copy to ", BOLD),
        ("rename from ", FAINT),
        ("rename to ", BOLD),
        ("similarity index ", FAINT),
        ("new file mode ", FAINT),
        ("deleted file mode ", FAINT),
        ("--- /dev/null", FAINT),
        ("+++ /dev/null", FAINT),
    ];
    fn consume_hunk_header(&mut self, line: &str) {
        self.consume_plain_linepart(HUNK_HEADER);

        if let Some(second_atat_index) = line.find(" @@ ") {
            // Highlight the function name
            self.consume_plain_linepart(FAINT);
            self.consume_plain_linepart(&line[..(second_atat_index + 4)]);
            self.consume_plain_linepart(BOLD);
            self.consume_plain_linepart(&line[(second_atat_index + 4)..]);
        } else {
            self.consume_plain_linepart(line);
        }

        self.consume_plain_line(NORMAL);
    }

        if line.starts_with("commit") {
            self.consume_plain_line(&format_commit_line(&line));
            return;
        }

        if line.starts_with("@@ ") {
            self.consume_hunk_header(&line);
            return;
        }
