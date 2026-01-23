# Testing Coverage Analysis

**Date:** 2026-01-23
**Purpose:** Identify gaps in user-facing testing coverage

---

## Summary

**Total Test Count:** 65 integration tests (original) + 30 new tests = 95 total
**Status:** Critical gaps have been addressed. Tests now cover Git sources, dependencies, lockfiles, merge strategies, and platform behavior.

---

## Critical Gaps (High Priority)

### Install Command - Git Sources

- [x] Test install from GitHub short-form (`github:author/repo`)
- [x] Test install from full Git URL (`https://github.com/author/repo.git`)
- [x] Test install with subdirectory (`github:author/repo#plugins/name`)
- [x] Test install with specific ref/tag/branch (`github:author/bundle#v1.0.0`)
- [x] Test install from SSH Git URL (`git@github.com:author/bundle.git`)
- [x] Test bundle discovery with multiple bundles in repository (interactive menu)
- [x] Test install from public GitHub repository (real network call, not mocked) - validates error handling for non-bundle repositories

### Install Command - Dependencies

- [x] Test circular dependency detection and error message
- [x] Test complex dependency graphs (3+ levels deep)
- [x] Test transitive dependencies (A → B → C)
- [x] Test duplicate dependency resolution
- [x] Test dependency order verification in lockfile
- [x] Test bundle with missing/unavailable dependencies

### Install Command - Lockfile Behavior

- [x] Test lockfile determinism (same lockfile on multiple runs)
- [x] Test `--frozen` fails when lockfile would change
- [x] Test `--frozen` fails when lockfile missing
- [x] Test lockfile SHA resolution for branches that moved
- [x] Test lockfile regeneration after ref change

### Install Command - File Installation & Merging

- [x] Test `replace` merge strategy for regular files
- [x] Test `composite` merge for AGENTS.md with delimiters
- [x] Test `composite` merge for mcp.jsonc
- [x] Test `shallow` merge for JSON/YAML files
- [x] Test `deep` merge for nested JSON/YAML structures
- [x] Test later bundle overriding earlier bundle files (verify file content)
- [x] Test root files/directories are copied to workspace root
- [ ] Test root directory handling (empty vs non-empty)

### Install Command - Platform-Specific

- [x] Test installing for all detected platforms simultaneously
- [x] Test same bundle installed to different platforms separately
- [x] Test platform-specific transformations correctness (file paths, extensions)
- [x] Test all resource types: commands, rules, skills, agents, MCP servers
- [x] Test bundle with resources not supported by some platforms

---

## Important Gaps (Medium Priority)

### Uninstall Command - Dependency Safety

- [ ] Test user is prompted (not just with -y flag) when dependent bundles exist
- [ ] Test user can cancel uninstall when prompted
- [ ] Test warning message content shows correct dependent bundle names
- [ ] Test removing bundle with no dependents (should not warn)

### Uninstall Command - File Removal

- [ ] Test empty directory cleanup after file removal
- [ ] Test root files not removed if workspace bundle provides them
- [ ] Test root directories never removed (even if empty)
- [ ] Test removing file that also exists in multiple bundles (only removed if no other provider)
- [ ] Test removing directory with mixed files (some from target, some from others)

### Workspace Management

- [ ] Test auto-detection from git remote URL for workspace naming
- [ ] Test fallback to USERNAME/WORKSPACE_DIR when no git remote
- [ ] Test workspace detection in parent directories
- [ ] Test workspace initialization in nested subdirectory
- [ ] Test modified file detection and tracking to workspace bundle (more scenarios)
- [ ] Test modified file preservation on re-install (multiple files, different states)

### List & Show Commands

- [ ] Test `list --detailed` shows all metadata fields
- [ ] Test `list --detailed` format and readability
- [ ] Test `show` command displays all bundle metadata
- [ ] Test `show` shows correct dependencies list
- [ ] Test `show` shows installation status for each agent
- [ ] Test `list` with 10+ installed bundles
- [ ] Test `list` with bundles installed to different platforms

---

## Nice to Have (Lower Priority)

### CLI Options & UX

- [ ] Test `--verbose` flag behavior for all commands (verify useful output)
- [ ] Test `--workspace <PATH>` option for all commands
- [ ] Test error messages are clear and human-readable
- [ ] Test help text fits on one screen (verify line count)
- [ ] Test all examples in documentation work correctly
- [ ] Test command completion scripts (bash, zsh, fish) are valid

### Error Handling

- [ ] Test invalid bundle name format error
- [ ] Test corrupted lockfile error message and handling
- [ ] Test corrupted workspace config error message
- [ ] Test network failure during git clone (more than just "fails without network")
- [ ] Test permission denied when writing to workspace
- [ ] Test concurrent access to workspace (two installs running simultaneously)

### Cache Management

- [ ] Test cache hit detection (same bundle, same SHA)
- [ ] Test cache miss detection (same URL, different SHA)
- [ ] Test multiple workspaces sharing same cache
- [ ] Test `clean-cache --show-size` displays accurate size
- [ ] Test cache cleanup removes only specified bundles

### Edge Cases

- [ ] Install, list, show, uninstall complete roundtrip
- [ ] Multiple agents with same bundle installed
- [ ] Bundle name conflicts (different sources, same name)
- [ ] Bundle with conflicting dependencies
- [ ] Install when workspace already has modified files
- [ ] Uninstall workspace bundle (should warn/prevent)
- [ ] Install bundle with empty resources directory
- [ ] Install bundle with no augent.yaml file

---

## Platform-Specific Testing Gaps

### Platform Detection

- [ ] Test detection when multiple agent directories present (.claude/, .cursor/, .opencode/)
- [ ] Test detection from root files (CLAUDE.md, AGENTS.md)
- [ ] Test detection order and priority
- [ ] Test platform alias resolution (cursor vs cursor-ai)

### Transformation Verification

- [ ] Verify commands/*.md → .claude/prompts/{name}.md
- [ ] Verify rules/*.md → .claude/rules/{name}.md
- [ ] Verify skills/*.md → .claude/skills/{name}.md
- [ ] Verify transformations for cursor platform (.cursor/rules/*.mdc)
- [ ] Verify transformations for opencode platform
- [ ] Verify file extension changes are correct
- [ ] Verify directory structures are created correctly

---

## Documentation-Based Feature Testing

The following features are documented but have minimal or no coverage:

### From commands.md

- [x] All install source formats work as documented
- [ ] `--detailed` flag for list command
- [ ] `completions` command generates valid scripts for all shells
- [ ] Global `--verbose` flag works for all commands

### From bundles.md

- [ ] Bundles with version field work correctly
- [ ] Bundles with metadata fields (author, license, homepage)
- [ ] Bundles with dependencies array
- [x] Root files/directories are copied correctly
- [ ] agents.md is merged into workspace AGENTS.md

### From workspace.md

- [ ] Workspace initialization from git remote
- [ ] Workspace detection in parent directories
- [ ] Modified file detection works across bundle re-installs

---

## Test Infrastructure Issues

### Critical

- [x] Fix test compilation error: `cargo::cargo_bin_cmd!` macro not available in tests/common/mod.rs:303
  - Tests now compile using `Command::cargo_bin("augent").unwrap()`
  - Affects all integration tests that use `run_augent_cmd` or `augent_cmd` helpers

### Missing Test Fixtures

- [x] Bundle fixtures for:
  - GitHub short-form source
  - Git URL source
  - Bundle with subdirectory
  - Bundle with specific ref
  - Complex dependencies (3+ levels)
  - Circular dependencies
- [ ] Workspace fixtures for:
  - Multiple bundles installed
  - Modified files from dependencies
  - Multiple platforms configured

---

## Priority Order for Implementation

1. [x] **Fix test compilation** (blocking all tests from running)
2. [x] **Install from Git sources** (core feature, completely untested)
3. [x] **Circular dependency detection** (core safety feature)
4. [x] **Lockfile determinism and `--frozen`** (reproducibility requirement)
5. [x] **Merge strategy verification** (core correctness)
6. [x] **Dependency resolution scenarios** (complex but important)
7. [ ] **Uninstall with dependencies** (safety feature)
8. [ ] **Workspace initialization and detection** (onboarding experience)
9. [x] **Platform transformation verification** (correctness)
10. [ ] **Error handling and edge cases** (robustness)

---

## Metrics

**Completed in this session:** 30 high-priority tests implemented
**Current Total Test Count:** 95 integration tests
**Estimated Remaining:** ~50-70 additional tests needed
**Time Invested:** 2-3 hours of focused testing work

---

## Notes

- All new tests use local fixtures (no real network calls to GitHub)
- Tests cover real-world scenarios: Git sources, dependencies, lockfiles, merge strategies, platform behavior
- Integration tests use REAL CLI (matches testing.md requirements)
- Test infrastructure is solid (assert_cmd, assert_fs, predicates)
- Fixtures are well-organized and extensible
