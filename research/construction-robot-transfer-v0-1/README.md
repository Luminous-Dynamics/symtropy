# Construction Robot Site Transfer Benchmark — v0.1

**Status:** research protocol, typed transfer-study contract and deterministic synthetic evaluator. Not an adaptive robot controller, safety system, ODD authority, field qualification or certification.

## Purpose

Measure whether a robot/task capability transfers from a source construction site and work package to genuinely different target conditions while maintaining independently accepted construction output. Count the work required to transfer: calibration, site setup, data/labels, engineering/operator effort, inspection, rework, and time to first accepted unit.

This benchmark distinguishes zero-shot transfer from calibration-only, bounded offline adaptation and human-supervised method requalification. A strong result after extensive tuning is valuable adapted performance, but is not zero-shot transfer.

## Why the field needs this benchmark

- Yu & Park (ISARC 2026) review 134 intelligent construction robotics papers and report only two reaching TRL 9 field validation, identifying a valley of death between prototype and deployment. [Paper](https://www.iaarc.org/publications/2026_proceedings_of_the_43rd_isarc_singapore/evaluation_of_technical_readiness_level_for_intelligent_construction_robots.html), DOI 10.22260/ISARC2026/0325.
- Jin et al. (2026) review 136 reports and 103 executing configurations for deep reinforcement learning in construction robotics. Their review reports 73 configurations where simulation or controlled-lab validation was the highest exposure. Of 30 in construction-representative, field or workflow-integrated settings, 28 documented task endpoint success, 12 measured physical work output beyond robot motion, and three characterized a state relevant to subsequent operations. [Review](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=7029004), revised 18 September 2026.
- Xu et al. (2026) review 375 construction-robotics studies (2023–2025): 58.9% lab prototypes/experiments, 19.3% field tests or real-world deployments, 13.9% simulation/virtual tests, and 7.1% conceptual/theoretical. [Review](https://www.sciencedirect.com/science/article/pii/S2590123026014404), DOI 10.1016/j.rineng.2026.110406.
- A 2026 review of 75 field deployments identifies recurring environmental, integration, hardware, scalability/cost and human–robot-interaction challenges. [Review](https://www.sciencedirect.com/science/article/pii/S2666165925002418).

The figures above are review-corpus observations, not population estimates of commercial robot performance or failure probabilities.

## Paradigm shift: from in-domain success to bounded transferability

Freeze the source model, policy, parameters and machine configuration; declare the target site/material/task domain; preserve target evaluation holdouts; then compare distinct transfer regimes. Charge all adaptation work to the target result, and report independently accepted functional output—not just command completion, predicted success or robot motion.

A transfer claim is limited to its declared task family, functional unit, acceptance profile, source/target domain and transfer regime. Out-of-domain inputs must remain unknown, unsupported or rejected, not silently treated as covered.

## Transfer regimes

1. **Zero-shot:** frozen source model/policy/configuration. No target task labels or policy updates. Mandatory site input admission and safety-authorized metrology/calibration may occur only if predeclared and recorded.
2. **Calibration-only:** task policy/model frozen; approved calibration parameters/reference measurements may change under a declared budget.
3. **Bounded offline adaptation:** target data/labels collected under a frozen sample and labour budget; candidate update occurs on a shadow copy and is independently evaluated before any use. No live unapproved self-modification.
4. **Human-supervised method adaptation:** authorized people change method/task parameters or mappings. Count engineering, reprogramming, review, revalidation and commissioning. This is requalified performance, not zero-shot.
5. Continual online adaptation is outside v0.1. It requires a separate safety/authority, rollback, drift and change-triggered requalification contract.

Do not pool results across regimes.

## Shift axes

Declare shifts independently and preserve important interactions:
- physical environment: terrain/floor/soil, slope, roughness, moisture, temperature, humidity, wind, rain, dust, illumination, vibration, access and site geometry;
- materials/work: material type, source, batch/lot, dimension/condition, cure/age, tool/fixture, consumables, access and staging;
- digital/operational: model/export schema, design revision, units, coordinate frame/control evidence, site epoch, calibration, task complexity, workflow dependencies, crew/shift and trade coordination;
- robot/fleet: serial, hardware revision, sensors/tools, software/firmware, model/policy digest, maintenance/wear state and single-robot/fleet conditions.

The profile does not invent universal site difficulty scores or universal minimum sample sizes.

## Experimental integrity

- Freeze source/target domain manifests, task family, functional unit, acceptance profile, configuration, transfer regime and adaptation budget before collection.
- Split by the independent unit appropriate to the claim: project/site/work package/material batch/robot serial/crew/shift. Frames and cycles are nested observations, not independent sites.
- Source-training IDs must be disjoint from target evaluation IDs. Target adaptation may share a declared target-domain identifier with target evaluation only where the study explicitly permits the same target site and instead holds out evaluation work packages/material lots/tasks.
- Target calibration/adaptation and evaluation task IDs must not overlap.
- Keep target evaluation and independent inspection evidence out of adaptation.
- Random frame-level splits or adjacent time windows do not constitute site transfer.
- Preserve failed, blocked, rejected, unattempted, uninspected, out-of-ODD and indeterminate work in denominators and reports.
- Use matched/crossover manual/current-practice baselines where feasible with the same task, work package, acceptance predicate and conditions.
- Pre-register power/precision analysis. Do not claim universal sample size.

## Outcomes

### Independent accepted output
- planned / attempted / first-pass inspected / first-pass accepted / finally accepted / rejected / uninspected / unresolved units;
- first-pass yield = first-pass accepted / first-pass inspected;
- accepted output per robot operating hour;
- accepted output per total workflow crew-hour;
- rework, inspection/reinspection, material waste and downstream rejection;
- geometry/process residuals and independent measurement uncertainty.

### Adaptation burden
- site setup, survey/control and calibration person-hours;
- target labelled observations/demonstrations and qualified independent samples;
- engineer/operator/spotter/inspection labour-hours;
- elapsed time from arrival to first independently accepted unit;
- rework/rejected units/material during adaptation;
- compute/data transfer cost when measured and comparable.

### Transfer retention
For matched source and target strata, report the ratio of finally accepted functional units per total workflow crew-hour, but only when task family, functional-unit class and acceptance-profile digest match exactly. Always publish absolute source and target rates, denominators and uncertainty. A zero or unavailable source rate makes the ratio not estimable, not infinite.

### Safety and authority
Safety remains a separate non-compensable external gate. This evaluator deliberately does not score safety, infer clearance, issue motion authority, or trade safety holds against productivity. It does not verify the independence/authenticity of the acceptance authority; it binds opaque receipt references and counts supplied dispositions.

## Comparative baselines

Use identical frozen source/target tasks to compare:
- frozen source policy, no target adaptation;
- same robot with target calibration only;
- target supervised adaptation under a fixed data/label budget;
- domain-randomized / sim-to-real model, with simulator ranges disclosed;
- few-shot/meta-learned adaptation under the same target-data/compute budget where applicable;
- matched human/manual current practice;
- vendor-recommended commissioning, charging all labour/cost;
- independent referee and measurement method.

No composite score may hide a false accepted unit, a shift in denominator, adaptation cost, or a hard out-of-domain failure.

## V0.1 evaluator

The dependency-free Rust core checks:
- source/target split leakage by declared dimension and source-to-target work-item separation;
- target adaptation work items are disjoint from target evaluation tasks;
- actual calibration/engineering/operator/inspection effort does not exceed separately declared budgets;
- accepted, rejected, uninspected and unresolved terminal dispositions do not exceed attempted units;
- target adaptation/evaluation work-item overlap, source/target work-item overlap and separately budgeted versus actual adaptation effort;
- accepted/inspected/attempted/planned count invariants;
- task-family, functional-unit and acceptance-profile matching before transfer-retention calculation;
- zero-denominator handling as not estimable;
- total-workflow crew-hour output rate and adaptation burden accounting;
- deterministic metric values and typed errors.

It validates a supplied data contract, not field truth. It does not authenticate evidence, calculate metrology uncertainty, approve a robot, estimate causal effects, perform power analysis, or establish deployment readiness.

## Implemented synthetic evaluator

The dependency-free Rust core validates transfer-regime constraints, group-level split leakage, disjoint target adaptation/evaluation work items, accepted-output denominators and matched source/target comparability. It computes first-pass yield using inspected units, finally accepted units per total workflow crew-hour and robot-hour, matched transfer-retention ratios, and an adaptation-burden summary. Zero denominators return not-estimable values rather than fabricated zero/infinity. The evaluator binds acceptance-profile digests but cannot authenticate the referee or establish physical truth.

## Prospective targets (hypotheses, not standards or achieved results)

- 100% of transfer claims bind source/target domains, task family, functional unit, acceptance profile, regime and model/configuration digests.
- Zero source-training / target-evaluation leakage and zero target-adaptation / target-evaluation work-item overlap.
- 100% accounting of target calibration, labels, adaptation and requalification effort.
- Zero uninspected, unresolved or rejected output counted as accepted.
- Zero silent extension beyond declared support domain.
- Publish retention and adaptation burden by site and condition; no universal retention threshold before task-specific precision analysis.
- Broad deployment claims require multiple independent target sites/projects, robot serials and crews selected by pre-registered precision/power design.

## Build and test

```sh
cargo test --manifest-path research/construction-robot-transfer-v0-1/Cargo.toml --locked
cargo clippy --manifest-path research/construction-robot-transfer-v0-1/Cargo.toml --locked --all-targets -- -D warnings
```

GitHub Actions pins Rust 1.96 and validates formatting, tests, locked metadata and strict Clippy on Ubuntu, Windows and macOS. The crate has no external dependencies and a committed package lockfile.

## Claim ceiling

Passing v0.1 demonstrates only that split, regime, denominator and matched-comparison rules are enforced for the synthetic contract. It does not prove site transfer, policy safety, universal generalization or simulation-to-field fidelity. Physical claims require applicable site/robot risk controls, independent metrology, competent acceptance and actual multi-site trials.
