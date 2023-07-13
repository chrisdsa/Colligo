pub const DEFAULT_MANIFEST_FILE: &str = r#"<?xml version="1.0" encoding="UTF-8" ?>
<manifest>
    <!-- Project's default settings -->
    <default revision="main" uri="gitlab.com"/>

    <!-- path is relative from where manifest is executed -->
    <project name="repo/name" path="path/folder" revision="branch"/>
    <project name="repo/name" path="folder" revision="tag"/>

    <!-- It is possible to duplicate file using linkfile or copyfile -->
    <!-- src path is relative to the project path -->
    <!-- dest path is relative to the manifest file -->
    <project uri="hostname.com" name="repo/name" path="folder">
        <linkfile src="filename" dest="new_filename"/>
        <copyfile src="filename" dest="new_filename"/>
    </project>

</manifest>"#;
