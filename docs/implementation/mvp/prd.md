# Product Requirements Document

## Goal

### What are we solving

As of today (2026-01-22), there are many AI coding platforms for developers to choose from, and more is expected to come. Some popular ones include OpenCode, Claude Code, Cursor, Codex CLI and GitHub Copilot. Many developers increasingly use more than one AI coding platform in the project scope.

There is no well-established process to manage these AI coding platforms' resources across different platforms or projects.

The resources are key assets for AI driven development process such as commands, rules, subagents, skills and MCP servers. Many of developers rely on maintaining their own set of resources and copy them across projects.

### What do we have today

As of 2026-01-22, the most promising solution to resolve these issues in an agent-independent manner is [OpenPackage](https://github.com/enulus/OpenPackage). The project is not fully matured yet (it has a central registry but does not share code for that registry) and lacks accelerating adoption even though the problem itself has been widely acknowledged and is pending solution.

OpenPackage also has a few design flaws due to it stemming from being designed on how other package managers (like npm) work, and not acknowledging that managing AI coding platform resources is whole lot of different from managing software packages.

Some examples of early design flaws include implementing development dependencies without careful thought when they are installed. Overall, development dependencies has little recognized use cases yet in managing resources for AI coding platforms, and it mostly confuses what gets installed and when.

On the other hand, it lacks useful features from real package managers like dependency locking. This is mandatory requirement when using version ranges in dependencies to ensure reproducibility in the first place across team members.

From the adoption point of view, likely its biggest issue is that is not designed with simplicity in mind from the ground up. Already there are too many commands. A few commands like `install` and `apply` are too ambiguous to be understood by their name, it is not clear what `save` does, what `add` adds and where `publish` puts the packages etc. without careful reading of documentation.

### How we will resolve it

We will now implement an AI package manager supporting various AI coding platforms and relying on OpenPackage's quite ok support for various AI coding platforms (talking on them as "platforms")

We will do this in a development-friendly manner. This means that it is not only easy, BUT OBVIOUS, to use for anyone who has used any package manager before in any programming language, and not only that but it is actually far simpler than that.

This means **we will not cargo cult any existing packager manager and we will not implement ANY bells and whistles than what is required to achieve our goal**.

Our goal is:

1) Implement AI coding platform and platform independent resource management,
2) in a lean, intuitive, developer friendly, non-documentation relying way,
3) with easy extensibility without code changes

To respond to fast evolving landscape of AI coding platforms and their features.

### What will 1.0 look like

#### Terminology

**Workspace** - working copy of git repository at hand on developer's machine

**Workspace_root** - the root directory of the workspace (usually where `.git` directory is and where AI coding platform specific directories are stored)

**Bundle** - a directory with platform independent resources (either in its root or in subdirectories) and optionally Augent bundle config files regarding the bundle (its dependencies, its name, optional metadata).

**Aug** - a file that is provided by a bundle in a platform-independent **format** (e.g. rule `<bundle_dir>/rules/debug.md` or mcp server `<bundle_dir>/mcp.jsonc`)

- `*resource` - a file that is provided by a bundle in a platform independent format (e.g. `<bundle_dir>rules/debug.md`) or a resource that has been installed by a bundle for a specific AI coding platform in its own format (e.g. `<workspace_root_dir>/.cursor/rules/debug.mdc`)

#### What we will have

- A platform-independent command line tool written in Rust, named Augent (the binary is `augent`)

- It works on MacOS, Linux and Windows. For two latter, it works on ARM64 and x86_64 architectures.

- It implements as few commands (`install`, `uninstall`, `list`, `show`) as possible.

- The app knows all the AI coding platform formats that OpenPackage currently does and can convert from platform independent resources to AI coding platform specific resources and back.

- All matured CLI tool practices and Rust development best practices are followed.

#### What we will not have

- Development dependencies (like npm or OpenPackage has) or cargo-culting any existing package managers for "nice to have" features that "might be useful in the future" (it is ABSOLUTELY TOO HARD to deprecate almost any of features in package manager)

- A centralized registry for publishing and distributing packages. We will distribute bundles in a single Git repository, in a clear text format, as that is what the developers want to go read in GitHub before taking a bundle into use

- We will not install bundles (outside of the current repository) anywhere else than from git repositories via https:// (or via ssh:// for private repositories). It has to support other sources than GitHub even though GitHub is likely the most popular for public bundles. We have to keep in mind that some organizations will have their own set of private bundles (git repositoriies).

### Type 1 decisions

Note: These fundamental decisions cannot be reversed so it is ABSOLUTELY REQUIRED to get them right from the beginning or not implement them at all.

#### Package format

- We implement a `bundle` (a lightweight package concept) as a directory in filesystem.

- The bundle is a directory in filesystem.

- Bundle is "published" to outer world via a Git repository.

- This git repository can be essentially anywhere (even in the same filesystem)
and we should not limit our implementation to only GitHub, GitLab, etc..

- Bundle can define dependencies to other bundles via `augent.yaml` file, but presence of that file is optional.

- Bundles are installed in order they are presented in `augent.yaml` so later bundles override the resources from earlier bundles where the resource file names overlap. For non-merged files (commands, rules, skills, root files), later bundles completely override earlier bundles if file names overlap. For merged files (AGENTS.md, mcp.jsonc, etc.), the merge behavior defined in the platform configuration applies instead of overriding. This override behavior is silent - no warnings are shown when later bundles override earlier bundles' files.

- Some resources are merged with the existing ones if they already exist for the agent, e.g. AGENTS.md and mcp.jsonc files. The merging strategy likely differs between these file types (AGENTS.md is markdown, mcp.jsonc is JSON). **TODO: Research OpenPackage's platforms.jsonc schema for the exact merging behavior for each file type.**

- Augent will know how to install the directory's content for AI coding platforms regardless of whether `augent.yaml`, `augent.lock`, `augent.workspace.yaml` is present in the directory. This ensures we maintain compatibility with Claude Code plugins, skills only repos, etc.

- If a dependency is installed in the workspace, then `augent.lock` if the same directory as `augent.yaml` has an entry for it. It does not necessary mean that that dependency's provided files are installed for all or even any of the agents. What is installed per agent (and "where did the resources come from" is tracked in `augent.workspace.yaml`)

#### Locking

- If bundle has `augent.yaml` and `install` has been run, it also has `augent.lock`. Install takes care of updating the lockfile unless `--frozen` is used.

- In `augent.yaml`, dependencies are specified with exact refs (branch names, tag names, or SHAs). The lockfile resolves these to exact git SHAs for reproducibility. Note: There is no concept of semantic versioning or version ranges.

- If `--frozen` is used, it fails if the lockfile is missing or if the resolved versions would change (e.g., remote's main branch has moved to a different commit than what's in the lockfile).

- To update to a new version of a dependency, the user manually edits the ref in `augent.yaml` (e.g., changes `ref: main` to the same branch name) and runs `install` again to refresh the lockfile.

- If bundle has the lockfile, the bundle's lockfile is read when installing a bundle as a dependency. The bundle's lockfile contains the name of the bundle and its checksum.

- In lockfile, all sources have been resolved TO BE EXACT, that is anything from github uses exact URL and SHA for a git repository. It may have org/user and ref info present but that is not used for resolving.

- Lockfile lists all the files that are provided by the bundle, NOT WHAT is necessarily installed by the bundle. This means, providing you install for all AI coding platforms, you should be able to track where the resources came from by starting from the end of the lockfile.

- The last entry in the lockfile is the bundle itself. Thus the bundles own resources (in the bundle's dir) always override the resources from earlier dependencies if the file names overlap, except where resources are merged.

- Each bundle in lockfile has a calculated `hash` per its contents (all files, including `augent.yaml`, but excluding its `augent.lock` and `augent.workspace.yaml` files). The hashing algorithm used is BLAKE3. Also the last entry, this bundle.

#### Workspace

- On first install, augent creates `augent.yaml`, `augent.lock`, and `augent.workspace.yaml` with a workspace bundle named `@author/workspace-dir-name`. The name is inferred from the git repository remote URL (org/user), with a fallback to `USERNAME/WORKSPACE_ROOT_DIR_NAME` if no git remote is configured.

- The user can change AI coding platforms' files directly in repo (e.g. `.opencode/commands/debug.md`). When `install` is run, it detects files that differ from the bundle's version and copies them to the workspace bundle's directory as platform independent resources. This includes modifications to files from bundle dependencies - the modified file is added to the workspace bundle and overrides the dependency's version for that specific file. This ensures `install` never overwrites local user changes.

- Modified file detection: augent uses `augent.workspace.yaml` to trace which bundle and git-SHA each file came from, then calculates the BLAKE3 checksum of the original file from the cached bundle and compares it to the current workspace file. If they differ, the file is considered modified.

- The mapping for bundle resources and AI coding platform resources is stored in `augent.workspace.yaml`. The format is file for each file that particular bundle provides, and for each AI coding platform that it is installed for (user can use `install --for <platform>...` to install some bundles only for specific platforms).

#### Sources

For v1.0, augent supports these bundle source types:

- **Local directory paths**: Relative paths within the current repository (e.g., `./bundles/my-bundle` or `../shared-bundle`)
- **Git repositories**: Full HTTPS or SSH URLs (e.g., `https://github.com/user/repo.git` or `git@github.com:user/repo.git`)
- **GitHub short-form**: `github:author/repo` or simply `author/repo` (defaults to GitHub)
- **Git repositories with subdirectory**: Git URLs can specify a subdirectory using `:` (e.g., `github:user/repo:plugins/bundle-name`)
- **Git repositories with ref**: Git URLs can specify a ref (branch, tag, or SHA) using `#` or `@` (e.g., `github:user/repo#v1.0.0` or `github:user/repo@v1.0.0`). The `#` syntax is preferred and follows URL standards.
- **Git repositories with ref and subdirectory**: Combine both using `#ref:subdir` or `@ref:subdir` (e.g., `github:user/repo#main:plugins/bundle-name`). The `#` syntax is preferred.

**Git Authentication**: For private git repositories, augent delegates entirely to git's native authentication system. This means it uses the user's existing SSH keys configured in `~/.ssh/` and git credential helpers. No separate authentication configuration is required for augent.

Any of these sources may or may not have `augent.yaml` and `augent.lock` files present - they will still work, but bundles without `augent.yaml` will not install any dependencies.

### Type 2 decisions (how we make it future proof)

#### Other sources

For 1.0.0, we should be able to install resources from:

- From Git repositories which store Claude Code plugin

- From Git repositories which store Claude Code marketplace (multiple Claude Code plugins)

Basically, the user can give any path or repo url or github:author/reponame to `install` and it will know how to handle it (as long as it has resources in its path or in its subdirectories). If there are multiple set of resources in the repo (e.g. aforementioned bundles, or multiple Claude Code plugins), it will show an interactive menu listing all discovered bundles/subdirectories and allow the user to select multiple ones to install. **Note**: If a subdirectory is explicitly specified in the source path, the menu is not shown and only that bundle is installed.

#### Other AI coding platforms

We adopt a `platforms.jsonc` configuration file approach to support ever increasing number of AI coding platforms and their features. It must be possible for the developer to add support to new AI coding platforms without changing the core code.

**TODO: Research OpenPackage's platforms.jsonc schema and implementation** including:

- Platform identifier (e.g., "opencode", "cursor", "claude")
- Directory structure where resources are stored
- File format transformations (agent-independent → agent-specific)
- Merge behavior for special files (AGENTS.md, mcp.jsonc, etc.)
- Root file/dir handling

Note: Since augent.workspace.yaml tracks which files are used by which bundle, we may later implement a mechanism to detect and remove unused bundles (bundles that don't provide any files that are currently in use). This is not in scope for v1.0.

If another package format becomes popular, we will collaborate to add support to import from it, but we will not compromise our goal to implement some features just for the sake of something being popular.

#### Distribution

Even though we don't have centralized registry, we might later implement a centralized search for all the public bundles as they are mostly stored in publicly indexable Git repositories on GitHub. We have to reserve the option for now that in that case the user can reference the bundle just by its name (it is a "well-known bundle").

## Implementation details

### CLI commands

augent install [--for <agent>...] [--frozen] <source>

- adds bundle to augent.yaml and augent.lock
- updates augent.workspace.yaml (per --for <platform>, otherwise detects all used AI coding platforms by checking for platform-specific directories like `.opencode/`, `.cursor/`, `.claude/`, etc. and targets those)
- installs bundles resources per AI coding platform in format that platform expects

augent uninstall <bundle-name>

- removes files from all agents it is installed in, but only removes files that are not currently provided/overridden by later bundles (to avoid removing files that belong to other bundles)
- updates augent.workspace.yaml
- removes bundle from augent.yaml and augent.lock

augent list - list installed bundles (name, source URL, enabled agents, file count)

augent show <name> - show information for a bundle (all metadata from augent.yaml, file list, installation status per agent)

augent help - show all available commands with brief descriptions (entire help must fit on one screen)
augent version - show version number, build info, and Rust version

### Error Handling

- When an error occurs (invalid source URL, repository doesn't exist, failed to resolve dependencies, etc.), augent shows a clear, human-readable error message and exits with a non-zero status code.
- No changes are made to the workspace when an error occurs - all operations are atomic and rollback any partial changes.
- This ensures the workspace is never left in an inconsistent state due to a failed operation.
- If configuration files (`augent.yaml`, `augent.lock`, `augent.workspace.yaml`) are corrupted or invalid, augent shows a clear error message indicating which file is invalid and what the problem is, then exits with a non-zero status code. No automatic fixes or restorations are performed.

### CI/CD Environments

- augent works naturally in CI/CD environments without special flags or configuration.
- For CI/CD workflows, use the `--frozen` flag to ensure reproducibility and fail fast if lockfiles are out of date.
- Git authentication in CI works the same as interactive development - configure SSH keys or tokens in the CI environment's git credential helper, and augent will use them automatically.

### files

## Workspace root structure

.augent/
--- augent.yaml - bundle config
--- augent.lock - bundles with resolved sources and included resources
--- augent.workspace.yaml - workspace config for per agent
--- bundles/
    |_ my-debug-bundle/
    |_ code-documentation/

## Global cache (shared across all workspaces)

~/.cache/augent/
--- bundles/
    |_ <url-path-slug>/
        |_ <git-sha>/

## AI coding platform directories (each platform expects its own directory structure)

.opencode/
--- commands/
--- rules/
--- skills/

.cursor/
--- rules/

.claude/
--- .clinerules
--- workspace/

## augent.yaml

This is the main config file for the bundle. It is optional, but if bundle has dependencies, it must be present.

```yaml
name: "@author/my-bundle"

bundles:
  - name: my-debug-bundle
    path: bundles/my-debug-bundle

  - name: code-documentation
    path: plugins/code-documentation
    git: https://github.com/wshobson/agents.git
    ref: main
```

### augent.lock

This is the lockfile which has resolved sources (like git repository URL and SHA) and included resources (like commands, rules, skills, etc.). Claude Code plugins will be handled as git repositories (with subdirectories) so source type is essentially always either `dir` (for bundles in the current repo) or `git` (for remote bundles or converted from Claude Code plugins).

```json
{
  "name": "@author/my-bundle",
  "bundles": [
    {
      "name": "my-debug-bundle",
      "source": {
        "type": "dir",
        "path": ".augent/bundles/my-debug-bundle",
        "hash": "blake3:abc123..."
      },
      "files": [
        "commands/debug.md"
      ]
    },
    {
      "name": "code-documentation",
      "source": {
        "type": "git",
        "url": "https://github.com/wshobson/agents.git",
        "ref": "main",
        "sha": "abc123def456",
        "path": "plugins/code-documentation",
        "hash": "blake3:abc456..."
      },
      "files": [
        "agents/code-reviewer.md",
        "agents/docs-architect.md",
        "agents/tutorial-engineer.md",
        "commands/code-explain.md",
        "commands/doc-generate.md"
      ]
    },
    {
      "name": "my-bundle",
      "source": {
        "type": "dir",
        "path": ".",
        "hash": "blake3:abc123..."
      },
      "files": [
        "commands/debug.md"
      ]
    }
  ]
}

```

### augent.workspace.yaml

This file tracks what files are installed from which bundles to which AI coding platforms. This file answers the question, where did e.g. `.cursor/rules/debug.mdc` came from? Do note that one platform specific resource can be only be installed from one bundle at the time, so in the end that file comes from platform independent format from this bundle, or its last dependency where that file is available. When a file is modified even if it came from dependency, that file is copied to the bundle's directory in platform independent format on `install`. This way, `install` does not override local changes either.

There may be option to disable some agents temporarily in the future so that is why `enabled` is used.

```yaml
name: "@author/my-bundle"

bundles:
  - name: my-debug-bundle
    enabled:
      commands/debug.md:
        - .opencode/commands/debug.md
        - .cursor/rules/debug.mdc

  - name: code-documentation
    enabled:
      agents/code-reviewer.md:
        - .opencode/agent/code-reviewer.md
      agents/docs-architect.md:
        - .opencode/agent/docs-architect.md
      agents/tutorial-engineer.md:
        - .opencode/agent/tutorial-engineer.md
      commands/code-explain.md:
        - .opencode/command/code-explain.md
      commands/doc-generate.md:
        - .opencode/command/doc-generate.md
```
