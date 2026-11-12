# Temporal Scan-to-BIM Sequence Replay v0.1

**Status:** P0 synthetic sequence evaluator implemented; exact-head Rust/CI qualification pending.  
**Owner:** #4846 — SCAN-BIM-TEMPORAL-SEQUENCE-001  
**Composes:** #4116 replay; #3331 component identity; #1607 survey evidence; #4086/#4088 admission; #4705/#4749 point-in-time evidence; #1226 blind custody.

## 1. Why sequence evaluation is a different task

Single-frame or ordered-tile association scores do not establish longitudinal identity continuity under active construction. A temporal Scan-to-BIM system must distinguish:

- the same physical component observed under different viewpoints;
- a component temporarily hidden by occlusion;
- nuisance registration drift;
- a true physical modification;
- removal and replacement by a distinct component;
- missing/late evidence that could not have informed an earlier decision.

This benchmark's unit is a **time-indexed building sequence**. All captures, tiles, epochs and derivatives from the same building remain one holdout group. Repeated frames never inflate the independent building count.

## 2. 2026 comparator: LiBiDT

Liu et al., *Multi-line LiDAR 3D construction environment modeling and BIM consistency update method for digital twins*, Scientific Reports 16 (2026), 27367, published 15 June 2026, is a strong recent temporal Scan-to-BIM comparator: https://doi.org/10.1038/s41598-026-58063-3

On CV4AEC 2024 Scan-to-BIM, the paper reports:
- association F1 = **0.837**;
- IDF1 = **0.861**;
- identity switches = **104**;
- Chamfer Distance / surface RMSE = **3.2 / 3.5 cm**;
- stabilized F1 = **0.847**;
- per-tile latency = **71.9 ms** on RTX 4090, batch size 1 with AMP.

The paper's strongest listed comparison, CoFiNet (Reg) + S2B, has association F1 0.822, IDF1 0.854 and 118 ID switches. The larger reported gains are temporal identity consistency and the EMA ablation, not a large association-F1 step change. These are the authors' study-specific figures and protocol, not universal industry benchmarks. [Full text](https://pmc.ncbi.nlm.nih.gov/articles/PMC13529799/) · [results table PDF](https://doi.org/10.1038/s41598-026-58063-3).

**Important limitation stated by the authors:** strict temporal validation using time-stamped multi-acquisition data under active construction is left to future work. The ordered floor-wise tile-stream case study is not equivalent to a held-out multi-epoch active-construction trial. This benchmark targets that missing validation layer.

Do not claim a LiBiDT reproduction until its exact method, code/reimplementation, split, labels, metrics and CV4AEC source data are available and pinned. A published table is a comparator, not a license to approximate its method or relabel an unmeasured run.

## 3. Paradigm: persistent physical state, not per-frame matching

```text
timestamped partial observations
       ↓
qualified survey / registration / time evidence
       ↓
persistent physical-component correspondence
       ↓
event attribution: unchanged / nuisance / physical change / replacement / unknown
       ↓
candidate state-transition disposition
       ↓
sequence replay + independent scoring
```

This evaluator does not generate or own physical IDs. Candidate track IDs are scored against evaluator-only physical entity IDs. #3331 remains the component identity owner and supplies identity-transition semantics. The evaluator exposes fragmentation, ID switches and false event attribution without becoming a second registry.

## 4. P0 implementation

The Rust crate is `tools/scan-bim-temporal-sequence-benchmark/`. Candidate outputs and evaluator truth are separate JSON inputs. The evaluator binds exact input bytes with SHA-256 in its report.

Initial seven sequences:
1. unchanged component across three acquisitions;
2. temporary occlusion followed by reappearance;
3. viewpoint-only change;
4. suspected registration drift;
5. qualified physical displacement beyond the task tolerance;
6. late-arriving scan that must not enter an earlier decision;
7. explicit component replacement with a new physical identity.

The reference fixture contains evaluator-only identity, visibility, event and time-validity facts. The candidate fixture contains track IDs, reported events, dispositions and exact evidence-event references.

## 5. Metrics and definitions

| Metric | P0 definition |
|---|---|
| IDF1 | (2,IDTP/(2,IDTP+IDFP+IDFN)), where IDTP comes from maximum-weight one-to-one matching of ground-truth entities to candidate tracks over visible observations |
| ID switches | Number of times a persistent evaluator entity is assigned a different candidate track across successive visible observations |
| Fragmentations | Number of re-acquired candidate track segments after a visible miss for an entity that remains visible |
| Event agreement | Exact match of event class and expected disposition for each sequence step |
| False physical-change events | Nuisance/unresolved truth reported or admitted as a physical change |
| Temporal evidence violations | Missing/duplicate event reference, unknown clock, inadequate precision, future measurement, late receipt, invalid-at-cutoff, known revocation, or unauthorized decision purpose |

P0's identity assignment uses a bounded exact solver (16 entities/tracks per sequence maximum). Larger external runs require a tested scalable assignment implementation with identical known-answer vectors. Do not silently replace the metric with greedy matching.

## 6. Temporal evidence boundary

V0.1 uses a single synthetic integer tick clock, explicitly named `synthetic-global-tick-v0`. It checks:

- measurement/event time ≤ decision cutoff;
- first-received/knowledge time ≤ decision cutoff;
- evidence validity interval contains the cutoff;
- a revocation known at/before cutoff excludes the evidence;
- a revocation learned after cutoff does not rewrite the prior snapshot;
- evidence is authorized for the declared decision purpose;
- NDT/repair outcomes do not leak into medium-loop building updates;
- timestamp precision is non-zero and no coarser than the declared decision resolution.

This is not a new bitemporal engine. General clock conversion, event selection, authority and receipt authentication remain with #4705/#4749 and existing evidence owners. The sequence evaluator audits only the declared synthetic receipt references.

## 7. Promotion and benchmark targets

### P0 synthetic contract
- 100% agreement on the frozen sequence outcomes.
- IDF1 = 1.0, zero identity switches and zero fragmentation on the known-good fixtures.
- zero nuisance-only false physical-change events.
- zero late/future/unauthorized/known-revoked evidence use.
- one positive qualified physical-change event is detected.
- deterministic semantic report under sequence, step, observation and evidence-array permutations; the exact input digests still change.

### P1 external temporal replay
- Reproduce the published LiBiDT CV4AEC protocol separately, where source code/data/labels and metric definitions are available.
- Construct genuinely timestamped multi-acquisition sequences across active construction/change, not only within-floor tile order.
- Hold out whole buildings. Keep all epochs, views, tiles, trajectories and processed derivatives of a building in one group.
- Use independent survey/checkpoint truth and #1226 blind custody; reference identity/event labels never enter candidate inputs.
- Compare LiBiDT/source-faithful baseline, CoFiNet + S2B, Scan2BIM-Net where reproducible, expert/manual associations, and pinned commercial outputs where they expose comparable identity decisions.
- Report IDF1, ID switches, fragmentation, event attribution, geometry error, temporal leakage, abstention and per-building denominators separately.
- Report latency at matched hardware and batch size. The published 71.9 ms RTX 4090 figure is not comparable to another device or an end-to-end capture workflow.

No scalar composite score may trade an ID switch, false physical-change event or temporal evidence leak against lower Chamfer distance.

### P1 proposed aspiration (not an achieved result)
At equal acquisition and compute budgets, first reproduce the published IDF1 0.861 on its exact protocol. Then target a statistically supported reduction in ID switches and nuisance false-change attribution on held-out active-construction sequences without degrading geometry or true-change sensitivity. Set numeric superiority thresholds only after the baseline distribution and effective building count are known.

## 8. Claim ceiling

P0 proves sequence-evaluator semantics on synthetic fixtures only. It does not prove real physical identity, sensor calibration, survey accuracy, construction safety, code compliance or SOTA superiority. P1 claims are limited to named buildings, capture protocols, sensor profiles, metric implementations and independent reference sources.
