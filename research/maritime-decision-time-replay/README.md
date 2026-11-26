# Maritime Decision-Time Replay (MDTR) v0.1

**Status:** executable synthetic prototype; not operationally qualified.  
**Research owner:** Symtropy.  
**Tracking issue:** [#7504](https://github.com/Luminous-Dynamics/symtropy/issues/7504).  
**Related owners:** #7488, #3683, #7400, #6997, #7231, #7357, #7360, #6662, #6578 and #7436.

## Research thesis

Evaluate the quality of a **justified decision**, not merely the quality of a schedule.

A policy may use only observations available, rights-cleared and decision-usable at its historical decision cutoff. It returns an action or abstains. An independent evaluator, outside the policy input boundary, adjudicates hard constraints and scores the realized outcome.

The target is a decision system that:
1. reconstructs the information set that was actually available;
2. generates feasible, bounded recovery alternatives;
3. preserves physical, authority and participant constraints independently;
4. accounts for observation cost and decision deadlines;
5. conserves cargo identity and quantities across every handoff;
6. measures service delivered to the intended receiver, not just terminal throughput.

A lower weighted objective never compensates for a hard-gate violation.

## Executable v0.1 slice

This directory now contains a dependency-free Rust crate, frozen TSV fixture corpus, separate JSON schemas for policy-visible input and evaluator-only truth, and a separately implemented test-only Python reference.

Run from the repository root:

```sh
cargo fmt --manifest-path research/maritime-decision-time-replay/Cargo.toml -- --check
cargo test --locked --manifest-path research/maritime-decision-time-replay/Cargo.toml
cargo clippy --locked --manifest-path research/maritime-decision-time-replay/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path research/maritime-decision-time-replay/Cargo.toml --bin mdtr-audit > mdtr-receipt.tsv
python3 research/maritime-decision-time-replay/scripts/independent_reference.py --receipt mdtr-receipt.tsv
```

The local crate is isolated as its own Cargo workspace so it can be qualified without editing the monorepo root workspace.

### What is implemented

- As-of visibility checks using publication time, retrieval time, rights, exact subject binding and decision-use interval.
- Event time is kept distinct from knowledge time: a future event can be known now; a late-published correction cannot rewrite an earlier information set.
- A narrow `PolicyInput` type that contains only the policy-visible event references.
- A separate `EvaluatorTruth` type for physical, authority, participant-floor and cargo outcomes.
- A deterministic example policy and post-decision evaluator.
- Known rejection precedence over uncertainty, while preserving all gate reasons.
- Cargo ledger checks in one explicit unit (container count): duplicate/overcount rejects; incomplete reconciliation abstains.
- Deterministic TSV receipt and hand-authored expected outcomes.
- 12 Rust unit tests and 14 frozen fixture episodes.
- Independent test-only Python reference logic that reads the fixture separately and checks the complete Rust receipt.

### What is not implemented or proven

- This is not yet a full episode JSON ingestion engine; the executable reads the documented 23-column TSV contract.
- The JSON schemas document the policy/evaluator boundary; they are not yet run through a schema validator in the Rust crate.
- The baseline policy is intentionally trivial. It selects the first admissible observation; it is not a scheduling optimizer.
- No MILP, CP-SAT, C&CG, ALNS, GNN/MAPPO, QMIX or terminal-system adapter is implemented here.
- No cryptographic information-set digest is emitted by this prototype.
- No simulator, live terminal feed, TOS mutation, berth booking, dispatch, custody transfer or payment is connected.
- No optimization gain, SOTA result, source authenticity, oracle independence, operational safety or real-port service improvement is claimed.

The frozen fixtures prove only that this narrow contract behaves as specified on these known-answer cases. They do not establish zero real-world risk.

## Decision-time information contract

An event can enter the policy view only if:
1. source rights permit this intended use;
2. its subject is exactly bound to the episode subject;
3. publisher visibility is at or before the decision cutoff;
4. receiver retrieval is at or before the decision cutoff;
5. the decision-use interval includes the cutoff.

Event time is recorded independently. It is not a substitute for publication or retrieval time. A future planned event may already be known; a past event published or retrieved after the cutoff was not yet known to the policy.

The policy receives the object described by [policy-view.schema.json](schema/policy-view.schema.json). The evaluator-only object is described by [evaluator-truth.schema.json](schema/evaluator-truth.schema.json). A production benchmark runner must materialize these as separate artifacts and process boundaries; the TSV fixture intentionally colocates them only for this compact test harness.

## Gate semantics

The evaluator applies independent, non-compensating gates:
- evidence rights and subject binding;
- physical feasibility;
- authority;
- participant service floors;
- cargo conservation.

Known violation => reject. Missing, unknown or unreconciled evidence => abstain. A fully passing bundle means only **eligible for decision review**. It is not execution authority.

The evaluator preserves known violations even when another gate is unknown. Digests, source IDs and distinct oracle IDs do not prove truth, authenticity or independence.

## Cargo conservation

The fixture uses integer container counts, not TEU, tonnes, vessel counts or moves. Its minimal ledger is:

`expected = receiver_accepted + in_transit + outstanding + unresolved`

A known duplicate or overcount rejects. A short/unreconciled ledger abstains rather than silently calling missing cargo delivered. A field implementation must model splits, merges, aliases, losses, handoffs and typed units explicitly; this minimal identity is not a full custody model.

## Regret and objective vector

For episode `ω`, admissible information set `I_t`, and evaluator-known feasible/authorized action set `A_f(ω)`, report per-objective regret:

```text
R_k(policy, ω) = J_k(policy(I_t), ω) - min[J_k(a, ω) for a in A_f(ω)]
```

The evaluator-only clairvoyant optimum is an information-relaxation bound, never a deployable policy. Score regret separately for receiver-service delay, missed connections, cost, energy, disruption recovery and participant displacement. Predefine how abstention/fallback cost is scored.

Do not compress all outcomes into one weighted sum. Report Pareto fronts and protected service floors.

## Expanded SOTA comparator map (literature through 26 November 2026)

The competition is no longer only berth allocation or FCFS.

| Research comparator | Published scope | MDTR implication |
|---|---|---|
| Li & Li, *J. Marine Science and Engineering*, 15 Sep 2026, [DOI 10.3390/jmse14181714](https://doi.org/10.3390/jmse14181714) | Two-stage berth/QC + AGV scheduling; RL-enhanced ALNS and GNN/MAPPO; cross-stage candidate coordination | Matched integrated seaside/AGV baseline; include buffer, route conflict and cross-stage effects |
| Zhang et al., *Computers & Industrial Engineering* 218, Aug 2026, [DOI 10.1016/j.cie.2026.112092](https://doi.org/10.1016/j.cie.2026.112092) | Two-stage robust berth/QC/yard allocation with accelerated column-and-constraint generation | Robust optimization comparator under matched arrival/workload uncertainty |
| Yang et al., *Journal of Industrial and Management Optimization* 22(11), 22 Oct 2026, [DOI 10.3934/jimo.2026198](https://doi.org/10.3934/jimo.2026198) | HA-QMIX combines AGV dispatch, charging, maintenance, degradation and stochastic failures | Add asset health, maintenance windows and throughput stability across seeds; not just nominal AGV routing |
| Li & Hu, *Journal of Cleaner Production* 574, 18 Aug 2026, [DOI 10.1016/j.jclepro.2026.149105](https://doi.org/10.1016/j.jclepro.2026.149105) | Joint berth, crane, yard-crane, onshore power and multi-source energy dispatch with carbon accounting | Add energy/OPS compatibility and carbon outcomes; compare centralized and coordinated methods |
| Wang et al., *Ocean Engineering* 358, 15 Jun 2026, [DOI 10.1016/j.oceaneng.2026.125850](https://doi.org/10.1016/j.oceaneng.2026.125850) | Robust berth/yard/shore-power planning under arrival uncertainty with adversarial hierarchical MARL | Compare robust recourse and adversarial scenario generation, not deterministic scheduling alone |

These papers are comparators, not proof that their algorithms are universally best. Their reported improvements are tied to their own datasets, constraints and compute settings. Cross-paper headline percentages are not valid matched baselines.

### What recent work changes about our ambition

1. **Integrated resource scheduling is already competitive.** We must compare against RL-ALNS/GNN-MAPPO and robust C&CG, not just FCFS or standalone berth allocation.
2. **Asset health is entering the decision loop.** AGV charging, degradation, stochastic failure and maintenance should be part of disruption scenarios and downstream cost.
3. **Energy is operationally coupled.** Berth commitments, quay/yard equipment, shore-power eligibility, energy storage and grid dispatch affect both cost and emissions.
4. **Throughput stability matters.** Report seed-to-seed variance and tail performance; a high mean with unstable throughput is not robust superiority.
5. **The differentiator remains decision-time assurance and whole-chain outcome.** We should not rebuild every optimizer. We should establish whether the chosen plan was justified, feasible and beneficial end to end.

## Paradigm shifts to prioritize

### 1. Decision-time replay, not hindsight replay

Every decision gets a frozen, rights-cleared information set. Later corrections may affect a later decision but cannot rewrite an earlier one.

### 2. Active sensing and value of information

The system can choose to act, wait, query, request a physical observation, select a bounded recovery, or abstain. Score information value net of observation cost, latency and the decision-use deadline.

### 3. Contingent recovery policies, not one brittle schedule

Compare precommitted schedules with bounded recourse trees: what action is selected after a specific event, with which admissible observations, and under what authorization? Evaluate out-of-sample disruption regret.

### 4. Cargo-conserved service, not terminal-only throughput

Track each expected unit through aliases, splits/merges, terminal exit, transport legs, custody handoffs and receiver acceptance. Unknown and censored outcomes remain explicit.

### 5. Health-, energy- and service-aware network optimization

Bring AGV/yard asset health, maintenance resources, onshore power, grid constraints, emissions and inland capacity into the objective vector and uncertainty scenarios. Keep each axis separately visible.

### 6. Evidence diversity, not source-count diversity

Model common-cause dependencies among feeds and oracles. Distinct IDs do not imply independent failure domains. Calibration/evaluation splits must be cluster-aware.

### 7. Causal field evaluation, not before/after marketing

A read-only shadow phase is necessary but not sufficient to prove benefit. Before a real intervention, preregister the outcome estimand, comparison cohort, confounders, exclusion rules and receiver-side measurement. Where operationally and ethically permissible, use randomized or stepped rollout; otherwise use a defensible quasi-experimental design with independent adjudication.

## Matched benchmark tournament

All policies must receive the same information cutoff, candidate universe, uncertainty scenario, hardware budget, wall-clock budget and stopping rule.

Required baseline families:
1. FCFS, greedy and current dispatch where access is permitted;
2. rolling-horizon scheduling and local repair;
3. exact MILP/CP-SAT on enumerable instances;
4. two-stage robust optimization / C&CG;
5. ALNS and RL-enhanced ALNS;
6. integrated GNN/MAPPO berth-QC-AGV methods;
7. HA-QMIX or equivalent health-aware AGV dispatch + maintenance;
8. energy/OPS-coupled MILP and robust/adversarial berth-yard-power policies;
9. branch-and-price where applicable;
10. strongest feasible same-information policy;
11. clairvoyant solution as a non-deployable information-relaxation bound only.

Published results without matched instances, data, constraints, compute and information cutoffs remain contextual literature comparisons, not head-to-head wins.

## Metrics and proposed targets

All values below are **research targets, not results**.

| Metric | Target |
|---|---:|
| Exact feasibility agreement on exhaustively enumerable instances | 100% |
| False-safe hard-constraint admissions in frozen adversarial suite | 0 |
| Unauthorized actions admitted | 0 |
| Post-cutoff information leakage | 0 |
| Modeled cargo duplication or silent loss | 0 |
| Exact-head deterministic receipt replay | Byte-identical |
| Median optimality gap on applicable mid-scale instances | ≤5% under a fixed, published budget |
| Receiver-service improvement / end-to-end p90 delay | 5–10% aspirational vs strongest matched baseline |
| Participant-floor regression | 0 admitted |

Also report:
- decision regret distributions and worst-decile/tail regret;
- value of information net of query cost and latency;
- abstention coverage versus selective risk;
- p50/p90/p95 receiver-accepted service time;
- onshore-power utilization, energy cost and emissions by source;
- maintenance cost, failure downtime and throughput variance across seeds;
- runtime, memory, optimality bound, timeout and incumbent quality;
- mutation score and every surviving mutation.

A finite corpus with zero failures does not establish zero real-world risk. Publish denominators, confidence intervals, failed runs and all exclusions.

## Industry interoperability and scorecards

- **DCSA:** consume its standardized Track & Trace event semantics and Port Call coordination interfaces. Do not invent competing shipping event vocabularies. Official references: [Track & Trace](https://dcsa.org/standards/track-and-trace) and [Port Call](https://dcsa.org/standards/port-call).
- **Kaleris N4:** treat it as an incumbent execution and optimization platform / integration target. Its [Yard Intelligence Suite announcement](https://kaleris.com/news/kaleris-launches-yard-intelligence-suite-to-overcome-rising-capacity-constraints-unlock-value-from-existing-systems-and-support-workforce-evolution/) is dated 12 May 2026; its [November 16 2026 product newsletter](https://kaleris.com/kaleris-tos-quarterly-newsletter-nov-2026/) describes continuing N4 4.0 evolution. Do not claim incumbent systems lack optimization or predictive intelligence.
- **World Bank / S&P CPPI 2025:** a port-level vessel-time comparator. Its methodology defines arrival/anchorage through departure from berth and workload-normalizes by ship/call characteristics. It is not a terminal-only metric or receiver-delivery metric. References: [CPPI](https://www.worldbank.org/en/topic/transport/publication/cppi) and [methodological note](https://thedocs.worldbank.org/en/doc/855fe5e7123d85b05d4e84c95b8fd1e2-0400012026/original/Methodology-Note.pdf).

Publish two separate scorecards:
1. **CPPI-compatible port-level vessel time**, with correct event boundaries and port aggregation.
2. **Terminal/cargo-service chain**, with unique unit/quantity-safe cargo denominator, custody, receiver acceptance, inland legs and tail performance.

Commercial performance datasets require explicit licensing and permitted-use review.

## South African evidence case

The 23 November 2026 public SAnews report repeated Transnet's aggregate update concerning 41 priority pharmaceutical containers: 18 reported cleared/delivered, seven expected inbound and 16 awaiting berth nomination. The public aggregate does not provide per-container identity, custody chain or receiver acceptance receipts. See [SAnews](https://www.sanews.gov.za/node/85835).

This is a claim-semantics negative control, not an assertion that the reported deliveries did not occur. Aggregate counts must not be promoted to unit-level independent verification. The next field step is read-only and rights-cleared; unit-level cargo and receiver evidence is required before a service-improvement claim.

## Integration / ownership

- #7488 consumes typed upstream gate receipts and tests composition. It must not duplicate capture, source registry, physical/authority, custody or statistical engines.
- #7400 / #1949 supply stale/conflicting evidence and regret foundations.
- #6997 supplies non-anticipative query policy and value-of-information logic.
- #7231 / #7357 / #7360 supply physical, authority, participant and dependence-aware qualification.
- #6662 supplies custody and receiver closure.
- #6578 / #7436 / #3683 supply intervention, quantity-safe service and end-to-end outcomes.
- Mycelix remains the authority for participant permissions, revocation, custody and operational commitments.
- Symtropy owns replay, benchmark orchestration, candidate comparison and evidence composition; it does not self-authorize operational mutations.

## Next executable increments

1. Validate the 0.1.0 exact-head Rust workflow and independent reference.
2. Expand the fixture from one event per episode to event histories with revisions, multiple sources and dependency clusters.
3. Add multi-unit custody transfers, aliases, split/merge and quantity-safe ledger conservation.
4. Add query-cost/latency and contingent recovery policy interfaces.
5. Add small enumerable berth/QC/AGV instances and exact MILP/CP-SAT reference solutions.
6. Add robust C&CG, RL-ALNS/GNN-MAPPO, HA-QMIX and energy/OPS adapters on identical frozen scenarios.
7. Build a coastal-to-inland simulator only after small-instance oracles and replay semantics are independently checked.
8. Advance a rights-cleared DGT read-only evaluation with receiver-side adjudication.

## Claim ceiling and safety boundary

Passing this synthetic prototype establishes only that the specified replay cases and reference evaluator agree. It does not authenticate real sources, prove physical oracle independence, establish statistical guarantees, demonstrate terminal performance, or authorize operational decisions.

Any field evaluation begins read-only, rights-cleared and independently adjudicated. Any later operational mutation requires a separate authority path, reconciliation and safety case. This crate does not authorize berth booking, dispatch, TOS mutation, custody transfer or payment.


## Tri-temporal replay extension (v0.2 research increment)

The `src/temporal.rs` module adds a separate projection over three axes:

1. **Physical valid time:** does the source claim apply at the physical instant being evaluated?
2. **Receiver knowledge time:** had the source published the revision and had this receiving system actually retrieved it by the decision cutoff, with rights valid at that cutoff?
3. **Independent adjudication time:** which adjudication revision had been recorded by the auditor's explicit audit cutoff?

The module exposes `TemporalCutoffs`, `TemporalEvidenceRevision`, `project_temporal_revision` and `select_decision_revisions`. Source-native event identity is keyed by `(source_id, event_id, source_revision)` rather than assuming event IDs are globally unique; the expected subject is checked before selection.

Important separation: the decision-revision selector does not inspect adjudication state. Later audit conflicts or later acceptance cannot change the source revision that was admissible to the historical policy. Audit projection separately selects the latest adjudication revision known by the audit cutoff and rejects conflicting same-revision adjudication records.

The temporal module currently has ten focused unit tests covering:
- later source correction vs historical decision cutoff;
- independent publication and receiver retrieval times;
- future plans known before their physical validity;
- later independent adjudication;
- physical validity and rights intervals;
- conflicting visible payloads under one source revision;
- decision projection independence from later audit conflicts;
- duplicate native event IDs from distinct sources remain distinct while shared dependency-cluster labels are preserved;
- visible wrong-subject records fail closed.

This remains a synthetic contract module. It is not yet a full ingestion adapter, cryptographic digest implementation, clock-uncertainty solver, or replacement for #7363's artifact/revision owner. See [#7512 — tri-temporal replay](https://github.com/Luminous-Dynamics/symtropy/issues/7512).
