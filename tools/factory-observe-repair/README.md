# Factory Observe–Repair Benchmark v0.1

**Owner:** Symtropy FACTORY-OBSERVE-REPAIR-001 #5520

## Purpose

This crate is the first narrow evaluator slice for the joint factory observe → diagnose → repair → reschedule programme. It evaluates repair-scope correctness after receiving:

1. a diagnosis/inspection receipt from the Symthaea active diagnosis owner;
2. a claim/operation impact-closure receipt from the generic engineering change-impact owner;
3. a candidate release/containment scope from a scheduling/recovery policy.

It does not compute test likelihoods, posterior beliefs, expected value of information, generic dependency closure, job-shop schedules, physical verification or production authorization.

## V0.1 contract

- Completed and in-flight operations are frozen commitments. They may be marked for containment but may not be released for rescheduling.
- Impacted planned operations must be released for repair/requalification.
- Unaffected planned operations may remain scheduled; unnecessary releases are measured separately.
- An observation from the same lineage group as the baseline sensor is not independent evidence; different labels also require an explicit independence disposition and a non-empty receipt reference from the evidence owner.
- Stale, blocked, unknown, inapplicable or unresolved inspection evidence blocks repair admission.
- Diagnosis and impact receipts must match the exact episode references.
- Unknown/duplicate operation identities fail closed.

## Metrics

The evaluator keeps the following quantities separate:

- affected/frozen/planned operation counts;
- released and contained operation counts;
- false-preserved affected operations;
- unnecessary releases;
- unaffected planned operations retained.

No weighted recovery score is emitted. Time to accepted output, schedule churn, schedule feasibility and diagnosis value are later composed metrics owned by their respective evaluators.

## Verification

    cargo fmt --manifest-path tools/factory-observe-repair/Cargo.toml -- --check
    cargo clippy --manifest-path tools/factory-observe-repair/Cargo.toml --all-targets -- -D warnings
    cargo test --manifest-path tools/factory-observe-repair/Cargo.toml --all-targets

This v0.1 crate uses no third-party Rust dependencies and no machine/actuator interfaces. All included cases are synthetic and dimensionless.

## Claim ceiling

The evaluator checks evidence references and repair-scope semantics only. It does not select inspections, diagnose physical faults, construct dependency graphs, schedule production, establish recovery-time improvement or authorize physical operations.

## Frozen scenario corpus

`fixtures/repair_scope_cases.tsv` contains 19 synthetic episodes. Its columns explicitly separate:
- measurement and baseline source groups;
- independence disposition and independence receipt ID;
- calibration, applicability and outcome status;
- affected, released and contained operation IDs;
- expected admission/block reason and the two scope-error metrics.

A `Pass` independence disposition requires a non-empty owner receipt reference. Different source labels alone do not prove independence. Unknown, blocked, missing or whitespace-only independence evidence blocks repair admission. The hidden-future paired rows have the same candidate-visible inputs and must yield identical evaluator results.

The corpus contains development fixtures only. It is not described as a blind test set, and synthetic receipt identifiers do not attest to real inspections.
