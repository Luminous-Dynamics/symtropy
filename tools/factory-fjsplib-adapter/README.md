# FJSPLib Text Adapter v0.1

**Owner:** Symtropy FACTORY-FJSPLIB-ADAPTER-001 #5459  
**Parent programme:** FACTORY-EXACT-SCHEDULE-001 #5446  
**Upstream benchmark:** https://scheduleopt.github.io/benchmarks/fjsplib

## Supported public formats

FJSPLib documents 138 classic flexible-job-shop instances, 264 machine-independent flexible-job-shop instances and 50 flexible-assembly job-shop instances. Its reference protocol compares CPLEX, CP Optimizer, OR-Tools CP-SAT and OptalCP under a standardized 10-minute budget and records best-known lower/upper bounds.

This adapter supports the two documented text formats:

- FJSP: header with job count, machine count and average flexibility; each subsequent job record contains its operation count followed by eligible-machine count and machine/duration pairs for each operation. The text format numbers machines from 1; the normalized model numbers them from 0 and stores each linear job route as explicit precedence edges.
- FAJSP: header with operation count, precedence-edge count and machine count; explicit before/after/job records; then alternative-machine records per operation. The text format numbers machines from 0. Explicit graph edges are preserved without flattening flexible-assembly precedence into linear job sequences.

## Normalized data contract

Every operation has a stable integer identity and one or more (machine, processing_ticks) alternatives. Precedence edges preserve the optional source job label. The model retains instance id, source family, format, machine count, declared job count and average flexibility where present.

The parser validates record counts, machine ranges, nonzero durations, nonempty alternatives, duplicate alternatives, precedence endpoints, duplicate edges, cycle freedom and source-format machine indexing. Unknown or malformed data fails closed.

A deterministic FNV-1a 64-bit canonical fingerprint is emitted for reproducibility. It is explicitly non-cryptographic and is not a signature or integrity authority.

## Verification

    cargo fmt --manifest-path tools/factory-fjsplib-adapter/Cargo.toml -- --check
    cargo clippy --manifest-path tools/factory-fjsplib-adapter/Cargo.toml --all-targets -- -D warnings
    cargo test --manifest-path tools/factory-fjsplib-adapter/Cargo.toml --all-targets
    cargo run --manifest-path tools/factory-fjsplib-adapter/Cargo.toml --bin fjsp-inspect -- --reference-fjsp
    cargo run --manifest-path tools/factory-fjsplib-adapter/Cargo.toml --bin fjsp-inspect -- --reference-fajsp
    cargo run --manifest-path tools/factory-fjsplib-adapter/Cargo.toml --bin fjsp-solve -- --reference-fjsp
    cargo run --manifest-path tools/factory-fjsplib-adapter/Cargo.toml --bin fjsp-solve -- --reference-fajsp

The tests use the official FJSPLib fattahi1 example in both text formats. They check one-based versus zero-based normalization and that the FJSP route structure and FAJSP precedence graph carry the same operation/machine/duration semantics.

## Claim ceiling

This adapter includes a bounded exact makespan oracle for tiny FJSP/FAJSP instances. It exhaustively enumerates every eligible-machine assignment and each resulting machine-order product, rejects cyclic precedence combinations, and derives earliest starts from a longest-path calculation. It is limited to nine operations, at most 100,000 assignments and at most 500,000 aggregate machine-order combinations; preflight overflow/bound failures return INCONCLUSIVE before partial search. For the documented Fattahi1 fixture, both text formats normalize to an exact makespan of 66 ticks after 16 assignments and 120 machine-order combinations (68 cyclic, 52 feasible). The fixture is synthetic and this result is not an industrial SOTA claim. The adapter does not reproduce a reference engine or establish real factory capacity.
