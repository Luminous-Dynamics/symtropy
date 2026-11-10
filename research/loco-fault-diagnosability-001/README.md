# LOCO-FAULT-DIAGNOSABILITY-001 — Action-conditioned fault identifiability and ambiguity

**Status:** finite interval-box reference and synthetic fixtures. No physical fault-detection or safety claim.

Issue: [#4470](https://github.com/Luminous-Dynamics/symtropy/issues/4470).

## Objective

Certify when currently plausible actuator/contact fault hypotheses are distinguishable from the sensor evidence available under a specific diagnostic action and operating context—and when they must remain ambiguous.

This complements:
- #4358/#4405 matched fault injection and diagnosis campaign;
- #4438/#4441 fault admission and graceful recovery;
- #4310/#4313 viability-conditioned value of information;
- #4004/#4230 contact evidence and provenance;
- #4119 action-time evidence validity;
- #4061/#4083 robust viability and hybrid transition semantics.

It does not create a sensor/actuator estimator, classifier, physical viability solver or probe selector.

## Paradigm shift: from classifier confidence to diagnosability

A classifier can have high average accuracy while two physical fault modes remain observationally indistinguishable under a particular action, sensor subset, load or gait phase. Examples include:
- joint lock vs encoder freeze;
- torque derating vs increased friction;
- actuator delay vs clock offset;
- contact slip vs actuator weakness;
- actuator fault vs correlated encoder/velocity fault.

A model cannot infer which of two hypotheses occurred when their possible observation sets overlap. In those cases a calibrated ambiguity set is the correct output, not a forced top-1 label.

## Research and industry reference points

- **Tian, Active Self-Diagnosis and Diagnosis-Gated Heterogeneous Recovery** (SSRN preprint, posted 24 Aug 2026): a Go2 MuJoCo digital twin performs a squat/hip-sway diagnostic movement exciting 12 actuated joints, estimates fault identity/type/severity/confidence, then selects among recovery modules. Treat as an exploratory preprint, not independently peer-reviewed hardware qualification. https://papers.ssrn.com/sol3/papers.cfm?abstract_id=7345456
- **ARER-MetaRL** (PLOS ONE, 16 Oct 2026): context inference and modular routing for locked-joint faults, up to six locked joints in Isaac Sim. This is an adaptive inference baseline, not an identifiability certificate. https://doi.org/10.1371/journal.pone.0361099
- **Adaptive Gait Timing** (arXiv:2608.07328, 7 Aug 2026): proprioceptive history reconstructs fault-relevant latent state and learns gait frequency; high-fidelity uneven-terrain simulation and flat-ground experiments on a 68 kg Kyon quadruped. https://arxiv.org/abs/2608.07328

The opportunity is to qualify when the supplied, source-pinned observation envelopes support unique isolation, before confidence-based policy routing.

## V0 set-valued observation contract

For a fault hypothesis h, diagnostic action u and fixed operating context c, a separately qualified source supplies a conservative observation envelope:

    Z(h, u, c) ⊆ R^d

V0 represents Z as an axis-aligned **closed interval box** over one exact feature schema. Every family binds:
- diagnostic action ID, duration and upstream viable-continuation receipt;
- physical fault-set digest and complete still-possible hypothesis IDs;
- model, actuator-response, domain and abstraction digests;
- sensor manifest, calibration and clock-map digests;
- stable feature IDs, units and integer fixed-point intervals;
- validity interval and evaluation time.

For hypotheses i,j, their boxes are certified separable iff at least one shared feature has disjoint closed intervals:

    upper_i[k] < lower_j[k]  OR  upper_j[k] < lower_i[k]

Touching endpoints overlap. If every dimension overlaps, the hypotheses are not certified distinguishable for this action/context. If either envelope is Unknown/unbounded, the pair is Unknown-overlapping, never separable.

Build an ambiguity graph with an edge for every overlapping or Unknown pair. Report connected components as conservative ambiguity groups. Unique model-conditional isolation requires a singleton component and a bounded envelope.

For an actual observation:
- evaluate against every envelope in the authoritative physical hypothesis set;
- one containing bounded envelope yields one model-conditional candidate;
- multiple containing envelopes yield the complete ambiguity set;
- an Unknown envelope prevents unique isolation;
- no matching envelope yields OOD/Unknown, never nearest-class forced assignment;
- missing/stale/unqualified sensor data yields Unknown or Invalid.

The controller's top-1 posterior cannot narrow the physical fault possibility set. #4438 must continue to admit actions over all still-possible hypotheses until independent evidence changes that set.

## Gate order

1. #4119 validates sensor/clock/calibration/actuator/model/domain identities and freshness.
2. #4313/#4293/#4061 independently admit the physical diagnostic action and every post-probe branch/latency continuation.
3. This auditor checks the supplied action-conditioned observation envelopes and reports pairwise separability/ambiguity.
4. Actual observations may narrow the *diagnostic candidate set* only when their source/schema/time are valid; this does not itself authorize a control action.
5. #4438 chooses only actions admitted across every physical fault hypothesis still possible. Ambiguous, Unknown and OOD observations preserve the conservative set.

Identifiability is not safety. A uniquely identified fault does not prove a compensating gait is viable.

## Numerical and identity rules

- Integer fixed-point features; checked range/schema validation; no floating epsilon.
- Exact common feature IDs and units across all bounded envelopes.
- Closed intervals; invalid if lower > upper.
- Unknown envelopes overlap every other hypothesis.
- Sensor common-cause group IDs are retained; channels derived from one encoder are not independent observations.
- Canonical ordering by hypothesis and feature ID; outputs invariant to input order.
- A narrower envelope than the upstream qualified bound is not accepted. V0 consumes a receipt/digest; independent source qualification remains external.
- V0 supports boxes only. For multimodal/non-rectangular observation sets, supply a conservative outer box or keep the result Unknown. Do not shrink bounds to force separability.

## Frozen known-answer/adversarial corpus

Machine-readable cases are in [fixtures/known_answers_v0_1.json](fixtures/known_answers_v0_1.json):
1. locked vs derated: disjoint velocity-residual intervals, separable;
2. low-load lock vs friction increase: overlap, preserve ambiguity;
3. touching interval endpoints overlap;
4. diagnostic excitation separates hypotheses that overlap under quiet stance;
5. an Unknown fault envelope overlaps all hypotheses;
6. sensor dropout/missing feature: no unique isolation;
7. actual observation inside one bounded envelope: one model-conditional candidate;
8. observation in multiple envelopes: complete ambiguity set;
9. observation outside every envelope: OOD/Unknown;
10. stale calibration/clock/actuator receipt: refuse;
11. changed physical fault set: refuse result reuse;
12. feature ID/unit mismatch: Invalid;
13. permutation of hypotheses/features preserves report;
14. sensor channels sharing an encoder retain common-cause identity.

## Matched comparisons

- Tian's squat/hip-sway active diagnostic maneuver (exploratory, simulator-only evidence): fixed probe vs random excitation vs sequential joint excitation vs viability-conditioned choice.
- ARER-MetaRL short-history inference: compare top-1 accuracy, false unique-isolation, true-fault-in-set coverage and ambiguity cardinality.
- Adaptive Gait Timing: compare history-conditioned latent inference and gait-frequency adaptation with set-based diagnosability output.
- Model-based residual/parity observers: matched sensors, fault classes, operating contexts and latency.
- Symtropy ablations: no envelope model, no common-cause metadata, no set-valued diagnosis, no viability admission.

Unavailable/unmatched baselines are literature-only; no reproduction is implied.

## Scorecard

Keep separate:
- pairwise separable / overlapping / Unknown hypothesis counts;
- ambiguity graph edges and component-size distribution;
- actual-observation unique / ambiguous / OOD / Unknown rates;
- false unique-isolation against evaluator-only ground truth;
- true fault containment in the candidate set;
- envelope calibration/coverage on held-out measured hardware trials;
- diagnostic action duration, energy, actuator peak and observation latency;
- source identity, feature coverage and sensor common-cause metadata;
- upstream viable-continuation receipt and remaining horizon.

Do not optimize top-1 accuracy alone. Prefer a broad correct ambiguity set to an overconfident wrong label.

## Proposed targets (not achieved)

- 100% agreement with independent brute-force box-intersection known answers.
- Zero unique isolation when intervals overlap or any plausible envelope is Unknown.
- Zero stale/mismatched model, actuator, sensor, calibration, clock, domain or physical-fault-set reuse.
- Zero OOD observations forced into a known class.
- Exact invariance under hypothesis/feature input permutations.
- Held-out hardware target: measured envelope coverage must be preregistered and maintained; any undercoverage invalidates unique-isolation claims in that domain.

## Claim ceiling

V0 qualifies finite interval geometry and identity/schema composition only. It does not prove that supplied envelopes cover real robot observations or that a fault is physically identifiable. Hardware claims require measured source envelopes, independent calibration/coverage validation and upstream viability receipts.


## 11. Executable finite Rust V0 reference

The standalone dependency-free Rust crate consumes:
- a complete authoritative physical fault-hypothesis set;
- one bounded interval box or explicit Unknown envelope per hypothesis;
- an exact feature/channel/unit schema with common-cause group IDs;
- source/model/actuator/domain/sensor/calibration/clock identities;
- a fresh diagnostic-action receipt carrying the upstream #4313/#4293 viable-continuation receipt digest;
- an optional timestamped actual observation.

It:
- validates that the diagnostic action was independently admitted and is current;
- requires envelope coverage for every still-possible physical hypothesis;
- computes pairwise separability using closed integer intervals;
- reports the strongest separating feature/margin for each separable pair;
- builds canonical connected components for the ambiguity graph;
- returns a model-conditional unique candidate, full ambiguity set, Unknown candidate or OOD disposition for a measured observation;
- refuses missing/mismatched features, units, clocks, calibration and physical-fault-set identities;
- makes Unknown envelopes overlap all hypotheses;
- preserves common-cause group identifiers in the result;
- canonicalizes hypothesis/feature order.

Seventeen deterministic tests cover separable and overlapping fault signatures, touching intervals, Unknown hypotheses, unique and ambiguous observations, OOD observations, missing sensor features, stale/mismatched receipts, non-admitted probes, feature schema/unit mismatch, permutation invariance, invalid intervals and common-cause metadata.

Qualification commands:

    cargo fmt --manifest-path research/loco-fault-diagnosability-001/Cargo.toml -- --check
    cargo test --manifest-path research/loco-fault-diagnosability-001/Cargo.toml --locked
    cargo clippy --manifest-path research/loco-fault-diagnosability-001/Cargo.toml --locked --all-targets -- -D warnings

A green exact-head workflow qualifies finite interval geometry and evidence-schema composition only. It does not prove that source envelopes cover physical sensor distributions or that a fault is physically diagnosable.
