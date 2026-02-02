# Install with Dependencies

## Workspace bundle

The workspace bundle lockfile is either in the workspace root or in the `.augent/` directory.

What is installed, is always dictated by the workspace `augent.lock` file.

The lockfile is installed in top-down order, and later bundles override earlier bundles if the file names overlap on some platform.

What has been installed per platform, is dictated by the workspace `augent.index.yaml`. This file is read on uninstall to know what to remove from platform dirs, and what else (resources from earlier bundles) becomes effective after removal.

Important: There are at max one `augent.lock` and one `augent.index.yaml` file in the workspace (git repository). Otherwise it is impossible to track in the scope of the workspace what has been installed and per what platform.

Note: The workspace does not necessarily have any bundles (no `augent.lock` or `.augent/augent.lock`): This is the case for resource only git repositories (resources dirs such as commands, agents are in the repo root or in some subdirectory) and Claude Marketplace plugins (which have plugins defined in `.claude-plugins/marketplace.json`).

### example: workspace bundle

Important: File `augent.lock` is first searched in the repository root,
then in the `.augent/augent.lock`.

The repository root takes precedence over the `.augent/` directory when installing the workspace bundle (either locally or via a git repository on another machine).

If `augent.lock` does not exist but there is nothing to install (no bundles or platforms selected), it will not be created.

If `augent.lock` does not exist, but there is something to install (or some platform), it is created in `.augent/augent.lock`. Location `./augent` is default when new workspace bundles are created.

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

If `augent.yaml` does not exist, it is created per all bundles from `augent.lock` (both dir and git bundles, in the same order). For dir bundles, name and path (relative from where `augent.yaml` is) are added. For git repositories, url and ref are added, also subdirectory if it is not repo root.

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
bundle dir root (where resources are), both for the workspace bundle and the
dir bundles if any.

## Dir bundle(s)

The workspace has at most one workspace bundle and zero or more dir bundles (each of which may or may not have a `augent.yaml` file).

Dir bundles may have dependencies on other bundles (either other dir bundles or git bundles) by having them listed in the dir's `augent.yaml` bundles section.

Important: The dir bundles do not have their own `augent.lock` or `augent.index.yaml` files.

All **installed** dir bundles (and their dependencies) are tracked in the workspace `augent.lock` and `augent.index.yaml`.

Important: This allows uninstall, list and show commands to work in the workspace. Similarly, there must be only one index which tracks all the effective resources per platforms on the workspace.

However, it is possible to install a dir bundle directly by its name or by its path without installing the workspace bundle.

Installing particular dir bundle updates the workspace `augent.lock` and `augent.index.yaml` (including its dependencies), but does not update the workspace `augent.yaml` (does not add it to the bundles section).

To install dir bundles as part installing the workspace bundle, you need to explicitly add them to the workspace `augent.yaml` bundles section.

```yaml
bundles:
- name: my-dir-bundle
  path: ../my-dir-bundle
```

### example: dir bundle

installing a dir bundle:

```bash
augent install ./my-dir-bundle
cd my-dir-bundle/ && augent install
```

does the following:
-> updates `.augent/augent.lock`
    - dependencies of my-dir-bundle-name come before my-dir-bundle
-> updates `.augent/augent.index.yaml`
    - dependencies of my-dir-bundle-name come before my-dir-bundle
-> updates `.augent/augent.yaml` as following:
    - takes the bundle name from the dir name
    - the dir bundle path is relative to dir where `augent.yaml` is
    - only the bundle itself is added in `augent.yaml`, not its dependencies:

```yaml
bundles:
- name: my-dir-bundle
    path: ../my-dir-bundle
```

or:

if `.augent/augent.yaml` is already as such:

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

## git bundle

When installing a git bundle, only the workspace `augent.lock` file is read,
neither the workspace `augent.yaml` nor any other `augent.yaml` in the repository.

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
`augent.lock`.

Note: This does not install the my-dir-bundle-name's dependencies,
as dependency relationships (i.e. what is a dependency of what)
are not stored in the workspace `augent.lock`.

Note that this is not possible:

```bash
augent install @owner/repo/@another-owner/repo
```

Even though @another-owner/repo was listed in the workspace `augent.lock`.
In this case you can install it directly by its @another-owner/repo name.

Note: Dependency relationships (i.e. what is a dependency of what) are not stored in the workspace `augent.lock` so you cannot go deeper, e.g. `augent install @owner/repo/my-le-name/my-sub-bundle-name` is not possible.

It is possible to install directly from a git repository subdirectory:

```bash
augent install @owner/repo:my-dir-bundle
```

In this case, the bundle is not required listed in the workspace `augent.lock`.
