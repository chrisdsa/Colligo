use crate::application::{DwlMode, ExitCode, VersionControl};
use crate::project::Project;
use log::{debug, error};
use std::io;
use std::path::Path;
use std::process::{Command, Output};

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
    fn clone(&self, manifest_dir: &str, project: &Project, mode: &DwlMode) -> Result<(), ExitCode> {
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

        let output = Command::new("git")
            .current_dir(manifest_dir)
            .args([
                "clone",
                url.as_str(),
                project.get_path().as_str(),
                "--quiet",
            ])
            .output();

        process_command_output(project, output, ExitCode::CloneFailed)
    }

    /// Clone a project from a URI to a path with a specific revision. The revision MUST be a branch
    /// or a tag. Commit ID are not supported.
    fn clone_lightweight(
        &self,
        manifest_dir: &str,
        project: &Project,
        mode: &DwlMode,
    ) -> Result<(), ExitCode> {
        let url = match mode {
            DwlMode::HTTPS => project.get_uri_https(),
            DwlMode::SSH => project.get_uri_ssh(),
        };
        let repo_path = Path::new(manifest_dir).join(project.get_path());

        debug!(
            "Cloning {} into {} @ {}",
            project.get_name(),
            repo_path.display(),
            project.get_revision()
        );

        let output = Command::new("git")
            .current_dir(manifest_dir)
            .args([
                "clone",
                "--depth",
                "1",
                "--branch",
                project.get_revision().as_str(),
                "--single-branch",
                "--quiet",
                url.as_str(),
                project.get_path().as_str(),
            ])
            .output();

        process_command_output(project, output, ExitCode::CloneFailed)
    }

    fn checkout(&self, manifest_dir: &str, project: &Project) -> Result<(), ExitCode> {
        let repo_path = Path::new(manifest_dir).join(project.get_path());

        debug!(
            "Checking out {} into {} @ {}",
            project.get_name(),
            repo_path.display(),
            project.get_revision()
        );

        let output = Command::new("git")
            .current_dir(&repo_path)
            .args(["fetch", "--prune", "--quiet"])
            .output();

        process_command_output(project, output, ExitCode::CheckoutFailed)?;

        let output = Command::new("git")
            .current_dir(&repo_path)
            .args(["checkout", project.get_revision(), "--quiet"])
            .output();

        process_command_output(project, output, ExitCode::CheckoutFailed)?;

        if is_branch(manifest_dir, project) {
            let output = Command::new("git")
                .current_dir(&repo_path)
                .args(["pull", "--ff-only", "--quiet"])
                .output();

            process_command_output(project, output, ExitCode::CheckoutFailed)?;
        }
        Ok(())
    }

    fn get_commit_id(&self, manifest_dir: &str, project: &Project) -> Result<String, ExitCode> {
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
            Err(e) => {
                error!("{e}");
                Err(ExitCode::GetCommitIdFailed)
            }
        }
    }
}

unsafe impl Sync for GitVersionControl {}

fn process_command_output(
    project: &Project,
    output: io::Result<Output>,
    code: ExitCode,
) -> Result<(), ExitCode> {
    match output {
        Ok(output) => {
            let message = String::from_utf8_lossy(&output.stderr).to_string();
            if is_message_ok(&message) {
                Ok(())
            } else {
                error!("[{}] {}", project.get_name(), message);
                Err(code)
            }
        }
        Err(e) => {
            error!("{e}");
            Err(code)
        }
    }
}

fn is_message_ok(message: &str) -> bool {
    if message.contains("fatal") || message.contains("error") {
        return false;
    }
    true
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
