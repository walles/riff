use crate::constants::*;

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
        .map(format_commit_part)
        .collect::<Vec<_>>();

    // FIXME: Placeholder logic until we have all tests
    let comma = format!("{}, {}", YELLOW, NORMAL);
    return format!(
        "{}{} ({}){}",
        YELLOW,
        commit_part,
        parenthesis_parts.join(&comma),
        NORMAL
    );
}

fn format_commit_part(part: &str) -> String {
    if part.starts_with("tag: ") {
        return format!("{}{}{}{}", BOLD, YELLOW, part, NORMAL);
    }

    return part.to_string();
}
