use std::env;
use std::process::Command;

fn main() {
    // If the environment variable is set from CI
    let sha;
    if let Ok(git_sha) = env::var("GIT_SHA") {
        sha = git_sha;
    } else {
        // Get the current git SHA using git
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()
            .expect("Failed to execute git command");

        sha = String::from_utf8(output.stdout).expect("Invalid UTF-8 sequence");
    }

    // Set the environment variable
    println!("cargo:rustc-env=GIT_SHA={}", &sha.trim()[..7]);
}
