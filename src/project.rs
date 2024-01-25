pub const DEFAULT_REVISION: &str = "main";
pub const DEFAULT_HOST: &str = "github.com";

// Action tags
const LINKFILE: &str = "linkfile";
const COPYFILE: &str = "copyfile";
const COPYDIR: &str = "copydir";
const DELETE_PROJECT: &str = "delete_project";

#[derive(Clone, PartialEq)]
pub enum ProjectFileAction {
    LinkFile(String, String),
    CopyFile(String, String),
    CopyDir(String, String),
}

#[derive(Clone, PartialEq)]
pub enum ProjectAction {
    FileAction(ProjectFileAction),
    DeleteProject,
}

pub struct Project {
    uri: String,
    name: String,
    revision: String,
    path: String,
    actions: Vec<ProjectAction>,
}

impl Project {
    pub fn new(uri: String, name: String, revision: String, path: String) -> Self {
        Self {
            uri,
            name,
            revision,
            path,
            actions: Vec::new(),
        }
    }

    pub fn pin(&self, commit_id: String) -> Self {
        Self {
            uri: self.uri.clone(),
            name: self.name.clone(),
            revision: commit_id,
            path: self.path.clone(),
            actions: self.actions.clone(),
        }
    }

    pub fn is_file_action(&self, action: &str) -> bool {
        matches!(action, LINKFILE | COPYFILE | COPYDIR)
    }

    pub fn add_file_action(&mut self, action: &str, src: String, dst: String) {
        match action {
            LINKFILE => {
                let file_action = ProjectFileAction::LinkFile(src, dst);
                self.actions.push(ProjectAction::FileAction(file_action));
            }
            COPYFILE => {
                let file_action = ProjectFileAction::CopyFile(src, dst);
                self.actions.push(ProjectAction::FileAction(file_action));
            }
            COPYDIR => {
                let file_action = ProjectFileAction::CopyDir(src, dst);
                self.actions.push(ProjectAction::FileAction(file_action));
            }
            _ => {}
        }
    }

    pub fn is_delete_project(&self, action: &str) -> bool {
        matches!(action, DELETE_PROJECT)
    }

    pub fn add_delete_project(&mut self) {
        if !self.actions.contains(&ProjectAction::DeleteProject) {
            self.actions.push(ProjectAction::DeleteProject);
        }
    }

    pub fn sort_actions(&mut self) {
        // Sort action to place delete project at the end
        self.actions.sort_by(|a, b| match (a, b) {
            (ProjectAction::DeleteProject, ProjectAction::DeleteProject) => {
                std::cmp::Ordering::Equal
            }
            (ProjectAction::DeleteProject, _) => std::cmp::Ordering::Greater,
            (_, ProjectAction::DeleteProject) => std::cmp::Ordering::Less,
            _ => std::cmp::Ordering::Equal,
        });
    }

    pub fn get_name(&self) -> &String {
        &self.name
    }

    pub fn get_uri(&self) -> &String {
        &self.uri
    }

    pub fn get_actions(&self) -> &Vec<ProjectAction> {
        &self.actions
    }

    pub fn get_revision(&self) -> &String {
        &self.revision
    }

    pub fn get_path(&self) -> &String {
        &self.path
    }

    pub fn get_uri_https(&self) -> String {
        format!("https://{}/{}.git", self.uri, self.name)
    }

    pub fn get_uri_ssh(&self) -> String {
        format!("git@{}:{}.git", self.uri, self.name)
    }
}
