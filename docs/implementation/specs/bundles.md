# Bundles (either with augent.yaml or without)

Installing resources:

- The bundles name is assumed universally unique
- If augent.yaml is present in the directory, the bundles in it are resolved recursively after.
- Installs bundles own resources last, they may override resources from earlier bundles if the file names overlap.

Always when installing a bundle:

- Augent config files are updated (unless the bundle of same name is installed already). The installed bundle info is always stored regardless it that bundle has augent.yaml itself or not
- The config files are assumed to be in the following locations in the repo where install is run (first match takes precedence):
  - this directory if it has augent.yaml (.)
  - ./augent.yaml (repo root)
  - ./.augent/augent.yaml (created by default by install if does exist)
- The lockfile is updated first, then `augent.yaml`, then `augent.index.yaml`
- If user installs multiple bundles in the repo, each of those bundles gets its own entry in augent.yaml, augent.lock, augent.index.yaml (it is part of the bundle name).

Installation order:

- All of augent files retain order in which things were installed.
- Augent.yaml includes only direct dependencies (as bundles: [])
- The lockfile has all the dependencies, and dependencies of dependencies recursively, this is in installation order as well.
- Similarly augent.index.yaml tracks in order in what came where (also from dependencies of dependencies) if that file is not overridden by a later bundle.

## install bundle from directory (type: dir)

Dir bundle's name is always as following in `augent.yaml`, `augent.lock`, `augent.index.yaml`:

name: dir-name

### without augent.yaml

user gives any of the following (assuming directory is at ./local-bundle):

- augent install ./local-bundle
- augent install local-bundle

where name is: local-bundle

-> installs all resources from path local-bundle

what is saved into augent.yaml:

```yaml
name: local-bundle
path: ./local-bundle
```

path is relative to the directory where augent.yaml is

### with augent.yaml

user gives any of the following (assuming directory is at ./local-bundle):

- augent install ./local-bundle
- augent install local-bundle

where name is: local-bundle

-> recursively installs all bundles and their resources in order in ./local-bundle/augent.yaml
-> bundles own resources are installed last from the dir where augent.yaml is

what is saved into augent.yaml:

```yaml
name: local-bundle
path: ./local-bundle
```

path is relative to the directory where augent.yaml is

## install from git repository (type: git)

By default, @owner/repo is assumed to be a git repository in GitHub.

Git bundle's name is always in the following format in `augent.yaml`, `augent.lock`, `augent.index.yaml`:

name: @<owner>/repo[/bundle-name[/deeper-bundle-name]][:path/from/repo/root]

ref is not part of the name but it gets separate field in augent.yaml and augent.lock. Important: the `augent.lock` always has `ref` and THE EXACT `sha` of the commit.

ref can be:

- branch name
- tag name
- SHA of a commit

and git operations are then done respectively.

If ref is not given, the repo's default branch is assumed.

We will not use #ref (or alternative form @ref) in the examples below.

### from repo root

#### bundles without augent.yaml

user gives any of the following:

- augent install owner/repo
- augent install @owner/repo
- augent install github:owner/repo
- augent install github:@owner/repo
- augent install <https://github.com/owner/repo.git> (HTTPS url)
- augent install <https://github.com/owner/repo/tree/main> (GitHub web UI url)
- augent install <git@github.com>:owner/repo.git (SSH url)

where name is: @owner/repo

-> installs all resources from the repo's root

what is saved into augent.yaml:

```yaml
name: '@owner/repo'
git: https://github.com/owner/repo.git
path: .
```

#### bundles with augent.yaml

user gives any of the following:

- augent install owner/repo
- augent install @owner/repo
- augent install github:owner/repo
- augent install github:@owner/repo
- augent install <https://github.com/owner/repo.git> (HTTPS url)
- augent install <https://github.com/owner/repo/tree/main> (GitHub web UI url)
- augent install <git@github.com>:owner/repo.git (SSH url)

where name is: @owner/repo

-> either of the following applies (but not both):
    - it if it is repo root, use that (path: `.`)
    - if it is in directory ./augent, use that (path: `./augent`)
-> recursively installs all bundles from that augent.yaml
-> bundles own resources are installed last from the dir where augent.yaml is

what is saved into augent.yaml:

```yaml
name: '@owner/repo'
git: https://github.com/owner/repo.git
path: ´. or ./augent
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

#### bundles without augent.yaml in subdirectory

user gives any of the following:

- augent install owner/repo:path/from/repo/root
- augent install @owner/repo:path/from/repo/root
- augent install github:owner/repo:path/from/repo/root
- augent install github:@owner/repo:path/from/repo/root
- augent install <https://github.com/owner/repo.git:path/from/repo/root>
- augent install <https://github.com/owner/repo/tree/main/path/from/repo/root>
- augent install <git@github.com>:owner/repo.git:path/from/repo/root

where name is: @owner/repo:path/from/repo/root

-> installs all resources from path/from/repo/root subdirectory

what is saved into augent.yaml:

```yaml
name: '@owner/repo:path/from/repo/root'
git: https://github.com/owner/repo.git
path: path/from/repo/root
```

#### bundles with augent.yaml in subdirectory

this is used for installing only some bundle's own dependency which is stored in the same git repository in a directory, without installing the parent bundle.

user gives any of the following:

- augent install owner/repo:path/from/repo/root
- augent install @owner/repo:path/from/repo/root
- augent install github:owner/repo:path/from/repo/root
- augent install github:@owner/repo:path/from/repo/root
- augent install <https://github.com/owner/repo.git:path/from/repo/root>
- augent install <https://github.com/owner/repo/tree/main/path/from/repo/root>
- augent install <git@github.com>:owner/repo.git:path/from/repo/root

where name is: @owner/repo:path/from/from/repo/root

-> recursively installs all bundles from path/from/from/repo/root's augent.yaml
-> bundles own resources are installed last from the dir where augent.yaml is

what is saved into augent.yaml:

```yaml
name: '@owner/repo/bundle-name'
git: https://github.com/owner/repo.git
path: path/from/from/repo/root
```

## Reference: Typical use cases

Install bundles from a git repository (either there is no augent.yaml or it is in the standard locations):

```text
augent install @owner/repo
```

Install bundles from a git repository's subdirectory (either there is no augent.yaml, or it is not in the standard locations):

```text
augent install @owner/repo:path/from/repo/root
```

Install only particular bundles from a git repository (there is augent.yaml in the standard locations, or there is .claude-plugin/marketplace.json):

```text
augent install @owner/repo/bundle-name
```

Install only particular bundles from git repository subdirectory (e.g. there is augent.yaml, or .claude-plugin/marketplace.json, but they are not in the standard locations):

```text
augent install @owner/repo/bundle-name:path/from/repo/root
```

Install only a bundle's own dependency named deeper-bundle-name (rare but must be possible):

```text
augent install @owner/repo/bundle-name/deeper-bundle-name
```
