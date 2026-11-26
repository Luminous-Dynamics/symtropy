# Preservation Service Reference Scorer

This zero-dependency Rust crate scores end-to-end service receipts against a frozen WorkloadContract. It intentionally does not read media, run a decoder, calculate digests, authenticate provenance, meter energy or independently attest a physical reader; a separate harness and verifier must produce those observations.

## Core distinction

- reconstructed bytes: candidate output;
- delivered bytes: payload that reached the declared service boundary;
- independently accepted bytes and objects: outputs accepted by the scorer after external verification.

The primary rate is independently accepted bytes divided by end-to-end elapsed time. Candidate-reported bytes or throughput are not acceptance evidence.

## Fail-closed rules

- Workload IDs and object accounting must be coherent.
- Exact requested byte/object counts are required for a pass.
- Digest failure, required manifest/provenance failure, false acceptance, extra/duplicate objects, deadline miss or malformed accounting blocks a pass.
- Missing required stage evidence, incomplete fixity/provenance/energy data, missing objects or abstention remains incomplete.
- A candidate full-success claim that the independent scorer cannot support is a hard failure.
- No run remains not evaluated.
- Energy is only compared when a declared meter boundary and a valid meter result exist.

The workload contract, including the mandatory stage list, is supplied by the frozen benchmark and not chosen by the candidate. Stage timings are observations; because stages may overlap, their durations are not blindly summed to recreate elapsed wall time.

## Commands

~~~bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
~~~

The authoring environment has no Cargo, rustc or rustfmt; tests are authored but not executed here. The synthetic receipts and schema are contract fixtures, not measured service results.

## Files

- ../PRESERVATION_SERVICE_TOURNAMENT_001.md — benchmark and system boundary.
- ../preservation_service_workloads_v0_1.schema.json — frozen workload definitions.
- ../fixtures/preservation_service_workloads_v0_1.json — six synthetic workloads.
- ../preservation_service_receipts_v0_1.schema.json — receipt KAT schema.
- ../fixtures/preservation_service_receipts_v0_1.json — scorer positive, negative and incomplete cases.
