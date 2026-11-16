# ROBOT-CONFIG-MISSION-IMPACT-001 — development artifacts v0.1

Status: public synthetic development package. This is a separate workstream from #5488; it measures mission consequences, not qualification applicability.

Issue: https://github.com/Luminous-Dynamics/symtropy/issues/5547

## Research question

When the same sampled work package is executed under source configuration A and target configuration B, what changes in complete-mission completion, independently accepted output, stage progression, intervention, recovery, resource use and reproducibility?

The benchmark combines:
- seeded scenario distributions and paired A/B conditions;
- stage-aware mission traces rather than aggregate success only;
- accepted-output and quality measurements;
- independent reconstruction and cross-lab variation;
- claim applicability and lifecycle references from existing owners.

## Artifacts

- `mission-impact-protocol-v0.1.md`: matched stochastic study design, stage vocabulary, comparator arms, metrics, statistics, physical-test gate and ownership boundaries.
- `stage-catalog-v0.1.json`: ten-stage reference vocabulary for task contract, site observation, frame/localization, navigation, base positioning, tool/calibration, operation, independent acceptance, recovery and state reconciliation.
- `event-type-catalog-v0.1.json`: explicit 44-event development catalog binding each event to a stage or a non-stage record and permitted source class; not a universal robotics ontology.
- `mission-pair-corpus-v0.1.json`: eight synthetic development pairs with exact source/target configuration references, one declared delta, matched task/site/acceptance conditions and candidate-visible event traces.
- `development-oracle-v0.1.json`: separate public development labels for target mission disposition, first observed divergence, stage progression and outcome sensitivity. This is not a hidden holdout.
- `mission-outcome-metrics-v0.1.json`: separate synthetic source/target outcome measures for accepted/rejected/held/aborted/unknown units, robot time, total crew time, energy, intervention, reinspection and uncertainty-guarded quality.
- `owner-crosswalk-v0.1.json`: proposed MIP-pair to #5488 RCQ-case family crosswalk pinned to the RCQ corpus blob SHA; #5411 transition receipt and #3605 owner acceptance artifact remain explicitly unpinned.
- `commercial-benchmark-crosswalk-v0.1.md`: public-source comparison and fair test tracks for Dusty FieldPrinter 2, HP SitePrint and Built Robotics RPD35/RPS25.
- `reference-workcell-pilot-v0.1.md`: bounded design/readiness plan for one mobile base and two interchangeable modules (survey/inspection and non-load-bearing layout/measurement); no hardware or trial result is claimed.
- `challenge-distribution-v0.1.json`: public synthetic development profile with eight change-family strata and 23 atomic named factor bundles. Bundles are symbolic; they do not yet provide physically qualified parameter ranges.
- `schemas/challenge-distribution-v0.1.schema.json` and `schemas/challenge-manifest-v0.1.schema.json`: closed schemas for the distribution profile and generated challenge manifest.
- `golden-vectors-v0.1.json` and `schemas/golden-vectors-v0.1.schema.json`: a public 128-challenge known-answer vector pinned to the cross-platform manifest digest.
- `schemas/mission-pair-corpus-v0.1.schema.json`: closed Draft 2020-12 schema for paired cases and event traces.
- `schemas/stage-catalog-v0.1.schema.json`: closed Draft 2020-12 schema for the stage vocabulary.
- `schemas/event-type-catalog-v0.1.schema.json`: closed Draft 2020-12 schema for event-to-stage/source mappings.
- `schemas/development-oracle-v0.1.schema.json`: closed Draft 2020-12 schema for public development labels.
- `schemas/mission-outcome-metrics-v0.1.schema.json`: closed Draft 2020-12 schema for synthetic per-pair outcome measurements.
- `schemas/owner-crosswalk-v0.1.schema.json`: closed Draft 2020-12 schema for owner references and conservative family-level case mappings.
- `stage-transition-challenges-v0.1.json`: 18 public synthetic stage-evidence challenges covering command-only completion, self-report, qualified observations, critical referee requirements, stale configuration/frame/calibration/clock evidence, interruption/resume, replay, conflict and safe hold.
- `stage-transition-oracle-v0.1.json` and closed schemas: separate public development dispositions for the finite evidence-gated stage-transition policy.
- `sensor-condition-challenges-v0.1.json` and `sensor-condition-oracle-v0.1.json`: 16 synthetic camera/LiDAR contamination, cleaning, calibration, reference-check and common-cause cases with separate public dispositions.
- `schemas/sensor-condition-challenges-v0.1.schema.json` and `schemas/sensor-condition-oracle-v0.1.schema.json`: closed Draft 2020-12 schemas for the sensor-condition corpus and oracle.
- `validator/`: isolated Rust 1.96 package that validates Draft 2020-12 schemas and cross-file corpus/oracle/event-to-stage/source/outcome/owner-crosswalk invariants; it also contains the deterministic public-development challenge generator and CLI. It validates the finite stage-transition policy and its public development oracle; it does not control a robot, score physical performance or determine qualification.
- `.github/workflows/robot-config-mission-impact-validation.yml`: focused Ubuntu/Windows/macOS schema, fixture, format, Clippy and CLI workflow.

All article IDs, configurations, task references, event IDs, timestamps and outcome-shaped values are invented fixtures. They are not measured robot outcomes or vendor performance data.

## Research motivation

1. Gläßer et al., *Benchmarking Real-World Robotic Manipulation through Stochastic Challenges*, CoRL 2026, arXiv:2611.12804. It proposes standardizing challenge distributions rather than only fixed scenes and reports 568 trials across two robot platforms and two labs. Its authors note that a single researcher rebuilt scenes in both labs, leaving independent-team reconstruction variation unmeasured.
   - https://arxiv.org/abs/2611.12804
   - https://rpl-bonn.github.io/stochastic-challenges/
2. Wang et al., *RoboCasa-Shift*, arXiv:2610.27861, 20 October 2026. Its matched clean/perturbed rollouts show why paired outcome sensitivity and stage-level failure progression can reveal differences hidden by aggregate success.
   - https://arxiv.org/abs/2610.27861
3. He et al., *MOST: Memory of Object States and Task Progress for Long-Horizon Mobile Manipulation*, arXiv:2611.00734, 31 October 2026. It motivates preserving task-relevant world-state changes and subtask progress through long missions.
   - https://arxiv.org/abs/2611.00734
4. Petkov et al., *LADDER: A Long-Horizon Robot Manipulation Benchmark*. It evaluates 145 tasks with 1–10 subgoals and exposes horizon-related failure and orchestration cost; the manipulation results are not construction baselines.
   - https://ladderrobotics.ai/research
5. Jiang et al., *What Are We Actually Benchmarking in Robot Manipulation?*, CoRL 2026, arXiv:2606.04233. It audits shortcut solvability, statistical significance, creeping overfitting and data-source dependence.
   - https://arxiv.org/abs/2606.04233
6. Ping et al., *DexArena: A Unified Benchmark for Long-Horizon Dexterous Manipulation*, arXiv:2611.02629. It uses explicit stage-level success conditions and multiple embodiments/end-effectors.
   - https://arxiv.org/abs/2611.02629

These papers concern general/household mobile manipulation. They inform benchmark methodology; their results are not construction-robot performance comparators.

## Composition

- Symtropy #5488: claim-specific evidence applicability after a configuration delta.
- Symtropy #5411: last accepted A to first accepted B reconfiguration chronology.
- Symtropy #3605: functional unit and accepted-output accounting.
- Symtropy #3523/#3524: task-specific construction-robot performance profiles.
- Symtropy #3505/#3522/#3531: physical procedures, trial receipts and evidence admissibility.
- Symtropy #3591: cross-lab metrology and reproducibility.
- Symtropy #3628: accepted production, wear, maintenance, repair and restoration.
- Symtropy #1834: robot capability/work-package compatibility.
- Symthaea #10543/#4918/#12046: generic change impact, dependency/common-mode lineage and evidence request planning.
- Symthaea #4863/#11866: lifecycle/degradation and productive work versus terminal reserve.

No second capability engine, evidence graph, accepted-output ledger, maintenance ledger, controller, digital-twin truth store or safety authority is proposed.

## Current verification

The parent integration branch's focused validation workflow also runs for pull requests targeting `research/robot-config-mission-impact-v0-1`, so this child research PR is checked at its merge head before composition.

The isolated Rust 1.96 package validates the public schemas and cross-file invariants, generates deterministic public synthetic challenge manifests, checks the 17-case stage-transition corpus against a separate oracle, and checks the 16-case sensor-condition corpus against its separate public oracle. Stage-transition checks fail closed on missing postcondition evidence, critical verifier-class mismatch, stale configuration/frame/calibration, unqualified clock mapping, unauthorized skip, resume revision mismatch, conflicting evidence and duplicate accepted output. Robot command acknowledgement and self-report alone cannot advance a stage.

The public development generator uses domain-separated SHA-256 seed derivation and SplitMix64 sampling; this is reproducible but not cryptographically secure. The focused Linux/macOS/Windows workflow exercises the generator and stage-transition fixtures. Exact-head results must be reported from the current workflow run, not inferred from an earlier revision.

This is not an evaluator-held blind set, a cryptographic custody system, a physical-scene generator, a simulator, a robot connection, HIL, or a physical test. Named factor bundles, stage evidence references and sensor-condition severities are symbolic synthetic fixtures. CI success proves software fixture conformance only.

## Nonclaims

The package establishes benchmark semantics and synthetic fixtures only. It does not establish robot performance, causal failure attribution, physical safety, calibration validity, qualification, construction productivity, standards conformity or commercial superiority. The public development oracle is visible and must not be scored as blind evaluation data.
