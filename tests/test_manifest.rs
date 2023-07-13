#[cfg(test)]
mod test_xml {
    use roxmltree::Document;

    const MANIFEST_PATH: &str = "./tests/manifest_example.xml";

    #[test]
    fn parse_manifest_example() {
        let file = std::fs::read_to_string(MANIFEST_PATH).expect("Unable to read file");
        let manifest = Document::parse(&file).expect("Unable to parse XML");

        let root = manifest.root_element();
        assert_eq!(root.tag_name().name(), "manifest");

        // Default parameters
        let default = manifest
            .root()
            .descendants()
            .find(|n| n.has_tag_name("default"))
            .unwrap();

        let revision = default.attribute("revision");
        assert_eq!(revision, Some("main"));

        let uri = default.attribute("uri");
        assert_eq!(uri, Some("gitlab.com"));

        // Projects
        let project_cnt = manifest
            .root()
            .descendants()
            .filter(|n| n.has_tag_name("project"))
            .count();
        assert_eq!(project_cnt, 3);

        // Get project actions
        let mut actions = Vec::<String>::new();
        for i in manifest
            .root()
            .descendants()
            .filter(|n| n.has_tag_name("project"))
        {
            if let Some(child) = i.first_element_child() {
                for j in child.next_siblings().filter(|n| n.is_element()) {
                    actions.push(j.tag_name().name().to_string());
                }
            }
        }
        assert_eq!(actions, ["linkfile", "copyfile"]);
    }
}

#[cfg(test)]
mod test_manifest {

    use manifest::project::Project;

    #[test]
    fn test_project_instance() {
        let project = Project::new(
            "gitlab.com".to_string(),
            "cdsa_rust/manifest".to_string(),
            "dev".to_string(),
            "./path".to_string(),
        );

        assert_eq!(project.get_revision(), "dev");
        assert_eq!(project.get_path(), "./path");

        assert_eq!(
            project.get_uri_https(),
            "https://gitlab.com/cdsa_rust/manifest.git"
        );
        assert_eq!(
            project.get_uri_ssh(),
            "git@gitlab.com:cdsa_rust/manifest.git"
        );
    }
}
