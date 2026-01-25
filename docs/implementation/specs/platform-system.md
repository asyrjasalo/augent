# Feature: Platform System

## Status

[x] Complete

## Overview

The platform system enables Augent to support multiple AI coding platforms through declarative configuration. It defines transformation rules, detection patterns, and merge strategies for converting universal bundle resources to platform-specific formats.

## Requirements

From PRD:

- Support multiple AI coding platforms (Claude Code, Cursor, OpenCode) with platform-specific formats
- Auto-detect which AI coding platforms are present in workspace
- Transform universal resources to platform-specific paths
- Apply merge strategies when multiple bundles provide same resource
- Enable adding new platforms through configuration files (no code changes)
- Support platform selection via `--for` flag

## Design

### Interface

**Platform detection (automatic):**

```bash
augent install github:author/bundle  # Installs for all detected platforms
```

**Platform selection (manual):**

```bash
augent install github:author/bundle --for cursor opencode
```

**Custom platform configuration:**

```bash
# Create .augent/platforms.jsonc
# Defines custom platform transformation rules
```

### Implementation

#### 1. Platform Configuration

Platforms are defined in JSONC format:

```jsonc
{
  "platforms": [
    {
      "id": "claude",
      "name": "Claude Code",
      "directory": ".claude",
      "detection": [".claude", "CLAUDE.md"],
      "transforms": [...]
    }
  ]
}
```

**Data structures:**

```rust
pub struct PlatformConfig {
    pub platforms: Vec<Platform>,
}

pub struct Platform {
    pub id: String,
    pub name: String,
    pub directory: String,
    pub detection: Vec<String>,
    pub transforms: Vec<TransformRule>,
}

pub struct TransformRule {
    pub from: String,      // Glob pattern for universal path
    pub to: String,        // Glob pattern for platform-specific path
    pub merge: MergeStrategy,
    pub extension: Option<String>,
}
```

#### 2. Platform Detection

Auto-detection scans workspace for platform indicators:

```rust
fn detect_platforms(workspace_path: &Path, config: &PlatformConfig) -> Vec<Platform> {
    let mut detected = Vec::new();

    for platform in &config.platforms {
        for pattern in &platform.detection {
            let target = workspace_path.join(pattern);

            // Check if pattern is directory
            if target.is_dir() {
                detected.push(platform.clone());
                break;
            }

            // Check if pattern is file
            if target.is_file() {
                detected.push(platform.clone());
                break;
            }
        }
    }

    detected
}
```

**Detection patterns:**

- `.claude/` → Claude Code detected
- `CLAUDE.md` → Claude Code detected
- `.cursor/` → Cursor detected
- `.opencode/` → OpenCode detected
- `AGENTS.md` → Cursor or OpenCode detected

#### 3. Platform Resolution

Platform names are resolved from various inputs:

```rust
fn resolve_platforms(
    platform_names: Vec<String>,
    auto_detected: Vec<Platform>,
    config: &PlatformConfig,
) -> Result<Vec<Platform>> {
    let mut platforms = Vec::new();

    for name in platform_names {
        if let Some(platform) = config.platforms.iter().find(|p| p.id == name) {
            platforms.push(platform.clone());
        } else {
            return Err(Error::UnknownPlatform(name));
        }
    }

    // If no platforms specified, use auto-detected
    if platforms.is_empty() {
        if auto_detected.is_empty() {
            return Err(Error::NoPlatformDetected);
        }
        platforms = auto_detected;
    }

    Ok(platforms)
}
```

**Resolution order:**

1. Explicitly specified via `--for` flag
2. Auto-detected from workspace
3. Error if neither specified nor detected

#### 4. Transformation Engine

Transformation engine maps universal paths to platform-specific paths:

```rust
pub struct TransformEngine {
    rules: HashMap<String, Vec<TransformRule>>, // Key: platform ID
}

impl TransformEngine {
    pub fn transform(&self, resource_path: &Path, platform: &Platform) -> Result<TransformedPath> {
        // Find matching transformation rule
        let rule = self.find_rule(resource_path, platform)?;

        // Extract name from source path
        let name = self.extract_name(resource_path, &rule.from)?;

        // Apply transformation template
        let target = rule.to.replace("{name}", &name);

        // Apply extension transformation
        let target = self.apply_extension(&target, &rule)?;

        // Construct full path
        let full_path = PathBuf::from(&platform.directory).join(&target);

        Ok(TransformedPath {
            path: full_path,
            merge_strategy: rule.merge,
        })
    }

    fn find_rule(&self, resource_path: &Path, platform: &Platform) -> Result<&TransformRule> {
        let rules = self.rules.get(&platform.id)
            .ok_or_else(|| Error::PlatformNotFound(platform.id.clone()))?;

        for rule in rules {
            if self.matches_pattern(resource_path, &rule.from) {
                return Ok(rule);
            }
        }

        Err(Error::NoTransformRule(resource_path.display().to_string()))
    }

    fn matches_pattern(&self, path: &Path, pattern: &str) -> bool {
        // Convert glob pattern to regex
        let regex = glob_to_regex(pattern);
        regex.is_match(&path.to_string_lossy())
    }
}
```

**Transformation examples:**

| Universal Path | Platform | Target Path |
|---------------|----------|-------------|
| `commands/deploy.md` | Claude | `.claude/prompts/deploy.md` |
| `rules/debug.md` | Cursor | `.cursor/rules/debug.mdc` |
| `skills/analyze.md` | OpenCode | `.opencode/skills/analyze.md` |
| `mcp.jsonc` | Any | `.platform/mcp.json` |

#### 5. Merge Strategies

Multiple strategies for handling conflicts:

```rust
pub enum MergeStrategy {
    Replace,    // Overwrite completely
    Shallow,     // Merge top-level keys
    Deep,        // Recursively merge nested
    Composite,   // Merge with delimiters
}

pub fn merge_files(
    existing: &str,
    new_content: &str,
    strategy: &MergeStrategy,
    bundle_name: &str,
) -> Result<String> {
    match strategy {
        MergeStrategy::Replace => Ok(new_content.to_string()),
        MergeStrategy::Shallow => shallow_merge(existing, new_content),
        MergeStrategy::Deep => deep_merge(existing, new_content),
        MergeStrategy::Composite => composite_merge(existing, new_content, bundle_name),
    }
}
```

**Composite merge example:**

```rust
fn composite_merge(existing: &str, new_content: &str, bundle_name: &str) -> String {
    let start_delim = format!("<!-- BEGIN: {} -->", bundle_name);
    let end_delim = format!("<!-- END: {} -->", bundle_name);

    // Remove old content from bundle if exists
    let cleaned = remove_delimited_content(existing, &start_delim, &end_delim);

    // Append new content with delimiters
    format!("{}\n{}\n{}\n{}\n", cleaned, start_delim, new_content, end_delim)
}
```

**Output example:**

```yaml
<!-- BEGIN: bundle-a -->
Content from bundle A
<!-- END: bundle-a -->

<!-- BEGIN: bundle-b -->
Content from bundle B
<!-- END: bundle-b -->
```

#### 6. Platform Loader

Platforms are loaded from built-in definitions and optional custom config:

```rust
pub struct PlatformLoader;

impl PlatformLoader {
    pub fn load(workspace_path: &Path) -> Result<PlatformConfig> {
        // Load built-in platforms
        let mut config = Self::load_builtin()?;

        // Merge custom platforms if exists
        let custom_path = workspace_path.join(".augent/platforms.jsonc");
        if custom_path.exists() {
            let custom = Self::load_custom(&custom_path)?;
            config = Self::merge_platforms(config, custom)?;
        }

        Ok(config)
    }

    fn load_builtin() -> Result<PlatformConfig> {
        // Hardcoded platform definitions
        Ok(PlatformConfig {
            platforms: vec![
                Platform {
                    id: "claude".to_string(),
                    name: "Claude Code".to_string(),
                    directory: ".claude".to_string(),
                    detection: vec![".claude".to_string(), "CLAUDE.md".to_string()],
                    transforms: vec![
                        TransformRule {
                            from: "commands/*.md".to_string(),
                            to: ".claude/prompts/{name}.md".to_string(),
                            merge: MergeStrategy::Replace,
                            extension: Some("md".to_string()),
                        },
                        // ... more rules
                    ],
                },
                // ... more platforms
            ],
        })
    }

    fn merge_platforms(builtin: PlatformConfig, custom: PlatformConfig) -> Result<PlatformConfig> {
        let mut platforms = builtin.platforms;

        // Add or override custom platforms
        for platform in custom.platforms {
            if let Some(pos) = platforms.iter().position(|p| p.id == platform.id) {
                platforms[pos] = platform; // Override
            } else {
                platforms.push(platform); // Add new
            }
        }

        Ok(PlatformConfig { platforms })
    }
}
```

### Error Handling

| Error Condition | Error Message | Recovery |
|----------------|----------------|----------|
| Unknown platform specified | "Unknown platform: {name}. Valid platforms: {list}" | Exit with error |
| No platform detected | "No platform detected. Specify with --for or initialize workspace" | Exit with error |
| No transformation rule | "No transformation rule for {path} on platform {platform}" | Skip file |
| Merge conflict | "Failed to merge {file}: {reason}" | Exit with error |
| Invalid platform config | "Invalid platforms.jsonc: {reason}" | Exit with error |

## Testing

### Unit Tests

- Platform detection from directories
- Platform detection from files
- Platform name resolution (explicit and auto-detect)
- Glob pattern matching
- Path transformation (all rules)
- Name extraction from paths
- Extension transformation
- All merge strategies
- Platform merging (override and add)

### Integration Tests

- Install with auto-detected platforms
- Install with explicit platform selection
- Install with multiple platforms via `--for`
- Custom platform configuration loaded and used
- Transformations applied correctly for each platform
- Merge strategies produce correct output
- Platform detection respects workspace changes

## References

- PRD: [Other AI coding platforms](../mvp/prd.md#other-ai-coding-platforms)
- ARCHITECTURE: [ADR-002: Platform System](../adrs/002-platform-system.md)
- ARCHITECTURE: [Platform Detection and Resource Transformation](../architecture.md#platform-detection-and-resource-transformation)
- USER DOCS: [Platform Support](../../platforms_schema.md)
- SCHEMA: [Platforms Configuration Schema](../../platforms_schema.md)
