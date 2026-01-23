# ADR-003: Locking Mechanism

**Status:** Accepted
**Date:** 2026-01-22

## Context

Need reproducible installations across team members and CI/CD.

## Decision

- `augent.lock` resolves all refs to exact git SHAs
- BLAKE3 hash for each bundle's contents
- All files provided by each bundle listed
- Lockfile updated on `install`, validated on `--frozen`

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
