use crate::application::{DwlMode, ManifestError, VersionControl};
use crate::project::Project;
use indicatif::ProgressBar;
use log::debug;
use regex::Regex;
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncReadExt};
use tokio::process::Command;

const DISPLAY_STATUS_SIZE: usize = 3;
const PROGRESS_REFRESH_RATE_MS: u64 = 100;

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

#[async_trait::async_trait]
impl VersionControl for GitVersionControl {
    async fn clone(
        &self,
        manifest_dir: &Path,
        project: &Project,
        mode: &DwlMode,
        pb: Option<&ProgressBar>,
        lightweight: bool,
    ) -> Result<(), ManifestError> {
        let url = match mode {
            DwlMode::HTTPS => project.get_uri_https(),
            DwlMode::SSH => project.get_uri_ssh(),
        };

        let repo_path = manifest_dir.join(project.get_path());

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
            .env("GIT_FLUSH", "1")
            .stderr(Stdio::piped())
            .stdout(Stdio::null());

        let display_status = [
            format!("{} cloning", project.get_path()),
            format!("{} complete", project.get_path()),
            format!("{} ERROR", project.get_path()),
        ];

        match process_command(&mut command, pb, &display_status).await {
            Ok(_) => {}
            Err(e) => {
                let msg = format!("{}\n{}\n", project.get_path(), e);
                return Err(ManifestError::FailedToCloneRepository(msg));
            }
        }

        if let Some(pb) = pb {
            pb.finish();
        }
        Ok(())
    }

    async fn checkout(
        &self,
        manifest_dir: &Path,
        project: &Project,
        pb: Option<&ProgressBar>,
        force: bool,
    ) -> Result<(), ManifestError> {
        let repo_path = manifest_dir.join(project.get_path());

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
        match process_command(&mut command, pb, &display_status).await {
            Ok(_) => {}
            Err(e) => {
                let msg = format!("{}\n{}\n", project.get_path(), e);
                return Err(ManifestError::FailedToCheckoutRepository(msg));
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
        match process_command(&mut command, pb, &display_status).await {
            Ok(_) => {}
            Err(e) => {
                let msg = format!("{}\n{}\n", project.get_path(), e);
                return Err(ManifestError::FailedToCheckoutRepository(msg));
            }
        }

        // Check if repository is dirty
        let output = Command::new("git")
            .current_dir(&repo_path)
            .args(["status", "--porcelain", "--untracked-files=no"])
            .output()
            .await;

        match output {
            Ok(output) => {
                let message = String::from_utf8_lossy(&output.stdout).to_string();
                if !message.is_empty() {
                    if let Some(pb) = pb {
                        pb.set_message(format!("{} ERROR", project.get_path()));
                    }
                    let msg = format!(
                        "{}, repository is dirty, please commit or stash your changes",
                        project.get_path()
                    );
                    return Err(ManifestError::FailedToCheckoutRepository(msg));
                }
            }
            Err(e) => {
                let msg = format!(
                    "{}. Failed to check repository status: \n{}\n",
                    project.get_path(),
                    e
                );
                return Err(ManifestError::FailedToCheckoutRepository(msg));
            }
        }

        if is_branch(manifest_dir, project).await {
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

            match process_command(&mut command, pb, &display_status).await {
                Ok(_) => {}
                Err(e) => {
                    let msg = format!("{}\n{}\n", project.get_path(), e);
                    return Err(ManifestError::FailedToCheckoutRepository(msg));
                }
            }
        }

        if let Some(pb) = pb {
            pb.finish();
        }
        Ok(())
    }

    async fn get_commit_id(
        &self,
        manifest_dir: &Path,
        project: &Project,
    ) -> Result<String, ManifestError> {
        let repo_path = manifest_dir.join(project.get_path());

        debug!(
            "Getting commit id for {} into {}",
            project.get_name(),
            repo_path.display()
        );

        let output = Command::new("git")
            .current_dir(&repo_path)
            .args(["rev-parse", "HEAD"])
            .output()
            .await;

        match output {
            Ok(output) => {
                let message = String::from_utf8_lossy(&output.stdout)
                    .to_string()
                    .replace(['\n', '\r'], "");
                Ok(message)
            }
            Err(e) => {
                let msg = format!("{path}\n{e}\n", path = project.get_path());
                Err(ManifestError::FailedToGetCommitId(msg))
            }
        }
    }

    async fn is_modified(
        &self,
        manifest_dir: &Path,
        project: &Project,
    ) -> Result<bool, ManifestError> {
        let repo_path = manifest_dir.join(project.get_path());

        let output = Command::new("git")
            .current_dir(&repo_path)
            .args(["status", "--porcelain", "--untracked-files=no"])
            .output()
            .await;

        match output {
            Ok(output) => {
                let message = String::from_utf8_lossy(&output.stdout).to_string();
                Ok(!message.is_empty())
            }
            Err(e) => {
                let msg = format!("{path}\n{e}\n", path = project.get_path());
                Err(ManifestError::FailedToDetermineIfRepoIsModified(msg))
            }
        }
    }
}

unsafe impl Sync for GitVersionControl {}

async fn process_command(
    command: &mut Command,
    pb: Option<&ProgressBar>,
    message: &[String; DISPLAY_STATUS_SIZE],
) -> Result<(), String> {
    const START_INDEX: usize = 0;
    const COMPLETE_INDEX: usize = 1;
    const ERROR_INDEX: usize = 2;
    const BUFFER_SIZE: usize = 128;

    let mut child = command.spawn().expect("Failed to launch command");

    let mut stderr_capture: Vec<String> = Vec::new();

    let re = Regex::new(r"(\d+)%").unwrap();

    if let Some(pb) = pb {
        pb.set_message(message[START_INDEX].clone());
    }

    let mut interval =
        tokio::time::interval(tokio::time::Duration::from_millis(PROGRESS_REFRESH_RATE_MS));
    let mut stderr = tokio::io::BufReader::new(child.stderr.take().expect("Failed to take stderr"));
    let mut read_buffer = [0u8; BUFFER_SIZE];

    loop {
        // Read stream from stderr. We cannot use lines since git is using \r to overwrite the line.
        tokio::select! {
            size = stderr.read(&mut read_buffer) => {
                if let Ok(size) = size {
                    // If the line contains progress information, update the progress bar
                    let msg = String::from_utf8_lossy(&read_buffer[..size]).to_string();
                    if let Some(cap) = re.captures(&msg) {
                        if let Some(match_) = cap.get(1) {
                            if let Ok(progress) = match_.as_str().parse::<u64>() {
                                if let Some(pb) = pb {
                                    pb.set_position(progress);
                                }
                            }
                        }
                    }
                    stderr_capture.push(msg);
                }
            }
            _ = interval.tick() => {}
            exit_status = child.wait() => {
                match exit_status {
                    Ok(exit_status) => {
                        if exit_status.success() {
                            // Print complete message
                            if let Some(pb) = pb {
                                pb.set_position(100);
                                pb.set_message(message[COMPLETE_INDEX].clone());
                            }
                            break;
                        } else {
                            // Update progress bar to show error
                            if let Some(pb) = pb {
                                pb.set_message(message[ERROR_INDEX].clone());
                            }

                            // Check for error messages in stderr
                            let mut lines = stderr.lines();
                            while let Ok(Some(line)) = lines.next_line().await {
                                stderr_capture.push(line);
                            }

                            let start = stderr_capture.iter().position(|line| line.contains("fatal") || line.contains("error"));
                            if let Some(start) = start {
                                let error_message: String = stderr_capture
                                .iter()
                                .skip(start)
                                .fold(String::new(), |acc, line| acc + line);
                                return Err(error_message);
                            }
                            return Err("Failed to capture git error message".to_string());
                        }
                    },
                    Err(e) => return Err(e.to_string()),
                }
            }
        }

        if let Some(pb) = pb {
            pb.tick();
        }
    }

    Ok(())
}

async fn is_branch<P: AsRef<Path>>(manifest_dir: P, project: &Project) -> bool {
    let repo_path = manifest_dir.as_ref().join(project.get_path());
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(["status"])
        .output()
        .await;
    match output {
        Ok(value) => {
            let message = String::from_utf8_lossy(&value.stdout).to_string();
            message.contains("On branch")
        }
        Err(_) => false,
    }
}
