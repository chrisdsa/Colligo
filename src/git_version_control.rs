use crate::application::{DwlMode, VersionControl};
use crate::project::Project;
use indicatif::ProgressBar;
use log::debug;
use regex::Regex;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};

const DISPLAY_STATUS_SIZE: usize = 3;
const PROGRESS_REFRESH_RATE_MS: u64 = 10;

pub struct GitVersionControl {}

impl GitVersionControl {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for GitVersionControl {
    fn default() -> Self {
        Self::new()
    }
}

impl VersionControl for GitVersionControl {
    fn clone(
        &self,
        manifest_dir: &str,
        project: &Project,
        mode: &DwlMode,
        pb: Option<&ProgressBar>,
        lightweight: bool,
    ) -> Result<(), String> {
        let url = match mode {
            DwlMode::HTTPS => project.get_uri_https(),
            DwlMode::SSH => project.get_uri_ssh(),
        };

        let repo_path = Path::new(manifest_dir).join(project.get_path());

        debug!(
            "Cloning {} into {}",
            project.get_name(),
            repo_path.display()
        );

        let mut command = Command::new("git");

        let args: Vec<&str> = if lightweight {
            [
                "clone",
                "--depth",
                "1",
                "--branch",
                project.get_revision().as_str(),
                "--single-branch",
                "--progress",
                url.as_str(),
                project.get_path().as_str(),
            ]
            .to_vec()
        } else {
            [
                "clone",
                url.as_str(),
                project.get_path().as_str(),
                "--progress",
            ]
            .to_vec()
        };

        command
            .current_dir(manifest_dir)
            .args(args)
            .stderr(Stdio::piped())
            .stdout(Stdio::null());

        let display_status = [
            format!("{} cloning", project.get_path()),
            format!("{} complete", project.get_path()),
            format!("{} ERROR", project.get_path()),
        ];

        match process_command(&mut command, pb, &display_status) {
            Ok(_) => {}
            Err(e) => {
                return Err(format!("[{}] Cloning error: \n{}\n", project.get_path(), e));
            }
        }

        if let Some(pb) = pb {
            pb.finish();
        }
        Ok(())
    }

    fn checkout(
        &self,
        manifest_dir: &str,
        project: &Project,
        pb: Option<&ProgressBar>,
        force: bool,
    ) -> Result<(), String> {
        let repo_path = Path::new(manifest_dir).join(project.get_path());

        debug!(
            "Checking out {} into {} @ {}",
            project.get_name(),
            repo_path.display(),
            project.get_revision()
        );

        let mut command = Command::new("git");
        command
            .current_dir(&repo_path)
            .args(["fetch", "--prune", "--progress"])
            .stderr(Stdio::piped())
            .stdout(Stdio::null());

        let display_status = [
            format!("{} fetch", project.get_path()),
            format!("{} complete", project.get_path()),
            format!("{} ERROR", project.get_path()),
        ];
        match process_command(&mut command, pb, &display_status) {
            Ok(_) => {}
            Err(e) => {
                return Err(format!("[{}] fetch error: \n{}\n", project.get_path(), e));
            }
        }

        let args = if force {
            ["checkout", project.get_revision(), "--progress", "--force"].to_vec()
        } else {
            ["checkout", project.get_revision(), "--progress"].to_vec()
        };
        let mut command = Command::new("git");
        command
            .current_dir(&repo_path)
            .args(args)
            .stderr(Stdio::piped())
            .stdout(Stdio::null());

        let display_status = [
            format!("{} checkout", project.get_path()),
            format!("{} complete", project.get_path()),
            format!("{} ERROR", project.get_path()),
        ];
        match process_command(&mut command, pb, &display_status) {
            Ok(_) => {}
            Err(e) => {
                return Err(format!(
                    "[{}] checkout error: \n{}\n",
                    project.get_path(),
                    e
                ));
            }
        }

        // Check if repository is dirty
        let output = Command::new("git")
            .current_dir(&repo_path)
            .args(["status", "--porcelain", "--untracked-files=no"])
            .output();

        match output {
            Ok(output) => {
                let message = String::from_utf8_lossy(&output.stdout).to_string();
                if !message.is_empty() {
                    if let Some(pb) = pb {
                        pb.set_message(format!("{} ERROR", project.get_path()));
                    }
                    return Err(format!(
                        "[{}] repository is dirty, please commit or stash your changes",
                        project.get_path()
                    ));
                }
            }
            Err(e) => {
                return Err(format!(
                    "[{}] failed to check repository status: \n{}\n",
                    project.get_path(),
                    e
                ));
            }
        }

        if is_branch(manifest_dir, project) {
            let mut command = Command::new("git");
            command
                .current_dir(&repo_path)
                .args(["merge", "--ff-only", "--progress"])
                .stderr(Stdio::piped())
                .stdout(Stdio::null());

            let display_status = [
                format!("{} merge", project.get_path()),
                format!("{} complete", project.get_path()),
                format!("{} ERROR", project.get_path()),
            ];

            match process_command(&mut command, pb, &display_status) {
                Ok(_) => {}
                Err(e) => {
                    return Err(format!("[{}] merge error: \n{}\n", project.get_path(), e));
                }
            }
        }

        if let Some(pb) = pb {
            pb.finish();
        }
        Ok(())
    }

    fn get_commit_id(&self, manifest_dir: &str, project: &Project) -> Result<String, String> {
        let repo_path = Path::new(manifest_dir).join(project.get_path());

        debug!(
            "Getting commit id for {} into {}",
            project.get_name(),
            repo_path.display()
        );

        let output = Command::new("git")
            .current_dir(&repo_path)
            .args(["rev-parse", "HEAD"])
            .output();

        match output {
            Ok(output) => {
                let message = String::from_utf8_lossy(&output.stdout)
                    .to_string()
                    .replace(['\n', '\r'], "");
                Ok(message)
            }
            Err(e) => Err(format!("ERROR [{path}]: {e}", path = project.get_path())),
        }
    }

    fn is_modified(&self, manifest_dir: &str, project: &Project) -> Result<bool, String> {
        let repo_path = Path::new(manifest_dir).join(project.get_path());

        let output = Command::new("git")
            .current_dir(&repo_path)
            .args(["status", "--porcelain", "--untracked-files=no"])
            .output();

        match output {
            Ok(output) => {
                let message = String::from_utf8_lossy(&output.stdout).to_string();
                Ok(!message.is_empty())
            }
            Err(e) => Err(format!("ERROR [{path}]: {e}", path = project.get_path())),
        }
    }
}

unsafe impl Sync for GitVersionControl {}

fn process_command(
    command: &mut Command,
    pb: Option<&ProgressBar>,
    message: &[String; DISPLAY_STATUS_SIZE],
) -> Result<(), String> {
    const START_INDEX: usize = 0;
    const COMPLETE_INDEX: usize = 1;
    const ERROR_INDEX: usize = 2;

    let mut child = command.spawn().expect("Failed to launch command");

    let mut stderr_capture = Vec::new();

    let re = Regex::new(r"(\d+)%").unwrap();

    if let Some(pb) = pb {
        pb.set_message(message[START_INDEX].clone());
    }

    let stderr = BufReader::new(child.stderr.take().expect("Failed to get stderr"));
    for line in stderr.lines() {
        let line = line.expect("Failed to read line");
        stderr_capture.push(line.clone());

        // If the line contains progress information, update the progress bar
        if let Some(cap) = re.captures(&line) {
            if let Some(match_) = cap.get(1) {
                if let Ok(progress) = match_.as_str().parse::<u64>() {
                    if let Some(pb) = pb {
                        pb.set_position(progress);
                    }
                }
            }
        }
        // Sleep for a short time to avoid hogging the CPU
        std::thread::sleep(std::time::Duration::from_millis(PROGRESS_REFRESH_RATE_MS));
    }

    let status = child.wait().expect("Failed to wait on child");

    if !status.success() {
        // Update progress bar to show error
        if let Some(pb) = pb {
            pb.set_message(message[ERROR_INDEX].clone());
        }

        // Check for error messages in stderr
        let error_message: String = stderr_capture
            .iter()
            .filter(|line| line.contains("fatal") || line.contains("error"))
            .fold(String::new(), |acc, line| acc + line + "\n");

        return Err(error_message);
    }

    // Print complete message
    if let Some(pb) = pb {
        pb.set_message(message[COMPLETE_INDEX].clone());
    }

    Ok(())
}

fn is_branch(manifest_dir: &str, project: &Project) -> bool {
    let repo_path = Path::new(manifest_dir).join(project.get_path());
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(["status"])
        .output();
    match output {
        Ok(value) => {
            let message = String::from_utf8_lossy(&value.stdout).to_string();
            message.contains("On branch")
        }
        Err(_) => false,
    }
}
