// use std::path::Path;

pub const DEFAULT_REVISION: &str = "main";
pub const DEFAULT_HOST: &str = "github.com";

#[derive(Clone)]
pub enum ProjectFileAction {
    LinkFile(String, String),
    CopyFile(String, String),
}

#[derive(Clone)]
pub enum ProjectAction {
    FileAction(ProjectFileAction),
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
        matches!(action, "linkfile" | "copyfile")
    }

    pub fn add_file_action(&mut self, action: &str, src: String, dst: String) {
        match action {
            "linkfile" => {
                let file_action = ProjectFileAction::LinkFile(src, dst);
                self.actions.push(ProjectAction::FileAction(file_action));
            }
            "copyfile" => {
                let file_action = ProjectFileAction::CopyFile(src, dst);
                self.actions.push(ProjectAction::FileAction(file_action));
            }
            _ => {}
        }
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
