# Maritime observability adapter benchmark P0.1

**Owner:** MARITIME-OBSERVABILITY-001 #4609  
**Parent engine:** Symtropy #4005 / `symtropy-sensor-observability-bench`  
**Status:** synthetic adapter conformance; no vessel instrumentation map validated

## Mapping contract

- Asset family = declared engine model/family.
- Asset configuration = exact configuration SHA-256.
- Operating domain = versioned engine regime.
- Installation profile = the declared `layout_profile_ref` plus configuration revision. `sensor_topology_sha256` remains null in P0 because no separate topology artifact bytes have been supplied; the adapter must not manufacture a digest from a label.
- `measurement_principle_ref` maps to the generic independent-principle group.
- `dependency_groups` remain separate and are exercised through explicit outage scenarios.
- State IDs are case-local and bound to `state_taxonomy_ref`; marine fault IDs are not translated to excavation labels.
- Only installed sensors enter the generic layout.
- Non-current quality/calibration and derived/reconstructed source roles cannot supply independent measured intervals.

## Frozen synthetic corpus

Five installed measurement principles, six state hypotheses, eleven required pairs, thirty response cells and two common-cause/outage scenarios. The fixture contains five in-domain states (normal plus four fault hypotheses) and one OOD state. The full unordered in-domain matrix plus reference-vs-OOD control yields eleven pairs: eight isolable, two non-isolable (normal-vs-pressure-sensor-fault; turbocharger-degradation-vs-cooling-restriction), and one not evaluated (normal-vs-OOD). The two latter physical fault signatures are deliberately equivalent; the faulty pressure sensor cannot validate itself.

A complete shared gateway loss withdraws all eight pairs that were isolable in the base synthetic report because every installed channel shares that gateway dependency. The fuel-pressure-transducer loss withdraws no additional pair: independent shaft-speed, exhaust-temperature, rack-position and vibration evidence remains sufficient for those declared pair distinctions. This is a synthetic software conformance outcome, not a physical resilience result.

## Claim ceiling

This demonstrates only adapter mapping and deterministic software treatment of fabricated response intervals and declared outage scenarios. It does not establish real transducer independence, shipboard fault coverage, diagnostic accuracy, maintenance authority, ODD clearance or permission to operate.


## P0.2 — separate detectability from fault isolation

The frozen matrix now contains all unordered pairs among the declared in-domain states and the reference-vs-OOD control. This is necessary because normal-vs-fault comparisons only establish detectability; they do not establish that two different fault hypotheses can be isolated from one another. The synthetic fixture intentionally makes turbocharger degradation and cooling restriction observationally equivalent while keeping both separable from the normal reference. The report must therefore return a shared ambiguity class for those two faults, not two independent unique diagnoses.

Per-fault assessments are derived from the declared reference-state pair and all required fault-fault pairs. A fault indistinguishable from the reference is UNOBSERVABLE; one distinguishable from the reference but not from another fault is assigned an AMBIGUITY_CLASS; OOD is UNSUPPORTED_REGIME; and missing pair evidence remains NOT_EVALUATED. Response-level evidence provenance is checked jointly with sensor source role, calibration and quality; a measured sensor inventory cannot promote a MODEL_ESTIMATE or OPERATOR_REPORTED response into independent measured evidence.


## P0.3 — local sensor clock failure and graceful diagnosability degradation

The common timebase artifact now establishes the clock-domain reference while individual installed sensors retain their own qualification state. UNQUALIFIED/UNKNOWN sensors remain visible in the inventory and are excluded from independent physical response evidence; null sensor-level qualification digest/uncertainty fields are allowed only in those states. A qualified sensor must bind the exact artifact ID/SHA and exact clock domain. An uncertainty claim below the artifact's declared lower bound is rejected; a bound above the layout maximum withdraws that sensor's evidence instead of discarding the complete assessment.

The calibration-outage negative control now checks graceful degradation correctly: losing exhaust-temperature calibration must remove that channel from evidence, but it must not force a pair to NOT_IDENTIFIABLE where independent shaft-speed, rack-position and vibration principles still discriminate it. A separate test preserves MODEL_ESTIMATE provenance on an actual fault hypothesis rather than the omitted healthy reference state.


## P0.5 — exact-head execution evidence (12 November 2026)

- Adapter branch head: `98a35a3c85f5318eacae854849ef5f1fe655a521`.
- Exact-head workflow runs: [43162088096](https://github.com/Luminous-Dynamics/symtropy/actions/runs/43162088096) and [43162080730](https://github.com/Luminous-Dynamics/symtropy/actions/runs/43162080730), both successful.
- The workflow executed 21 Rust unit tests (21 passed), strict Clippy, the frozen corpus and deterministic receipt checks. It uploaded `maritime-observability-adapter-report`.
- Generated case SHA-256: `8dfce2d66cdcca7788a2413c6c4f884864f49c38b7ffb0c5d199fb517073fda2`.
- Generated synthetic clock-artifact SHA-256: `b4d2b0bae0b963afc4612bf7b0b4822e8d8d66cff4c810a2bde6f348d85f0e15`.
- Report result: five installed sensors; five declared independent measurement-principle groups; eleven required pairs (8 isolable, 2 non-isolable, 1 not evaluated); one pairwise ambiguity class; two replayed sensor-loss scenarios.
- The report keeps all six embedded artifact-digest slots null. It binds the exact source commit, case bytes and synthetic clock-artifact bytes separately. Null means the artifact bytes were not supplied; it does not mean a zero hash or inferred provenance.

The passing workflow establishes reproducibility of this synthetic software contract only. It does not establish measured sensor independence, real clock accuracy, physical marine-engine fault separability, vessel performance or regulatory compliance.


## P0.6 — dependency-map review and common-cause coverage

A declared dependency map is itself evidence and can be incomplete. The adapter now requires an explicit per-sensor dependency-review state. Synthetic fixtures are marked `synthetic_conformance` and cannot claim independent review. For measured cases, only `independently_reviewed` dependency maps with a separately hashed review record are eligible to support physical evidence; `unreviewed` and `unknown` sensors remain visible but are excluded.

The validator requires explicit outage scenarios for each dependency group shared by multiple installed sensors. The synthetic suite includes gateway-only, clock-only, combined gateway-plus-clock, and single fuel-pressure sensor loss. The combined case verifies that two shared dependencies failing together do not restore evidence merely because they are declared separately. No physical engine topology or dependency review is claimed by this synthetic fixture.


## P0.8 — synchronized state-window admission

Each fault-state response matrix now requires one common `measurement_window_ref` across all installed channels. The adapter rejects a state whose channels combine different windows. This prevents pairwise signatures from being compared when they were acquired from different operating episodes, transients or controller modes. This is a deterministic contract over declared window identity; it does not independently validate the physical clock network or prove that the windows are correctly segmented.


## P0.9 — exact diagnosability resilience and minimal cut sets

A named outage suite is useful but does not establish the full resilience envelope of an installed measurement layout. P0.9 adds exact combinatorial enumeration over one atom for each installed sensor, one atom for every dependency group shared by at least two installed sensors, and every combination through the declared maximum cut order.

For each critical pair that is isolable in the baseline, the adapter replays the same domain-neutral pairwise evaluator under each outage combination and records every inclusion-minimal set that removes identifiability. It reports minimum cut order and the tolerated failure-atom count before the first cut. A pair already non-isolable or not evaluated in the baseline receives `baseline_not_qualified`; where no cut appears within the declared order, the report states only a lower bound.

The search is deterministic and exhaustive within explicit bounds. v0.1 permits at most 16 failure atoms and cut order at most 2; both values must be declared together or the runtime uses the documented 12-atom / order-2 defaults. Exceeding a bound produces an error, not a sampled or heuristic result. Failure atoms are structural outage hypotheses, not probabilistically independent events. This does not estimate reliability, failure rates or fault probabilities.


## P0.10 — dynamic signatures beyond static response envelopes

A static interval signature can lose temporal ordering and lag information. P0.10 introduces a separate event-aligned trajectory comparison without rewriting the existing static pair report. The synthetic fixture keeps turbocharger-degradation and cooling-restriction ranges identical per channel (same marginal minima and maxima) while giving shaft-speed and vibration trajectories different time orderings.

Admission rules: same declared timebase and event-alignment reference; independently qualified event alignment for measured cases; sensor timebase/calibration/dependency/source-role qualification; exact A/B event-relative sample grid; binding to the corresponding qualified measurement-window references and artifact identities; and a sustained run of pairwise non-overlapping value intervals. The sustained dwell rule prevents a single transient outlier from becoming a diagnosability claim. All time samples from one physical sensor share one measurement-principle group and count at most once.

The same domain-neutral interval/evidence/dependency-cut kernel is used on the aligned time-slice signatures. Results are supplementary: static non_isolable remains static non_isolable; temporal output is a separately scoped result. A synthetic temporal separation is not a marine-engine measurement or validation claim.
