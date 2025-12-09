use crate::default_manifest::DEFAULT_MANIFEST_FILE;
use crate::project::{Project, ProjectAction, ProjectFileAction};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::collections::VecDeque;
use std::fmt::Display;
use std::fs;
use std::fs::File;
use std::io::Write;
#[cfg(target_os = "linux")]
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::sync::mpsc::{channel, Sender};

use crate::version_control::GitVersionControl;
#[cfg(target_os = "windows")]
use std::os::windows::fs::symlink_file as symlink;

// Application description
pub const APP_NAME: &str = "Colligo";

// Command line arguments
pub const GENERATE_MANIFEST: &str = "generate";

pub const MANIFEST_INPUT: &str = "input";
pub const MANIFEST_INPUT_DEFAULT: &str = "manifest.xml";

pub const SYNC: &str = "sync";
pub const PIN: &str = "pin";

pub const LIGHT: &str = "light";
pub const QUIET: &str = "quiet";
pub const FORCE: &str = "force";
pub const HTTPS: &str = "https";

pub const LIST: &str = "list";
pub const STATUS: &str = "status";

#[derive(Clone)]
pub enum DwlMode {
    HTTPS,
    SSH,
}

pub trait ManifestParser {
    /// Parse a manifest file and return a vector of projects.
    fn parse(&self, file: &str) -> Result<Vec<Project>, ManifestError>;

    /// Compose a manifest file from a vector of projects.
    fn compose(&self, projects: &[Project]) -> Result<String, ManifestError>;
}

/// VersionControl is a trait that defines the methods to interact with a version control system.
/// The manifest file path is used to avoid changing the working directory which may results in
/// unexpected behavior when multiple threads are used.
/// When using the lightweight option, the revision MUST be a branch or a tag. Commit ID are not supported.
#[async_trait::async_trait]
pub trait XVersionControl: Send + Sync {
    /// Clone a project from a URI to a path.
    async fn clone(
        &self,
        manifest_dir: &Path,
        project: &Project,
        mode: &DwlMode,
        pb: Option<&ProgressBar>,
        lightweight: bool,
    ) -> Result<(), ManifestError>;

    /// Update a project to a specific revision.
    async fn checkout(
        &self,
        manifest_dir: &Path,
        project: &Project,
        pb: Option<&ProgressBar>,
        force: bool,
    ) -> Result<(), ManifestError>;

    /// Status of a project.
    async fn get_commit_id(
        &self,
        manifest_dir: &Path,
        project: &Project,
    ) -> Result<String, ManifestError>;

    /// Return true if the project has modified file(s).
    async fn is_modified(
        &self,
        manifest_dir: &Path,
        project: &Project,
    ) -> Result<bool, ManifestError>;
}

#[derive(Debug, Clone, PartialEq)]
pub enum ManifestError {
    FailedToInitialize(String),
    FailedToGetAbsolutePath(String),
    FileDoesNotExist(String),
    FailedToReadManifest(String),
    FailedToParseManifest(String),
    FailedToGenerateDefaultManifest(String),
    FailedToComposeManifest(String),
    FailedToExecuteAction(String),
    FailedToCheckoutRepository(String),
    FailedToGetCommitId(String),
    FailedToDetermineIfRepoIsModified(String),
    FailedToSync(String),
    FailedToSaveFile(String),
    MissingDependency(String),
}

impl Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManifestError::FailedToInitialize(e) => {
                write!(f, "Failed to initialize repository: {}", e)
            }
            ManifestError::FailedToGetAbsolutePath(e) => {
                write!(f, "Failed to get absolute path of {}", e)
            }
            ManifestError::FileDoesNotExist(e) => {
                write!(f, "File does not exist: {}", e)
            }
            ManifestError::FailedToReadManifest(e) => {
                write!(f, "Failed to read manifest file: {}", e)
            }
            ManifestError::FailedToParseManifest(e) => {
                write!(f, "Failed to parse manifest file: {}", e)
            }
            ManifestError::FailedToComposeManifest(e) => {
                write!(f, "Failed to compose manifest file: {}", e)
            }
            ManifestError::FailedToExecuteAction(e) => {
                write!(f, "Failed to execute action: {}", e)
            }
            ManifestError::FailedToCheckoutRepository(e) => {
                write!(f, "Failed to checkout repository: {}", e)
            }
            ManifestError::FailedToGetCommitId(e) => {
                write!(f, "Failed to get commit id: {}", e)
            }
            ManifestError::FailedToDetermineIfRepoIsModified(e) => {
                write!(f, "Failed to determine if repo is modified: {}", e)
            }
            ManifestError::FailedToSync(e) => {
                write!(f, "Failed to sync manifest: {}", e)
            }
            ManifestError::FailedToSaveFile(e) => {
                write!(f, "Failed to save file: {}", e)
            }
            ManifestError::MissingDependency(e) => {
                write!(f, "Failed to find dependency: {}", e)
            }
            ManifestError::FailedToGenerateDefaultManifest(e) => {
                write!(f, "Failed to generate default manifest file: {}", e)
            }
        }
    }
}

pub struct ManifestInstance {
    /// Absolute path to the manifest file
    filename: PathBuf,
    /// Manifest content
    file: String,
    /// Vector with all projects
    projects: Vec<Project>,
}

impl ManifestInstance {
    pub fn try_from<P: AsRef<Path>>(filename: P) -> Result<Self, ManifestError> {
        let file = read_manifest(&filename)?;

        let filename = filename.as_ref().canonicalize().map_err(|_| {
            let name = filename.as_ref().display().to_string();
            ManifestError::FailedToGetAbsolutePath(name)
        })?;

        Ok(Self {
            filename,
            file,
            projects: Vec::new(),
        })
    }

    pub fn get_filename(&self) -> &PathBuf {
        &self.filename
    }

    pub fn get_file(&self) -> &String {
        &self.file
    }

    pub fn get_projects(&self) -> &Vec<Project> {
        &self.projects
    }

    pub fn parse(&mut self) -> Result<(), ManifestError> {
        let parser = crate::xml_parser::XmlParser::new();
        self.projects = parser.parse(&self.file)?;
        Ok(())
    }

    pub async fn sync(
        &self,
        mode: &DwlMode,
        lightweight: bool,
        quiet: bool,
        force: bool,
    ) -> Result<(), ManifestError> {
        // Prepare progress bar
        let multi_progress = MultiProgress::new();
        let style = ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40} {pos:>3}/{len:3} {msg}")
            .expect("Failed to create progress bar style")
            .progress_chars("##-");

        let mut progress_bars: Vec<Option<ProgressBar>> = Vec::new();

        for _ in self.projects.iter() {
            let pb = if !quiet {
                let instance = multi_progress.add(ProgressBar::new(100));
                instance.set_style(style.clone());
                Some(instance)
            } else {
                None
            };

            progress_bars.push(pb);
        }

        // Create channel to save result
        let (tx, mut rx) = channel(255);

        // Handle
        let mut _handles = Vec::new();

        // Spawn a thread for each project
        for project in self.projects.iter() {
            let tx = tx.clone();
            let pb = progress_bars.pop().expect("Failed to get progress bar");
            let dir = self.get_manifest_dir();
            let project = project.clone();
            let mode = mode.clone();

            let handle = tokio::task::spawn(async move {
                let mut result;

                let vcs = GitVersionControl::new();

                result = vcs.init(&dir, &project, &mode).await;
                send_result(&tx, result.clone()).await;

                if result.is_ok() {
                    result = vcs
                        .checkout(&dir, &project, pb.as_ref(), force, lightweight)
                        .await;
                    send_result(&tx, result.clone()).await;
                }

                if result.is_ok() {
                    result = execute_actions(&dir, &project);
                    send_result(&tx, result.clone()).await;
                }

                if let Some(pb) = pb {
                    pb.finish();
                }
            });

            _handles.push(handle);
        }
        drop(tx);

        // Wait for task to finish
        // Save error messages if any
        let mut errors: Vec<String> = Vec::new();
        while let Some(error) = rx.recv().await {
            if let Err(e) = error {
                errors.push(format!("{e}"))
            }
        }

        // If there is any error, return it
        if !errors.is_empty() {
            // Save all error message in string, separated by a new line
            let mut error_msg = String::new();
            error_msg.push_str("\n\n");

            for msg in errors.iter() {
                error_msg.push_str(msg);
                error_msg.push('\n');
            }

            return Err(ManifestError::FailedToSync(error_msg));
        }

        Ok(())
    }

    pub async fn pin(&self) -> Result<Self, ManifestError> {
        let mut projects: Vec<Project> = Vec::new();
        let vcs = GitVersionControl::new();
        let manifest_dir = self.get_manifest_dir();

        for project in self.projects.iter() {
            let commit_id = vcs.get_commit_id(manifest_dir.as_path(), project).await?;
            let pinned_project = project.pin(commit_id);
            projects.push(pinned_project);
        }

        let parser = crate::xml_parser::XmlParser::new();
        let file = parser.compose(&projects)?;

        Ok(Self {
            filename: self.filename.clone(),
            file,
            projects,
        })
    }

    fn get_manifest_dir(&self) -> PathBuf {
        let file_path = Path::new(&self.filename);
        let abs_path = file_path.canonicalize().unwrap_or("./".into());
        let workdir = abs_path.parent().unwrap_or("./".as_ref());

        workdir.to_path_buf()
    }
}

pub fn generate_default_manifest(destination: &String) -> Result<(), ManifestError> {
    let mut file = File::create(destination).map_err(|e| {
        let msg = format!("Failed to create manifest file {destination}: {e}");
        ManifestError::FailedToGenerateDefaultManifest(msg)
    })?;

    file.write_all(DEFAULT_MANIFEST_FILE.as_bytes())
        .map_err(|e| {
            let msg = format!("Failed to write to manifest file {destination}: {e}");
            ManifestError::FailedToGenerateDefaultManifest(msg)
        })?;
    Ok(())
}

fn read_manifest<P: AsRef<Path>>(filename: P) -> Result<String, ManifestError> {
    if !filename.as_ref().exists() {
        return Err(ManifestError::FileDoesNotExist(
            filename.as_ref().display().to_string(),
        ));
    }

    fs::read_to_string(filename).map_err(|e| ManifestError::FailedToReadManifest(e.to_string()))
}

async fn send_result(sender: &Sender<Result<(), ManifestError>>, code: Result<(), ManifestError>) {
    sender
        .send(code.clone())
        .await
        .expect("Failed to send result through channel.");
}

fn execute_actions(manifest_dir: &Path, project: &Project) -> Result<(), ManifestError> {
    for action in project.get_actions() {
        match action {
            ProjectAction::FileAction(ProjectFileAction::LinkFile(src, dest)) => {
                let dest = manifest_dir.join(dest);
                let src = manifest_dir.join(project.get_path()).join(src);

                let relative_src = get_relative_path_for_symlink(&src, &dest);
                prepare_file_destination(&dest)?;

                symlink(&relative_src, &dest).map_err(|e| {
                    let msg = format!(
                        "Failed to create symlink {} to {}: {e}",
                        src.display(),
                        dest.display()
                    );
                    ManifestError::FailedToExecuteAction(msg)
                })?;
            }
            ProjectAction::FileAction(ProjectFileAction::CopyFile(src, dest)) => {
                let src = Path::new(manifest_dir).join(project.get_path()).join(src);
                let dest = Path::new(manifest_dir).join(dest);
                prepare_file_destination(&dest)?;
                fs::copy(&src, &dest).map_err(|e| {
                    let msg = format!(
                        "Failed to copy file {} to {}: {e}",
                        src.display(),
                        dest.display()
                    );
                    ManifestError::FailedToExecuteAction(msg)
                })?;
            }
            ProjectAction::FileAction(ProjectFileAction::CopyDir(src, dest)) => {
                let src = Path::new(manifest_dir).join(project.get_path()).join(src);
                let dest = Path::new(manifest_dir).join(dest);
                prepare_file_destination(&dest)?;
                copy_directory(&src, &dest).map_err(|e| {
                    let msg = format!(
                        "Failed to copy directory {} to {}: {e}",
                        src.display(),
                        dest.display()
                    );
                    ManifestError::FailedToExecuteAction(msg)
                })?;
            }
            ProjectAction::DeleteProject => {
                let dest = Path::new(manifest_dir).join(project.get_path());
                fs::remove_dir_all(&dest).map_err(|e| {
                    let msg = format!("Failed to remove directory {}: {e}", dest.display());
                    ManifestError::FailedToExecuteAction(msg)
                })?;
            }
        }
    }
    Ok(())
}

pub fn save_file(filename: &String, content: &String) -> Result<(), ManifestError> {
    let mut file = File::create(filename).map_err(|e| {
        let msg = format!("Failed to create file {filename}: {e}");
        ManifestError::FailedToSaveFile(msg)
    })?;

    file.write(content.as_bytes()).map_err(|e| {
        let msg = format!("Failed to write to file {filename}: {e}");
        ManifestError::FailedToSaveFile(msg)
    })?;
    Ok(())
}

pub fn assert_dependencies() -> Result<(), ManifestError> {
    const GIT: &str = "git";
    Command::new(GIT)
        .arg("--version")
        .output()
        .map_err(|_| ManifestError::MissingDependency(GIT.to_string()))?;
    Ok(())
}

fn prepare_file_destination(dest: &PathBuf) -> Result<(), ManifestError> {
    // Inspect without following symlinks
    match fs::symlink_metadata(dest) {
        Ok(meta) => {
            let ft = meta.file_type();
            if ft.is_symlink() {
                fs::remove_file(dest).map_err(|e| {
                    ManifestError::FailedToExecuteAction(format!(
                        "Failed to remove symlink {}: {e}",
                        dest.display()
                    ))
                })?;
            } else if ft.is_dir() {
                fs::remove_dir_all(dest).map_err(|e| {
                    ManifestError::FailedToExecuteAction(format!(
                        "Failed to remove directory {}: {e}",
                        dest.display()
                    ))
                })?;
            } else {
                fs::remove_file(dest).map_err(|e| {
                    ManifestError::FailedToExecuteAction(format!(
                        "Failed to remove file {}: {e}",
                        dest.display()
                    ))
                })?;
            }
        }
        Err(e) => {
            // If the path doesn't exist, that's fine; otherwise return error.
            if e.kind() != std::io::ErrorKind::NotFound {
                return Err(ManifestError::FailedToExecuteAction(format!(
                    "Failed to stat {}: {e}",
                    dest.display()
                )));
            }
        }
    }

    // Create parent folder if needed
    let parent = dest.parent().unwrap_or("./".as_ref());
    if let Err(e) = fs::create_dir_all(parent) {
        let msg = format!(
            "Failed to create parent directory {}: {e}",
            parent.display(),
        );
        return Err(ManifestError::FailedToExecuteAction(msg));
    }

    Ok(())
}

/// Return the relative path seen from base to create a symlink.
fn get_relative_path_for_symlink(path: &Path, base: &Path) -> PathBuf {
    let base_dir = base.parent().unwrap_or("".as_ref());

    let path_dir = path.parent().unwrap_or("".as_ref());
    let path_filename = path.file_name().unwrap_or("".as_ref());

    let path = pathdiff::diff_paths(path_dir, base_dir).unwrap_or("./".into());
    path.join(path_filename)
}

fn copy_directory(src: &Path, dest: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dest)?;

    let mut queue = VecDeque::new();
    queue.push_back(src.to_path_buf());

    while let Some(src_path) = queue.pop_front() {
        let dest_path = dest.join(src_path.strip_prefix(src).unwrap());

        if src_path.is_dir() {
            fs::create_dir_all(&dest_path)?;

            for entry in fs::read_dir(src_path)? {
                let entry = entry?;
                queue.push_back(entry.path());
            }
        } else {
            fs::copy(&src_path, &dest_path)?;
        }
    }

    Ok(())
}

pub fn list_projects_path(manifest: &ManifestInstance, workdir: &Path) -> Vec<String> {
    let mut output = Vec::with_capacity(manifest.get_projects().len());

    let manifest_dir = manifest.get_manifest_dir();
    let manifest_dir = Path::new(&manifest_dir);

    for project in manifest.get_projects() {
        let repo_abs_path = manifest_dir.join(project.get_path());
        let rel_path = pathdiff::diff_paths(repo_abs_path, workdir).unwrap_or("./".into());

        output.push(rel_path.as_path().display().to_string());
    }

    output
}

pub async fn get_projects_status(manifest: &ManifestInstance, workdir: &Path) -> Vec<String> {
    struct ProjectStatus {
        status: String,
        path: String,
    }

    let mut project_status = Vec::with_capacity(manifest.get_projects().len());

    let manifest_dir = manifest.get_manifest_dir();
    let manifest_dir_path = Path::new(&manifest_dir);
    let vcs = GitVersionControl::new();

    for project in manifest.get_projects() {
        let status = match vcs.is_modified(&manifest_dir, project).await {
            Ok(false) => "".to_string(),
            Ok(true) => " (modified)".to_string(),
            Err(ManifestError::FailedToDetermineIfRepoIsModified(e)) => e,
            Err(_) => "Unknown error".to_string(),
        };

        let repo_abs_path = manifest_dir_path.join(project.get_path());
        let rel_path = pathdiff::diff_paths(repo_abs_path, workdir)
            .unwrap_or("./".into())
            .display()
            .to_string();

        project_status.push(ProjectStatus {
            status,
            path: rel_path,
        });
    }

    let max_path_len = project_status
        .iter()
        .map(|x| x.path.len())
        .max()
        .unwrap_or(0);

    let output = project_status
        .iter()
        .map(|x| {
            let padding = " ".repeat(max_path_len - x.path.len());
            format!("{}{}{}", x.path, padding, x.status)
        })
        .collect();

    output
}
