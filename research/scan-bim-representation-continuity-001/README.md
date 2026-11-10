# SCAN-BIM-REPRESENTATION-CONTINUITY-001 — evidence continuity across representation changes

**Status:** P0 typed audit, artifact-bound claims, CLI and synthetic fixtures implemented; exact-head validation pending.  
**Issue:** [#4471](https://github.com/Luminous-Dynamics/symtropy/issues/4471)  
**Parent delivery work:** [#4271 / PR #4273](https://github.com/Luminous-Dynamics/symtropy/pull/4273)  
**Branch:** research/scan-bim-representation-continuity-v0-1

## Research question

When building information changes representation—from raw observations to registered point clouds, fitted surfaces, spatial/semantic graphs, parametric objects and IFC—does the model preserve the evidence supporting each claim, the uncertainty attached to it, and the correspondence to the same physical building entity?

The October 2026 representation-centred review by Wang et al. identifies continuity of measurement support, provenance, entity correspondence, spatial relations, uncertainty and model state as a gap across representation interfaces. Malihi's July 2026 position paper argues for lifecycle- and LoD-dependent uncertainty envelopes calibrated against benchmark residuals and physical behaviour.

This work turns that gap into a testable transition audit. It does not claim those papers' proposed research directions are already solved by this implementation.

### Research sources

- Wang, Zhang, Liu & Xing, *From Measured Geometry to Editable BIM: A Representation-Centered Review of Existing-Building Scan-to-BIM Reconstruction*, Buildings 16(20), 4095, 16 October 2026. https://doi.org/10.3390/buildings16204095
- Malihi, *Towards Uncertainty-Aware Digital Twins in the Built Environment*, Digital Engineering, online 24 July 2026, article 100140. https://doi.org/10.1016/j.dte.2026.100140
- Ersöz & Bosché, geometric matching confidence via coverage, distribution and distance, validated on 46 PCD/BIM pairs and 4,000+ IFC elements. https://doi.org/10.1016/j.autcon.2025.106636
- Jayasinghe & Brilakis, completion benchmark leakage through complete-object bounds. https://doi.org/10.1016/j.autcon.2026.107052
- Trinh, Hou & Wu, structural reconstruction from incomplete point clouds, controlled laboratory results including LOA30 and selected LOA40. https://doi.org/10.1016/j.autcon.2026.107161

## Ownership boundaries

This is a thin audit adapter and synthetic benchmark. It composes existing authorities and does not create a new evidence store:

| Concern | Existing owner |
|---|---|
| Survey observations, transforms and covariance | #1607 |
| Component/physical identity and lifecycle | #3331 |
| Reconstruction, element association, topology and mutation admission | #4086 |
| Replay artifact provenance and candidate/evaluator isolation | #4116 |
| Deterministic surface sampling | #4225 |
| Matching-confidence reproduction | #4246 / PR #4247 |
| Evaluator-only visibility | #4292 / PR #4449 |
| IFC syntax/schema/IDS/interoperability and edit-round-trip | BUILT-INTEROP owners and #4271 / PR #4273 |

The audit consumes exact evidence and identity receipts from those owners. It does not authenticate external receipts or independently recalculate their domain results.

## P0 implementation

The Rust module is tools/scan-bim-replay-contract/src/continuity.rs, exported by the existing scan-bim-replay-contract crate. It uses the existing serde/serde_json dependencies; no new runtime dependency is introduced.

The versioned receipt binds:
- exact replay and scene identity;
- source and output representation kinds and artifact digests;
- tool/version/configuration identity;
- typed entity correspondence events (1:1, split, merge, replacement, unresolved) linked to an identity receipt;
- per-property claim lineage and claim origin (measured, inferred, assumed, unknown);
- exact evidence references supporting each claim;
- conservative absolute geometric uncertainty before and after the transition;
- declared approximation error from the representation operation;
- whether an uncertainty update is representation-only or conditioned on new independent checkpoint evidence.

### P0 rules

1. A measured claim needs direct source evidence (raw measurement, survey solution or independent checkpoint) bound to the exact source artifact digest.
2. An inferred claim needs source evidence, a declared derivation method and an output-value digest.
3. An assumed claim needs explicit assumptions and an output-value digest.
4. An unknown claim needs an explicit reason and must not pretend to have a value digest.
5. One-to-one, split, merge and replacement correspondences must satisfy their cardinalities. Unresolved correspondence is reported as unresolved, never silently counted as preserved.
6. Under the P0 conservative_absolute_bound_m metric, a representation-only transition cannot convert unknown uncertainty to known or reduce its bound. The output bound must be at least the input bound plus the declared deterministic approximation-error bound.
7. An evidence-conditioned uncertainty update must reference additional independent_checkpoint artifacts, a named estimator and a calibration-profile ID. Those checkpoint artifact IDs/digests must not be the same as the source representation artifacts.
8. This is a structural receipt check only: it does not prove checkpoint independence, estimator validity or empirical calibration. #1607 owns those measurements and receipts.
9. Unsupported uncertainty models (e.g. a covariance that cannot be reduced to the P0 absolute-bound form without losing correlations) remain outside the P0 metric; do not coerce them to a scalar.
10. Missing/invalid fields fail closed; unknown fields are rejected.

### Why conservative absolute bounds first?

This P0 profile uses an absolute geometric error bound so that a simple, conservative propagation rule is auditable:

u_after >= u_before + approximation_error_bound

This is not a replacement for covariance propagation. In a later profile, #1607's qualified covariance/transform receipts can be composed with Jacobian-based propagation (including correlations and process/model residuals). The P0 scalar rule intentionally avoids assuming Gaussian errors or independence when those assumptions are not established.

### Synthetic fixture

fixtures/known_answers_v0_1.json contains one canonical raw-observation → surface-mesh transition with:
- exact source scan and transform digests;
- a one-to-one physical-element correspondence;
- a measured wall-axis claim with a direct source-observation link;
- a 2.0 mm input absolute bound;
- a declared 0.5 mm approximation bound;
- a 2.5 mm output bound.

Negative controls mutate the fixture to cover derived-geometry-only measured claims, unjustified uncertainty shrinkage, unknown-to-known promotion, unsupported conditioned updates, reused source evidence, invalid split cardinality, orphaned claim evidence, duplicate target identity and unknown numeric values.

## Executable P0 audit

The replay-contract workspace now exposes the thin audit as a CLI:

```sh
cargo run --locked \
  --manifest-path tools/scan-bim-replay-contract/Cargo.toml \
  --bin scan_bim_continuity_audit -- \
  research/scan-bim-representation-continuity-001/fixtures/known_answers_v0_1.json
```

The CLI reads the receipt bytes, rejects unknown JSON fields through the typed `deny_unknown_fields` contract, runs the continuity audit and emits a machine-readable report. It does not open or authenticate the referenced scan/mesh artifacts; it checks only the artifact identities/digests declared inside the receipt.

P0 now additionally binds every claim to the exact transition output artifact that contains its target entity. When a later transition reuses an artifact ID previously emitted by an earlier transition, the SHA-256 and evidence role must match exactly. Reusing an ID with changed bytes or role is rejected.

Current test inventory: 19 representation-continuity tests. Claim target artifacts must also carry the `geometry_output` or `representation_output` role; a transform/calibration artifact cannot satisfy a claim's representation target merely by sharing an ID. The replay-contract workflow now runs the CLI against the canonical fixture and checks the one-transition/one-measured-claim/one-correspondence report. These tests remain synthetic contract checks, not IFC or physical-building validation.

## Metrics and targets

These are research targets, not achieved results.

| Track | Metric | P0 target |
|---|---|---|
| Evidence lineage | Measured target claims with complete direct source chain | 100% |
| Claim origin | Inferred/assumed/unknown promotion to measured without raw evidence | Zero |
| Uncertainty | Representation-only shrinkage and unknown-to-known promotion | Zero |
| Approximation | Declared residual vs output absolute bound | 100% of analytic fixtures conserve the bound |
| Identity | One-to-one/split/merge/replacement cardinality | 100% expected cases |
| Unresolved mapping | Ambiguous/unresolved correspondences | Explicitly reported; never PASS as preserved |
| Reproducibility | Input-order-invariant receipt | 100% on pinned fixtures |
| External calibration | Predicted interval coverage vs held-out metrology | Nominal 95% intervals target empirical 95% coverage, measured on independent buildings |

No weighted composite can override an orphaned measured claim, unjustified uncertainty reduction, identity collision or unresolved hard requirement.

## Model readiness and validation maturity

The programme now has a separate scorecard derived from the October 2026 Scan-to-Engineering review's EMRL/VM framing: [SCAN_BIM_MODEL_READINESS_AND_VALIDATION_MATURITY_V0_1.md](SCAN_BIM_MODEL_READINESS_AND_VALIDATION_MATURITY_V0_1.md).

We report two axes, never one composite:
- **EMRL-1…4:** geometry-ready, information-ready, analysis-ready, decision-ready (task-conditional model capability).
- **LVM-0…5:** a Luminous-defined validation ladder from unvalidated/pipeline-only through synthetic contracts, published benchmark replay, independent metrology, independent interoperability/replication and longitudinal operational validation.

The LVM names are our proposed operationalization; we do not claim they reproduce the paper's exact VM-0…5 criteria without reviewing its full text. A rich model with weak evidence must not be presented as decision-ready. Every level is bound to an exact task, requirement profile, model revision, data population and evidence window.

## P1 evaluation

1. Compose the audit with #4271's exact IFC/IDS and controlled edit/round-trip outputs.
2. Consume survey transforms and covariance from #1607; do not implement another metrology engine.
3. Consume identity event receipts from #3331.
4. Test a rights-cleared IFC-labelled corpus (BIMNet subject to access/licence review) and ISPRS where the selected metric applies.
5. Use whole-building splits; never let points from one building leak across development/evaluation groups.
6. Compare at least two independent import/edit/export toolchains and report exactly what evidence, properties, relations and uncertainty references survive.
7. Calibrate uncertainty only against held-out independent checkpoints, not the data used to fit geometry or tune the reconstruction.

## Claim ceiling

P0 establishes only declared receipt semantics on synthetic examples. It does not establish:
- that a scan, survey solution, transform or checkpoint is physically accurate;
- that an uncertainty estimator is calibrated;
- that a checkpoint is truly independent or properly surveyed;
- that a mesh/IFC parser or extractor is correct;
- that the target geometry is an accurate as-built model;
- that identity mapping is authoritative outside the declared receipt;
- that model changes are safe, code-compliant or authorized.

A future P1 conformance claim must name the exact dataset, independent tool versions, schema/profile, survey evidence and validation population.
