# DA — Data Availability

Data Availability layer with KZG-style commitments, erasure coding, and light-client sampling.

## Components

| Component | Description |
|---|---|
| `DataBlob` / `KzgCommitment` | Blob data with SHA-256 commitments (simplified; production would use real KZG) |
| `ErasureCoder` | XOR-based erasure coding (4 data + 2 parity chunks) with recovery path |
| `AvailabilitySampler` | Random sampling requiring ≥50% of chunks for availability |
| `DaLightClient` | Light-client verification via random sampling of chunks |
| `AvailabilityProver` | Merkle inclusion proof generation for blob chunks |

## Notes

- Uses SHA-256 hash as a simplified KZG commitment (MVP only; production should use `rust-kzg` or `c-kzg`).
- Erasure coding uses XOR-based parity instead of full Reed-Solomon.
