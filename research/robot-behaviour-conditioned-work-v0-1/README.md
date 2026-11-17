# Robot Behaviour-Conditioned Work v0.1

Parent: Symtropy #5685  
Portfolio: Symtropy #5700  
Status: first standalone Rust evaluator tranche; compile and CI qualification pending.

## Purpose

This package demonstrates a bounded 2×2 non-load-bearing panel article. It derives next work packages from already admitted accepted outputs, explicit WIP records and scoped access observations. Different valid installation paths create different seam-inspection obligations.

The package is deliberately read-only:
- it consumes accepted-output references; it does not accept work;
- it consumes WIP dispositions; it does not become the WIP authority;
- it consumes access observations; it does not establish metrology or site-access truth;
- it emits benchmark plan items; it does not issue robot commands;
- it does not select or authorize construction methods.

## Model

1. An accepted anchor-frame output enables panel-install packages.
2. An accepted pair of adjacent panels creates the corresponding seam-inspection package.
3. Four accepted panels plus four accepted seam inspections create a final functional-check package.
4. Completion requires an upstream accepted reference for that final functional-check package.
5. Partial WIP emits a reconciliation requirement; rejected WIP emits a review requirement; in-progress and unknown WIP suppress duplicate issuance.
6. Scoped blocked or unknown access prevents only the affected branch from being marked ready. A missing access observation is Unknown, never implicitly Available; nominal fixtures provide explicit scoped Available observations.
7. Robot-reported completion is deliberately ignored.

The 2×2 article is a benchmark fixture, not a structural design or construction procedure.

## Determinism and integrity

The evaluator uses sorted maps/sets and lexically sorted output IDs. Its input is a snapshot, not an event log; trusted time, event ordering, evidence authentication and physical chronology remain with upstream owners.

Malformed identifiers, duplicate accepted packages, duplicate unit IDs, conflicting accepted/WIP records, duplicate access observations and impossible prerequisites produce Inconsistent rather than guessed progress.

## Run

From this directory:

    cargo fmt --all -- --check
    cargo clippy --all-targets -- -D warnings
    cargo test
    cargo run

No third-party Rust dependencies are required.

Unit tests contain public development cases and known-answer assertions. They are not blind holdouts, real site observations or physical trial results.

## Ownership

- #3605 owns accepted functional units, rejected/partial output and WIP semantics.
- #5657 owns evidence-carrying stage transitions and interruption recovery.
- #5411 owns reconfiguration chronology.
- #5488 owns claim-specific qualification applicability.
- #1607 owns surveying, metrology, frames and calibration.
- #3522/#3531 own physical receipts and evidence admissibility.
- #5685 owns behavior-conditioned benchmark fixtures and paired comparison logic.

This evaluator does not replace any of those owners.
