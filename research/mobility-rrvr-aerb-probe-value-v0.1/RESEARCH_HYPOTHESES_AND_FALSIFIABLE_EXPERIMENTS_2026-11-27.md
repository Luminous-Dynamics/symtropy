# RRVR / AERB research hypotheses and falsifiable experiments

**Date:** 2026-11-27  
**Status:** research hypotheses and experimental plan; not achieved results  
**Parent:** AERB #7631, v0.1 draft PR #7633, anticipatory probe-value work #7694 / PR #7695

## 1. The research objective

The objective is not to make a robot that can always explain what happened. It is to make a robot-control system that can identify whether it has enough evidence to permit the next physical effect, and can acquire the most decision-relevant evidence when it does not.

A robot can be highly competent at task completion, yet still have poor execution integrity:
- a command may have been dispatched without acknowledgement;
- the effect may have occurred despite a lost reply;
- part of the intended effect may have occurred;
- a duplicate or stale-epoch effect may have occurred;
- an old command may still be buffered downstream;
- another command holder may still be able to act;
- a seemingly complete observation may omit an effect-producing path.

The central research object is therefore not one predicted state. It is the **decision-relevant set of possible execution histories**, refined only by evidence whose source, timing, scope and provenance are qualified.

The fundamental safety boundary is that the cognitive system proposes and evaluates. An independently enforced endpoint/authority mechanism controls physical effect admission.

## 2. Paradigm shifts and falsifiable hypotheses

### H1 — Execution finality is an independently evidenced property, not an acknowledgement state

**Hypothesis:** a command's acknowledgement status is insufficient to establish whether the physical effect occurred. An independently evidenced finality contract will reduce false NoEffect / duplicate-retry decisions compared with acknowledgement-only or coordinator-journal-only reasoning.

**Experiment:**
- Inject lost, delayed, duplicated, reordered and stale acknowledgements.
- Independently vary dispatch, actuator acceptance, partial effect, completed effect and downstream buffering.
- Compare acknowledgement-only, coordinator-log-only, endpoint-journal-only, sensor-only and combined evidence qualification.
- Preserve every unknown, disputed and unresolved case.

**Primary falsifier:** independent effect evidence and scope closure do not reduce false finality or unsafe fresh-attempt recommendations on matched cases, or the reduction is explained by a different task distribution / information budget.

### H2 — Set-valued execution histories are safer than premature point-state collapse

**Hypothesis:** preserving an explicit support over plausible histories reduces false-finality and false-unique recommendations under missing, correlated, stale or contradictory observations.

**Experiment:**
- Compare a single best-guess state, a calibrated Bayesian posterior, and a prior-free set-valued support.
- Inject common-cause sensors, hidden forwarding paths, model mismatch and clock drift.
- Score truth inclusion, false exclusion, support size, selective risk, abstention and downstream decision regret.
- Hold out controller, endpoint, sensor, firmware and scenario families.

**Primary falsifier:** set-valued support does not improve false-exclusion / unsafe-decision trade-offs over matched calibrated or rule-based baselines, or it only achieves coverage by making every case uninformatively broad.

### H3 — Decision value is a better sensing objective than information gain alone

**Hypothesis:** selecting observations by downstream decision-loss reduction will outperform entropy/EIG-only and cheapest-first selection when observations are informative but decision-irrelevant or low-information but safety-critical.

**Experiment:**
- Include high-entropy probes with zero decision-loss reduction.
- Include low-information probes that distinguish NoEffect from Applied or unresolved-scope cases.
- Compare no probe, fixed wait, cheapest, random, fixed cadence, event-triggered, entropy/EIG-if-calibrated, Bayesian EVSI-if-calibrated and prior-free robust minimax.
- Keep task loss, acquisition cost, latency, compute, energy and communication as separate metrics.

**Primary falsifier:** at matched evidence and resource budgets, decision-focused selection does not improve risk/utility frontier or does so only through an unjustified loss table.

### H4 — Observation policies should be sequential, conditional and decision-sufficient

**Hypothesis:** a bounded branch-dependent observation policy can achieve lower worst-case terminal decision loss or resource use than any fixed single-probe schedule, while avoiding unnecessary sensing on branches where another probe cannot change the decision.

**Experiment:**
- Use a public, tiny exact finite-world fixture with a separately written exhaustive policy-tree oracle.
- Compare fixed probe lists, myopic one-step minimax, sequential minimax, fixed-cadence sensing and calibrated Bayesian sequential planning where justified.
- Include a branch-unlock case where the first observation reveals which second probe can be useful.
- Record terminal decision, compatible state support, full resource path, deadline and branch reason.
- Require stop/hold where a follow-up does not strictly improve the frozen terminal objective.

**Primary falsifier:** sequential plans do not improve worst-case loss or Pareto resource outcomes over one-step policies on held-out branch structures, or the candidate policy depends on hidden evaluator partitions.

**Existing composition:** Symthaea #13571 supplies the finite-horizon prior-free sequential oracle and an independent exhaustive enumerator. Symthaea #13911 adds an AERB adaptive-policy example stacked on that work; it remains unqualified until exact-head Rust gates pass.

### H5 — Counterfactual integrity requires exactly one factual branch

**Hypothesis:** explicitly separating factual execution, candidate prediction, evaluator counterfactual and actually executed probe result prevents simulated futures from being laundered into physical evidence.

**Experiment:**
- Inject counterfactual records into candidate outputs and evidence ledgers.
- Attempt to replay, merge, duplicate or supersede predicted branches as though they were physical actions.
- Verify that the actual ledger contains only the one branch the harness executed and only observations returned with valid source and scope receipts.
- Use negative controls for hidden truth, generator seed and evaluator loss leakage.

**Primary falsifier:** the separation cannot be enforced outside mere JSON field omission, or the benchmark evaluator accepts a model rollout as a physical observation.

### H6 — Scope closure is a distinct evidence obligation from observing the robot's pose

**Hypothesis:** independent evidence about position/effect cannot prove that all command holders and forwarding edges are unable to create a later effect. A scope-closure receipt bound to the effect-producing system boundary is necessary before the open-world residual can be narrowed.

**Experiment:**
- Build hidden, stale and delayed command-holder paths.
- Compare coordinator assertions, local queue snapshots, endpoint journal reads, pose/force sensors and independently adjudicated scope-closure evidence.
- Change system boundary, controller version, queue generation and authority epoch while reusing old receipts.
- Verify that stale / wrong-scope receipts do not narrow support.

**Primary falsifier:** pose or local queue observations consistently prove system-wide quiescence under the declared adversarial boundary, or the closure receipt does not improve any false-finality endpoint.

### H7 — Temporal/HDC cognition adds value only when its predictions improve evidence decisions

**Hypothesis:** Symthaea's HDC identity/context binding plus a temporal predictor (CfC/LTC or matched conventional baselines) can improve the candidate's support coverage and observation selection over rules-only, HDC-only and temporal-only methods, without weakening the evidence authority boundary.

**Experiment:**
- Compare rules-only, HDC-only, constant-velocity, Kalman/LQG, AR, GRU/LSTM/TCN/state-space, CfC/LTC, and HDC+temporal combinations.
- Match training examples, candidate-visible evidence, inference compute, memory and probe budgets.
- Evaluate calibration and selective risk only when probabilities are independently calibrated.
- Hold out new controllers, robot models, sensor profiles, clock domains and fault families.

**Primary falsifier:** the HDC/temporal combination has no held-out benefit over simple rules or conventional temporal baselines, or its predictions cause unsafe support shrinkage under model mismatch.

### H8 — Assurance must remain valid at the physical decision boundary

**Hypothesis:** a candidate's evidence-based recommendation is not sufficient unless the live execution gate revalidates command identity, endpoint profile, authority epoch, queue generation, scope, evidence freshness and unresolved in-flight work immediately before any new effect.

**Experiment:**
- Mutate endpoint profiles, authority epochs and queue generations between observation and attempted effect.
- Inject late-arriving buffered commands after coordinator restart.
- Replay stale, revoked, wrong-scope and disputed receipts.
- Attempt time-of-check/time-of-use races and duplicated dispatch.
- Verify that unknown outcome holds mutation until reconciliation, rather than inferring failure from silence.

**Primary falsifier:** the external gate admits effects under any of the preregistered stale, unresolved or wrong-scope negative controls.

## 3. The research architecture

Use existing components with explicit ownership:

| Layer | Responsibility | Authority boundary |
|---|---|---|
| Candidate observer (Symthaea) | Predict possible histories, preserve uncertainty, propose probes and non-actuating conditional policies | Cannot authorize physical effect |
| Evidence adapter (RRVR / Symtropy) | Validate decision cut, source role, event/acquisition time, scope, digest, signature and observation quality | Cannot invent source truth |
| One-step robust evaluator | Calculate prior-free guaranteed decision-loss reduction | Evaluator-side model only |
| Sequential policy oracle | Enumerate branch-dependent probe/stop/terminal-disposition policies | Evaluator-side oracle only |
| Calibrated active-sensing planner | Expected task-loss reduction and component-wise sensing-resource frontier where probabilistic assumptions are justified | Calibration/provenance required |
| Independent execution gate | Enforce durable intent, endpoint effect fencing, scope closure, authority and dispatch admission | Sole owner of physical effect admission |
| Independent campaign evaluator | Score hidden truth, false exclusions, policy-tree loss, replay and statistics | Never exposed to candidate |

The architecture is a chain of separately verifiable responsibilities:

```text
Factual evidence at decision cut
        |
        v
Candidate support + predicted observation outcomes
        |
        v
Candidate proposes non-actuating probe / policy tree
        |
        v
Harness executes one permitted observation only
        |
        v
Independent evidence qualification
        |
        v
Candidate updates support / selects terminal disposition
        |
        v
Independent authority + effect-finality gate
        |
        v
Possible next physical effect
```

All unexecuted branches remain counterfactual-only. A predicted outcome is never a sensor receipt; a recommendation is never dispatch authority.

## 4. Statistical and benchmark validity

- Freeze scenario families, hidden truth, outcome supports, decision-loss rows and probe cost/deadline contracts before candidate evaluation.
- Use independent trajectory/scenario clusters for uncertainty estimates. Frames and repeated attempts within one trajectory are not independent replicates.
- Pair native and RRVR-overlay arms on identical host episodes and retain the host task-success/recovery metric.
- Keep invalid, unknown, stale, disputed, missing and unresolved episodes visible in denominators.
- Report selective risk versus coverage and include always-abstain / constant-disposition controls.
- Include source-identity, task, robot, controller, firmware, endpoint, sensor and clock-domain holdouts.
- Freeze candidate model/checkpoint, adapters and inference resource limits.
- Blind evaluator truth, supports, generator seed and loss tables from the candidate process.
- Publish null results and confidence assumptions; no weighted composite score.

## 5. What it would mean to beat the state of the art

AERB should not claim to beat RoboCasa365 on task success, SafeVLA-Bench on its native safety score, or VLA-FixBench on recovery unless we run the same benchmark version and protocol.

The meaningful competitive result is a matched, independently reproduced **assurance frontier** showing one or more of:
- fewer false-finality / unsafe fresh-attempt decisions at the same host task-success and recovery level;
- higher possible-history coverage at the same abstention rate;
- lower worst-case terminal decision loss at the same observation budget and deadline;
- lower probe-selection regret and unnecessary sensing at matched resource vectors;
- shorter time-to-safe-reconciliation without more false closure;
- transfer to held-out controller, endpoint, sensor and task families;
- better assurance with lower p95/p99 compute and sensing overhead.

Use a multi-axis Pareto frontier, not a single weighted leaderboard. An improvement in one safety-critical axis cannot erase a regression in another.

## 6. Publication milestones

1. **Protocol / schema:** v0.2 candidate envelope and non-actuating policy plan; exact schema/fixture checks.
2. **Finite-world reference:** one-step robust adapter plus sequential policy oracle; independent brute-force agreement.
3. **Blind process-model campaign:** randomized fault delivery, complete denominators, hidden truth and evaluator-side recomputation.
4. **Host benchmark overlay:** RoboRecover, LIBERO-RECOVER and VLA-FixBench, preserving native metrics.
5. **Simulator/HIL:** independently captured effect, command-path, reset and sensor receipts.
6. **Controlled physical evaluation:** independent safety oversight and deployment-specific risk assessment.

No lower-tier result establishes a higher-tier claim. Passing this benchmark does not certify a safety function or robot cell.
