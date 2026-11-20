# Per-step AC operating-point certificate contract v0.1

**Owner:** POWER-AC-STEP-CERT-001 #6331  
**Programme:** #5241 Power Fabric benchmark  
**Related:** #6320 / PR #6326 external comparator contract; #6235 service outcome; #3418 electrical resilience scope.

## Purpose

This package defines a per-time-step receipt contract for an independent AC solver to qualify candidate restoration schedules. It is not an AC solver. It prevents a planner's DC/graph feasibility flag from being mistaken for an independently verified AC operating point.

## Three separate qualification axes

1. **Static AC snapshot:** AC power-flow/OPF convergence and declared voltage, thermal, transformer, P/Q and source constraints for one exact topology and dispatch state.
2. **Inter-temporal constraints:** chronological SOC/fuel, ramping, switching and reserve continuity over the horizon.
3. **Dynamic/protection:** transient/frequency/voltage recovery, inverter control, fault ride-through, protection coordination and switching transients.

A static AC pass does not imply dynamic/protection pass. The accepted fixture intentionally leaves dynamic qualification not assessed.

## Receipt binding

Every step and every topology/dispatch-changing boundary requires exactly one receipt bound to:
- step ID and interval;
- network model revision and parameter-set digest;
- topology-state and dispatch-state digests matching the evaluation grid and planner summary;
- exact solver version/build/configuration;
- canonical input and output artifacts with SHA-256;
- model class and declared applicability;
- convergence residual, tolerance and criteria;
- source reachability and all energized island/slack source records;
- voltage, branch thermal, transformer, generator/inverter P-Q and other applicable constraint assessments.

Missing, stale or mismatched receipts are incomplete/inconclusive, not passes. A solver that fails to converge is inconclusive unless its explicit solver contract classifies a proven infeasibility.

## Synthetic fixtures

- **Accepted static AC only:** two steps with distinct state digests, converged static AC receipts and accepted inter-temporal evidence; dynamic/protection remains not assessed.
- **DC-feasible / AC-infeasible:** planner marks both steps feasible under DC OPF, but the independent AC receipt at step s1 reports no admissible operating point and explicit voltage/thermal failures. The schedule must be rejected.

Values and artifact hashes are illustrative synthetic placeholders; they are not outputs from an electrical solver or utility network.

## SOTA boundary

LARA's inspected public environment source includes graph connectivity/radiality and aggregate MPS P/Q/S checks but no AC/voltage model in that audited source. Lin & Xu (Applied Sciences 2026, DOI 10.3390/app162110584) explicitly evaluate AC operating points and report a DC-only schedule without an AC operating point at 8 of 22 steps on IEEE 118-bus. That makes per-step AC receipts necessary for fair comparison. This contract does not reproduce that paper or validate any network.

## Claim boundary

Contract/fixture only. No AC power flow, OPF, transient simulation, protection coordination, energization, service availability or operational authorization is produced.

## Current external comparators (November 2026)

- **Explainable Agentic AI for Distribution System Load Restoration** (Chowdhury & Li, arXiv:2611.22016, 17 November 2026): five LLM agents (planner guidance, strategic planning, decision arbitration, memory and explanation), constrained feasible-action catalogues, receding-horizon planning and deterministic pandapower AC checks on IEEE 33/69/118. The paper reports 250 scenarios per topology (200/25/25 train/validation/test for RL baselines), and compares against DQN, Actor-Critic, MCTS and a single-LLM ablation. Its multi-agent system reports about 25.8% average weighted-load improvement over its single-agent ablation, but also 94.64k tokens and 26.06 minutes per scenario versus 25.38k tokens and 21.25 minutes for the single-agent version. This is an arXiv preprint, not an independently reproduced result. We must include it; constrained LLM planning plus AC checks is not novel by itself.
- **Multi-Stage Coordinated Restoration for Unbalanced Distribution Networks with Multiplexed Soft Open Point** (Sun et al., Sustainable Energy, Grids and Networks, online 7 November 2026, DOI 10.1016/j.segan.2026.102576): phase-specific three-phase restoration across pre-event preparation, fault location/isolation and service restoration; asymmetric VSC ports, feeder-selection switches and phase-/feeder-specific assistance. The schema's unbalanced fixture now exercises required phase A/B/C voltage and thermal evidence.
- **Coordinated sequential service restoration for hybrid AC/DC distribution networks with multi-mode VSC FIDs and ESS** (Journal of Energy Storage, 15 November 2026, DOI 10.1016/j.est.2026.123636): 207-node hybrid AC/DC system, multi-mode VSC/flexible interconnection devices, black-start storage, cold-load pickup and rolling-horizon MILP. Treat as a separate hybrid AC/DC tier.
- **OPF-based restoration with frequency dynamics** (PSCC 2026 presentation; DOI 10.1016/j.epsr.2026.113573): IEEE 9-bus dynamic frequency restoration work demonstrates that a static-optimal switching sequence may violate dynamic constraints. Do not combine a static AC pass with a dynamic stability claim.

The differentiator to test is therefore the evidence-carrying contract: exact step/state/model hashes, declared constraint coverage, per-phase evidence, independent service trajectories and explicit separation of static AC, inter-temporal operation and dynamic/protection—not simply putting an LLM in front of a power-flow check.
