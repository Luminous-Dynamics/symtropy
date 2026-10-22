# Robotics Fault-Response Dynamics Benchmark v0.1

**Status:** protocol and synthetic contract fixture only. This document reports no measured robot performance or safety qualification.

## Why a separate benchmark?

A task-success score can hide whether a robot recognized trouble promptly, reached a safe state before the situation worsened, needlessly stopped during a false alarm, or resumed before the fault was cleared. A fault-response benchmark therefore records the *trajectory through the event*, not just the final binary result.

This complements the general benchmark-validity audit (paired candidates, held-out families/sources, shortcut controls, calibration and family-cluster uncertainty) and the terrain-mobility contract. It also complements NIST's component-measurement approach, which calls out mobility, perception, dexterity and safety as parts of the integrated system and describes disturbance-sensitive mobile-robot measurement. See [NIST's Performance Assessment Framework for Robotic Systems](https://www.nist.gov/programs-projects/performance-assessment-framework-robotic-systems) and [NIST's Mobility Performance of Robotic Systems](https://www.nist.gov/programs-projects/mobility-performance-robotic-systems).

## Paradigm shift: score the response curve, not only the ending

Each experiment compares a matched, no-fault control with a challenge episode using the same declared policy, scenario family, seed, and evidence class. The challenge may contain a real injected fault or a sham alert. Required event times are relative to episode start and use milliseconds.

The auditor records:
- **Detection latency** = fault-detection time − physical fault-onset time.
- **Time to safe disposition** = safe-state time − fault-onset time.
- **Recovery latency** = service-restoration time − fault-onset time.
- **Useful-work retention** = challenge useful-work units / matched control useful-work units, when both values are known and the control denominator is positive.
- **False-abstention rate** on sham/no-hazard episodes, separately from hard safety failures.
- Safety violations, missing detections, missed safe-state targets, unauthorized restoration, and missing measurements as separate outcomes.

No single weighted score is produced. A fast policy with an unsafe response cannot offset that failure with high useful-work retention; a cautious policy does not get to hide avoidable stops behind a high safety rate.

## Response contracts

Each paired case declares one contract:

- `safe_state_required`: a physical fault is present and the policy must detect it and enter a safe disposition within any declared scenario-specific latency limits.
- `hold_until_authorized`: a physical fault requires a safe disposition; service may resume only after recorded clearance and explicit recovery authorization.
- `continue_unless_fault_confirmed`: a sham alert has no corresponding physical fault. An unnecessary abort is counted as false abstention, not as a safety failure by itself.

Latency targets are per-scenario engineering requirements, not universal values. The auditor checks a declared target where present; it does not invent a safe deadline. Missing event times remain missing and never become zero. Timestamp ordering, policy pairing and evidence class are checked explicitly.

## Minimum benchmark design

For serious evaluation, report results by fault family, platform, operating-design-domain stratum, policy, and evidence class. Freeze train/validation/test separation before tuning; keep complete fault families and source environments held out. Pair policies on identical initial state, scenario, seed and injected event schedule. Repeat across a preregistered number of seeds and independent families; resampling seeds alone does not create independent terrain families.

The protocol should eventually be run on:
1. deterministic contract fixtures and adversarial mutations;
2. a physics/sensor simulator with calibrated fault onset and ground truth;
3. hardware-in-the-loop with synchronized event clocks;
4. supervised, low-consequence field trials under separately approved operating procedures.

Report raw episode receipts, the number of eligible/missing pairs, confidence intervals by family, and evidence maturity. Synthetic, simulator, HIL, and physical-field evidence must never be blended into one headline number.

## Prospective research targets

These are hypotheses to test, not results or existing safety limits:

- **Zero** safety violations and **zero** service restorations before fault clearance and authorization in the qualified challenge set.
- **100%** of scenarios with a required safe response meet their preregistered, scenario-specific detection and safe-state deadlines—or are explicitly marked as a deadline miss.
- Reduce median and tail detection/safe-state latency versus the conservative baseline without increasing false-abstention rate on matched benign/sham cases.
- Preserve more useful work under non-catastrophic faults while preserving all hard safety gates; show the trade-off frontier rather than a scalar average.
- Demonstrate improvements on independently held-out fault families and field conditions, not just the public synthetic fixture.

Do not claim the programme exceeds SOTA until comparisons use the same task, platform, fault distribution, evaluation budget, success rubric, sensor access and evidence class, with uncertainty and failures reported. The [2026 robotics benchmark audit](https://arxiv.org/abs/2606.04233) is a useful warning: shortcut solvability, non-significant gains, test-set overfitting and data-source dependence can undermine apparent state-of-the-art results.

## Exact evidence

The Rust auditor emits the SHA-256 of the exact input bytes, the evaluator commit, per-pair latency/retention metrics, safety-gate disposition, aggregate fault-response metrics, and findings. This protects traceability; it does not make sensor clocks, simulator dynamics, or the physical model true.

## Claim ceiling

The committed fixture is synthetic and designed to exercise the protocol. A passing audit establishes internal contract consistency for the supplied data only. It does not establish physical safety, generalization, reliability, standards conformity, model fidelity, or robot readiness. ISO 18646-5:2026 provides legged-locomotion performance methods but explicitly does not validate safety requirements ([ISO catalogue](https://committee.iso.org/standard/86850.html)).
