use crate::default_manifest::DEFAULT_MANIFEST_FILE;
use crate::project::{Project, ProjectAction, ProjectFileAction};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::collections::VecDeque;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::{fs, thread};

#[cfg(target_os = "linux")]
use std::os::unix::fs::symlink;

#[cfg(target_os = "windows")]
use std::os::windows::fs::symlink_file as symlink;
use crate::git_version_control::GitVersionControl;

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

pub enum DwlMode {
    HTTPS,
    SSH,
}

pub trait ManifestParser {
    /// Parse a manifest file and return a vector of projects.
    fn parse(&self, file: &str) -> Result<Vec<Project>, String>;

    /// Compose a manifest file from a vector of projects.
    fn compose(&self, projects: &[Project]) -> Result<String, String>;
}

/// VersionControl is a trait that defines the methods to interact with a version control system.
/// The manifest file path is used to avoid changing the working directory which may results in
/// unexpected behavior when multiple threads are used.
/// When using the lightweight option, the revision MUST be a branch or a tag. Commit ID are not supported.
pub trait VersionControl {
    /// Clone a project from a URI to a path.
    fn clone(
        &self,
        manifest_dir: &str,
        project: &Project,
        mode: &DwlMode,
        pb: Option<&ProgressBar>,
        lightweight: bool,
    ) -> Result<(), String>;

    /// Update a project to a specific revision.
    fn checkout(
        &self,
        manifest_dir: &str,
        project: &Project,
        pb: Option<&ProgressBar>,
        force: bool,
    ) -> Result<(), String>;

    /// Status of a project.
    fn get_commit_id(&self, manifest_dir: &str, project: &Project) -> Result<String, String>;

    /// Return true if the project has modified file(s).
    fn is_modified(&self, manifest_dir: &str, project: &Project) -> Result<bool, String>;
}

pub struct ManifestInstance {
    filename: String,
    file: String,
    projects: Vec<Project>,
}

impl ManifestInstance {
    pub fn new(filename: Option<&String>) -> Result<Self, String> {
        let (name, file) = read_manifest(filename)?;

        Ok(Self {
            filename: name,
            file,
            projects: Vec::new(),
        })
    }

    pub fn get_filename(&self) -> &String {
        &self.filename
    }

    pub fn get_file(&self) -> &String {
        &self.file
    }

    pub fn get_projects(&self) -> &Vec<Project> {
        &self.projects
    }

    pub fn parse(&mut self, parser: &dyn ManifestParser) -> Result<(), String> {
        self.projects = parser.parse(&self.file)?;
        Ok(())
    }

    pub fn sync(
        &self,
        mode: &DwlMode,
        lightweight: bool,
        quiet: bool,
        force: bool,
    ) -> Result<(), String>
    {
        // Prepare progress bar
        let multi_progress = MultiProgress::new();
        let style = ProgressStyle::default_bar()
            .template("{spinner:.white} [{elapsed_precise}] {bar:40} {pos:>3}/{len:3} {msg}")
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
        let (tx, rx) = channel();
        let tx = Arc::new(Mutex::new(tx));

        // Spawn a thread for each project
        thread::scope(|s| {
            for project in self.projects.iter() {
                let tx = Arc::clone(&tx);

                let pb = progress_bars.pop().expect("Failed to get progress bar");

                s.spawn(move || {
                    let dir = self.get_manifest_dir();
                    let mut result = Ok(());

                    let vcs = GitVersionControl::new();

                    if is_ok_to_clone(&dir, project.get_path()) {
                        result = vcs.clone(&dir, project, mode, pb.as_ref(), lightweight);
                        send_result(&tx, &result);
                    }

                    if result.is_ok() {
                        result = vcs.checkout(&dir, project, pb.as_ref(), force);
                        send_result(&tx, &result);
                    }

                    if result.is_ok() {
                        result = execute_actions(&dir, project);
                        send_result(&tx, &result);
                    }

                    if let Some(pb) = pb {
                        pb.finish();
                    }
                });
            }
            drop(tx);
        });

        // Save error messages if any
        let errors: Vec<String> = rx
            .iter()
            .filter_map(|code| match code {
                Ok(_) => None,
                Err(msg) => Some(msg),
            })
            .collect();

        // If there is any error, return it
        if !errors.is_empty() {
            // Save all error message in string, separated by a new line
            let mut error_msg = String::new();
            error_msg.push_str("\n\n");

            for msg in errors.iter() {
                error_msg.push_str(msg);
                error_msg.push('\n');
            }

            return Err(error_msg);
        }

        Ok(())
    }

    pub fn pin(&self, parser: &dyn ManifestParser) -> Result<Self, String>
    {
        let mut projects: Vec<Project> = Vec::new();
        let vcs = GitVersionControl::new();

        for project in self.projects.iter() {
            let commit_id = vcs.get_commit_id(&self.get_manifest_dir(), project)?;
            let pinned_project = project.pin(commit_id);
            projects.push(pinned_project);
        }

        let file = parser.compose(&projects)?;

        Ok(Self {
            filename: self.filename.clone(),
            file,
            projects,
        })
    }

    fn get_manifest_dir(&self) -> String {
        let file_path = Path::new(&self.filename);
        let abs_path = file_path.canonicalize().unwrap_or("./".into());
        let workdir = abs_path.parent().unwrap_or("./".as_ref());

        workdir.to_string_lossy().to_string()
    }
}

pub fn generate_default_manifest(destination: &String) -> Result<(), String> {
    let file = File::create(destination);

    match file {
        Ok(_) => {}
        Err(_) => return Err(format!("Failed to create manifest file {destination}")),
    }

    let res = file.unwrap().write_all(DEFAULT_MANIFEST_FILE.as_bytes());
    match res {
        Ok(_) => {}
        Err(_) => return Err(format!("Failed to write to manifest file {destination}")),
    }
    Ok(())
}

fn read_manifest(filename: Option<&String>) -> Result<(String, String), String> {
    let default = MANIFEST_INPUT_DEFAULT.to_string();
    let filename = filename.unwrap_or(&default);
    let file_path = Path::new(&filename);

    if !file_path.exists() {
        return Err(format!("{filename} does not exist"));
    }

    let file = fs::read_to_string(file_path);
    match file {
        Ok(file) => Ok((filename.clone(), file)),
        Err(e) => Err(e.to_string()),
    }
}

// Ok to clone if the directory does not exist or is empty.
fn is_ok_to_clone(manifest_dir: &String, path: &String) -> bool {
    let dir = Path::new(manifest_dir).join(path);

    if dir.exists() && dir.is_dir() {
        let res = dir.read_dir();
        match res {
            Ok(value) => value.count() == 0,
            Err(_) => true,
        }
    } else {
        true
    }
}

fn send_result(sender: &Arc<Mutex<Sender<Result<(), String>>>>, code: &Result<(), String>) {
    sender
        .lock()
        .expect("Failed to lock mutex.")
        .send(code.clone())
        .expect("Failed to send result through channel.");
}

fn execute_actions(manifest_dir: &String, project: &Project) -> Result<(), String> {
    for action in project.get_actions() {
        match action {
            ProjectAction::FileAction(ProjectFileAction::LinkFile(src, dest)) => {
                let dest = Path::new(manifest_dir).join(dest);
                let src = Path::new(manifest_dir).join(project.get_path()).join(src);

                let relative_src = get_relative_path_for_symlink(&src, &dest);
                prepare_file_destination(&dest)?;

                let res = symlink(&relative_src, &dest);
                match res {
                    Ok(_) => {}
                    Err(_) => {
                        return Err(format!(
                            "Failed to create symlink {} to {}",
                            src.to_string_lossy(),
                            dest.to_string_lossy()
                        ));
                    }
                }
            }
            ProjectAction::FileAction(ProjectFileAction::CopyFile(src, dest)) => {
                let src = Path::new(manifest_dir).join(project.get_path()).join(src);
                let dest = Path::new(manifest_dir).join(dest);
                prepare_file_destination(&dest)?;
                let res = fs::copy(&src, &dest);
                match res {
                    Ok(_) => {}
                    Err(_) => {
                        return Err(format!(
                            "Failed to copy file {} to {}",
                            src.to_string_lossy(),
                            dest.to_string_lossy()
                        ));
                    }
                }
            }
            ProjectAction::FileAction(ProjectFileAction::CopyDir(src, dest)) => {
                let src = Path::new(manifest_dir).join(project.get_path()).join(src);
                let dest = Path::new(manifest_dir).join(dest);
                prepare_file_destination(&dest)?;
                let res = copy_directory(&src, &dest);
                match res {
                    Ok(_) => {}
                    Err(_) => {
                        return Err(format!(
                            "Failed to copy directory {} to {}",
                            src.to_string_lossy(),
                            dest.to_string_lossy()
                        ));
                    }
                }
            }
            ProjectAction::DeleteProject => {
                let dest = Path::new(manifest_dir).join(project.get_path());
                let res = fs::remove_dir_all(&dest);
                match res {
                    Ok(_) => {}
                    Err(_) => {
                        return Err(format!(
                            "Failed to remove directory {}",
                            dest.to_string_lossy()
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn save_file(filename: &String, content: &String) -> Result<(), String> {
    let mut file = match File::create(filename) {
        Ok(value) => value,
        Err(_) => {
            return Err(format!("Failed to create file {filename}"));
        }
    };

    let res = file.write(content.as_bytes());
    match res {
        Ok(_) => {}
        Err(_) => {
            return Err(format!("Failed to write to file {filename}"));
        }
    }
    Ok(())
}

pub fn assert_dependencies() -> Result<(), String> {
    let output = Command::new("git").arg("--version").output();

    match output {
        Ok(_) => Ok(()),
        Err(_) => Err("Git is not installed".to_string()),
    }
}

fn prepare_file_destination(dest: &PathBuf) -> Result<(), String> {
    // Delete destination if it exists
    if dest.exists() {
        let res = fs::remove_file(dest);
        match res {
            Ok(_) => {}
            Err(_) => {
                return Err(format!("Failed to remove file {}", dest.to_string_lossy()));
            }
        }
    }

    // Create folder is needed
    let parent = dest.parent().unwrap_or("./".as_ref());
    let _ = fs::create_dir_all(parent);

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

pub fn get_projects_status(
    manifest: &ManifestInstance,
    workdir: &Path,
) -> Vec<String> {
    struct ProjectStatus {
        status: String,
        path: String,
    }

    let mut project_status = Vec::with_capacity(manifest.get_projects().len());

    let manifest_dir = manifest.get_manifest_dir();
    let manifest_dir_path = Path::new(&manifest_dir);
    let vcs = GitVersionControl::new();

    for project in manifest.get_projects() {
        let status = match vcs.is_modified(&manifest_dir, project) {
            Ok(false) => "".to_string(),
            Ok(true) => " (modified)".to_string(),
            Err(e) => e,
        };

        let repo_abs_path = manifest_dir_path.join(project.get_path());
        let rel_path = pathdiff::diff_paths(repo_abs_path, workdir)
            .unwrap_or("./".into())
            .to_string_lossy()
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
