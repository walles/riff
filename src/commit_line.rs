use crate::constants::*;
use itertools::Itertools;

// Highlight lines starting with "commit: "

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

    let parenthesis_parts = without_trailing_parenthesis
        .unwrap()
        .split(", ")
        .collect_vec();
    let current_branch = compute_current_branch(&parenthesis_parts);

    let comma = format!("{}, {}", YELLOW, NORMAL);
    return format!(
        "{}{} ({}{}){}",
        YELLOW,
        commit_part,
        parenthesis_parts
            .iter()
            .map(|part| format_commit_part(part, &current_branch))
            .join(&comma),
        YELLOW,
        NORMAL
    );
}

fn format_commit_part(part: &str, current_branch: &Option<String>) -> String {
    if part.starts_with("tag: ") {
        return format!("{}{}{}{}", BOLD, YELLOW, part, NORMAL);
    }

    // FIXME: Can we do this with one readable if-statement instead?
    if let Some(current_branch_4_realz) = current_branch {
        if current_branch_4_realz == part {
            return format!("{}{}{}{}", BOLD, GREEN, part, NORMAL);
        }
    }

    // Handle "HEAD -> current_branch"
    if let Some(head_branch) = part.strip_prefix("HEAD -> ") {
        return format!("{}{}HEAD -> {}{}{}", BOLD, CYAN, GREEN, head_branch, NORMAL);
    }

    // Assume this is a branch, but not the current one
    return format!("{}{}{}{}", BOLD, RED, part, NORMAL);
}

fn compute_current_branch(candidates: &Vec<&str>) -> Option<String> {
    // If we find multiple options, pick the one with the lowest number of
    // slashes
    let mut fewest_slashes: Vec<&str> = vec![];
    let mut lowest_slash_count = usize::MAX;
    for candidate in candidates {
        if candidate.starts_with("tag: ") {
            // This is not a branch name
            continue;
        }

        if let Some(headless) = candidate.strip_prefix("HEAD -> ") {
            // Found it, this is conclusive!
            return Some(headless.to_owned());
        }

        let candidate_slash_count = candidate.matches('/').count();
        if candidate_slash_count > lowest_slash_count {
            // This one is worse than what we already have, never mind
            continue;
        }

        if candidate_slash_count < lowest_slash_count {
            // This one is better, replace the others
            fewest_slashes.clear();
        }

        lowest_slash_count = candidate_slash_count;
        fewest_slashes.push(candidate);
    }

    if fewest_slashes.is_empty() {
        return None;
    }

    if fewest_slashes.len() == 1 {
        return Some(fewest_slashes[0].to_owned());
    }

    // Give up, we don't know
    return None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    use pretty_assertions::assert_eq;

    #[test]
    fn test_compute_current_branch() {
        // git show 686f3d7aefe9597395020ff0219eebc90e363d47
        assert_eq!(
            None,
            compute_current_branch(&"tag: 2.15".split(", ").collect_vec())
        );

        assert_eq!(
            Some(String::from("master")),
            compute_current_branch(
                &"HEAD -> master, tag: 2.20.0, origin/master, origin/HEAD"
                    .split(", ")
                    .collect_vec()
            )
        );

        assert_eq!(
            Some(String::from("master")),
            compute_current_branch(
                &"tag: 2.20.0, origin/master, origin/HEAD, master"
                    .split(", ")
                    .collect_vec()
            )
        );

        assert_eq!(
            Some(String::from("walles/threaded")),
            compute_current_branch(
                &"origin/walles/threaded, walles/threaded"
                    .split(", ")
                    .collect_vec()
            )
        );

        assert_eq!(
            Some(String::from("xeago-master")),
            compute_current_branch(&"xeago/master, xeago-master".split(", ").collect_vec())
        );
    }

    const NORMAL_INTENSITY: &str = "\x1b[22m";

    #[test]
    // Verify that the blue background goes all the way from the start of the
    // string to its end
    fn test_format_commit_line_tags_branches() {
        assert_eq!(
            "".to_owned() +
            YELLOW +
            BLUE_TO_END_OF_LINE +
            "commit 62da46c7b300321119d399bdc69bfb2d56d5da57 (" +
            BOLD +
            "tag: 2.21.0"+
            NORMAL_INTENSITY+
            ", "+
            BOLD + RED +
            "origin/master" +
            NORMAL_INTENSITY + YELLOW +
            ", " +
            BOLD + RED +
            "origin/HEAD" +
            NORMAL_INTENSITY + YELLOW +
            ", " +
            BOLD + GREEN +
            "master" +
            NORMAL_INTENSITY + YELLOW +
            ")" +
            NORMAL,
        // This commit is from the master branch
        format_commit_line("commit 62da46c7b300321119d399bdc69bfb2d56d5da57 (tag: 2.21.0, origin/master, origin/HEAD, master)"));
    }
}
