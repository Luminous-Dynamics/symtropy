# Scan-to-BIM Admission Oracle

Standalone Rust package for the finite synthetic contract in #4086. It is deliberately separate from the built-asset defect/repair oracle in #4062.

## Run

```sh
cargo fmt --manifest-path tools/scan-bim-admission-oracle/Cargo.toml -- --check
cargo test --manifest-path tools/scan-bim-admission-oracle/Cargo.toml
cargo clippy --manifest-path tools/scan-bim-admission-oracle/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path tools/scan-bim-admission-oracle/Cargo.toml -- --corpus docs/research/built-asset/scan-bim/v0.1/corpus.json --counterfactuals docs/research/built-asset/scan-bim/v0.1/counterfactual-pairs.json
```

The CLI evaluates both the 40-case admission corpus and the paired counterfactual manifest, then emits a deterministic JSON report with corpus/manifest digests, case dispositions and pair-level results. The manifest also declares grouped training/evaluation identities and a candidate-input allowlist; every case must bind to a declared held-out group, the oracle rejects overlapping groups and obvious ground-truth/reference-bound input markers. This is a contract-level guard, not a substitute for an independent data custodian enforcing access to held-out labels and geometry.

## Scope

The v0.1 oracle evaluates a small, declared admission contract over candidate geometry summaries, element identity/class, visibility, uncertainty, evidence lineage, topology, upstream survey-product scope, change-estimate receipt, registration-stability receipt, independent-checkpoint receipt and revision-bound modify proposals. It does not compute geometric deviation from raw point clouds: the deviation and uncertainty are typed upstream measurements supplied to the decision oracle and must be independently qualified by their producing pipeline. It also rejects duplicate IDs/edges/targets, unknown schema fields, stale model revisions, incomplete visibility, unsupported identity changes and uncertainty intervals crossing the declared tolerance, common-mode registration drift, unresolved change-estimate or registration stability, stale/mismatched receipt scope, invalid or missing receipt digests, and missing independent-checkpoint evidence. It does not register point clouds, fit geometry, infer semantic labels, write IFC, certify surveys or authorize physical work.

Expected labels are used only to compare the oracle output after candidate evaluation. The evaluator does not read expected outcomes as inference inputs.

## Paired counterfactual challenge

The default evaluator also validates `docs/research/built-asset/scan-bim/v0.1/counterfactual-pairs.json`:

- Ten evidence-gating nuisance pairs on a tolerance-clearing apparent change: suspected registration drift, unresolved change estimate, missing checkpoint, unresolved survey/checkpoint evidence, missing change/registration receipt digests, and mismatched change/registration/checkpoint receipt scope.
- Three no-change false-positive controls: starting from `NO_CHANGE_SUPPORTED`, perturb only registration stability, change-estimate status or checkpoint status. Each must abstain rather than invent a physical change.
- One positive physical-change sensitivity pair: a no-change control versus a tolerance-clearing displacement under the same qualified survey evidence.
- Every pair shares the same reference and held-out building group. Nuisance pairs must be exactly identical outside the single declared JSON-pointer intervention. The positive pair is identical outside the declared geometry and delta fields.
- A nuisance treatment may never produce `ADMIT_CANDIDATE_DELTA` or `REAL_CHANGE_CANDIDATE`. A physical-change sensitivity pair must have a `NO_CHANGE_SUPPORTED` control and an `ADMIT_CANDIDATE_DELTA` treatment.

The paired suite checks structural counterfactual consistency over 14 synthetic pairs. Pointwise case reports are canonicalized by case ID and pair reports by pair ID; a regression test reverses the corpus array and checks semantic report invariance while retaining a different exact-input digest. Nuisance interventions must point to scalar/nullable leaves; object/array subtrees are rejected, and array-element leaf pointers are supported. Only the positive physical-change pair may additionally declare the array-valued `/delta/target_ids` field to represent an empty-to-one target-set proposal. It is not proof of real-world causal attribution or physical independence.


## Claim ceiling

A passing suite proves only agreement with 40 hand-authored synthetic fixtures and their explicit rules. It does not establish real scan accuracy, sensor performance, generalization, building safety, structural adequacy, code compliance, legal survey status, or operational productivity.
