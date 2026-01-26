# Uninstall with Dependencies

## Overview

When uninstalling a bundle from an Augent workspace, the `augent uninstall` command will automatically uninstall any transitive dependencies that are no longer needed by other bundles.

This document describes:

- How dependency detection works
- When dependencies are removed
- When dependencies are preserved
- Important gotchas and edge cases

## How It Works

### Dependency Detection

The uninstall command uses the **lockfile installation order** to detect dependencies:

1. **Lockfile Order**: Bundles are stored in the lockfile in dependency order. Dependencies come BEFORE the bundles that depend on them.
   - Example: If Bundle A depends on Bundle B, the lockfile order is: `[B, A]`
   - For multi-level dependencies: `[C, B, A]` means A→B→C

2. **Detection Algorithm**: When uninstalling a bundle, the command:
   - Identifies which bundles come BEFORE it in the lockfile
   - Checks if any remaining (non-uninstalled) bundle needs them
   - If no remaining bundle needs a dependency, marks it for removal
   - Repeats until no more dependencies can be removed

### File Removal Safety

When removing a bundle's files, the command ensures:

1. **Only applicable files are removed**: Uses `determine_files_to_remove()` to check:
   - Is this bundle the only provider of this file?
   - OR are all other providers of this file being removed?
   - OR are all other providers earlier in the lockfile (thus overridden)?

2. **Platform-specific handling**: Files are only removed from platforms where the bundle was installed:
   - Uses `workspace_config.get_locations()` to find platform-specific paths
   - Example: A `.cursor/` file is only removed from the `.cursor/` directory
   - Never removes from platforms where the bundle wasn't installed

3. **Atomic operations**: All file operations are tracked and can be rolled back on failure

## When Dependencies Are Removed

Dependencies are automatically removed when:

### ✅ All Dependents Are Being Uninstalled

```yaml
# augent.yaml
bundles:
  - name: bundle-a   # Direct install

# augent.lock (order)
# [bundle-c, bundle-b, bundle-a]
# where: a→b→c
```

**Action**: Uninstall bundle-a
**Result**: bundle-b and bundle-c are also removed ✓

### ✅ Only Selected Dependents Are Being Removed

```yaml
# augent.yaml
bundles:
  - name: bundle-a
  - name: bundle-x

# augent.lock (order)
# [bundle-c, bundle-b, bundle-a, bundle-x]
# where: a→b→c, x has no deps
```

**Action**: Uninstall bundle-a
**Result**:

- bundle-b and bundle-c are removed ✓
- bundle-x remains (it has no dependents)

### ✅ Dependency Is Needed by Another Bundle

```yaml
# augent.yaml
bundles:
  - name: bundle-a
  - name: bundle-b

# augent.lock (order)
# [bundle-c, bundle-a, bundle-b]
# where: a→c, b→c (shared dependency)
```

**Action**: Uninstall bundle-a
**Result**:

- bundle-a is removed ✓
- bundle-c is preserved (bundle-b still needs it) ✓

## When Dependencies Are Preserved

Dependencies are preserved when:

### ❌ Another Non-Removed Bundle Depends on Them

As shown above, if bundle-a and bundle-b share a common dependency (bundle-c), and you uninstall bundle-a, then bundle-c is preserved because bundle-b still needs it.

### ❌ The Dependency Appears Later in Lockfile

In rare cases, if a bundle appears LATER in the lockfile than a remaining bundle, the dependency is preserved because it may be an override or optional component.

## Important Gotchas

### 🚨 Gotcha 1: Transitive Dependencies NOT in augent.yaml

**Issue**: When you install a bundle with dependencies, only the root bundle appears in `augent.yaml`, NOT the transitive dependencies.

```yaml
# augent.yaml after: augent install ./bundles/bundle-a (where a→b→c)
bundles:
  - name: bundle-a  # ← Only this appears!
  # bundle-b and bundle-c are NOT here
```

**Why**: The workspace's `augent.yaml` only declares direct/root bundles. Transitive dependencies are automatically managed through their parent bundle's declarations in the dependency tree.

**Impact**:

- You cannot directly reinstall bundle-b or bundle-c by editing `augent.yaml`
- They are managed implicitly through their parent bundles
- This is by design - the workspace config should only declare what YOU explicitly installed

### 🚨 Gotcha 2: Uninstall Order Matters

When you uninstall bundle-a which depends on bundle-b, the bundles are removed in **reverse topological order**:

1. First bundle-a is uninstalled
2. Then bundle-b is uninstalled

If bundle-a has files that override bundle-b's files, those overrides are preserved in the correct order during removal.

**Example**:

```text
# Installation order: [b, a]  (b is installed first)
# File conflicts: both b and a provide config.json
# a's config.json overrides b's

# During uninstall:
1. a's files (including config.json override) are removed
2. b's files are removed
3. Result: workspace is clean, no leftover overrides
```

### 🚨 Gotcha 3: No "Dry Run" - Changes Are Immediate

The `augent uninstall` command makes changes immediately:

- Files are deleted
- Configuration files are updated
- No preview or confirmation (except `-y` flag for the initial prompt)

**Mitigation**: Use git to version control your workspace:

```bash
git status                           # See what will change
augent uninstall bundle-a -y         # Make changes
git diff .augent/                    # See what changed
# To revert: manually restore .augent/ from your editor or git history
```

### 🚨 Gotcha 4: Modified Files Are Preserved

If you modified a file that a bundle provides, the file is **NOT removed** during uninstall.

**Example**:

```text
# bundle-a provides: .cursor/config.json
# You modify: .cursor/config.json locally

augent uninstall bundle-a -y
# Result: .cursor/config.json is PRESERVED (not deleted!)
# You need to manually clean it up
```

**Why**: This prevents data loss from accidentally removing your customizations.

**Mitigation**: Check for modified files before uninstalling:

```bash
augent show bundle-a  # See what files it provides
git status .cursor/   # Check what you modified
```

### 🚨 Gotcha 5: Platform-Specific Files

The command only removes files from platforms where the bundle was installed.

**Example**:

```yaml
# workspace.yaml after installing bundle-a for claude and cursor
bundles:
  - name: bundle-a
    enabled:
      commands/cmd.md:
        - .claude/commands/cmd.md
        - .cursor/commands/cmd.md

# Uninstall bundle-a
# Result: cmd.md is removed from BOTH .claude/ and .cursor/
```

**If bundle-a was only installed for claude** (`--for claude`):

```yaml
bundles:
  - name: bundle-a
    enabled:
      commands/cmd.md:
        - .claude/commands/cmd.md

# Uninstall bundle-a
# Result: cmd.md is removed from .claude/ only
#         .cursor/commands/cmd.md is untouched (it came from another bundle)
```

### 🚨 Gotcha 6: File Overrides and Precedence

When multiple bundles provide the same file, only the LATEST one in lockfile order is actually used.

**Example**:

```text
# Lockfile order: [b, a]  (a is later, so a's files override b's)
# Both provide config.json
# User sees: a's version of config.json

augent uninstall bundle-a -y
# Result: a's config.json is removed
#         b's config.json is now revealed!
# User now sees: b's version of config.json
```

**Impact**: Uninstalling a bundle may cause a "fallback" to an earlier bundle's version of the same file. This is correct behavior but can be surprising.

**Mitigation**: Be aware of what other bundles provide the same files:

```bash
augent list --detailed  # See all bundles and their files
# or check augent.lock manually
```

### 🚨 Gotcha 7: Circular Dependencies Are Not Allowed

The resolver prevents circular dependencies, but if somehow a cycle exists, it will be detected and uninstall will fail.

**Example** (should never happen):

```text
bundle-a → bundle-b → bundle-a  # ❌ Circular!
```

**Prevention**: The resolver checks for cycles during install, so this cannot occur under normal circumstances.

## Configuration Files Impact

### augent.yaml (Workspace bundle config)

- Root bundles are removed from `bundles:` list
- Transitive dependencies are already NOT in this file
- After uninstall, the direct install declaration is gone

### augent.lock (Resolved lockfile)

- All uninstalled bundles (direct + transitive) are removed
- Remaining bundles keep their resolved SHAs
- Lockfile is reorganized to maintain correct order

### augent.workspace.yaml (File locations)

- All entries for uninstalled bundles are removed
- File location mappings are cleaned up
- Empty platform directories (`.cursor/`, `.claude/`, etc.) may be removed if empty

## Best Practices

### ✅ DO: Use Version Control

```bash
git status                    # Before uninstall
augent uninstall bundle-a -y
git diff .augent/             # After uninstall
git add .augent/
git commit -m "Remove bundle-a and dependencies"
```

### ✅ DO: Check What Will Be Removed

```bash
augent show bundle-a          # See what files it provides
augent list                   # See all installed bundles
# Then decide if uninstall will cause issues
```

### ✅ DO: Understand Your Dependency Tree

Keep a mental model (or document) of:

- Which bundles depend on which
- Which bundles provide conflicting files
- Which bundles you installed directly vs. came as dependencies

### ❌ DON'T: Uninstall Without Understanding Dependencies

```bash
augent uninstall bundle-x -y  # ❌ Risky if you don't know what depends on it!
```

### ❌ DON'T: Manually Edit augent.lock

The lockfile is generated automatically. Manually editing it can cause inconsistencies.

### ❌ DON'T: Assume Uninstall Cleans Everything

Some files may remain (modified files, platform-specific files from other sources, etc.).

After uninstall, it's good practice to:

```bash
git status                     # Check for leftover files
git clean -fd                  # Remove untracked files if desired
```

## Rollback on Failure

If an error occurs during uninstall:

1. **Transaction system tracks changes**: A backup of configuration files is made before uninstall
2. **Automatic rollback**: If any error occurs, the backup is restored
3. **Manual recovery**: If rollback fails, restore the config files from your editor or from a previous git commit (e.g. `git show HEAD:.augent/augent.yaml` and write the output back to the files).

## Examples

### Example 1: Simple Chain Uninstall

```bash
# Installed: augent install ./bundles/bundle-a
# Structure: a → b → c

$ augent list
Installed bundles (3):
  - bundle-a
  - bundle-b
  - bundle-c

$ augent uninstall bundle-a -y
Uninstalling 2 dependent bundle(s) that are no longer needed:
  - bundle-b
  - bundle-c
Uninstalling bundle: bundle-a
Uninstalling bundle: bundle-b
Uninstalling bundle: bundle-c

$ augent list
No bundles installed.
```

### Example 2: Shared Dependency Preserved

```bash
# Installed:
#   augent install ./bundles/bundle-a  (depends on c)
#   augent install ./bundles/bundle-b  (also depends on c)
# Structure: c is shared, a→c, b→c

$ augent list
Installed bundles (3):
  - bundle-c
  - bundle-a
  - bundle-b

$ augent uninstall bundle-a -y
Uninstalling bundle: bundle-a
Bundle 'bundle-a' uninstalled successfully

$ augent list
Installed bundles (2):
  - bundle-c  # ← Preserved because bundle-b needs it!
  - bundle-b
```

### Example 3: Platform-Specific Installation

```bash
# Installed: augent install ./bundles/bundle-a --for claude
# Files are only in .claude/ directory

$ augent uninstall bundle-a -y
Uninstalling bundle: bundle-a
Removed 3 file(s) from .claude/

$ ls -la .cursor/
# ← Untouched (bundle wasn't installed for cursor)
```

## Related Documentation

- [Install Command](./install-command.md) - Understanding how dependencies are resolved during install
- [Architecture](../architecture.md) - How the system tracks dependencies
- [Bundle Format](../../bundles.md) - How to declare dependencies in augent.yaml
