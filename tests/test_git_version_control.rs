#[cfg(test)]
mod test_git_version_control {

    use colligo::application::{DwlMode, VersionControl};
    use colligo::git_version_control::GitVersionControl;
    use colligo::project::Project;
    use git2::Repository;

    #[test]
    fn clone_project_ssh() {
        const PROJECT_URI: &str = "gitlab.com";
        const PROJECT_NAME: &str = "cdsa_rust/manifest";
        const PROJECT_PATH: &str = "/tmp/manifest/tests/git_test/m_repo";
        const PROJECT_REVISION: &str = "dev";
        const CURRENT_DIR: &str = ".";
        let project = Project::new(
            PROJECT_URI.to_string(),
            PROJECT_NAME.to_string(),
            PROJECT_REVISION.to_string(),
            PROJECT_PATH.to_string(),
        );

        let git: Box<dyn VersionControl> = Box::new(GitVersionControl::new());
        let result = git.clone(CURRENT_DIR, &project, &DwlMode::HTTPS);
        let _ = git.checkout(CURRENT_DIR, &project);

        assert!(result.is_ok());
        let repo = Repository::open(PROJECT_PATH).expect("Unable to open repository");
        let head = repo.head().expect("Unable to get head");
        assert_eq!(head.name(), Some("refs/heads/dev"));

        std::fs::remove_dir_all(PROJECT_PATH).unwrap();
    }

    #[test]
    fn clone_project_lightweight_ssh() {
        const PROJECT_URI: &str = "gitlab.com";
        const PROJECT_NAME: &str = "cdsa_rust/manifest";
        const PROJECT_PATH: &str = "/tmp/manifest/tests/git_test/m_repo_light";
        const PROJECT_REVISION: &str = "v0.0.0";
        const CURRENT_DIR: &str = ".";
        let project = Project::new(
            PROJECT_URI.to_string(),
            PROJECT_NAME.to_string(),
            PROJECT_REVISION.to_string(),
            PROJECT_PATH.to_string(),
        );

        let git: Box<dyn VersionControl> = Box::new(GitVersionControl::new());
        let result = git.clone_lightweight(CURRENT_DIR, &project, &DwlMode::HTTPS);

        assert!(result.is_ok());
        let repo = Repository::open(PROJECT_PATH).expect("Unable to open repository");
        let head = repo.head().expect("Unable to get head");
        assert_eq!(
            head.peel_to_commit().unwrap().id().to_string(),
            "565b113e57b2c67dcaa3e7c2b5040cf4715221df"
        );

        let commit_id = git.get_commit_id(CURRENT_DIR, &project).unwrap();
        assert_eq!(commit_id, "565b113e57b2c67dcaa3e7c2b5040cf4715221df");

        std::fs::remove_dir_all(PROJECT_PATH).unwrap();
    }
}
