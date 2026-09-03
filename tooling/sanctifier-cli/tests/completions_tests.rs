use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_completions_bash() {
    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    cmd.arg("completions")
        .arg("bash")
        .assert()
        .success()
        .stdout(predicate::str::contains("_sanctifier()"));
}

#[test]
fn test_completions_zsh() {
    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    cmd.arg("completions")
        .arg("zsh")
        .assert()
        .success()
        .stdout(predicate::str::contains("#compdef sanctifier"));
}

#[test]
fn test_completions_fish() {
    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    cmd.arg("completions")
        .arg("fish")
        .assert()
        .success()
        .stdout(predicate::str::contains("complete -c sanctifier"));
}

#[test]
fn test_completions_powershell() {
    let mut cmd = Command::cargo_bin("sanctifier").unwrap();
    cmd.arg("completions")
        .arg("powershell")
        .assert()
        .success()
        .stdout(predicate::str::contains("Register-ArgumentCompleter"));
}
