use crate::constants::*;
use itertools::Itertools;

pub fn format_commit_line(line: &str) -> String {
    let parts = line.split('(').collect::<Vec<_>>();
    if parts.len() == 1 {
        // Just "commit: 123abc", color it all yellow
        return format!("{}{}{}", YELLOW, line, NORMAL);
    }

    let commit_part = parts[0].trim();
    let without_trailing_parenthesis = parts[1].strip_suffix(')');
    if without_trailing_parenthesis == None {
        // No final parenthesis, this is weird, fall back to showing everything
        // in yellow
        return format!("{}{}{}", YELLOW, line, NORMAL);
    }

    let parenthesis_parts = without_trailing_parenthesis.unwrap().split(", ");
    let current_branch = compute_current_branch(&parenthesis_parts);

    // FIXME: Placeholder logic until we have all tests
    let comma = format!("{}, {}", YELLOW, NORMAL);
    return format!(
        "{}{} ({}){}",
        YELLOW,
        commit_part,
        parenthesis_parts
            .map(|part| format_commit_part(part, &current_branch))
            .join(&comma),
        NORMAL
    );
}

fn format_commit_part(part: &str, current_branch: &Option<String>) -> String {
    if part.starts_with("tag: ") {
        return format!("{}{}{}{}", BOLD, YELLOW, part, NORMAL);
    }

    // FIXME: Is there a better way to express this condition than using
    // part.to_string()? Won't that allocate memory and stuff? I want something
    // that just compares the strings for equality.
    if &Some(part.to_string()) == current_branch {
        return format!("{}{}{}{}", BOLD, GREEN, part, NORMAL);
    }

    // FIXME: Handle "HEAD -> current_branch"

    return part.to_string();
}

fn compute_current_branch(_candidates: &std::str::Split<&str>) -> Option<String> {
    // FIXME: Implement this properly!
    return None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    use pretty_assertions::assert_eq;

    #[test]
    fn test_compute_current_branch() {
        assert_eq!(
            Some(String::from("master")),
            compute_current_branch(
                &"HEAD -> master, tag: 2.20.0, origin/master, origin/HEAD".split(", ")
            )
        );

        assert_eq!(
            Some(String::from("master")),
            compute_current_branch(&"tag: 2.20.0, origin/master, origin/HEAD, master".split(", "))
        );

        assert_eq!(
            Some(String::from("walles/threaded")),
            compute_current_branch(&"origin/walles/threaded, walles/threaded".split(", "))
        );

        // git show 686f3d7aefe9597395020ff0219eebc90e363d47
        assert_eq!(None, compute_current_branch(&"tag: 2.15".split(", ")));

        assert_eq!(
            Some(String::from("xeago-master")),
            compute_current_branch(&"xeago/master, xeago-master".split(", "))
        );
    }
}
