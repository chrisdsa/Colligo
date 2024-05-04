# This is a branch to test Colligo

Adding a commit with the words fatal and error in the commit message. 

# manifest

Manage the project's source dependencies by having an XML file describing where to clone / update git repositories to a
specific revision / branch / commit.

## Usage

### Create a manifest

Create a manifest file named `manifest.xml` in the root of the project. You can use the
`manifest --generate manifest.xml` command to generate a manifest with the file format and comments describing the
different options.

```bash
manifest --generate manifest.xml
```

### Clone / Update repositories

To download the repositories described in the manifest, or update them to the revision specified in the manifest, use
the `--sync` option. The `--https` option can be used to use HTTPS instead of SSH to clone the repositories. The
default name for the manifest is `manifest.xml`, but you can specify a different name with the `--input` option.

```bash
manifest --sync [--input your_manifest.xml] [--https]
```

### Pin manifest to current commit id

To pin each repository revision to the current revision commit id, use the `--pin` option. You must provide the name of
the file where to write the pinned manifest. It goes through all the repositories described in the manifest (
default `manifest.xml` or the one provided by `--input`) and change the revision to the current commit id. It then saves
the new manifest to the file specified by `--pin`.

```bash
manifest --pin pinned_manifest.xml [--input your_manifest.xml]
```

## Motivation

The objective of this project is to provide a simple tool to manage the source dependencies of a project. It is inspired
by the [repo](https://source.android.com/setup/develop/repo) tool from the Android Open Source Project.
However, unlike repo, the manifest does not require a separate repository to be created. It thus simplifies the setup
of a project, simplify the CI/CD pipeline and makes changes to the manifest easier to track. 
