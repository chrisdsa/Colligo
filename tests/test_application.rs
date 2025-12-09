#[cfg(test)]
mod test_application {
    use colligo::application::{generate_default_manifest, DwlMode, ManifestInstance};
    use colligo::default_manifest::DEFAULT_MANIFEST_FILE;
    use git2::Repository;

    static HTTPS_URL: &str = "https://gitlab.com/cdsa_rust/colligo.git";

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

        let manifest =
            ManifestInstance::try_from(MANIFEST_PATH).expect("Failed to create manifest instance");
        assert_eq!(manifest.get_file(), include_str!("manifest_example.xml"));
    }

    #[test]
    fn get_manifest_file_invalid() {
        const MANIFEST_PATH: &str = "./tests/error_example.xml";

        let manifest = ManifestInstance::try_from(MANIFEST_PATH);

        match manifest {
            Err(_) => { /* Ok */ }
            _ => panic!("Expected an error"),
        }
    }

    #[tokio::test]
    async fn sync_empty_project_ssh() {
        // Setup
        const ORIGINAL_MANIFEST_PATH: &str = "./tests/manifest_example.xml";
        let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
        let manifest = temp_dir.path().join("manifest.xml");
        std::fs::copy(ORIGINAL_MANIFEST_PATH, &manifest).unwrap();

        // Test
        let mut manifest =
            ManifestInstance::try_from(manifest).expect("Failed to create manifest instance");

        manifest.parse().expect("Unable to parse manifest");

        manifest
            .sync(&DwlMode::HTTPS, false, false, false)
            .await
            .expect("Unable to sync manifest");

        // Assert
        let project_0_path = temp_dir.path().join("dev");
        let project_1_path = temp_dir.path().join("release/v0");
        let project_2_path = temp_dir.path().join("no_revision");
        const COMMIT_V0: &str = "565b113e57b2c67dcaa3e7c2b5040cf4715221df";
        const REF_DEV: &str = "refs/heads/dev";
        const REF_MAIN: &str = "refs/heads/main";

        let repo_0 = Repository::open(project_0_path).expect("Unable to open project 0 repository");
        let repo_1 = Repository::open(project_1_path).expect("Unable to open project 1 repository");
        let repo_2 = Repository::open(project_2_path).expect("Unable to open project 2 repository");

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
            repo_2.head().expect("Unable to get head").name().unwrap(),
            REF_MAIN
        );

        // Assert linkfile and copyfile
        assert_eq!(
            std::fs::read_to_string(temp_dir.path().join("no_revision/README.md"))
                .expect("Unable to read linkfile"),
            std::fs::read_to_string(temp_dir.path().join("new_folder/ln_README.md"))
                .expect("ln_README.md"),
        );

        assert_eq!(
            std::fs::read_to_string(temp_dir.path().join("no_revision/README.md"))
                .expect("Unable to read linkfile"),
            std::fs::read_to_string(temp_dir.path().join("cp_README.md")).expect("cp_README.md"),
        );
    }

    #[tokio::test]
    async fn sync_project_on_old_commit() {
        // Setup
        const ORIGINAL_MANIFEST_PATH: &str = "./tests/manifest_symlink_dir.xml";
        let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
        let manifest = temp_dir.path().join("manifest.xml");
        std::fs::copy(ORIGINAL_MANIFEST_PATH, &manifest).unwrap();

        let project_0_path: String = temp_dir.path().join("dev").display().to_string();
        let project_1_path: String = temp_dir.path().join("release/v0").display().to_string();
        let project_2_path: String = temp_dir.path().join("no_revision").display().to_string();

        let _ = std::process::Command::new("git")
            .args(["clone", HTTPS_URL, &project_0_path, "--quiet"])
            .output();
        let _ = std::process::Command::new("git")
            .args(["clone", HTTPS_URL, &project_1_path, "--quiet"])
            .output();
        let _ = std::process::Command::new("git")
            .args(["clone", HTTPS_URL, &project_2_path, "--quiet"])
            .output();

        // Create files where the symlink and copied file will be
        std::fs::create_dir(temp_dir.path().join("new_folder")).expect("failed to create dir");
        std::fs::File::create(temp_dir.path().join("new_folder/ln_README.md"))
            .expect("Failed to create file");
        std::fs::File::create(temp_dir.path().join("cp_README.md")).expect("Failed to create file");
        // create symlink to a directory
        std::os::unix::fs::symlink(&project_0_path, temp_dir.path().join("./new_folder/src_linked")).expect("failed to create symlink");

        // Test
        let mut manifest =
            ManifestInstance::try_from(manifest).expect("Failed to create manifest instance");

        manifest.parse().expect("Unable to parse manifest");

        manifest
            .sync(&DwlMode::HTTPS, false, false, false)
            .await
            .expect("Unable to sync manifest");

        // Assert
        const COMMIT_V0: &str = "565b113e57b2c67dcaa3e7c2b5040cf4715221df";
        const REF_DEV: &str = "refs/heads/dev";
        const REF_MAIN: &str = "refs/heads/main";

        let repo_0 = Repository::open(project_0_path).expect("Unable to open project 0 repository");
        let repo_1 = Repository::open(project_1_path).expect("Unable to open project 1 repository");
        let repo_2 = Repository::open(project_2_path).expect("Unable to open project 2 repository");

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
            repo_2.head().expect("Unable to get head").name().unwrap(),
            REF_MAIN
        );

        // Assert linkfile and copyfile
        assert_eq!(
            std::fs::read_to_string(temp_dir.path().join("no_revision/README.md"))
                .expect("Unable to read original file"),
            std::fs::read_to_string(temp_dir.path().join("new_folder/ln_README.md"))
                .expect("ln_README.md"),
        );

        assert_eq!(
            std::fs::read_to_string(temp_dir.path().join("no_revision/README.md"))
                .expect("Unable to read linkfile"),
            std::fs::read_to_string(temp_dir.path().join("cp_README.md")).expect("cp_README.md"),
        );
    }

    #[tokio::test]
    async fn pin_manifest() {
        // Setup
        const ORIGINAL_MANIFEST_PATH: &str = "./tests/manifest_example.xml";
        let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
        let manifest = temp_dir.path().join("manifest.xml");
        std::fs::copy(ORIGINAL_MANIFEST_PATH, &manifest).unwrap();

        let project_0_path: String = temp_dir.path().join("dev").display().to_string();
        let project_1_path: String = temp_dir.path().join("release/v0").display().to_string();
        let project_2_path: String = temp_dir.path().join("no_revision").display().to_string();

        let _ = std::process::Command::new("git")
            .args(["clone", HTTPS_URL, &project_0_path, "--quiet"])
            .output();
        let _ = std::process::Command::new("git")
            .args(["clone", HTTPS_URL, &project_1_path, "--quiet"])
            .output();
        let _ = std::process::Command::new("git")
            .args(["clone", HTTPS_URL, &project_2_path, "--quiet"])
            .output();

        let _ = std::process::Command::new("git")
            .current_dir(&project_0_path)
            .args(["checkout", "v0.0.0", "--quiet"])
            .output();
        let _ = std::process::Command::new("git")
            .current_dir(&project_1_path)
            .args(["checkout", "v0.1.0", "--quiet"])
            .output();
        let _ = std::process::Command::new("git")
            .current_dir(&project_2_path)
            .args(["checkout", "v0.1.1", "--quiet"])
            .output();

        // Test
        let mut manifest =
            ManifestInstance::try_from(manifest).expect("Failed to create manifest instance");

        manifest.parse().expect("Unable to parse manifest");

        let pinned = manifest.pin().await.expect("Unable to pin manifest");

        // Assert
        const COMMIT_V0_0_0: &str = "565b113e57b2c67dcaa3e7c2b5040cf4715221df";
        const COMMIT_V0_1_0: &str = "633fcde4a51809adfffa2d65d68a3ac687d93826";
        const COMMIT_V0_1_1: &str = "61518393fd65f7bf57be144f4498112d5c503d36";
        assert_eq!(pinned.get_projects()[0].get_revision(), COMMIT_V0_0_0);
        assert_eq!(pinned.get_projects()[1].get_revision(), COMMIT_V0_1_0);
        assert_eq!(pinned.get_projects()[2].get_revision(), COMMIT_V0_1_1);
    }

    #[tokio::test]
    async fn sync_with_force_option() {
        // Setup: Sync project
        const ORIGINAL_MANIFEST_PATH: &str = "./tests/manifest_example.xml";
        let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
        let manifest = temp_dir.path().join("manifest.xml");
        std::fs::copy(ORIGINAL_MANIFEST_PATH, &manifest).unwrap();

        let mut manifest =
            ManifestInstance::try_from(manifest).expect("Failed to create manifest instance");

        manifest.parse().expect("Unable to parse manifest");

        manifest
            .sync(&DwlMode::HTTPS, false, false, false)
            .await
            .expect("Unable to sync manifest");

        // Save current README in dev/
        let original_readme = std::fs::read_to_string(temp_dir.path().join("dev/README.md"))
            .expect("Unable to read README");

        // Modify the README in dev/
        std::fs::write(
            temp_dir.path().join("dev/README.md"),
            "This is a new README",
        )
        .expect("Unable to modify README");

        // Sync again with force option
        manifest
            .sync(&DwlMode::HTTPS, false, false, true)
            .await
            .expect("Unable to sync manifest");

        // Assert: README in dev/ should be the same as the original README
        assert_eq!(
            std::fs::read_to_string(temp_dir.path().join("dev/README.md"))
                .expect("Unable to read README"),
            original_readme
        );
    }

    #[tokio::test]
    async fn sync_when_source_are_modified() {
        // Setup: Sync project
        const ORIGINAL_MANIFEST_PATH: &str = "./tests/manifest_example.xml";
        let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
        let manifest = temp_dir.path().join("manifest.xml");
        std::fs::copy(ORIGINAL_MANIFEST_PATH, &manifest).unwrap();

        let mut manifest =
            ManifestInstance::try_from(manifest).expect("Failed to create manifest instance");

        manifest.parse().expect("Unable to parse manifest");

        manifest
            .sync(&DwlMode::HTTPS, false, false, false)
            .await
            .expect("Unable to sync manifest");

        // Modify the README in dev/
        std::fs::write(
            temp_dir.path().join("dev/README.md"),
            "This is a new README",
        )
        .expect("Unable to modify README");

        // Sync again without force option
        let result = manifest.sync(&DwlMode::HTTPS, false, false, false).await;

        // Assert we get an error
        match result {
            Err(e) => assert_eq!(
                e.to_string(),
                "Failed to sync manifest: \n\nFailed to checkout repository: ./dev, repository is dirty, please commit or stash your changes\n"
            ),
            _ => panic!("Expected an error"),
        }
    }

    #[tokio::test]
    async fn sync_when_new_file_is_added() {
        // Setup: Sync project
        const ORIGINAL_MANIFEST_PATH: &str = "./tests/manifest_example.xml";
        let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
        let manifest = temp_dir.path().join("manifest.xml");
        std::fs::copy(ORIGINAL_MANIFEST_PATH, &manifest).unwrap();

        let mut manifest =
            ManifestInstance::try_from(manifest).expect("Failed to create manifest instance");

        manifest.parse().expect("Unable to parse manifest");

        manifest
            .sync(&DwlMode::HTTPS, false, false, false)
            .await
            .expect("Unable to sync manifest");

        // Write a new file in dev/
        std::fs::write(temp_dir.path().join("dev/README"), "This is a new README")
            .expect("Unable to modify README");

        // Sync again with force option
        let result = manifest.sync(&DwlMode::HTTPS, false, false, true).await;

        // Assert we get no error
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn sync_with_delete_repository_action() {
        // Setup: Sync project
        const ORIGINAL_MANIFEST_PATH: &str = "./tests/manifest_del_repo.xml";
        let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
        let manifest = temp_dir.path().join("manifest.xml");
        std::fs::copy(ORIGINAL_MANIFEST_PATH, &manifest).unwrap();

        let mut manifest =
            ManifestInstance::try_from(manifest).expect("Failed to create manifest instance");

        manifest.parse().expect("Unable to parse manifest");

        manifest
            .sync(&DwlMode::HTTPS, false, false, false)
            .await
            .expect("Unable to sync manifest");

        // Assert linkfile and copyfile
        // Read file from dev directory since no_revision is deleted.
        assert_eq!(
            std::fs::read_to_string(temp_dir.path().join("dev/README.md"))
                .expect("Unable to read linkfile"),
            std::fs::read_to_string(temp_dir.path().join("new_folder/ln_README.md"))
                .expect("ln_README.md"),
        );

        assert_eq!(
            std::fs::read_to_string(temp_dir.path().join("dev/README.md"))
                .expect("Unable to read linkfile"),
            std::fs::read_to_string(temp_dir.path().join("cp_README.md")).expect("cp_README.md"),
        );

        // Assert delete action
        assert!(!std::path::Path::new(&temp_dir.path().join("no_revision")).exists());
    }

    #[tokio::test]
    async fn sync_with_copydir_action() {
        // Setup: Sync project
        const ORIGINAL_MANIFEST_PATH: &str = "./tests/manifest_copydir.xml";
        let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
        let manifest = temp_dir.path().join("manifest_example.xml");
        std::fs::copy(ORIGINAL_MANIFEST_PATH, &manifest).unwrap();

        let mut manifest =
            ManifestInstance::try_from(manifest).expect("Failed to create manifest instance");

        manifest.parse().expect("Unable to parse manifest");

        manifest
            .sync(&DwlMode::HTTPS, false, false, false)
            .await
            .expect("Unable to sync manifest");

        // Assert copydir action
        assert_eq!(
            std::fs::read_to_string(temp_dir.path().join("dev/folder/src/version.rs"))
                .expect("Unable to read src/version.rs"),
            std::fs::read_to_string(temp_dir.path().join("new_dev/src/version.rs"))
                .expect("copydir failed"),
        );
    }

    #[tokio::test]
    async fn sync_with_copydir_action_file_and_dir_exists() {
        // Setup: Sync project
        const ORIGINAL_MANIFEST_PATH: &str = "./tests/manifest_copydir.xml";
        let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
        let manifest = temp_dir.path().join("manifest_example.xml");
        std::fs::copy(ORIGINAL_MANIFEST_PATH, &manifest).unwrap();

        // Create destination file and directory before syncing
        let dest_path = temp_dir.path().join("./new_dev/test");
        std::fs::create_dir_all(dest_path).expect("failed to create dest dir");

        let mut manifest =
            ManifestInstance::try_from(manifest).expect("Failed to create manifest instance");

        manifest.parse().expect("Unable to parse manifest");

        manifest
            .sync(&DwlMode::HTTPS, false, false, false)
            .await
            .expect("Unable to sync manifest");

        // Assert copydir action
        assert_eq!(
            std::fs::read_to_string(temp_dir.path().join("dev/folder/src/version.rs"))
                .expect("Unable to read src/version.rs"),
            std::fs::read_to_string(temp_dir.path().join("new_dev/src/version.rs"))
                .expect("copydir failed"),
        );
    }
}
