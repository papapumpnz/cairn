# Dependency audit

| Dependency | Purpose | Enabled features | Policy notes |
|---|---|---|---|
| `blake3` | Stable semantic and payload identities | `std` | CPU-only cryptographic hash; canonical bytes are its only semantic input |
| `half` | RFC 8949 preferred-width float encoding | `std` | CPU-only numeric conversion; NaNs are normalized before encoding |

