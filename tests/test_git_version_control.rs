#[cfg(test)]
mod test_git_version_control {

    use colligo::application::{DwlMode, VersionControl};
    use colligo::git_version_control::GitVersionControl;
    use colligo::project::Project;
    use git2::Repository;

    #[tokio::test]
    async fn clone_project_ssh() {
        const PROJECT_URI: &str = "gitlab.com";
        const PROJECT_NAME: &str = "cdsa_rust/manifest";
        const PROJECT_REVISION: &str = "dev";
        let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");
        let project = Project::new(
            PROJECT_URI.to_string(),
            PROJECT_NAME.to_string(),
            PROJECT_REVISION.to_string(),
            temp_dir.path().display().to_string(),
        );

        let git = GitVersionControl::new();
        let result = git
            .clone(temp_dir.path(), &project, &DwlMode::HTTPS, None, false)
            .await;
        let _ = git.checkout(temp_dir.path(), &project, None, false).await;

        assert!(result.is_ok());
        let repo = Repository::open(temp_dir.path()).expect("Unable to open repository");
        let head = repo.head().expect("Unable to get head");
        assert_eq!(head.name(), Some("refs/heads/dev"));
    }

    #[tokio::test]
    async fn clone_project_lightweight_ssh() {
        const PROJECT_URI: &str = "gitlab.com";
        const PROJECT_NAME: &str = "cdsa_rust/manifest";
        const PROJECT_REVISION: &str = "v0.0.0";
        let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");

        let project = Project::new(
            PROJECT_URI.to_string(),
            PROJECT_NAME.to_string(),
            PROJECT_REVISION.to_string(),
            temp_dir.path().display().to_string(),
        );

        let git = GitVersionControl::new();
        let result = git
            .clone(temp_dir.path(), &project, &DwlMode::HTTPS, None, true)
            .await;

        assert!(result.is_ok());
        let repo = Repository::open(temp_dir.path()).expect("Unable to open repository");
        let head = repo.head().expect("Unable to get head");
        assert_eq!(
            head.peel_to_commit().unwrap().id().to_string(),
            "565b113e57b2c67dcaa3e7c2b5040cf4715221df"
        );

        let commit_id = git.get_commit_id(temp_dir.path(), &project).await.unwrap();
        assert_eq!(commit_id, "565b113e57b2c67dcaa3e7c2b5040cf4715221df");
    }

    #[tokio::test]
    async fn commit_with_error_word_in_message() {
        // Do not flag as ERROR commit with "error" or "fatal" in message
        const PROJECT_URI: &str = "gitlab.com";
        const PROJECT_NAME: &str = "cdsa_rust/manifest";
        const PROJECT_REVISION: &str = "main";
        let temp_dir = tempfile::TempDir::new().expect("failed to create temp dir");

        let project = Project::new(
            PROJECT_URI.to_string(),
            PROJECT_NAME.to_string(),
            PROJECT_REVISION.to_string(),
            temp_dir.path().display().to_string(),
        );

        let git = GitVersionControl::new();
        let result = git
            .clone(temp_dir.path(), &project, &DwlMode::HTTPS, None, false)
            .await;
        assert!(result.is_ok());

        let project = Project::new(
            PROJECT_URI.to_string(),
            PROJECT_NAME.to_string(),
            "dev".to_string(),
            temp_dir.path().display().to_string(),
        );

        let result = git.checkout(temp_dir.path(), &project, None, false).await;
        assert!(result.is_ok());
    }
}
