use crate::application::{DwlMode, ManifestError};
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

impl Default for GitVersionControl {
    fn default() -> Self {
        Self::new()
    }
}

impl GitVersionControl {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn init(
        &self,
        manifest_dir: &Path,
        project: &Project,
        mode: &DwlMode,
    ) -> Result<(), ManifestError> {
        let url = match mode {
            DwlMode::HTTPS => project.get_uri_https(),
            DwlMode::SSH => project.get_uri_ssh(),
        };

        let repo_path = manifest_dir.join(project.get_path());

        debug!(
            "Initializing {} into {}",
            project.get_name(),
            repo_path.display()
        );

        init_repository(&repo_path).await?;
        init_origin(&repo_path, &url).await?;

        Ok(())
    }

    pub async fn checkout(
        &self,
        manifest_dir: &Path,
        project: &Project,
        pb: Option<&ProgressBar>,
        force: bool,
        lightweight: bool,
    ) -> Result<(), ManifestError> {
        let repo_path = manifest_dir.join(project.get_path());

        debug!(
            "Checking out {} into {} @ {}",
            project.get_name(),
            repo_path.display(),
            project.get_revision()
        );

        let args = get_fetch_args(&repo_path, lightweight, project.get_revision()).await?;

        let mut command = Command::new("git");
        command
            .current_dir(&repo_path)
            .args(args)
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
        } else if lightweight {
            ["checkout", "FETCH_HEAD", "--progress"].to_vec()
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

    pub async fn get_commit_id(
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

    pub async fn is_modified(
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

                    // Capture info on what is being done
                    let info = msg.split(":").next().unwrap_or("").trim().to_string();

                    // Capture progress percentage and update progress bar
                    if let Some(cap) = re.captures(&msg) {
                        if let Some(match_) = cap.get(1) {
                            if let Ok(progress) = match_.as_str().parse::<u64>() {
                                if let Some(pb) = pb {
                                    pb.set_position(progress);
                                    pb.set_message(format!("{} {}", message[START_INDEX].clone(), info));
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

async fn init_repository(path: &Path) -> Result<(), ManifestError> {
    // Create directory if it does not exist
    if !path.exists() {
        tokio::fs::create_dir_all(path)
            .await
            .map_err(|e| ManifestError::FailedToInitialize(e.to_string()))?;
    }
    // Init git repository
    let out = Command::new("git")
        .current_dir(path)
        .args(["init", "--quiet"])
        .output()
        .await
        .map_err(|e| ManifestError::FailedToInitialize(e.to_string()))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(ManifestError::FailedToInitialize(
            "git init failed".to_string(),
        ))
    }
}

async fn init_origin(path: &Path, url: &str) -> Result<(), ManifestError> {
    // check if origin exists
    let exists = Command::new("git")
        .current_dir(path)
        .args(["remote", "get-url", "origin"])
        .output()
        .await
        .map(|o| o.status.success())
        .map_err(|e| ManifestError::FailedToInitialize(e.to_string()))?;

    if exists {
        // update existing origin
        let out = Command::new("git")
            .current_dir(path)
            .args(["remote", "set-url", "origin", url])
            .output()
            .await
            .map_err(|e| ManifestError::FailedToInitialize(e.to_string()))?;
        if out.status.success() {
            Ok(())
        } else {
            Err(ManifestError::FailedToInitialize(
                "git remote set-url failed".to_string(),
            ))
        }
    } else {
        // add new origin
        let out = Command::new("git")
            .current_dir(path)
            .args(["remote", "add", "origin", url])
            .output()
            .await
            .map_err(|e| ManifestError::FailedToInitialize(e.to_string()))?;
        if out.status.success() {
            Ok(())
        } else {
            Err(ManifestError::FailedToInitialize(
                "git remote add failed".to_string(),
            ))
        }
    }
}

async fn get_fetch_args(
    manifest_dir: &Path,
    lightweight: bool,
    revision: &str,
) -> Result<Vec<String>, ManifestError> {
    // Is shallow?
    let out = Command::new("git")
        .current_dir(manifest_dir)
        .args(["rev-parse", "--is-shallow-repository"])
        .output()
        .await
        .map_err(|e| ManifestError::FailedToCheckoutRepository(e.to_string()))?;

    if !out.status.success() {
        return Err(ManifestError::FailedToCheckoutRepository(
            "git rev-parse failed".to_string(),
        ));
    }

    let is_shallow = String::from_utf8_lossy(&out.stdout)
        .to_string()
        .contains("true");

    // Set args
    if lightweight {
        Ok(vec![
            "fetch".to_string(),
            "--progress".to_string(),
            "--tags".to_string(),
            "--prune".to_string(),
            "--depth".to_string(),
            "1".to_string(),
            "origin".to_string(),
            revision.to_string(),
        ])
    } else if is_shallow {
        Ok(vec![
            "fetch".to_string(),
            "--progress".to_string(),
            "--tags".to_string(),
            "--prune".to_string(),
            "--unshallow".to_string(),
            "origin".to_string(),
        ])
    } else {
        Ok(vec![
            "fetch".to_string(),
            "--progress".to_string(),
            "--tags".to_string(),
            "--prune".to_string(),
            "origin".to_string(),
        ])
    }
}
