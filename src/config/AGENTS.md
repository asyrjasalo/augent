# Config - Configuration Management

**Overview**: Three configuration types (bundle, lockfile, index) with custom serde implementations and field-count optimization.

## STRUCTURE
```
src/config/
├── mod.rs              # Re-exports all config types
├── bundle/             # augent.yaml: bundle metadata and dependencies
│   ├── mod.rs
│   ├── dependency.rs      # BundleDependency struct
│   ├── serialization.rs    # Custom serde with macro
│   └── tests.rs
├── lockfile/            # augent.lock: locked Git refs and SHAs
│   ├── mod.rs
│   ├── bundle.rs          # LockedBundle struct
│   ├── source.rs          # LockedSource enum (tagged)
│   ├── serialization.rs    # Field-count optimization
│   └── tests.rs
├── index/               # augent.index.yaml: workspace installation index
│   ├── mod.rs
│   ├── bundle.rs          # WorkspaceBundle struct
│   ├── serialization.rs    # Index config pattern
│   └── tests.rs
├── marketplace/         # Marketplace configuration
│   ├── mod.rs
│   └── operations.rs
└── utils.rs             # BundleContainer trait for iteration
```

## WHERE TO LOOK

| Task | Location | Notes |
|-------|----------|-------|
| Bundle config | `bundle/mod.rs`, `bundle/serialization.rs` | augent.yaml parsing, dependencies |
| Lockfile | `lockfile/mod.rs`, `lockfile/serialization.rs` | Locked sources, Git SHAs, hash verification |
| Index | `index/mod.rs`, `index/serialization.rs` | Workspace bundles, enabled platforms |
| Utils | `utils.rs` | BundleContainer trait for polymorphic iteration |

## CONVENTIONS

### Custom Serde Pattern
```rust
// Macro for optional field serialization
macro_rules! serialize_optional_field {
    ($state:expr, $name:expr, $value:expr) => {
        if let Some(val) = $value {
            $state.serialize_field($name, val)?;
        }
    };
}

// Field counting for optimization
let mut field_count = 2; // name + bundles
if description.is_some() { field_count += 1; }
let mut state = serializer.serialize_struct("BundleConfig", field_count)?;
```

### Name Injection Pattern
```rust
// Serialize empty name
state.serialize_field("name", "")?;

// Later replace with actual workspace name
format_yaml_with_workspace_name(yaml, workspace_name)
```

### Tagged Enums
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum LockedSource {
    Dir { path: String, hash: String },
    Git { url: String, sha: String, hash: String },
}
```

## ANTI-PATTERNS

- **NEVER reorder git dependencies** - Maintains exact order in `BundleConfig::add_dependency()`
- **NEVER delete augent.yaml** - Persists once created

## UNIQUE STYLES

- **Field-count optimization**: Skip None fields in output for smaller configs
- **Name field injection**: Serialize empty, replace from filesystem path
- **Trait-based iteration**: `BundleContainer<B>` for polymorphic bundle access
