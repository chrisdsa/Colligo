use std::process::Command;

fn main() {
    // Get the current git SHA
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("Failed to execute git command");

    let git_sha = String::from_utf8(output.stdout).expect("Invalid UTF-8 sequence");

    // Set the environment variable
    println!("cargo:rustc-env=GIT_SHA={}", &git_sha.trim()[..7]);
}
