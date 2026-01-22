# Product Requirements Document

## What it solves

As of today (2026-01-22), there are many AI agents for developers to choose from and more is expected to come. Some popular ones include OpenCode, Claude Code, Cursor, Codex CLI and GitHub Copilot.

Increasingly a project uses more than one AI agent. There is no standard way to manage these agents' resources, such as commands, rules, subagents, skills and MCP servers. This applies not only accross agents but also across projects.

## Current state

Currently the most promising solution to resolve the issue [OpenPackage](https://github.com/enulus/OpenPackage).

It has some design flaws due to it mostly being designed on how other package managers (like npm) work. Managing AI agent resources is whole lot of different from managing software packages.

However, the biggest of its problems is that it is designed with simplicity in mind for developers. There are too many commands, some commands like `install` and `apply` are too ambiguous, it is not clear what `save` does and where `publish` puts the packages etc.

We will implement a configuration manager supporting various AI agents which is easy to use for anyone who has used a package manager before.

We will not cargo cult any other existing manager and we will not implement any other bells and whistles than what is needed to 1) implement AI agent resource management in 2) a reproducible and 3) AI agent independent way.

## Design principles

What we will have:

- A platform-independent command line tool written in Rust, named Augent (the binary is `augent`)
- It implements as few commands (`install`, `uninstall`, `list`, `show`) as possible to do the jobo, so it is 100% intuitive for developers to use if you have ever installed dependencies and this even without even looking at the documentation.
- All matured CLI tool practices and Rust development best practices are followed

What we will not have:

- Development dependencies (like npm or OpenPackage has) or cargo-culting any existing package managers for "nice to have" features that "might be useful in the future" (it is FUCKING HARD to deprecate any of features in package manager)
- A centralized registry for publishing and distributing packages. We will store our bundles (lightweight "packages") in a Git repo in a clear text format as that's what the developers want to go see in GitHub when they see the bundle name.
- Thus we will not install bundles (outside of the current repository) anywhere else than from git repositories via https:// (or via ssh:// for private repositories)

How we will do it:

- We implement a `bundle` (a lightweight package concept) as a directory in filesystem.
- If bundle has `augent.yaml` file, it also has `augent.lock`.
- Bundle can define dependencies to other bundles. If not, then `augent.yaml` and thus `augent.lock` are factually optional, but bundle should still work.
- If `augent.yaml` is created on first install, then the bundle is named `@author/bundle-name`. This is intended to be equal to the git repository and organization under which is stored. You can usually infer both from the git repostitory remote URL.
- If bundle has lockfile, the bundle's lockfile is read when installing a bundle as a dependency. The bundle's lockfile contains the exact version, the resolved source (that is the exact URL and SHA for a git repository). The bundle installation must be 00% reproducible only with the lockfile (if not taking in the account the agent specific enablements in `augent.workspace.yaml`).
- Bundle has a calculated `hash` per its contents (all files, including `augent.yaml` but excluding its lockfile, as the name of the related bundle and its hash is stored there)
- Bundles are installed in order they are presented in `augent.yaml` so later bundles override resources from earlier bundles if the file names overlap.
- Some resources are merged, e.g. AGENTS.md and mcp.jsonc files, see OpenPackage's implementation for the reference.
- Bundles own resources (under the bundle's dir) always override the resources from dependencies.
- Bundle is stored in Git repositories (either in this repo or in other repo).
- One git repo can store multiple bundles in different subdirectories.
- The app knows all the AI agent formats that OpenPackage currently does and can convert from AI agent independent resources to agent specific resources.
- The user can change one AI agents' files directly in repo (e.g. `.opencode/commands/debug.md` and `install` will detect that these are different from the bundle they came from and copies them to the bundle's directory as AI agent independent resources)
- This mapping is stored in `augent.workspace.yaml` file for each file that a bundle provides, for each AI agent that it is installed for (user can use `install --for <agent>...` to install some bundles only for specific agents).

Where can you install bundles from:

- Other directories in the current repo (reference by relative path)
- From Git repositories https://, ssh:// (optionally the particular ref|branch|tag)

These may or may not have `augent.yaml` and `augent.lock` files present and it will work, just not install any other bundles as dependencies.

In addition, you should be able to install resources from:

- From Git repositories which store Claude Code plugin
- From Git repositories which store Claude Code marketplace (multiple Claude Code plugins)

Basically, you can give any path or repo url or github:author/reponame to `install` and it will know how to handle it (as long as it has resources in its path or in its subdirectories). If there are multiple set of resources in the repo (e.g. aforementioned bundles, or multiple Claude Code plugins), it will show you the menu and ask you to select the ones you want to install.

Future note:

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
