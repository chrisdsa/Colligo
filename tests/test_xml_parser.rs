#[cfg(test)]
mod test_xml_parser {

    use colligo::application::ManifestParser;
    use colligo::project::{ProjectAction, ProjectFileAction};
    use colligo::xml_parser::XmlParser;

    #[test]
    fn parse_valid_xml() {
        const MANIFEST_PATH: &str = "./tests/manifest_example.xml";

        let parser: Box<dyn ManifestParser> = Box::new(XmlParser::new());
        let file = std::fs::read_to_string(MANIFEST_PATH).expect("Unable to read file");
        let manifest = parser.parse(&file).expect("Unable to parse XML");

        const PROJECT_URI_SSH: &str = "git@gitlab.com:cdsa_rust/manifest.git";
        const PROJECT_URI_HTTPS: &str = "https://gitlab.com/cdsa_rust/manifest.git";
        const PROJECT_0_PATH: &str = "./dev";
        const PROJECT_1_PATH: &str = "release/v0";
        const PROJECT_2_PATH: &str = "./no_revision";
        const PROJECT_0_REVISION: &str = "dev";
        const PROJECT_1_REVISION: &str = "v0.0.0";
        const PROJECT_2_REVISION: &str = "main";
        const PROJECT_2_ACTION_SRC: &str = "./README.md";
        const PROJECT_2_ACTION_LN_DEST: &str = "./new_folder/ln_README.md";
        const PROJECT_2_ACTION_CP_DEST: &str = "./cp_README.md";
        const EXPECTED_PROJECT_COUNT: usize = 3;

        // Assert project count
        assert_eq!(manifest.len(), EXPECTED_PROJECT_COUNT);

        // Assert URI
        assert_eq!(manifest[0].get_uri_ssh(), PROJECT_URI_SSH);
        assert_eq!(manifest[0].get_uri_https(), PROJECT_URI_HTTPS);
        assert_eq!(manifest[1].get_uri_ssh(), PROJECT_URI_SSH);
        assert_eq!(manifest[1].get_uri_https(), PROJECT_URI_HTTPS);
        assert_eq!(manifest[2].get_uri_ssh(), PROJECT_URI_SSH);
        assert_eq!(manifest[2].get_uri_https(), PROJECT_URI_HTTPS);

        // Assert path
        assert_eq!(manifest[0].get_path(), PROJECT_0_PATH);
        assert_eq!(manifest[1].get_path(), PROJECT_1_PATH);
        assert_eq!(manifest[2].get_path(), PROJECT_2_PATH);

        // Assert revision
        assert_eq!(manifest[0].get_revision(), PROJECT_0_REVISION);
        assert_eq!(manifest[1].get_revision(), PROJECT_1_REVISION);
        assert_eq!(manifest[2].get_revision(), PROJECT_2_REVISION);

        // Assert actions
        assert_eq!(manifest[0].get_actions().len(), 0);
        assert_eq!(manifest[1].get_actions().len(), 0);
        assert_eq!(manifest[2].get_actions().len(), 2);

        match &manifest[2].get_actions()[0] {
            ProjectAction::FileAction(ProjectFileAction::LinkFile(src, dst)) => {
                assert_eq!(src, PROJECT_2_ACTION_SRC);
                assert_eq!(dst, PROJECT_2_ACTION_LN_DEST);
            }
            _ => panic!("Expected ProjectAction::FileAction(ProjectFileAction::LinkFile)"),
        }

        match &manifest[2].get_actions()[1] {
            ProjectAction::FileAction(ProjectFileAction::CopyFile(src, dst)) => {
                assert_eq!(src, PROJECT_2_ACTION_SRC);
                assert_eq!(dst, PROJECT_2_ACTION_CP_DEST);
            }
            _ => panic!("Expected ProjectAction::FileAction(ProjectFileAction::LinkFile)"),
        }
    }

    #[test]
    fn compose_manifest() {
        const MANIFEST_PATH: &str = "./tests/pinned_manifest_example.xml";

        let parser: Box<dyn ManifestParser> = Box::new(XmlParser::new());
        let file = std::fs::read_to_string(MANIFEST_PATH).expect("Unable to read file");
        let manifest = parser.parse(&file).expect("Unable to parse XML");

        let composed = parser.compose(&manifest).expect("Unable to compose XML");

        assert_eq!(composed, file);
    }
}
