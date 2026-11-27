# AV Spatial Observability World-Pair Binding v0 — audit protocol

**Status:** evaluator-side registry scaffold. All nine pairs are currently `unbound`.  
**Parent:** [#7769](https://github.com/Luminous-Dynamics/symtropy/issues/7769)  
**Scenario source:** [identifiability matrix](./scenario-matrix-identifiability-v0.tsv)  
**World contract:** [#1587 / PR #1587](https://github.com/Luminous-Dynamics/symtropy/pull/1587), with coordinate-frame extension in [PR #1589](https://github.com/Luminous-Dynamics/symtropy/pull/1589)  
**Agent observation contract:** [#6201](https://github.com/Luminous-Dynamics/symtropy/pull/6201)  
**Independent map evaluator:** [#7693](https://github.com/Luminous-Dynamics/symtropy/issues/7693)  
**Blind custody:** [#6754](https://github.com/Luminous-Dynamics/symtropy/issues/6754)

## 1. Purpose and ownership

This registry binds each public identifiability pair ID to two evaluator-side world-manifest artifacts and the candidate-visible stream commitments generated from those worlds. It is a thin crosswalk, not another world, sensor, frame, replay, or truth schema.

- World structure, sensor profiles, frames, world generations and solver references are owned by `WorldManifestV1` from #1587/#1589.
- Candidate-visible raw video/PCM artifacts, clock generations, capture intervals and availability-ordered transport events are owned by #6201.
- Scenario IDs and world-generation semantics remain with #1595.
- Hidden-world matching and spatial-claim validation remain with #7693.
- Process isolation, sealed truth and custody receipts remain with #6754.

This registry is evaluator-only. It must never be serialized into the candidate workload, made available through environment variables, paths, artifact names or debugging responses, or mounted into the candidate process.

## 2. Current binding inventory

The registry is intentionally initialized with all nine matrix pairs and explicit nulls. **Null means unbound, not a wildcard, equality assertion or default value.** No manifest, media, or observation is implied by a pair ID.

| Pair | Mode | Initial state |
|---|---|---|
| `pair-coverage-absence-01` | `coverage_qualified_negative_evidence` | UNBOUND |
| `pair-delayed-identifiability-01` | `shared_prefix_then_probe` | UNBOUND |
| `pair-global-gauge-01` | `gauge_equivalent_worlds` | UNBOUND |
| `pair-instance-identity-01` | `shared_prefix_then_instance_cue` | UNBOUND |
| `pair-motion-decomposition-01` | `shared_prefix_then_independent_constraint` | UNBOUND |
| `pair-probe-decision-invariance-01` | `shared_prefix_then_decision_invariant_probe` | UNBOUND |
| `pair-scale-anchor-arrival-01` | `shared_prefix_then_metric_anchor` | UNBOUND |
| `pair-symmetry-breaking-01` | `shared_prefix_then_traversal` | UNBOUND |
| `pair-world-equivalence-01` | `identical_prefix_then_probe` | UNBOUND |

## 3. Required WorldManifestV1 binding

For each A/B world slot:

1. Resolve an exact, immutable manifest artifact and its SHA-256 over the exact bytes.
2. Parse and structurally validate the referenced `WorldManifestV1` schema revision. Record `scenario_id`, `world_generation`, and manifest schema version from the parsed artifact; do not copy them from a hand-authored label.
3. Resolve every referenced frame, sensor profile, calibration artifact, world asset and relevant solver by the identifiers and digests in the owning manifest.
4. Verify every sensor profile's frame reference belongs to that manifest's frame topology. A camera profile is not implied by the scalar-only `SensorProfileV1` channel model.
5. Keep WorldManifest validation distinct from physical validity: manifest admission does not prove camera photometry, lens distortion, acoustic propagation, clock synchronization or real sensor calibration.

The manifest artifact reference in this registry uses the existing `ArtifactRef` shape: `artifact_id`, `uri`, and `sha256`. It does not inline or fork `WorldManifestV1`.

## 4. Candidate-visible prefix equivalence audit

For pairs with a shared/identical prefix, equality is a result of rendered and replayed artifacts, not a declarative flag.

At the preregistered decision cutoff, independently compute the canonical candidate-input bundle for world A and world B from:
- admitted raw media artifact bytes and exact digest/length;
- observation metadata and capture support intervals;
- clock-domain identity including generation and time-scale;
- calibration/sensor-profile references exposed to the candidate;
- transport disposition, availability tick and arrival order;
- transcript/catalogue order and any permitted session metadata.

For `byte_exact` admission, require:
- identical byte digest and byte length for each paired candidate-visible artifact;
- identical canonical observation/transport transcript digest through the cutoff;
- identical exposed sensor/calibration/profile and query/action contract digests;
- no pair ID, world key, hidden seed, truth coordinate or hidden-label side channel in candidate input, file path, event ID, log, environment or API response.

The two per-world digests are both recorded; an auditor compares them. JSON Schema cannot enforce equality between A/B fields. A schema pass alone never advances `binding_status`.

For stochastic sensing, do not call two streams distributionally equivalent because a finite sample happens to match. Freeze an equivalence margin, test statistic, replicate count, confidence/error allocation, seed policy and held-out nuisance strata before generating the cohort. A non-significant difference test is insufficient unless the preregistered equivalence test is powered and its confidence interval lies inside the declared margin.

## 5. First distinguishing observation and temporal rules

Where a pair has a later distinguishing observation:
- its capture support must be grounded in the owning observation contract;
- its availability tick must be strictly later than the first decision cutoff;
- it must be an actual delivered observation, not a hidden-world delta or evaluator event;
- its artifact digest and observation identity must resolve to the #6201 replay ledger;
- capture time and availability time remain distinct; cross-clock comparisons require an admitted clock transform from the canonical owner.

A dropped, late, denied, stale, malformed, or incomparable observation cannot be promoted to the first distinguishing evidence. It remains an explicit failure/unknown/out-of-domain case.

For the decision-invariant pair, the observation must change the evaluator's admissible topology/identity set but leave the current DecisionNeed action partition unchanged. Record both results; do not count reduced map uncertainty as decision-value gain.

## 6. Readiness promotion

Promotion is monotonic and evidence-gated. Each pair carries a `readiness_evidence` object with explicit artifact references for:
- `manifest_validation_report`;
- `prefix_equivalence_audit`;
- `replay_run_receipt`;
- `rights_admission_receipt`;
- `split_integrity_receipt`;
- `custody_receipt`;
- frozen candidate commit SHA and model/config SHA-256.

A readiness-stage label without its required receipt is invalid. The Draft 2020-12 schema enforces stage-local requirements; CI additionally enforces registry-wide consistency and rejects false-promotion mutations.

Promotion is monotonic and evidence-gated:

1. `unbound`: pair IDs and intended mode only; no world or candidate artifact claim.
2. `manifest_bound`: both exact WorldManifestV1 artifacts resolve and pass the owning structural validator; frame/sensor/profile references resolve.
3. `prefix_committed`: canonical per-world candidate-prefix and transcript digests have been computed from the exact #6201 candidate workload. The audit has checked byte/digest equality or the preregistered distributional equivalence test as appropriate.
4. `rendered_replay_verified`: required media exists, integrity is verified, the actual #6201 causal replay reproduces the availability and transport contract, and a replay receipt is committed in `readiness_evidence`. The first distinguishing observation is confirmed where required.
5. `blind_ready`: #6754 custody, rights admission, split-integrity, isolation negatives, frozen candidate/model/config and attempt-ledger requirements are independently satisfied and each exact receipt is committed in `readiness_evidence`.

No stage may be inferred from the presence of a manifest URI, a schema-valid JSON record, a PNG capture receipt, or a queued Actions workflow. Missing or unsupported rendering/sensor capability remains `NOT_EVALUATED`; it is never converted to a lower-quality PASS.

## 7. Pair-mode specific checks

- `identical_prefix_then_probe`: worlds differ in hidden topology while candidate input bytes and metadata remain identical through the cutoff; a later permitted probe is the first visible discriminator.
- `shared_prefix_then_probe`: retain the ambiguous alternatives until the declared later evidence is actually delivered.
- `gauge_equivalent_worlds`: world truth differs only by the declared global SE(3) gauge; relative geometry, distances and graph structure must be invariant under the evaluator alignment. Gauge alignment is hidden.
- `shared_prefix_then_metric_anchor`: monocular relative geometry remains scale-ambiguous until a metric anchor with source/calibration provenance arrives.
- `shared_prefix_then_instance_cue`: similar appearance and occlusion do not establish physical-instance identity; only the admitted distinguishing observation can resolve it.
- `shared_prefix_then_independent_constraint`: independent time/pose evidence must separate ego-motion from object motion without counting shared roots twice.
- `shared_prefix_then_traversal`: symmetric graph embeddings stay unresolved until the transition/traversal evidence arrives.
- `coverage_qualified_negative_evidence`: out-of-view and occluded locations remain Unknown; absence is scoped to declared coverage, time and detection envelope.
- `shared_prefix_then_decision_invariant_probe`: a later observation revises map detail but preserves the current decision class; map identification and decision value are scored separately.

## 8. Structural checks already run

Authoring-time structural checks against the source matrix confirmed:
- nine unique pair IDs;
- two unique case IDs per pair;
- one `control` and one `mutated` row per pair;
- ten TSV fields per scenario row;
- exact pair/case inventory was copied into this registry.

These checks do not validate a WorldManifestV1 artifact, rendered media bytes, candidate-prefix equality, runtime replay, schema conformance, custody or a model result. The JSON Schema and registry still need an independent Draft 2020-12 validator in repository CI.

## 9. Explicit nonclaims

No WorldManifestV1 pair has yet been bound, no indoor camera producer has been qualified, no candidate media has been rendered/replayed for these pairs, no hidden truth is sealed, and no model has been scored. This contribution only makes the evaluator-side binding contract and its unbound state explicit.


## 10. Evidence-gated schema and negative controls

The current registry is all `unbound`, with every readiness receipt, world manifest, truth crosswalk, prefix digest and replay reference explicitly null.

The schema's conditional branches enforce:
- manifest artifact + truth crosswalk + manifest-validation receipt before `manifest_bound`;
- exact candidate-input/transcript digests, shared sensor/trajectory/sampling/transport/schema commitments and a prefix-audit receipt before `prefix_committed`;
- replay receipt before `rendered_replay_verified`;
- rights, split-integrity and custody receipts plus frozen candidate/model identities before `blind_ready`.

Registry-wide stages cannot overstate the least-ready pair. The exact-head workflow tests five invalid promotion mutations (manifest, prefix, replay, blind-ready pair and blind-ready registry) and must reject each. A JSON Schema check does not prove receipt authenticity or physical truth; the independent owners still validate each referenced artifact and custody claim.


## 11. Statistical equivalence for stochastic prefixes

A deterministic fixture must use byte-exact equality where the renderer/input contract promises determinism. A stochastic sensor pair may use `distributionally_equivalent` only after a preregistered equivalence analysis; a non-significant difference test is not an equivalence result.

The evaluator must commit:
- metric/statistic and its raw calibrated units;
- lower/upper equivalence margins with operational justification;
- alpha and confidence level (for TOST at alpha .05, the corresponding 90% interval must lie wholly inside both bounds);
- target power at least .90 and sample-size plan;
- independent cluster unit (scene, episode, recording session, sensor run or calibration block), cluster count, within-cluster replicates and paired-seed schedule;
- multiplicity family and adjustment;
- preregistration, raw-results and analysis-code receipts.

Frames/samples within one scene/session/episode are correlated; do not count them as independent units. The required independent cluster count must meet the preregistered sample-size plan. If the plan is underpowered, return `UNDERPOWERED` / `NOT_EVALUATED`. For multivariate distributions, use a preregistered distance/statistic (e.g. energy distance, MMD or Wasserstein) with a justified upper equivalence bound; do not choose embeddings/thresholds after observing results.

The schema and exact-head workflow enforce required artifacts and cross-field conditions. This establishes a valid evidence envelope only; the evaluator must still review the statistical design and reproduce the reported result independently.
