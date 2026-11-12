# Second-life battery mission qualification audit v0.1

Status: read-only eligibility and evidence-consistency contract. Not a state-of-health model, BMS, laboratory procedure, safety certification, regulatory passport submission or product-release authority.

## Purpose and owner boundaries

Consume, rather than duplicate:
- EV-SOH transfer #11252 for health/RUL and uncertainty;
- EV pack capability #11271 and mission capability #11589 for topology and time-indexed service envelope;
- dynamic electrothermal ECM #11485 for capability provenance;
- Mycelix battery passport #6062 and lifecycle refresh #6076 for identity/history projection;
- EV-BATTERY-CIRCULARITY #2155 for repair/recovery route economics;
- EV-BATTERY-THERMAL-PROPAGATION-EVIDENCE #4879 for multilevel thermal evidence and product-change blockers.

This slice checks whether the exact source pack, its history, authority decisions, BMS pathway and capability evidence support a declared second- or third-life mission. It does not calculate SOH, infer physical life, or operate/disassemble a pack.

## Paradigm shift

Move from **SOH-threshold reuse** to **mission-specific qualification under evidence and safety gates**.

Equal nominal SOH does not imply equal risk or mission utility. Cell imbalance, impedance dispersion, incident history, thermal exposure, isolation faults, BMS compatibility and uncertainty can change what service is possible. The evaluator consumes lower-bound capability from owner systems and refuses to replace missing evidence with a generic health percentage.

## Hard invariants

1. Exact pack/configuration and cell-membership closure precede any disposition.
2. History must be complete from first commissioning; partial history is UNKNOWN, not “no incident.”
3. Adverse events may be considered only when explicitly adjudicated and their evidence is admitted.
4. A thermal-propagation record marked only CANDIDATE_FOR_HUMAN_REVIEW is not an accepted safety decision. An accepted decision must bind a competent-authority decision reference.
5. BMS strategy (unchanged, authorized firmware update, gateway, replacement) must be verified for the target pack/mission. This tool never performs updates or actuation.
6. Time-indexed lower-bound usable energy, power, temperature envelope and availability must satisfy the exact mission.
7. Rights, transport/handling, electrical isolation/protection and quality-control gates are non-compensable.
8. All ten hard gates must be present exactly once and PASS with admitted evidence.
9. Synthetic test fixtures can only produce SYNTHETIC_FIXTURE_ONLY, never an eligible disposition.
10. Any unknown, failed, incomplete or unsupported gate blocks automatic eligibility and yields ABSTAIN.

## v0.1 outputs

- ELIGIBLE_FOR_SECOND_LIFE_MISSION
- ELIGIBLE_FOR_THIRD_LIFE_BACKUP
- ABSTAIN_INSUFFICIENT_EVIDENCE
- SYNTHETIC_FIXTURE_ONLY

An eligible software disposition is still not safety certification, legal compliance, commissioning authorization or a warranty decision. Qualified engineering, independent testing, applicable standards and economic-operator obligations remain external gates.

## Benchmark protocol

Compare the following baselines on identical held-out original vehicle/pack cohorts and mission scenarios:
1. single SOH-threshold routing;
2. capacity + DCIR/impedance rules;
3. pack SOH plus cell-dispersion/imbalance rules;
4. degradation-mode-aware assessment where admitted measurements exist;
5. lower-bound mission capability plus all hard gates and ABSTAIN.

Hold out entire original vehicles, packs, cell families, OEMs, chemistries, lots, first-life duty regimes and repurposed configurations. Never split sibling cells/modules from one pack across train/test.

Report unsafe false acceptance (primary), false rejection, abstention coverage, mission-feasibility lower-bound coverage, reserve violations, thermal derating, availability, EFC/calendar life, BMS adaptation, commissioning yield/time, cost per lifetime delivered kWh and lifecycle impact. Do not collapse these into one compensatory score.

## Industry/standards context

IEEE 2993-2025 addresses design and use of second-life EV batteries in energy storage systems (≤10 kV); this contract stores its reference but does not assert conformance. EU Batteries Regulation 2023/1542 has separate provisions for preparation for reuse/repurposing and battery status documentation. Jurisdiction and operator-specific applicability remains a competent legal/compliance decision.

Research sources:
- IEEE 2993-2025 catalogue: https://standards.ieee.org/ieee/519/10617/
- EU Batteries Regulation consolidated text: https://eur-lex.europa.eu/eli/reg/2023/1542/2026-08-13/eng/
- Second-life BMS review (29 Oct 2026): https://doi.org/10.1109/ACCESS.2026.3748316
- Second-life stationary storage and degradation history: https://www.mdpi.com/1996-1073/19/9/2026

## First implementation slice

Dependency-free Rust auditor, versioned Draft 2020-12 manifest and synthetic known-answer/adversarial corpus. It validates typed identity, evidence admission, history semantics, thermal-authority status, BMS target compatibility, mission/capability envelope, ten hard gates and deterministic reports. It does not execute physical tests, estimate battery health, run a BMS or authorize handling/commissioning.

## Claim ceiling

A passing software audit demonstrates only internal consistency of declared identity, evidence, eligibility and mission-comparison rules on the authored corpus. It does not certify a battery, prove absence of latent defects, authorize disassembly/testing/transport/commissioning, establish legal compliance, guarantee safety, predict physical life, or prove economic/environmental superiority. All included fixtures are synthetic.
