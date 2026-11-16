# FJSPLib Text Adapter v0.1

**Owner:** Symtropy FACTORY-FJSPLIB-ADAPTER-001 #5459  
**Parent programme:** FACTORY-EXACT-SCHEDULE-001 #5446  
**Upstream benchmark:** https://scheduleopt.github.io/benchmarks/fjsplib

## Supported public formats

FJSPLib documents 452 logical instances: 138 classic flexible-job-shop, 264 machine-independent flexible-job-shop and 50 flexible-assembly instances. The upstream text repository contains 402 FJSP and 452 FAJSP representations (854 source files total); the 402 non-assembly cases have paired encodings, while the 50 assembly cases are FAJSP-only. A complete hash-pinned metadata index is in `corpus/fjsplib-index-2026-10-22.tsv`. Its reference protocol compares CPLEX, CP Optimizer, OR-Tools CP-SAT and OptalCP under a standardized 10-minute budget and records best-known lower/upper bounds.

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

The Rust tests use the official FJSPLib fattahi1 example in both text formats and compare normalized operation alternatives and precedence edges for paired fattahi5, fattahi10 and fattahi20 FJSP/FAJSP instances. Focused CI also imports the frozen 8-file Fattahi smoke set (fattahi1/5/10/20 × FJSP/FAJSP). These samples span multiple instance sizes but are not the full FJSPLib corpus. They check one-based versus zero-based normalization and preserve operation alternatives plus explicit precedence graphs across the paired formats.

## Exact upstream fixture provenance and reference bound

Both frozen Fattahi1 text fixtures now match the raw upstream files exactly at FJSPLib repository revision `8a6e36ef31263d36975e56fb481e0ba04e188d6f`:

- FJSP: blob `84e868f74b099f68d05352d645cdbcaf444ddd6b`, 58 bytes.
- FAJSP: blob `e9eeea2fb0b9382e4857b44cff2cd5d5d2fd0483`, 65 bytes (no trailing newline).

The independent upstream `flexible-jobshop/solutions/bks.json` record marks `fattahi1` as `closed`, with lower bound = upper bound = **66**. Its bound history attributes both values to OptalCP on an Intel i7-1185G7 on 2025-08-29 (the archive records no standalone certificate). The Rust complete enumeration and independent CP-SAT model are separate reproducibility checks against this archived reference. No timing superiority is implied.

## Full corpus inventory

See [corpus/README.md](corpus/README.md) and `corpus/fjsplib-index-2026-10-22.tsv` for the full logical-instance index, all 854 source-file representations, upstream Git blob IDs and class/format counts. The index is pinned to upstream revision `8a6e36ef31263d36975e56fb481e0ba04e188d6f`; CI checks the inventory's Git blob and aggregate counts. Full-corpus retrieval and benchmarking is tracked separately in Symtropy #5528.

## Independent CP-SAT cross-check

A benchmark-only Python adapter at `reference/ortools_fattahi1_check.py` independently parses both frozen `fixtures/fattahi1.fjsp` and `fixtures/fattahi1.fajsp` inputs and builds an optional-interval CP-SAT model. CI installs the pinned OR-Tools 9.15.6755 package and requires each representation to return OPTIMAL with objective and best bound both equal to 66.

This adapter is isolated to test/benchmark verification. The parser, exact solver, schedule verifier and any future runtime implementation remain Rust. Agreement on this one tiny known-answer case is a semantic cross-check, not a performance comparison or SOTA claim. The independent checker has its own parser/model and does not call the Rust exact solver.

## External comparison protocol to adopt

The live FJSPLib reference page reports a 600-second comparison on a Windows PC with an Intel i7 4-core 3 GHz CPU and 32 GB RAM. Its published engine configuration is:

- CPLEX 22.1.1.0;
- CP Optimizer 22.1.1.0 with zero gap tolerance;
- OR-Tools CP-SAT 9.15.6755 using its default configuration;
- OptalCP Academic 2026.4.0 using maximum propagation and zero gap tolerance (plus the benchmark's documented parameter changes).

FJSPLib publishes geometric lower-bound ratio, geometric upper-bound ratio and shifted geometric gap metrics. For an open instance, report the primal upper bound, dual lower bound, elapsed time, best-known upper/lower bounds and the resulting gap; do not infer optimality from a strong feasible schedule. For a closed instance, require an independently defensible matching upper/lower bound or an external proof.

The reference machine/software setup is not directly matched by our CI runner. Any competitive claim requires a dedicated pinned-hardware run with the same time budget, comparable solver settings and complete raw result retention. The tiny Fattahi1 fixture verifies exact search and cross-format semantics only; it does not establish performance relative to CP-SAT, CPLEX, CP Optimizer or OptalCP.

Official benchmark reference: https://scheduleopt.github.io/benchmarks/fjsplib

## Claim ceiling

This adapter includes a bounded exact makespan oracle for tiny FJSP/FAJSP instances. It exhaustively enumerates every eligible-machine assignment and each resulting machine-order product, rejects cyclic precedence combinations, and derives earliest starts from a longest-path calculation. It is limited to nine operations, at most 100,000 assignments and at most 500,000 aggregate machine-order combinations; preflight overflow/bound failures return INCONCLUSIVE before partial search. For the documented Fattahi1 fixture, both text formats normalize to an exact makespan of 66 ticks after 16 assignments and 120 machine-order combinations (68 cyclic, 52 feasible). The fixture is synthetic and this result is not an industrial SOTA claim. The adapter does not reproduce a reference engine or establish real factory capacity.


### Source-format metadata nuance

The FJSP header declares its job count. The FAJSP header does not: it provides operation count, precedence-edge count and machine count; each precedence edge preserves the source job label in its third field. Consequently, FAJSP inputs retain edge-level job labels but expose `declared_job_count = None`. Do not infer a declared count from the number of distinct edge labels: a job with a single operation has no precedence edge and may not be represented in that label set. This is a conservative metadata boundary; scheduling uses the explicit operation and precedence graph, not an inferred total job count.
