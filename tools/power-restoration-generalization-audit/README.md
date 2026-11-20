# Power restoration generalization split audit v0.1

**Owner:** Symtropy #6421  
**Comparator:** Chowdhury & Li, arXiv:2611.22016 (17 Nov 2026)  
**Physical tier:** #6353  
**Programme:** #6320 / #5241

## Purpose

A random scenario split on a fixed feeder template is not an unseen-topology generalization test. This tool audits a preregistered train/validation/test manifest against the actual frozen benchmark cases, recomputes canonical case digests and derives an identifier-invariant topology-screening fingerprint.

## Input

The CLI accepts one JSON SplitAuditInput containing:
- manifest: schema/version, policy ID, split seed and exactly one assignment per case;
- cases: the frozen benchmark-case objects from the case interchange contract.

Each assignment pins case ID, split, reviewed topology-family ID, canonical case SHA-256 and expected topology fingerprint. The CLI recomputes both digests rather than trusting the manifest's declarations.

## Fingerprinting

The fingerprint combines:
- the distribution power graph (nodes, branch connectivity/type, normally-energized classification, switch/tie connectivity and declared branch electrical-data presence);
- the road graph and its power-node interface mapping;
- electrical-data maturity, graph sizes and branch electrical-data coverage.

Dynamic scenario variables are intentionally excluded from the topology fingerprint: load magnitude and placement, fault location, initial switch state, crew/mobile-source starting positions and temporary road closures. Those belong in scenario/observation split metadata, not the feeder's static topology identity.

The graph hash uses attributed Weisfeiler-Lehman color refinement with undirected edges and sorted neighborhoods. Node/branch/switch IDs are excluded so simple identifier renaming does not evade the structural screen. Load magnitudes/locations, fault positions, current switch positions, crew/MPS starting locations, road closures and scenario seeds are excluded, so ordinary scenario variations on the same feeder retain the same fingerprint.

**Important limitation:** Weisfeiler-Lehman color refinement is not a complete graph-isomorphism algorithm. It can collide for non-isomorphic graphs, and bounded refinement can miss distant structural differences in very large graphs. Therefore this is a leakage-screening control, not a proof of topology uniqueness. A reviewed topologyFamilyId remains mandatory and must group all scenarios, variants and derived cases from the same feeder family.

## Leakage rules

The audit reports leakage when:
- one declared topology family occurs in more than one split;
- one computed topology fingerprint occurs across train/validation/test;
- a case ID is missing, duplicated or assigned twice;
- the supplied case SHA or topology fingerprint does not match the exact parsed case;
- any of the train, validation or test splits is empty.

A run with leakage findings returns a machine-readable report with exit code 3. Invalid manifests or case bindings return exit code 1. Only a complete, no-leakage audit exits 0.

## Run

    cargo fmt --manifest-path tools/power-restoration-generalization-audit/Cargo.toml --all -- --check
    cargo clippy --manifest-path tools/power-restoration-generalization-audit/Cargo.toml --all-targets -- -D warnings
    cargo test --manifest-path tools/power-restoration-generalization-audit/Cargo.toml --all-targets
    cargo run --manifest-path tools/power-restoration-generalization-audit/Cargo.toml --bin power-restoration-generalization-audit -- split-audit-input.json

## Benchmark protocol

Use the tool to enforce:
1. within-feeder scenario splits (reproduction of published setups);
2. leave-one-feeder-out splits;
3. topology-family holdouts (radial/weakly meshed, lateral depth, tie placement, source location);
4. equipment-profile holdouts;
5. compound topology + DER + load + observation shifts.

Never claim arbitrary unseen-topology generalization from randomized outages on a fixed IEEE-33/69/118 base topology. Keep each feeder family wholly in one split, freeze the manifest before planner tuning and publish it with the evaluation receipts.

## Claim boundary

Canonical case binding and structural split-leakage screening only. No proof of full graph isomorphism, absence of all semantic or temporal leakage, planner generalization, electrical feasibility or field readiness.
