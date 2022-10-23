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

    // FIXME: Placeholder logic until we have all tests
    let comma = format!("{}, {}", YELLOW, NORMAL);
    return format!(
        "{}{} ({}){}",
        YELLOW,
        commit_part,
        parenthesis_parts
            .iter()
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

    // Handle "HEAD -> current_branch"
    if let Some(head_branch) = part.strip_prefix("HEAD -> ") {
        return format!("{}{}HEAD -> {}{}{}", BOLD, CYAN, GREEN, head_branch, NORMAL);
    }

    // FIXME: Handle current branch

    // FIXME: Handle any non-current branches

    return part.to_string();
}

fn compute_current_branch(candidates: &Vec<&str>) -> Option<String> {
    // Filter out realistic candidates
    let mut possible_candidates: Vec<String> = vec![];
    for part in candidates {
        if part.starts_with("tag: ") {
            // This is not a branch name
            continue;
        }

        if let Some(headless) = part.strip_prefix("HEAD -> ") {
            // Found it, this is conclusive!
            return Some(headless.to_owned());
        }

        possible_candidates.push(part.to_string());
    }

    // Pick obvious choices
    if possible_candidates.is_empty() {
        return None;
    }
    if possible_candidates.len() == 1 {
        return Some(possible_candidates[0].to_owned());
    }

    // We have multiple options, pick the one with the lowest number of slashes
    // FIXME: Can we just start with this loop?
    let mut fewest_slashes: Vec<String> = vec![];
    let mut lowest_slash_count = usize::MAX;
    for candidate in possible_candidates {
        let candidate_slash_count = candidate.matches('/').count();
        if candidate_slash_count > lowest_slash_count {
            // This one is worse, never mind
            continue;
        }

        if candidate_slash_count < lowest_slash_count {
            // This one is better, replace the others
            fewest_slashes.clear();
        }

        lowest_slash_count = candidate_slash_count;
        fewest_slashes.push(candidate);
    }

    assert!(!fewest_slashes.is_empty());
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
}
