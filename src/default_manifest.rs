pub const DEFAULT_MANIFEST_FILE: &str = r#"<?xml version="1.0" encoding="UTF-8" ?>
<manifest>
    <!-- Project's default settings -->
    <default revision="main" uri="hostname.com"/>

    <!-- path is relative from where manifest is executed -->
    <project name="repo/name" path="path/folder" revision="branch"/>
    <project name="repo/name" path="folder" revision="tag"/>

    <!-- It is possible to duplicate file using linkfile or copyfile -->
    <!-- It is also possible to copy recursively a directory using copydir -->
    <!-- src path is relative to the project path -->
    <!-- dest path is relative to the manifest file -->
    <!-- delete_project is used to delete the directory at the given path. Always executed last -->
    <project uri="hostname.com" name="repo/name" path="folder">
        <linkfile src="filename" dest="new_filename"/>
        <copyfile src="filename" dest="new_filename"/>
        <copydir src="directory" dest="new_directory"/>
        <delete_project/>
    </project>

</manifest>"#;
