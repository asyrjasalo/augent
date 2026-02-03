# Install with Dependencies

## Workspace bundle

The workspace bundle lockfile (`augent.lock`) is either in the workspace root or in the `.augent/` directory. If it in the workspace root, it takes presendence. In this case, `augent.yaml` and `augent.index.yaml` are also in the workspace root, or created in the workspace root.

What version are installed, is always dictated by the workspace `augent.lock` file.

The lockfile is installed in top-down order, and later bundles override files from earlier bundles if the file paths and names overlap when installed on a particular platform.

What has been installed per platform, is dictated by the workspace `augent.index.yaml`. This file is read on uninstall to know what to remove from platform dirs, and what else (resources from earlier bundles) becomes enabled ective after removal. It only keeps tracks of files that are effective, e.g. if two bundles provide the same file on the same platform, only the later bundle's file is tracked as that is enabled. On install and uninstall what is enabled may change depending and what files the bundle provides and on what platform(s) are detected or selected.

Important: There are at max one `augent.lock` and one `augent.index.yaml` file in the workspace. Otherwise it would be impossible to track in the scope of the workspace what versions has been installed and on what platform(s) their files are enabled.

Note: The workspace does not necessarily have any bundles (neither `augent.lock` nor `.augent/augent.lock`): This is the case for resource only git repositories (resources dirs such as commands, agents are in the repo root or in some subdirectory) and Claude Marketplace plugins (which have plugins defined in `.claude-plugins/marketplace.json`).

### example: workspace bundle

Important: File `augent.lock` is first searched in the repository root,
then in the `.augent/augent.lock`.

The repository root takes precedence over the `.augent/` directory when installing the workspace bundle (either locally or via a git repository on another machine).

If `augent.lock` does not exist but there is nothing to install (no bundles or platforms selected), it will not be created. If `augent.lock` does not exist, but there is something to install (some platform is detected or selected), it is created in `.augent/augent.lock`. Location `./augent` is default when new workspace bundle is created.

Only after `augent.lock` is created, and `augent.index.lock` has been populated, `augent.yaml` is created.

If the `augent.lock` exists in the repository root (`./`), installing the workspace bundle:

```bash
augent install
```

does the following:
-> updates `./augent.lock`
-> creates or updates `./augent.index.yaml`
-> creates or updates `./augent.yaml`

If `augent.index.yaml` does not exist, it is created per all bundles from `augent.lock` and per their files listed there. The platforms are detected per workspace unless they have been asked earlier by the install command. If the index did not already exist, it is assumed by default that later bundles in lockfile override earlier bundles if overlapping file names on a platform.

If `augent.yaml` does not exist, it is created per bundles from `augent.lock`. For each dir bundle, their path is searched for `augent.yaml` so that it is known what bundles are dependencies of what. Thus when `augent.yaml` is (re-)created from `augent.lock`, it must only have direct dependencies in the order they came from the lockfile. not dependencies of dependencies.

When adding bundle to `augent.yaml`, for dir bundles, name and path are added. Path is relative to the directory where `augent.yaml` is. For git repositories, url and ref are added, also subdirectory if does not install from the repo root.

If `augent.lock` exists in the `.augent/` directory, installing the workspace bundle:

```bash
augent install
```

or:

```bash
cd .augent/ && augent install
```

does the following:
-> updates `.augent/augent.lock`
-> creates or updates `./augent/.augent.index.yaml`
-> creates or updates `./augent/.augent.yaml`

Behavior is the same as if `augent.lock` existed in the repository root as
the lockfile paths are relative to the repo root, with the exception that the dir bundle paths in `.augent/augent.yaml` are relative to `.augent/` (if it was in root, the path were relative to the root).

Enabled file paths in `augent.index.yaml` are always relative to the
bundle dir root (where the resources are), both for the workspace bundle and the
dir bundles.

## Dir bundle(s)

The workspace has at most one workspace bundle and zero or more dir bundles (each of which may or may not have a `augent.yaml` file). If workspace bundle does not have `augent.yaml`, only its resources are installed. The bundle is marked installed in the workspace `augent.lock` and `augent.index.yaml` in any case.

Dir bundles may have dependencies on other bundles (either other dir bundles or git bundles) by having them listed in the dir's `augent.yaml` bundles section. This file is used to decide what bundles to install when installing the dir bundle, or when installing the dir bundle as dependency of the workspace bundle.

Very important: The dir bundles do not have their own `augent.lock` or `augent.index.yaml` files. All installed dir bundles (and their dependencies) are tracked in the workspace `augent.lock` and `augent.index.yaml`.

This allows uninstall, list and show commands to work in the workspace. Similarly, there must be only one index which tracks all the effective resources per platforms on the workspace.

However, it is possible to install a dir bundle directly by its name or by its path without installing the workspace bundle.

Installing a particular dir bundle updates the workspace `augent.lock` and `augent.index.yaml` (including its dependencies), but does not update the workspace `augent.yaml` (does not add it to the bundles section).

**Important**: When installing a dir bundle directly:

- If `augent.yaml` does not exist, it is **not** created
- If `augent.yaml` exists, it is **not** modified
- **augent.yaml is NEVER removed if it is present**

When installing the workspace bundle (`augent install` without args):

- `augent.yaml` is always created or updated (even if empty of dependencies, to preserve workspace metadata)

To install dir bundles as part installing the workspace bundle, you need to explicitly add them to the workspace `augent.yaml` bundles section.

```yaml
bundles:
- name: my-dir-bundle
  path: ../my-dir-bundle
```

### example: dir bundle

case a:installing a dir bundle:

```bash
augent install ./my-dir-bundle
```

or (equivalent):

```bash
cd my-dir-bundle/ && augent install
```

does the following:
-> updates `.augent/augent.lock`
    - dependencies of my-dir-bundle-name come before my-dir-bundle
-> updates `.augent/augent.index.yaml`
    - dependencies of my-dir-bundle-name come before my-dir-bundle
-> `.augent/augent.yaml` is not updated

case b: if `.augent/augent.yaml` is already as such:

```yaml
bundles:
- name: my-dir-bundle-name
    path: ../my-dir-bundle
```

this:

```bash
augent install my-dir-bundle-name
```

does the following:
-> updates `.augent/augent.lock`
    - dependencies of my-dir-bundle-name come before my-dir-bundle
-> updates `.augent/augent.index.yaml`
    - dependencies of my-dir-bundle-name come before my-dir-bundle
-> `.augent/augent.yaml` is not updated

## git bundle

When installing a git bundle, only the workspace `augent.lock` file is read,
neither the workspace `augent.yaml` nor any other `augent.yaml` in the repository.

On install, git bundles are always added to the `augent.yaml` when augent install is run. If augent install <git-bundle> is run in the workspace, workspace `augent.yaml` is updated. If augent install <git-bundle> is run inside a dir bundle, the dir bundle's `augent.yaml` is updated.

When installing a git bundle directly (via URL):

- If `augent.yaml` does not exist, it is **created** with the git bundle added
- If `augent.yaml` exists, it is **updated** by adding the git bundle (if not already in bundles)
- **augent.yaml is NEVER removed** once it has been created

### example: git bundle

installing the workspace bundle:

```bash
augent install @owner/repo
```

installing a dir bundle without installing the workspace bundle:

```bash
augent install @owner/repo/my-dir-bundle-name
```

Where my-dir-bundle-name is the name of the dir bundle in the workspace
`augent.lock`. For each dir bundle, its path is searched for `augent.yaml` and then install is run on that path. Note that in all cases, this updates the workspace `augent.lock` and `augent.index.yaml`.

This is not possible:

```bash
augent install @owner/repo/@another-owner/repo
```

Even though @another-owner/repo was listed in the workspace `augent.lock`.
In this case you can install it directly by its @another-owner/repo name.

It is possible to install directly from a git repository subdirectory without installing the repo's workspace bundle:

```bash
augent install @owner/repo:my-dir-bundle
```

In this case, the bundle is not required listed in the workspace `augent.lock` and the path is searched for `augent.yaml`. In this case, the workspace (where it installed in) `augent.lock` and `augent.index.yaml` are updated as usual.
