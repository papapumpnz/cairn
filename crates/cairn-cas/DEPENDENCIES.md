# Dependency audit

| Dependency | Purpose | Enabled features | Policy notes |
|---|---|---|---|
| `cairn-core` | Shared canonical hashes and artifact identifiers | workspace | Tagged WP01-owned substrate contract |
| `blake3` | Streaming verification without copying large blobs | `std` | CPU-only; must match `cairn-core` reference hashes |

