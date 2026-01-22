# Product Requirements Document

## Goal

### What are we solving

As of today (2026-01-22), there are many AI agents for developers to choose from, and more is expected to come. Some popular ones include OpenCode, Claude Code, Cursor, Codex CLI and GitHub Copilot. Many developers increasingly use more than one AI agent in the project scope.

There is no well-established process to manage these AI coding agents' resources across different agents or projects.

The resources are key assets for AI driven development process such as commands, rules, subagents, skills and MCP servers. Many of developers rely on maintaining their own set of resources and copy them across projects.

### What do we have today

As of 2026-01-22, the most promising solution to resolve these issues in an agent-independent manner is [OpenPackage](https://github.com/enulus/OpenPackage). The project is not fully matured yet (it has a central registry but does not share code for that registry) and lacks accelerating adoption even though the problem itself has been widely acknowledged and is pending solution.

OpenPackage also has a few design flaws due to it stemming from being designed on how other package managers (like npm) work, and not aknowledging that managing AI agent resources is whole lot of different from managing software packages.

Some examples of early design flaws include implementing development dependencies without careful thought when they are installed. Overall, development dependencies has little recognized use cases yet in managing resources for AI agents, and it mostly confuses what gets installed and when.

On the other hand, it lacks useful features from real package managers like dependency locking. This is mandatory requirement when using version ranges in dependencies to ensure reproducibility in the first place across team members.

From the adoption point of view, likely its biggest issue is that is not designed with simplicity in mind from the ground up. Already there are too many commands. A few commands like `install` and `apply` are too ambiguous to be understood by their name, it is not clear what `save` does, what `add` adds and where `publish` puts the packages etc. without careful reading of documentation.

### How we will resolve it

We will now implement an AI configuration manager (not package manager) supporting various AI agents and relying on OpenPackage's quite ok support for various AI agents (talking on them as "platforms")

We will do this in a development-friendly manner. This means that it is not only easy, BUT OBVIOUS, to use for anyone who has used any package manager before in any programming language, and not only that but it is actually far simpler than that.

This means **we will not cargo cult any existing packager manager and we will not implement ANY bells and whistles than what is required to active our goal**.

Our goal is:

1) Implement AI coding agent and platform independent resource management,
2) in a lean, intuitive, developer friendly, non-documentation relying way,
3) with easy extensibility without code changes

To respond to fast evolving landscape of AI coding agents and their features.

### What will 1.0 look like

#### Terminology

**Workspace** - working copy of git repository at hand on developer's machine

**Workspace_root** - the root directory of the workspace (usually where `.git` directory is and where AI coding agent specific directories are stored)

**Bundle** - a directory with AI coding agent independent augs (either in its root or in subdirectories) and optionally Augment bundle config files regarding the bundle (its dependencies, its name, optional metadata).

**Bundle config files** - `augent.yaml`, `augent.lock` and `augent.workspace.yaml` files.

**Aug** - a file that is provided by a bundle in an AI coding agent **independent** format (e.g. rule `<bundle_dir>/rules/debug.md` or mcp server `<bundle_dir>/mcp.jsonc`)

***Bundle root file/dir*** - a file or directory that is provided by a bundle (`<bundle_dir>/root/file.md` or `<bundle_dir>/root/dir`) and is copied **as is** in the workspace root directory when the bundle is installled.

**Resource** -

*- `*resource` - a file that is provided by a bundle in an AI coding agent independent format (e.g. `<bundle_dir>rules/debug.md`)
- `augmentation` - a resource that has been installed by a bundle for a specific AI coding agent in its own format (e.g. `<workspaec_root_dir>/cursor/rules/debug.mdc`)

#### What we will have

- A platform-independent command line tool written in Rust, named Augent (the binary is `augent`)

- It works on MacOS, Linux and Windows. For two latter, it works on ARM64 and x86_64 architectures.

- It implements as few commands (`install`, `uninstall`, `list`, `show`) as possible.

- The app knows all the AI agent formats that OpenPackage currently does and can convert from AI agent independent resources to agent specific resources and back.

- All matured CLI tool practices and Rust development best practices are followed.

#### What we will not have

- Development dependencies (like npm or OpenPackage has) or cargo-culting any existing package managers for "nice to have" features that "might be useful in the future" (it is ABSOLUTELY TOO HARD to deprecate almost any of features in package manager)

- A centralized registry for publishing and distributing packages. We will distribute bundles in a single Git repository, in a clear text format, as that is what the developers want to go read in GitHub before taking a bundle into use

- We will not install bundles (outside of the current repository) anywhere else than from git repositories via https:// (or via ssh:// for private repositories). It has to support other sources than GitHub even though GitHub is likely the most popular for public bundles. We have to keep in mind that some organizations will have their own set of private bundles (git repositoriies).

### Type 1 decisions

Note: These fundamental decisions cannot be reversed so it is ABSOTELY REQUIRED to get them right from the beginning or not implement them at all.

#### Package format

- We implement a `bundle` (a lightweight package concept) as a directory in filesystem.

- The bundle is a directory in filesystem.

- Bundle is "published" to outer world via a Git repository.

- This git repository can be essentially anywhere (even in the same filesystem)
and we should not limit our implementation to only GitHub, GitLab, etc..

- Bundle can define dependencies to other bundles via `augent.yaml` file, but presence of that file is optional.

- Bundles are installed in order they are presented in `augent.yaml` so later bundles override the augs from earlier bundles where the aug file names overlap.

- Some resources are merged with the existing ones if they already exist for the agent, e.g. AGENTS.md and mcp.jsonc files, see OpenPackage's implementation for the reference.

- Augment will know how to install the directory's content for AI coding agents regardless of whether `augment.yaml`, `augent.lock`, `augent.workspace.yaml` is present in the directory. This ensures we maintain compatibility with Claude Code plugins, skills only repos, etc.

- If a dependency is installed in the workspace, then `augent.lock` if the same directory as `augent.yaml` has an entry for it. It does not necessary mean that that dependency's provided files are installed for all or even any of the agents. What is installed per agent (and "where did the resources come from" is tracked in `augent.workspace.yaml`)

#### Locking

- If bundle has `augent.yaml` and `install` has been run, it also has `augent.lock`. Install takes care of updating the lockfile unless `--frozen` is used. If `--frozen` is used, then it fails if the lockfile is missing or it does not match `augent.yaml`.

- If bundle has the lockfile, the bundle's lockfile is read when installing a bundle as a dependency. The bundle's lockfile contains the name of the bundle and its checksum.

- In lockfile, all sources have been resolved TO BE EXACT, that is anything from github uses exact URL and SHA for a git repository. It may have org/user and ref info present but that is not used for resolving.

- Lockfile lists all the files that are provided by the bundle, NOT WHAT is necessarily installed by the bundle. This means, providing you install for all AI agents, you should be able to track where the resources came from by starting from the end of the lockfile.

- The last entry in the lockfile is the bundle itself. Thus the bundles own augs (in the bundle's dir) always override the resources from earlier dependencies if the file names overlap, except where resources are merged.

- Each bundle in lockfile has a calculated `hash` per its contents (all files, including `augent.yaml`, but excluding its `augent.lock` and `augent.workspace.yaml` files). Also the last entry, this bundle.

#### Workspace

- If `augent.yaml` is created on first install, then the bundle is named `@author/bundle-name`. This is intended to be equal to the git repository and organization under which is stored. You can infer both from the git repostitory remote URL, if not use `USERNAME/WORKSPACE_ROOT_DIR_NAME` as fallback.

- The user can change one AI agents' files directly in repo (e.g. `.opencode/commands/debug.md` and `install` will detect that these are different from the bundle they came from and copies them to the bundle's directory as AI agent independent resources)

- The mapping for bundle's aug files and ai agent's resources is stored in `augent.workspace.yaml`. The format is file for each file that partiuclar bundle provides, and for each AI agent that it is installed for (user can use 0`install --for <agent>...` to install some bundles only for specific agents).

#### Sources

- Other directories in the current repo (reference by a relative path)

- From Git repositories, optionally the particular ref|branch|tag, and from the particular subdirectory of the repository. Git repositories can store multiple bundles in different subdirectories.

- Any of sources may not have `augent.yaml` and `augent.lock` files present and it will work - just then it will not install any other bundles as dependencies.

### Type 2 decisions (how we make it future proof)

#### Other sources

For 1.0.0, we should be able to install resources from:

- From Git repositories which store Claude Code plugin

- From Git repositories which store Claude Code marketplace (multiple Claude Code plugins)

Basically, the user can give any path or repo url or github:author/reponame to `install` and it will know how to handle it (as long as it has resources in its path or in its subdirectories). If there are multiple set of resources in the repo (e.g. aforementioned bundles, or multiple Claude Code plugins), it will show you the menu and ask you to select the ones you want to install.

#### Other AI agents

We adopt `platforms.jsonc` approach from OpenPackage to support ever increasing number of AI agents and their features. It must be possible for the developer to add support to new AI agents without changing the core code.

If another package format becomes popular, we will collaborate to add support to import from it, but we will not compromise our goal to implement some features just for the sake of something being popular.

#### Distribution

Even though we don't have centralized registry, we might later implement a centralized search for all the public bundles as they are mostly stored in publicly indexable Git repositories on GitHub. We have to reserve the option for now that in it that case the user can reference the bundle just by its
name (it is a "well-known bundle")

## Implementation details

### CLI commands

opkg install [--for <agent>...] [--frozen] <source>
- adds bundle to augent.yaml and augent.lock
- updates augent.workspace.yaml (per --for <agent>, otherwise detects all used AI agents in repo and targets those)
- installs bundles resources per AI agent in format that agent expects

opkg uninstall
- removes files from all agents is installed in
- updates augent.workspace.yaml
- removes bundle from augent.yaml and augent.lock

opkg list - list installed bundles

opkg show <name> - show information for a bundle

opkg help
opkg version

### files

.augent/
--- bundles
    |_ augent.yaml - bundle config
    |_ augent.lock - bundles with resolved sources and included resources
    |_ augent.workspace.yaml - workspace config for per agent
    |_ bundles/
--- resources
    |_ agents/commands/rules/skills
    |_ AGENTS.md
    |_ mcp.jsonc
    |_ root/

### augent.yaml

This is the main config file for the bundle. It is optional, but if bundle has dependencies, it must be present.

```yaml
name: "@author/my-bundle"

bundles:
  - name: my-debug-bundle
    subdirectory: bundles/my-debug-bundle

  - name: code-documentation
    subdirectory: plugins/code-documentation
    git: https://github.com/wshobson/agents.git
    ref: main
```

### augent.lock

This is the lockifle which has resolved sources (like git repository URL and SHA) and included resources (like commands, rules, skills, etc.). Claude Code plugins will be handled as git repositories (with subdirectories) so source type is essentially always either `dir` (for bundles in the current repo) or `git` (for remote bundles or converted from Claude Code plugins).


```json
{
  "name": "@author/my-bundle",
  "bundles": [
    {
      "name": "my-debug-bundle",
      "source": {
        "type": "dir",
        "path": ".augent/bundles/my-debug-bundle",
        "hash": "sha256:abc123..."
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
        "hash": "sha256:abc456..."
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
        "hash": "sha256:abc123..."
      },
      "files": [
        "commands/debug.md"
      ]
    }
  ]
}

```

### augent.workspace.yaml

This file tracks what files are installed from which bundles to which AI agents. This file answers the question, where did e.g. `cursor/rules/debug.mdc` came from? Do note that one agent specific resource can be only be instaled from one bundle at the time, so in the end that file comes from AI agent independent format from this bundle, or its last dependency where that file is available. When a file is modified even if it came from dependency, that file is copied to the bundle's directory in agent independent format on `install`. This way, `install` does not override local changes either.

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
