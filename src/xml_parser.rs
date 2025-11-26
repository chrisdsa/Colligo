use crate::application::{ManifestError, ManifestParser};
use crate::project::{Project, ProjectAction, ProjectFileAction, DEFAULT_HOST, DEFAULT_REVISION};
use log::warn;
use roxmltree::{Document, Node};

const XML_HEADER: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";
const ROOT_BEGIN: &str = "<manifest>\n";
const ROOT_END: &str = "</manifest>\n";

pub struct XmlParser {}

struct DefaultParameters {
    revision: String,
    uri: String,
}

impl DefaultParameters {
    fn new(doc: &Document) -> Self {
        let default = doc.root().descendants().find(|n| n.has_tag_name("default"));

        let revision = default
            .as_ref()
            .and_then(|n| n.attribute("revision"))
            .unwrap_or(DEFAULT_REVISION)
            .to_string();

        let uri = default
            .as_ref()
            .and_then(|n| n.attribute("uri"))
            .unwrap_or(DEFAULT_HOST)
            .to_string();

        Self { revision, uri }
    }
}

impl XmlParser {
    pub fn new() -> XmlParser {
        XmlParser {}
    }
}

impl Default for XmlParser {
    fn default() -> Self {
        Self::new()
    }
}

impl ManifestParser for XmlParser {
    fn parse(&self, file: &str) -> Result<Vec<Project>, ManifestError> {
        let mut projects: Vec<Project> = Vec::new();

        let parsed_xml = parse_xml_file(file)?;
        let default = DefaultParameters::new(&parsed_xml);

        // Parse projects
        for project in parsed_xml
            .root()
            .descendants()
            .filter(|n| n.has_tag_name("project"))
        {
            let name = get_name(&project)?;
            let path = get_path(&project)?;
            let revision = get_revision(&project, &default);
            let uri = get_uri(&project, &default);
            let mut instance = Project::new(uri, name, revision, path);

            add_actions(&mut instance, &project)?;
            projects.push(instance);
        }

        Ok(projects)
    }

    fn compose(&self, projects: &[Project]) -> Result<String, ManifestError> {
        let mut xml = String::new();
        xml.push_str(XML_HEADER);
        xml.push_str(ROOT_BEGIN);

        for project in projects.iter() {
            xml.push_str(project_to_xml(project).as_str());
        }

        xml.push_str(ROOT_END);
        Ok(xml)
    }
}

fn parse_xml_file(file: &'_ str) -> Result<Document<'_>, ManifestError> {
    let xml = Document::parse(file);
    let parsed_xml;
    match xml {
        Ok(_) => {
            parsed_xml = xml.unwrap();
            Ok(parsed_xml)
        }
        Err(_) => {
            let msg = "Unable to parse XML".to_string();
            Err(ManifestError::FailedToParseManifest(msg))
        }
    }
}

fn get_name(node: &Node) -> Result<String, ManifestError> {
    match node.attribute("name") {
        Some(value) => Ok(value.to_string()),
        None => {
            let msg = "<project --> name= <-- /> is missing".to_string();
            Err(ManifestError::FailedToParseManifest(msg))
        }
    }
}

fn get_path(node: &Node) -> Result<String, ManifestError> {
    match node.attribute("path") {
        Some(value) => Ok(value.to_string()),
        None => {
            let msg = "<project --> path= <-- /> is missing".to_string();
            Err(ManifestError::FailedToParseManifest(msg))
        }
    }
}

fn get_revision(node: &Node, default: &DefaultParameters) -> String {
    node.attribute("revision")
        .unwrap_or(&default.revision)
        .to_string()
}

fn get_uri(node: &Node, default: &DefaultParameters) -> String {
    node.attribute("uri").unwrap_or(&default.uri).to_string()
}

fn add_actions(instance: &mut Project, node: &Node) -> Result<(), ManifestError> {
    if let Some(child) = node.first_element_child() {
        for action in child.next_siblings().filter(|n| n.is_element()) {
            let action_name = action.tag_name().name().to_string();

            if instance.is_file_action(&action_name) {
                if action.has_attribute("src") && action.has_attribute("dest") {
                    let src = action.attribute("src").unwrap().to_string();
                    let dest = action.attribute("dest").unwrap().to_string();
                    instance.add_file_action(&action_name, src, dest);
                } else {
                    let msg = "<[linkfile or copyfile] /> is missing src or dest".to_string();
                    return Err(ManifestError::FailedToParseManifest(msg));
                }
            } else if instance.is_delete_project(&action_name) {
                instance.add_delete_project();
            } else {
                warn!("Warning: <{action_name} /> is not a valid action. Ignored")
            }
        }
    }
    instance.sort_actions();
    Ok(())
}

fn project_to_xml(project: &Project) -> String {
    const PROJECT_END: &str = "    </project>\n";

    let mut xml: String;
    if project.get_actions().is_empty() {
        xml = format!(
            "    <project uri=\"{uri}\" name=\"{name}\" path=\"{path}\" revision=\"{revision}\"/>\n",
            uri = project.get_uri(),
            name = project.get_name(),
            path = project.get_path(),
            revision = project.get_revision(),
        )
    } else {
        xml = format!(
            "    <project uri=\"{uri}\" name=\"{name}\" path=\"{path}\" revision=\"{revision}\">\n",
            uri = project.get_uri(),
            name = project.get_name(),
            path = project.get_path(),
            revision = project.get_revision(),
        );

        for action in project.get_actions() {
            let action_xml = match action {
                ProjectAction::FileAction(ProjectFileAction::LinkFile(src, dest)) => {
                    format!("        <linkfile src=\"{src}\" dest=\"{dest}\"/>\n",)
                }
                ProjectAction::FileAction(ProjectFileAction::CopyFile(src, dest)) => {
                    format!("        <copyfile src=\"{src}\" dest=\"{dest}\"/>\n",)
                }
                ProjectAction::FileAction(ProjectFileAction::CopyDir(src, dest)) => {
                    format!("        <copydir src=\"{src}\" dest=\"{dest}\"/>\n",)
                }
                ProjectAction::DeleteProject => "        <delete-project/>\n".to_string(),
            };
            xml.push_str(&action_xml);
        }
        xml.push_str(PROJECT_END);
    }
    xml
}
