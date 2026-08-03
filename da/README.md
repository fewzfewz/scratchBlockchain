# DA — Data Availability

Data Availability layer with Merkle-root commitments, Reed–Solomon erasure coding, and light-client sampling.

## Components

| Component | Description |
|---|---|
| `DataBlob` / `KzgCommitment` | Blob data with Merkle-root commitments (path to full KZG via `c-kzg`) |
| `ErasureCoder` | Reed–Solomon erasure coding (4 data + 2 parity chunks) with recovery |
| `AvailabilitySampler` | Random sampling requiring ≥50% of chunks for availability |
| `DaLightClient` | Light-client verification via random sampling of chunks |
| `AvailabilityProver` | Merkle inclusion proof generation for blob chunks |

## Notes

- Commitments use **Merkle roots over 4KB chunks** (upgraded from single SHA-256 hash).
- Erasure coding uses **`reed-solomon-erasure`** over GF(2^8).
- Production mainnet should migrate commitments to `rust-kzg` / `c-kzg` (see `AUDIT_READINESS.md`).
