use crate::default_manifest::DEFAULT_MANIFEST_FILE;
use crate::project::{Project, ProjectAction, ProjectFileAction};
use log::{debug, error};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;

#[cfg(target_os = "linux")]
use std::os::unix::fs::symlink;

#[cfg(target_os = "windows")]
use std::os::windows::fs::symlink_file as symlink;

// Application description
pub const APP_NAME: &str = "Manifest";

// Command line arguments
pub const GENERATE_MANIFEST: &str = "generate";

pub const MANIFEST_INPUT: &str = "input";
pub const MANIFEST_INPUT_DEFAULT: &str = "manifest.xml";

pub const SYNC: &str = "sync";
pub const PIN: &str = "pin";

pub const LIGHT: &str = "light";
pub const HTTPS: &str = "https";

#[derive(Debug, Clone)]
#[repr(u8)]
pub enum ExitCode {
    Success = 0,
    NoManifest = 100,
    GenerationFailed = 101,
    ManifestInvalid = 102,
    CloneFailed = 103,
    CheckoutFailed = 104,
    GetCommitIdFailed = 105,
    WorkdirFailed = 106,
    CreateSymlinkFailed = 107,
    CopyFailed = 108,
    RemoveFailed = 109,
    SaveFailed = 110,
    DependencyMissing = 111,
}

impl From<ExitCode> for std::process::ExitCode {
    fn from(code: ExitCode) -> Self {
        std::process::ExitCode::from(code as u8)
    }
}

pub enum DwlMode {
    HTTPS,
    SSH,
}

pub trait ManifestParser {
    /// Parse a manifest file and return a vector of projects.
    fn parse(&self, file: &str) -> Result<Vec<Project>, ExitCode>;

    /// Compose a manifest file from a vector of projects.
    fn compose(&self, projects: &[Project]) -> Result<String, ExitCode>;
}

/// VersionControl is a trait that defines the methods to interact with a version control system.
/// The manifest file path is used to avoid changing the working directory which may results in
/// unexpected behavior when multiple threads are used.
pub trait VersionControl {
    /// Clone a project from a URI to a path.
    fn clone(&self, manifest_dir: &str, project: &Project, mode: &DwlMode) -> Result<(), ExitCode>;

    /// Clone a project from a URI to a path with a specific revision. The revision MUST be a branch or a tag. Commit ID are not supported.
    fn clone_lightweight(
        &self,
        manifest_dir: &str,
        project: &Project,
        mode: &DwlMode,
    ) -> Result<(), ExitCode>;

    /// Update a project to a specific revision.
    fn checkout(&self, manifest_dir: &str, project: &Project) -> Result<(), ExitCode>;

    /// Status of a project.
    fn get_commit_id(&self, manifest_dir: &str, project: &Project) -> Result<String, ExitCode>;
}

pub struct ManifestInstance {
    filename: String,
    file: String,
    projects: Vec<Project>,
}

impl ManifestInstance {
    pub fn new(filename: Option<&String>) -> Result<Self, ExitCode> {
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

    pub fn parse(&mut self, parser: &dyn ManifestParser) -> Result<(), ExitCode> {
        self.projects = parser.parse(&self.file)?;
        Ok(())
    }

    pub fn sync<T>(&self, vcs: &T, mode: &DwlMode) -> Result<(), ExitCode>
    where
        T: VersionControl + Sync,
    {
        let (tx, rx) = channel();
        let tx = Arc::new(Mutex::new(tx));

        thread::scope(|s| {
            for project in self.projects.iter() {
                let tx = Arc::clone(&tx);

                s.spawn(move || {
                    let dir = self.get_manifest_dir();
                    let mut result = Ok(());
                    if is_ok_to_clone(&dir, project.get_path()) {
                        result = vcs.clone(&dir, project, mode);
                        send_result(&tx, &result);
                    }

                    if result.is_ok() {
                        result = vcs.checkout(&dir, project);
                        send_result(&tx, &result);
                    }

                    if result.is_ok() {
                        result = execute_actions(&dir, project);
                        send_result(&tx, &result);
                    }
                });
            }
            drop(tx);
        });

        // Return the first error code if any
        for code in rx.iter() {
            match code {
                ExitCode::Success => {}
                _ => return Err(code),
            }
        }

        debug!("Sync done");
        Ok(())
    }

    pub fn pin<T>(&self, vcs: &T, parser: &dyn ManifestParser) -> Result<Self, ExitCode>
    where
        T: VersionControl,
    {
        let mut projects: Vec<Project> = Vec::new();

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

pub fn generate_default_manifest(destination: &String) -> Result<(), ExitCode> {
    let file = File::create(destination);

    match file {
        Ok(_) => {}
        Err(_) => return Err(ExitCode::GenerationFailed),
    }

    let res = file.unwrap().write_all(DEFAULT_MANIFEST_FILE.as_bytes());
    match res {
        Ok(_) => {}
        Err(_) => return Err(ExitCode::GenerationFailed),
    }
    Ok(())
}

fn read_manifest(filename: Option<&String>) -> Result<(String, String), ExitCode> {
    let default = MANIFEST_INPUT_DEFAULT.to_string();
    let filename = filename.unwrap_or(&default);
    let file_path = Path::new(&filename);

    if !file_path.exists() {
        error!("{filename} does not exist");
        return Err(ExitCode::NoManifest);
    }

    let file = std::fs::read_to_string(file_path);
    match file {
        Ok(file) => Ok((filename.clone(), file)),
        Err(e) => {
            error!("{e}");
            Err(ExitCode::NoManifest)
        }
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

fn send_result(sender: &Arc<Mutex<Sender<ExitCode>>>, code: &Result<(), ExitCode>) {
    let exit_code = match code {
        Ok(_) => ExitCode::Success,
        Err(e) => e.clone(),
    };
    sender
        .lock()
        .expect("Failed to lock mutex.")
        .send(exit_code)
        .expect("Failed to send result through channel.");
}

fn execute_actions(manifest_dir: &String, project: &Project) -> Result<(), ExitCode> {
    for action in project.get_actions() {
        match action {
            ProjectAction::FileAction(ProjectFileAction::LinkFile(src, dest)) => {
                let dest = Path::new(manifest_dir).join(dest);
                let src = Path::new(manifest_dir).join(project.get_path()).join(src);

                let relative_src = get_relative_path(&src, &dest);
                prepare_file_destination(&dest)?;

                let res = symlink(&relative_src, &dest);
                match res {
                    Ok(_) => {}
                    Err(_) => {
                        error!(
                            "Failed to create symlink {} to {}",
                            src.to_string_lossy(),
                            dest.to_string_lossy()
                        );
                        return Err(ExitCode::CreateSymlinkFailed);
                    }
                }
            }
            ProjectAction::FileAction(ProjectFileAction::CopyFile(src, dest)) => {
                let src = Path::new(manifest_dir).join(project.get_path()).join(src);
                let dest = Path::new(manifest_dir).join(dest);
                prepare_file_destination(&dest)?;
                let res = std::fs::copy(&src, &dest);
                match res {
                    Ok(_) => {}
                    Err(_) => {
                        error!(
                            "Failed to copy file {} to {}",
                            src.to_string_lossy(),
                            dest.to_string_lossy()
                        );
                        return Err(ExitCode::CopyFailed);
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn save_file(filename: &String, content: &String) -> Result<(), ExitCode> {
    let mut file = match File::create(filename) {
        Ok(value) => value,
        Err(_) => {
            error!("Failed to create file {filename}");
            return Err(ExitCode::SaveFailed);
        }
    };

    let res = file.write(content.as_bytes());
    match res {
        Ok(_) => {}
        Err(_) => {
            error!("Failed to write to file {filename}");
            return Err(ExitCode::SaveFailed);
        }
    }
    Ok(())
}

pub fn assert_dependencies() -> Result<(), ExitCode> {
    let output = Command::new("git").arg("--version").output();

    match output {
        Ok(_) => Ok(()),
        Err(_) => {
            error!("Git is not installed");
            Err(ExitCode::DependencyMissing)
        }
    }
}

fn prepare_file_destination(dest: &PathBuf) -> Result<(), ExitCode> {
    // Delete destination if it exists
    if dest.exists() {
        let res = std::fs::remove_file(dest);
        match res {
            Ok(_) => {}
            Err(_) => {
                error!("Failed to remove file {}", dest.to_string_lossy());
                return Err(ExitCode::RemoveFailed);
            }
        }
    }

    // Create folder is needed
    let parent = dest.parent().unwrap_or("./".as_ref());
    let _ = std::fs::create_dir_all(parent);

    Ok(())
}

fn get_relative_path(path: &Path, base: &Path) -> PathBuf {
    let base_dir = base.parent().unwrap_or("".as_ref());

    let path_dir = path.parent().unwrap_or("".as_ref());
    let path_filename = path.file_name().unwrap_or("".as_ref());

    let path = pathdiff::diff_paths(path_dir, base_dir).unwrap_or("./".into());
    path.join(path_filename)
}
