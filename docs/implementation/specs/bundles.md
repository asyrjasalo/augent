# Bundles (either with augent.yaml or without)

## Workspace Bundle Naming

The **workspace bundle name** is no longer stored in config files. It is automatically inferred based on workspace location:

- Git repositories: `@owner/repo` (extracted from origin remote)
- Non-git directories: `@username/directory-name` (fallback using username and directory name)

## Installing Resources

- Bundle names (in the bundles section) are assumed to be universally unique
- What to install is not dictated by augent.yaml, it is dictated by augent.lock
- If augent.lock is present in the directory, the bundles in it are installed in order
- If there are any resources in the bundle having augent.lock, the last entry in augent.lock is the bundle itself, and the same is for augent.yaml
- This ensures that the bundle's own resources are installed last

## Always when installing a bundle

- Augent config files are updated (unless the bundle of the same name is installed already)
- The installed bundle info is always stored regardless if that bundle has augent.lock itself or not (i.e. information on non-augent bundles, e.g. resource only bundles, or claude marketplace plugins, must be retained by exact git repo SHA for reproducibility)
- The config files are assumed to be in the following locations in the repo where install is run (first match takes precedence):
  - this directory if it has augent.lock (.)
  - ./augent.lock (repo root)
  - ./.augent/augent.lock (created by default by install if it does not exist)
- The lockfile is updated first, then `augent.yaml`, then `augent.index.yaml`
- If user installs multiple bundles in the repo, each of those bundles gets its own entry in augent.yaml, augent.lock, augent.index.yaml (it is part of the bundle name). Note: dependencies of dependencies are not stored in augent.yaml, they are stored in augent.lock

## Important

- All of augent files retain order in which things were installed
- augent.yaml includes only direct dependencies of this bundle (as bundles: [])
- The lockfile has all the dependencies, and dependencies of dependencies recursively, in installation order as well
- Similarly augent.index.yaml tracks in order what came where (also from dependencies of dependencies if still effective for platforms i.e. not overridden by later bundles)

## install bundle from directory (type: dir)

Dir bundle's name in `augent.yaml`, `augent.lock`, and `augent.index.yaml` is the name defined in the `augent.yaml` dependency, not the directory name.

### without augent.lock

user gives any of the following (assuming directory is at ./local-bundle):

- augent install ./local-bundle
- augent install local-bundle

where name is: local-bundle

-> installs all resources from path ./local-bundle

what is saved into augent.yaml:

```yaml
name: local-bundle
path: ./local-bundle
```

for dir bundles, path is relative to the directory where augent.lock is

### with augent.lock

user gives any of the following (assuming directory is at ./local-bundle):

- augent install ./local-bundle
- augent install local-bundle

where name is: local-bundle

-> install.augent.lock (bundles own resources last)

what is saved into augent.yaml:

```yaml
name: local-bundle
path: ./local-bundle
```

path is relative to the directory where augent.lock is

## install from git repository (type: git)

By default, @owner/repo is assumed to be a git repository in GitHub.

Git bundle's name is always in the following format in `augent.yaml`, `augent.lock`, `augent.index.yaml`:

@<owner>/repo[/bundle-name[/deeper-bundle-name]][:path/from/repo/root]

Note: / is used to separate bundle names, and optional path (to subdirectory) is only given after :

ref is never part of the name but it gets an own field in augent.yaml (even if default) and augent.lock. Important: the `augent.lock` always has `ref` and also THE EXACT `sha` of the commit. Otherwise the setup is not reproducible per lockfile.

If ref is not given, the git repo's default branch is read and used (usually either main or master).

ref can be:

- branch name
- tag name
- SHA of a commit

We will not use #ref (or alternative form @ref) in the examples below, but the operations are done respectively in that ref.

### from repo root

This applies when : is not given after the bundle name.

#### bundles without augent.lock

user gives any of the following:

- augent install owner/repo
- augent install @owner/repo
- augent install github:owner/repo
- augent install github:@owner/repo
- augent install https://github.com/owner/repo.git (HTTPS url)
- augent install https://github.com/owner/repo/tree/main (GitHub web UI url)
- augent install git@github.com:owner/repo.git (SSH url)

where name is: @owner/repo

-> installs all resources from the repo's root

what is saved into augent.yaml:

```yaml
name: '@owner/repo'
git: https://github.com/owner/repo.git
path: .
```

#### bundles with augent.lock

user gives any of the following:

- augent install owner/repo
- augent install @owner/repo
- augent install github:owner/repo
- augent install github:@owner/repo
- augent install https://github.com/owner/repo.git (HTTPS url)
- augent install https://github.com/owner/repo/tree/main (GitHub web UI url)
- augent install git@github.com:owner/repo.git (SSH url)

where name is: @owner/repo

-> first of the following applies (but not both):
    - if `augent.lock` is in the repo root, use that (path: `.`)
    - if `augent.lock` is in directory ./augent, use that (path: `./augent`)
-> install augent.lock (bundles own resources last)

what is saved into augent.yaml:

```yaml
name: '@owner/repo'
git: https://github.com/owner/repo.git
path: . or ./augent
```

#### install from git repository which stores claude code marketplace format

user gives: augent install davila7/claude-code-templates

-> prompts to select for bundles to install
-> user selects "git-workflow"

now name is known: @davila7/claude-code-templates/git-workflow

-> installs "git-workflow" resources (uses .claude-plugin/marketplace.json to resolve the path to all resources)

what is saved into augent.yaml:

```yaml
name: '@davila7/claude-code-templates/git-workflow'
path: $claude-plugin/git-workflow
```

Where $claude-plugin is a "virtual path". Claude Marketplace plugins must be supported. More repository formats will be supported in the future.

or directly:

user gives: augent install davila7/claude-code-templates/git-workflow

name is known: @davila7/claude-code-templates/git-workflow

-> installs "git-workflow" resources (uses .claude-plugin/marketplace.json to resolve the path to all resources)

in both cases, what goes into augent.yaml:

```yaml
name: '@davila7/claude-code-templates/git-workflow'
path: $claude-plugin/git-workflow
```

### install from git repository's subdirectory

#### bundles without augent.lock in subdirectory

user gives any of the following:

- augent install owner/repo:path/from/repo/root
- augent install @owner/repo:path/from/repo/root
- augent install github:owner/repo:path/from/repo/root
- augent install github:@owner/repo:path/from/repo/root
- augent install https://github.com/owner/repo.git:path/from/repo/root
- augent install https://github.com/owner/repo/tree/main/path/from/repo/root
- augent install git@github.com:owner/repo.git:path/from/repo/root

where name is: @owner/repo:path/from/repo/root

-> installs all resources from path/from/repo/root subdirectory

what is saved into augent.yaml:

```yaml
name: '@owner/repo:path/from/repo/root'
git: https://github.com/owner/repo.git
path: path/from/repo/root
```

#### bundles with augent.lock in subdirectory

this is used for installing only some bundle's own dependency which is stored in the same git repository in a directory, without installing the parent bundle.

user gives any of the following:

- augent install owner/repo:path/from/repo/root
- augent install @owner/repo:path/from/repo/root
- augent install github:owner/repo:path/from/repo/root
- augent install github:@owner/repo:path/from/repo/root
- augent install https://github.com/owner/repo.git:path/from/repo/root
- augent install https://github.com/owner/repo/tree/main/path/from/repo/root
- augent install git@github.com:owner/repo.git:path/from/repo/root

where name is: @owner/repo:path/from/from/repo/root

-> install augent.lock (bundles own resources last)

what is saved into augent.yaml:

```yaml
name: '@owner/repo/bundle-name'
git: https://github.com/owner/repo.git
path: path/from/from/repo/root
```

## Reference: Typical use cases

Install bundles from a git repository (either there is no augent.lock or it is in the standard locations):

```text
augent install @owner/repo
```

Install bundles from a git repository's subdirectory (either there is no augent.lock, or it is not in the standard locations):

```text
augent install @owner/repo:path/from/repo/root
```

Install only particular bundles from a git repository (there is augent.lock in the standard locations, or there is .claude-plugin/marketplace.json):

```text
augent install @owner/repo/bundle-name
```

Install only particular bundles from git repository subdirectory (e.g. there is augent.lock, or .claude-plugin/marketplace.json, but they are not in the standard locations):

```text
augent install @owner/repo/bundle-name:path/from/repo/root
```

Install only a bundle's own dependency named deeper-bundle-name (rare but must be possible):

```text
augent install @owner/repo/bundle-name/deeper-bundle-name
```
