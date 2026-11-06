// jj runs diff tools in a temporary directory containing left/ and right/
// copies of the trees, deleting that directory when the tool exits. It leaves
// PWD pointing at the user's directory though, so that's where hyperlinks
// should point.

#![cfg(unix)]

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

const OLD_C: &str = "int main() {\n  1;\n  2;\n  3;\n  4;\n  5;\n}\n";
const NEW_C: &str = "int main() {\n  1;\n  2;\n  3;\n  4;\n  50;\n}\n";

fn file_url(path: &Path) -> String {
    let canonical = std::fs::canonicalize(path).expect("Path should canonicalize");
    format!("file://{}", canonical.display())
}

fn write(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

#[test]
fn test_jj_diff_tool_links_to_pwd_not_temp_dir() {
    let repo = tempfile::tempdir().unwrap();
    write(&repo.path().join("m.c"), NEW_C);

    let jj_temp = tempfile::tempdir().unwrap();
    write(&jj_temp.path().join("left/m.c"), OLD_C);
    write(&jj_temp.path().join("right/m.c"), NEW_C);

    // This is what jj does: cwd is the temp dir, PWD is the user's directory
    let output = Command::new(env!("CARGO_BIN_EXE_riff"))
        .args(["--color=on", "--no-pager", "left", "right"])
        .current_dir(jj_temp.path())
        .env("PWD", repo.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    let repo_url = file_url(&repo.path().join("m.c"));
    assert!(
        stdout.contains(&format!("{}#6", repo_url)),
        "Expected a link to {}#6 in:\n{}",
        repo_url,
        stdout
    );

    let temp_dir_url = file_url(jj_temp.path());
    assert!(
        !stdout.contains(&temp_dir_url),
        "Expected no links into {} in:\n{}",
        temp_dir_url,
        stdout
    );
}

#[test]
fn test_links_relative_to_cwd_when_not_found_under_pwd() {
    // This is what "cargo test" does when run from a subdirectory: PWD is the
    // subdirectory, cwd is the package root.
    let cwd = tempfile::tempdir().unwrap();
    write(&cwd.path().join("m.c"), NEW_C);
    let pwd = tempfile::tempdir().unwrap();

    let diff = "--- a/m.c\n+++ b/m.c\n@@ -2,6 +2,6 @@ int main() {\n   1;\n   2;\n   3;\n   4;\n-  5;\n+  50;\n }\n";

    let mut child = Command::new(env!("CARGO_BIN_EXE_riff"))
        .args(["--color=on", "--no-pager"])
        .current_dir(cwd.path())
        .env("PWD", pwd.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(diff.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    let cwd_url = file_url(&cwd.path().join("m.c"));
    assert!(
        stdout.contains(&cwd_url),
        "Expected a link to {} in:\n{}",
        cwd_url,
        stdout
    );
}
