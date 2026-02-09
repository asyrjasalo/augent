# Config - Configuration File Handling

**Overview**: Data structures for augent.yaml, augent.lock, augent.index.yaml, and marketplace.json.

## STRUCTURE

```
src/config/
├── mod.rs             # Re-exports (20 lines)
├── bundle/            # Bundle config (augent.yaml)
├── lockfile/          # Lockfile with resolved Git refs
├── index/             # Workspace index (augent.index.yaml)
├── marketplace/        # Claude Marketplace config
└── utils/             # Config utilities
```

## CONFIGURATION TYPES

| File          | Module           | Purpose                        |
|---------------|------------------|--------------------------------|
| augent.yaml   | bundle           | Bundle metadata, dependencies  |
| augent.lock   | lockfile         | Resolved Git refs, hash        |
| augent.index  | index            | Per-platform file mappings     |
| marketplace   | marketplace      | Claude Marketplace conversion   |

## WHERE TO LOOK

| Task | Location |
|-------|----------|
| Bundle config parsing | bundle/mod.rs |
| Lockfile management | lockfile/mod.rs |
| Workspace index | index/mod.rs |
| Marketplace config | marketplace/mod.rs |

## CONVENTIONS

- **Four config types**: Bundle, Lockfile, Index, Marketplace
- **`pub use`** for re-exports: `BundleConfig`, `Lockfile`, `WorkspaceConfig`, `MarketplaceConfig`
- **Multi-subdir structure**: Each config type has its own module

## ANTI-PATTERNS

- **NEVER modify lockfile without resolution** - Lockfile is authoritative
- **NEVER ignore index** - Maps installed resources to bundles
