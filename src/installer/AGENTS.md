# Installer - File Installation

**Overview**: Multi-stage pipeline for transforming universal bundle resources to platform-specific formats and installing to workspace.

## STRUCTURE

```
src/installer/
├── mod.rs              # Installer struct (349 lines)
├── discovery.rs        # Resource discovery, filtering
├── detection.rs        # Platform and binary file detection
├── file_ops.rs         # Copy, merge, read, write
├── parser.rs           # Frontmatter parsing
├── writer.rs           # Output writing
├── formats/           # Platform-specific format converters (17 platforms)
│   ├── mod.rs
│   ├── claude.rs
│   ├── cursor.rs
│   └── ...
└── files.rs            # Legacy file operations (re-export)
```

## INSTALLATION PIPELINE

```text
1. Discovery
   └─ Scan bundle for resources
   └─ Identify resource types (commands, skills, mcp.json, etc.)
   └─ Parse frontmatter (platform-specific metadata)

2. Platform Detection
   └─ Detect target platforms in workspace
   └─ Select platforms for installation

3. Format Conversion
   └─ Transform universal → platform format
   └─ Apply merge strategies (Replace, Shallow, Deep, Composite)

4. File Installation
   └─ Resolve target paths
   └─ Handle conflicts
   └─ Copy/merge files
```

## WHERE TO LOOK

| Task | Location |
|-------|----------|
| Resource discovery | `discovery.rs` |
| Platform detection | `detection.rs` |
| Format conversion | `formats/*.rs` |
| File merging | `file_ops.rs` |
| Main installer | `mod.rs: install_bundle(), install_bundles()` |

## CONVENTIONS

- **Installer** tracks `installed_files: HashMap<String, InstalledFile>`
- **ResourceInstallContext** for per-resource installation
- **Target path**: `platform_root.join(resource.bundle_path.relative)`

## ANTI-PATTERNS

- **NEVER install without platform detection** - Use `detection.rs` first
- **NEVER merge without checking conflicts** - `file_ops.rs` handles conflicts
