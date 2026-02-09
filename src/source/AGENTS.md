# Source - Bundle Source Parsing

**Overview**: Handles parsing and resolving bundle sources from various URL formats (24 lines).

## SUPPORTED FORMATS

- **Local directory**: `./bundles/my-bundle`, `../shared-bundle`
- **Git repos**: `https://github.com/user/repo.git`, `git@github.com:user/repo.git`
- **GitHub short-form**: `github:author/repo`, `author/repo`
- **GitHub web UI**: `https://github.com/user/repo/tree/ref/path`
- **With ref**: `github:user/repo#v1.0.0`, `github:user/repo@v1.0.0`
- **With path**: `github:user/repo:plugins/bundle-name`
- **With ref and path**: `github:user/repo:plugins/bundle-name#main`

## STRUCTURE

```
src/source/
├── mod.rs            # Re-exports (24 lines)
├── bundle_source.rs   # BundleSource enum and parsing
├── git_source.rs      # GitSource struct and URL parsing
└── bundle.rs          # Fully resolved bundle model with validation
```

## WHERE TO LOOK

| Task | Location |
|-------|----------|
| Source parsing | `bundle_source.rs` |
| Git URL parsing | `git_source.rs` |
| Bundle model | `bundle.rs` |

## CONVENTIONS

- **`pub use`** for re-exports: `BundleSource`, `GitSource`
- **Three modules**: `bundle_source`, `git_source`, `bundle`

## ANTI-PATTERNS

- **NEVER assume source format** - Use `BundleSource` parsing
- **NEVER validate source without parsing** first
