# ADR-003: Locking Mechanism

**Status:** Accepted
**Date:** 2026-01-22

## Context

Need reproducible installations across team members and CI/CD.

## Decision

- `augent.lock` always has `ref` and the **exact SHA** of the commit for every Git bundle (reproducibility)
- BLAKE3 hash for each bundle's contents
- All files provided by each bundle listed
- Lockfile updated first on `install` (then augent.yaml, then augent.index.yaml); validated on `--frozen`

## Lockfile Format

```json
{
  "name": "@author/my-bundle",
  "bundles": [
    {
      "name": "dependency",
      "source": {
        "type": "git",
        "url": "https://github.com/...",
        "sha": "abc123...",
        "hash": "blake3:..."
      },
      "files": ["commands/debug.md"]
    }
  ]
}
```

## Consequences

- Exact reproducibility with `--frozen`
- Can trace any file back to its source bundle
- Hash verification detects tampering
