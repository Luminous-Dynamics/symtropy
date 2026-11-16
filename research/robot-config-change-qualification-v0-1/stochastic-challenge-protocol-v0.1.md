# Stochastic challenge protocol v0.1

Status: proposed methodology for later held-out and physical comparison. Not yet implemented or validated.

## 1. Purpose

A fixed public test set is useful for debugging, but it is a weak final test of whether a robot-configuration change-impact system generalizes. Participants can memorize case identities, vendor/model strings, known deltas or the public oracle.

This protocol standardizes a **distribution of reproducible configuration changes and claim profiles**, while withholding the evaluator's sampled seed set until the scored run is complete.

It adapts the stochastic-challenge method described by Gläßer et al. (CoRL 2026, arXiv:2611.12804) to robot configuration/evidence applicability. It does not claim to reproduce their mobile-manipulation benchmark or statistical model.

## 2. Public generator, private scored samples

Publish and pin:
- challenge-generator source revision and build provenance;
- generator schema and canonicalization version;
- component and interface catalog;
- admissible parameter ranges and cross-parameter constraints;
- change-family probabilities and stratification policy;
- ODD strata and claim-criticality definitions;
- deterministic PRNG algorithm/version;
- generation procedure, rejection rules and case-normalization rules;
- independent oracle version and scoring protocol, without publishing evaluator-held answer keys.

For each challenge sample, bind:
- generator revision and manifest digest;
- seed identifier and evaluator-held seed material;
- sampled source configuration and target delta;
- claim/dependency profile revision;
- evidence/applicability profile and relevant ODD;
- source and target canonical configuration identities;
- evaluator-only expected claim dispositions;
- all rejected draws and the reason for rejection, so sampling is auditable.

Participants may know the generator and distribution. They must not receive the private scored seed set or labels before committing their candidate output. A seed commitment is a custody/integrity aid, not proof that the generator or physical observation is authentic; use the existing blind custody owner (#1300) for any true blind campaign.

The 16-case public corpus in this directory remains the development set. Never mix its score with a held-out stochastic challenge score.

## 3. Sampling model

Sample configuration changes at the level of **complete robot articles/configurations**, not independent claims or video frames.

Stratify across:
- platform family and serial/article generation;
- base/drive, arm/joint, tool/end-effector, payload, sensor/mount/calibration, battery/power, firmware/controller, safety chain, timebase and frame/site-context change families;
- criticality (safety, quality, operational);
- in-envelope, boundary, out-of-envelope and unknown transformations;
- single changes and selected pairwise/three-way interactions;
- ODD strata and task/acceptance profiles;
- evidence completeness, common-cause lineage and referee independence.

Keep common seed sets for paired robot/lab comparisons where practical. Also draw evaluator-held configuration seeds independently from the public development set to test generalization. Do not make every participant run only the same small, known configuration list.

Sampling rules must preserve physically meaningful constraints. For example:
- payload mass, centre of mass and inertia must be sampled jointly, not independently if the combination is physically impossible;
- sensor mount and extrinsic transforms must be sampled as a coherent pair;
- battery mass, power interface, voltage/current envelope and health state must remain internally consistent;
- drive/tire, surface and stopping claims must be sampled as matched operating conditions;
- firmware/control changes must bind the affected control and safety dependency paths;
- a site/map/frame epoch change must bind exact frame and context identities.

A rejected impossible draw is recorded and replaced according to a preregistered rule. Never silently trim hard cases after seeing system performance.

## 4. Repeats, seeds and uncertainty

Separate at least these sources of variation:
- configuration-seed difficulty;
- attempt-to-attempt variation for a fixed configuration;
- robot serial/article variation;
- laboratory, operator, instrument and calibration-session variation;
- site/environment/ODD variation;
- evaluator/oracle variation where independent implementations are used.

Repeated attempts on one configuration estimate repeatability for that configuration. They do not replace sampling more distinct configuration seeds. Repeated sensor frames, events, heartbeat messages or claim rows do not count as independent robot trials.

Use a pilot to estimate between-seed and within-seed variance for each primary endpoint. Then pre-register:
- the minimum meaningful difference (or equivalence margin);
- type-I error and target power;
- required number of independent configuration seeds;
- repeat attempts per seed;
- cross-lab/common-seed allocation;
- clustering and missing-data/censoring rules;
- handling of safety stops, incomplete runs and not-evaluated cases.

Use an analysis appropriate to the endpoint: hierarchical/mixed-effects or cluster-robust methods for continuous and binary outcomes, and time-to-event/recurrent-event methods for maintenance and failure data. Report intervals and per-stratum results. Do not borrow a sample-size formula from a different benchmark without checking its assumptions.

For the physical round-robin stage, compose #3591 and its metrology/statistical protocol. Do not treat two laboratories as independent if they share a reference clock, calibration root, instrument, raw evidence, model or evaluator code.

## 5. Benchmark-validity diagnostics

Every scored version must include all four diagnostics from the 2026 manipulation-benchmark audit:

1. **Shortcut solvability:** test a trivial model-name/serial/metadata matcher, a fixed lookup table and simple local-component invalidation. If a cheap probe approaches the full system, the benchmark is not demonstrating dependency reasoning.
2. **Statistical significance:** publish per-seed data, interval estimates, sample-size rationale and the detectable effect. Do not treat a rank change smaller than uncertainty as progress.
3. **Creeping overfit:** perturb configurations within the declared support, sample unseen combinations and periodically refresh evaluator-held seeds. Performance must not depend on a narrow known set.
4. **Data-source dependence:** separate source records and evidence used to develop the adapter from evaluator-held challenge records. Report whether the candidate saw the same component catalog, vendor source, serial unit, calibration root or upstream dataset.

Also test:
- label leakage through case IDs, filenames, ordering or metadata;
- oracle leakage through shared parser, canonicalizer, fixture generator or dependency code;
- distribution shift and out-of-domain abstention;
- benchmark gaming by always preserving, always invalidating or returning Unknown;
- sample duplication and accidental pseudo-replication;
- hidden rejection/exclusion of failed, unsafe, incomplete or unreconciled runs.

A valid benchmark should make simple shortcuts fail for principled reasons, not merely because the strings were renamed.

## 6. Scoring and reporting

Report each axis independently:
- safety/quality-critical false-preservation count (hard target: zero);
- claim-level recall for affected evidence, stratified by criticality;
- false invalidation and unnecessary requalification;
- exact agreement with independent exhaustive oracle on supported finite instances;
- UnknownImpact/Unsupported and abstention coverage;
- number and cost of competent evidence requests;
- human reconstruction/review effort;
- compute, latency and memory overhead;
- performance by change family, platform family, ODD, site, lab and serial/article;
- missing, invalid, aborted, safety-stopped and not-evaluated denominators.

Do not rank systems by one composite score. A system that reduces requalification by falsely preserving a critical safety claim fails, regardless of speed or cost.

## 7. Physical challenge gate

The stochastic protocol does not authorize physical testing. A later campaign requires:
- a guarded, task-specific test procedure and risk assessment;
- exact robot/tool/payload/software/safety configuration binding;
- safe and reversible module changes;
- independent calibrated reference instruments;
- metrology suitability for the predeclared decision margins;
- qualified operators and competent independent adjudication;
- test-site permission and applicable authority;
- raw evidence custody and a reconciliation process after power/network loss;
- #3505/#3522 physical trial evidence and #3591 interlaboratory design.

Do not deliberately degrade a safety function, bypass an interlock, expose workers to a hazard, or fault-inject a powered robot outside a separately approved safeguarded method. Synthetic fault injection and physical testing remain different evidence classes.

## 8. External research references

- Gläßer et al., *Benchmarking Real-World Robotic Manipulation through Stochastic Challenges*, CoRL 2026, arXiv:2611.12804: https://arxiv.org/abs/2611.12804
- Stochastic Challenges project: https://rpl-bonn.github.io/stochastic-challenges/
- Jiang et al., *What Are We Actually Benchmarking in Robot Manipulation?*, CoRL 2026, arXiv:2606.04233: https://arxiv.org/abs/2606.04233
- Benchmark audit diagnostics and artifacts: https://ripl.github.io/manipulation_benchmark_audit/
- Symtropy #3591 cross-laboratory reproducibility: https://github.com/Luminous-Dynamics/symtropy/issues/3591
- Symtropy #1300 blind challenge custody: https://github.com/Luminous-Dynamics/symtropy/issues/1300
