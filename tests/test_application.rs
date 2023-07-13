#[cfg(test)]
mod test_application {

    use git2::Repository;
    use manifest::application::{
        generate_default_manifest, DwlMode, ExitCode, ManifestInstance, ManifestParser,
    };
    use manifest::default_manifest::DEFAULT_MANIFEST_FILE;
    use manifest::git_version_control::GitVersionControl;
    use manifest::xml_parser::XmlParser;

    #[cfg(target_os = "linux")]
    use std::os::unix::fs::symlink;

    static HTTPS_URL: &str = "https://gitlab.com/cdsa_rust/manifest.git";

    #[test]
    fn generate_default_xml_valid_destination() {
        const DESTINATION: &str = "tests/project.xml";

        let _ = generate_default_manifest(&DESTINATION.to_string());

        let file = std::fs::read_to_string(DESTINATION);
        assert!(file.is_ok());
        assert_eq!(file.unwrap(), DEFAULT_MANIFEST_FILE);
        std::fs::remove_file(DESTINATION).unwrap();
    }

    #[test]
    fn get_manifest_file_valid() {
        const MANIFEST_PATH: &str = "./tests/manifest_example.xml";

        let manifest = ManifestInstance::new(Some(&MANIFEST_PATH.to_string())).unwrap();
        assert_eq!(manifest.get_file(), include_str!("manifest_example.xml"));
    }

    #[test]
    fn get_manifest_file_invalid() {
        const MANIFEST_PATH: &str = "./tests/error_example.xml";

        let manifest = ManifestInstance::new(Some(&MANIFEST_PATH.to_string()));

        match manifest {
            Err(ExitCode::NoManifest) => { /* Ok */ }
            _ => panic!("Expected ExitCode::ManifestInvalid"),
        }
    }

    #[test]
    fn get_manifest_file_default() {
        const MANIFEST_PATH: &str = "./manifest.xml";
        std::fs::copy("./tests/manifest.xml", MANIFEST_PATH).unwrap();

        let manifest = ManifestInstance::new(None).unwrap();

        assert_eq!(manifest.get_file(), include_str!("manifest.xml"));
        std::fs::remove_file(MANIFEST_PATH).unwrap();
    }

    #[test]
    fn sync_empty_project_ssh() {
        // Setup
        const ORIGINAL_MANIFEST_PATH: &str = "./tests/manifest_example.xml";
        const TEST_PATH: &str = "/tmp/manifest_test_empty_dir";
        const TEST_MANIFEST_PATH: &str = "/tmp/manifest_test_empty_dir/manifest_example.xml";
        std::fs::create_dir_all(TEST_PATH).unwrap();
        std::fs::copy(ORIGINAL_MANIFEST_PATH, TEST_MANIFEST_PATH).unwrap();

        // Test
        let mut manifest = ManifestInstance::new(Some(&TEST_MANIFEST_PATH.to_string()))
            .expect("Unable to get manifest file");

        let parser: Box<dyn ManifestParser> = Box::new(XmlParser::new());
        manifest
            .parse(parser.as_ref())
            .expect("Unable to parse manifest");

        let git: Box<GitVersionControl> = Box::new(GitVersionControl::new());
        manifest
            .sync(git.as_ref(), &DwlMode::HTTPS)
            .expect("Unable to sync manifest");

        // Assert
        const PROJECT_0_PATH: &str = "/tmp/manifest_test_empty_dir/dev";
        const PROJECT_1_PATH: &str = "/tmp/manifest_test_empty_dir/release/v0";
        const PROJECT_2_PATH: &str = "/tmp/manifest_test_empty_dir/no_revision";
        const COMMIT_V0: &str = "565b113e57b2c67dcaa3e7c2b5040cf4715221df";
        const REF_DEV: &str = "refs/heads/dev";

        let repo_0 = Repository::open(PROJECT_0_PATH).expect("Unable to open project 0 repository");
        let repo_1 = Repository::open(PROJECT_1_PATH).expect("Unable to open project 1 repository");
        let repo_2 = Repository::open(PROJECT_2_PATH).expect("Unable to open project 2 repository");

        assert_eq!(
            repo_0.head().expect("Unable to get head").name().unwrap(),
            REF_DEV
        );
        assert_eq!(
            repo_1
                .head()
                .expect("Unable to get head")
                .peel_to_commit()
                .unwrap()
                .id()
                .to_string(),
            COMMIT_V0
        );
        assert_eq!(
            repo_2
                .head()
                .expect("Unable to get head")
                .peel_to_commit()
                .unwrap()
                .id()
                .to_string(),
            COMMIT_V0
        );

        // Assert linkfile and copyfile
        assert_eq!(
            std::fs::read_to_string("/tmp/manifest_test_empty_dir/no_revision/README.md")
                .expect("Unable to read linkfile"),
            std::fs::read_to_string("/tmp/manifest_test_empty_dir/new_folder/ln_README.md")
                .expect("ln_README.md"),
        );

        assert_eq!(
            std::fs::read_to_string("/tmp/manifest_test_empty_dir/no_revision/README.md")
                .expect("Unable to read linkfile"),
            std::fs::read_to_string("/tmp/manifest_test_empty_dir/cp_README.md")
                .expect("cp_README.md"),
        );

        // Cleanup
        std::fs::remove_dir_all(TEST_PATH).unwrap();
    }

    #[test]
    fn sync_project_on_old_commit() {
        // Setup
        const ORIGINAL_MANIFEST_PATH: &str = "./tests/manifest_example.xml";
        const TEST_PATH: &str = "/tmp/manifest_test_not_empty";
        const TEST_MANIFEST_PATH: &str = "/tmp/manifest_test_not_empty/manifest_example.xml";
        std::fs::create_dir_all(TEST_PATH).unwrap();
        std::fs::copy(ORIGINAL_MANIFEST_PATH, TEST_MANIFEST_PATH).unwrap();

        const PROJECT_0_PATH: &str = "/tmp/manifest_test_not_empty/dev";
        const PROJECT_1_PATH: &str = "/tmp/manifest_test_not_empty/release/v0";
        const PROJECT_2_PATH: &str = "/tmp/manifest_test_not_empty/no_revision";

        let _ = std::process::Command::new("git")
            .args(["clone", HTTPS_URL, PROJECT_0_PATH, "--quiet"])
            .output();
        let _ = std::process::Command::new("git")
            .args(["clone", HTTPS_URL, PROJECT_1_PATH, "--quiet"])
            .output();
        let _ = std::process::Command::new("git")
            .args(["clone", HTTPS_URL, PROJECT_2_PATH, "--quiet"])
            .output();

        std::fs::create_dir_all("/tmp/manifest_test_not_empty/new_folder").unwrap();

        symlink(
            "/tmp/manifest_test_not_empty/no_revision/README.md",
            "/tmp/manifest_test_not_empty/new_folder/ln_README.md",
        )
        .unwrap();

        std::fs::copy(
            "/tmp/manifest_test_not_empty/no_revision/README.md",
            "/tmp/manifest_test_not_empty/cp_README.md",
        )
        .unwrap();

        // Test
        let mut manifest = ManifestInstance::new(Some(&TEST_MANIFEST_PATH.to_string()))
            .expect("Unable to get manifest file");

        let parser: Box<dyn ManifestParser> = Box::new(XmlParser::new());
        manifest
            .parse(parser.as_ref())
            .expect("Unable to parse manifest");

        let git: Box<GitVersionControl> = Box::new(GitVersionControl::new());
        manifest
            .sync(git.as_ref(), &DwlMode::HTTPS)
            .expect("Unable to sync manifest");

        // Assert
        const COMMIT_V0: &str = "565b113e57b2c67dcaa3e7c2b5040cf4715221df";
        const REF_DEV: &str = "refs/heads/dev";

        let repo_0 = Repository::open(PROJECT_0_PATH).expect("Unable to open project 0 repository");
        let repo_1 = Repository::open(PROJECT_1_PATH).expect("Unable to open project 1 repository");
        let repo_2 = Repository::open(PROJECT_2_PATH).expect("Unable to open project 2 repository");

        assert_eq!(
            repo_0.head().expect("Unable to get head").name().unwrap(),
            REF_DEV
        );
        assert_eq!(
            repo_1
                .head()
                .expect("Unable to get head")
                .peel_to_commit()
                .unwrap()
                .id()
                .to_string(),
            COMMIT_V0
        );
        assert_eq!(
            repo_2
                .head()
                .expect("Unable to get head")
                .peel_to_commit()
                .unwrap()
                .id()
                .to_string(),
            COMMIT_V0
        );

        // Assert linkfile and copyfile
        assert_eq!(
            std::fs::read_to_string("/tmp/manifest_test_not_empty/no_revision/README.md")
                .expect("Unable to read linkfile"),
            std::fs::read_to_string("/tmp/manifest_test_not_empty/new_folder/ln_README.md")
                .expect("ln_README.md"),
        );

        assert_eq!(
            std::fs::read_to_string("/tmp/manifest_test_not_empty/no_revision/README.md")
                .expect("Unable to read linkfile"),
            std::fs::read_to_string("/tmp/manifest_test_not_empty/cp_README.md")
                .expect("cp_README.md"),
        );

        // Cleanup
        std::fs::remove_dir_all(TEST_PATH).unwrap();
    }

    #[test]
    fn pin_manifest() {
        // Setup
        const ORIGINAL_MANIFEST_PATH: &str = "./tests/manifest_example.xml";
        const TEST_PATH: &str = "/tmp/manifest_test_pin";
        const TEST_MANIFEST_PATH: &str = "/tmp/manifest_test_pin/manifest_example.xml";
        std::fs::create_dir_all(TEST_PATH).unwrap();
        std::fs::copy(ORIGINAL_MANIFEST_PATH, TEST_MANIFEST_PATH).unwrap();

        const PROJECT_0_PATH: &str = "/tmp/manifest_test_pin/dev";
        const PROJECT_1_PATH: &str = "/tmp/manifest_test_pin/release/v0";
        const PROJECT_2_PATH: &str = "/tmp/manifest_test_pin/no_revision";

        let _ = std::process::Command::new("git")
            .args(["clone", HTTPS_URL, PROJECT_0_PATH, "--quiet"])
            .output();
        let _ = std::process::Command::new("git")
            .args(["clone", HTTPS_URL, PROJECT_1_PATH, "--quiet"])
            .output();
        let _ = std::process::Command::new("git")
            .args(["clone", HTTPS_URL, PROJECT_2_PATH, "--quiet"])
            .output();

        let _ = std::process::Command::new("git")
            .current_dir(PROJECT_0_PATH)
            .args(["checkout", "v0.0.0", "--quiet"])
            .output();
        let _ = std::process::Command::new("git")
            .current_dir(PROJECT_1_PATH)
            .args(["checkout", "v0.0.0", "--quiet"])
            .output();
        let _ = std::process::Command::new("git")
            .current_dir(PROJECT_2_PATH)
            .args(["checkout", "v0.0.0", "--quiet"])
            .output();

        // Test
        let mut manifest = ManifestInstance::new(Some(&TEST_MANIFEST_PATH.to_string()))
            .expect("Unable to get manifest file");

        let parser: Box<dyn ManifestParser> = Box::new(XmlParser::new());
        manifest
            .parse(parser.as_ref())
            .expect("Unable to parse manifest");

        let git: Box<GitVersionControl> = Box::new(GitVersionControl::new());
        let pinned = manifest
            .pin(git.as_ref(), parser.as_ref())
            .expect("Unable to pin manifest");

        // Assert
        const COMMIT_V0: &str = "565b113e57b2c67dcaa3e7c2b5040cf4715221df";
        assert_eq!(pinned.get_projects()[0].get_revision(), COMMIT_V0);
        assert_eq!(pinned.get_projects()[1].get_revision(), COMMIT_V0);
        assert_eq!(pinned.get_projects()[2].get_revision(), COMMIT_V0);

        // Cleanup
        std::fs::remove_dir_all(TEST_PATH).unwrap();
    }
}
