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


### Independent tri-temporal fixtures and receipt (v0.2)

The temporal projection module now has its own grouped history fixture at `fixtures/temporal-cases.tsv` and deterministic executable `temporal-audit`. The corpus has 13 named temporal histories, including late publish/retrieval, physical validity, effective rights, future-known plans, audit cutoff, later source correction, native IDs from distinct sources, subject mismatch and conflicting source/adjudication revisions.

The independent Python reference re-parses those histories without importing the Rust crate. It checks both the hand-authored expected projection and the complete Rust temporal receipt. The exact-head workflow now emits two artifacts in one run: the original MDTR policy receipt and a separate tri-temporal audit receipt. The expected Rust test count is 22 (12 base + 10 temporal) before any further cases are added.

Run the complete check from the repository root:

```sh
cargo fmt --manifest-path research/maritime-decision-time-replay/Cargo.toml -- --check
cargo test --locked --manifest-path research/maritime-decision-time-replay/Cargo.toml
cargo clippy --locked --manifest-path research/maritime-decision-time-replay/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path research/maritime-decision-time-replay/Cargo.toml --bin mdtr-audit > mdtr-receipt.tsv
cargo run --locked --manifest-path research/maritime-decision-time-replay/Cargo.toml --bin temporal-audit > mdtr-temporal-receipt.tsv
python3 research/maritime-decision-time-replay/scripts/independent_reference.py --receipt mdtr-receipt.tsv --temporal-receipt mdtr-temporal-receipt.tsv
```

The temporal reference is a separate implementation and a separate receipt, not a second import or call into the Rust projection. Both the Rust selector and Python reference preserve source-local native event identity; a shared dependency-cluster label is retained as metadata but is not interpreted as statistical independence.

The v0.2 head is not qualified until the exact commit's dedicated workflow is green. Do not infer a pass from the earlier v0.1 workflow result.


## Additional topology-aware and dynamic optimization comparators (26 November 2026)

The 2026 comparator set also shows that a single generic "integrated terminal optimizer" is too coarse a benchmark category:

- **Deep RL-enhanced branch-and-price (March 2026):** integrated berth allocation, quay-crane assignment and yard assignment; uses a learned branching policy and a tripartite graph representation of the branch-and-price tree. This is a solver-acceleration comparator, distinct from plain C&CG or ALNS. DOI: [10.1016/j.cie.2026.111815](https://doi.org/10.1016/j.cie.2026.111815).
- **Dynamic AGV/yard-crane scheduling (January 2026):** reactive deep-RL integration of AGV and yard-crane operations under observed information rather than fixed handling/transport times. DOI: [10.1016/j.engappai.2025.112912](https://doi.org/10.1016/j.engappai.2025.112912).
- **Tidal ports and approach bridges (November 2026):** a new *Transportation Research Part B* study integrates berth and yard space allocation under tide-dependent mooring orientation and truck traffic over constrained approach bridges, using branch-and-price and data from two Ningbo-Zhoushan terminals. DOI: [10.1016/j.trb.2026.103584](https://doi.org/10.1016/j.trb.2026.103584).

The last paper matters for our modelling strategy: the physical topology and access corridors may form the binding resource, not berth length or crane count alone. A globally useful benchmark should parameterize tide windows, one-way bridges, narrow access corridors, turning/holding capacity, tug/pilot windows, yard access and inland gate/rail slots as time-indexed shared resources. These constraints must be explicit in the same physical feasibility oracle, rather than treated as soft penalties in a scheduler.

This suggests a **time-expanded, multi-layer resource graph**:
- nodes represent berth/yard/transfer/receiver states and physical interfaces;
- edges represent transport or handoff capabilities with duration, capacity, direction and operating windows;
- resource-time reservations represent cranes, AGVs, bridge segments, pilot/tug windows, OPS berths and inland slots;
- evidence receipts qualify whether a resource state or reservation was actually known and authorized at a decision cutoff.

For new ports, geography, tides, topology and standards vary. The benchmark should therefore measure transfer across *held-out port topologies*, not just held-out random seeds on one terminal layout. It must not imply that one topology or operating pattern generalizes globally without external validation.

## Benchmark extension priorities after v0.2

1. **Temporal histories:** clock uncertainty intervals, timezone normalization provenance, source revision supersession, effective-dated rights and append-only adjudication ordering.
2. **Network topology:** time-expanded berth/yard/bridge/AGV/rail/road/receiver graph with explicit capacities and restricted access.
3. **Contingent recovery:** non-anticipative recourse policies with query/wait/act/abstain options and hard decision deadlines.
4. **Resource state:** AGV/YC/QC health, charging, maintenance and shared repair resources; OPS/energy/carbon constraints.
5. **Benchmark transfer:** train/develop on selected topologies, freeze the evaluator, then test on held-out port geometries and disruption families.
6. **Field evaluation:** rights-cleared read-only terminal evidence, independent physical milestone adjudication, and unit-level receiver-side closure before any performance claim.

The v0.2 Rust temporal module has not yet qualified clock-uncertainty or source-revision supersession semantics. The above are explicit future acceptance targets, not implemented claims.


## Additional digital-twin, sea–rail and automation comparators (26 November 2026)

The field has moved beyond the assumption that a digital twin is merely a passive simulator:

- **Digital twin + expert-system integration review (published 1 April 2026):** a PRISMA-guided systematic review of 75 studies examines expert-system reasoning, adaptation, cross-domain coordination and sustainability optimization in container-terminal digital twins. Its central implication is that predictive reasoning and adaptive decision support are already active research directions. MDTR must therefore not claim novelty simply for "adding intelligence" to a digital twin. Its target is a separately testable assurance boundary: independent source/revision history, historical information availability, rights, hard-gate authority, dependence, and receiver-side outcomes. DOI: [10.1080/18366503.2026.2646718](https://doi.org/10.1080/18366503.2026.2646718).
- **Sea–rail intermodal optimization (published 7 January 2026):** integrates berth allocation and quay-crane scheduling with container storage and train operation scheduling in a sea–rail terminal. This is a reminder that inland rail is a coupled resource and service constraint, not merely a reporting metric. DOI: [10.1049/itr2.70123](https://doi.org/10.1049/itr2.70123).
- **Port equipment automation review (2026):** an equipment-level review of 47 qualitative-core papers and 124 review/conceptual papers highlights the interplay of cranes, AGVs, automated stacking equipment, digital twins, data protocols, energy management and cybersecurity. This supports a benchmark that tests interoperability and cyber-physical availability, not just solver optimality. DOI: [10.1186/s12544-026-00816-2](https://doi.org/10.1186/s12544-026-00816-2).
- **DCSA documentation migration (15 October 2026):** DCSA moved standard documentation into its consolidated public reference, covering Port Call, Track & Trace and other releases. Use the active reference artifacts and conformance guidance as the interface baseline; a conformant payload still does not prove source truth, physical movement or receiver acceptance. References: [DCSA announcement](https://dcsa.org/newsroom/standard-documentation-moved-to-reference), [Port Call 2.0 implementation guide](https://reference.dcsa.org/content/standards/releases/port-call/v2/pc-v2-implementation-guide).

### Revised paradigm

The research target is not "a smarter digital twin". It is an **evidence-qualified cyber-physical decision plane** that can sit across multiple digital twins, terminal operating systems, voyage/berth-alignment services and inland operators.

It should preserve and independently evaluate:
- which source revision each twin or operator actually received;
- whether a model state is observed, inferred, predicted or simulated;
- the model's valid-time and calibration envelope;
- source/common-cause dependencies and clock uncertainty;
- physical feasibility and authority as separate receipts;
- the receiver/cargo outcome after the intervention.

A prediction from a digital twin is not an observation. A simulator result is not a physical measurement. Expert-system output is not execution authority. Each claim type must retain its origin and ceiling.

### Global topology-transfer requirement

The exact-resource micro-oracle should grow from one tiny scenario into held-out topology families:
- tidal berth and approach-bridge corridors;
- sea–rail intermodal yards and train departure windows;
- one-way access routes and time-indexed turning/holding capacity;
- equipment charging, maintenance and shared repair crews;
- shore-power capable berths and grid/storage capacity;
- inland gate and receiver appointment capacity.

Measure train/test transfer across port topology and operating regime, not only random seeds from one geometry. Use public AIS only for vessel movement/call timing; unit-level cargo identity and receiver service require rights-cleared sources.


## Comparator refresh — November 2026

The external baseline is moving; this research track should not rely on older berth-only benchmarks.

### New strongest comparator details

- **Tidal port + approach bridge branch-and-price (November 2026):** Pang et al., *Transportation Research Part B* 213, DOI 10.1016/j.trb.2026.103584, jointly model berth-segment occupancy, tide-dependent vessel mooring orientation, yard allocation and truck flows over constrained approach bridges. The paper reports up to 10.9% better objective value than the best Gurobi solution within a two-hour limit on its large instances. That is a result on the authors' Ningbo-Zhoushan data and model—not a directly comparable gain on our synthetic cases. This must be a named exact/heuristic comparator, not just a literature citation.
- **Health-aware AGV + maintenance (22 October 2026):** Yang et al., *Journal of Industrial and Management Optimization* 22(11), DOI 10.3934/jimo.2026198, combine assignment, routing, charging, maintenance, degradation and stochastic failure in HA-QMIX. The authors explicitly report throughput variability across five seeds as a remaining limitation. Our tournament should publish more seeds, confidence intervals, tail throughput and paired-seed differences; one winning seed is insufficient.
- **Integrated berth/QC/AGV (15 September 2026):** Li & Li, DOI 10.3390/jmse14181714, combine RL-enhanced ALNS and GNN/MAPPO across berth, QC, buffer and AGV stages. Their work underlines that stage-coupling and buffer occupancy are necessary baseline dimensions.
- **OPS/energy dispatch (18 August 2026):** Li & Hu, DOI 10.1016/j.jclepro.2026.149105, couple berth, QC/YC, shore power and multi-source energy dispatch with carbon accounting. Their reported cost results are model/dataset-specific; we should compare against a reproduced model, not quote headline savings as expected field improvement.
- **Robust arrival/OPS planning (15 June 2026):** Wang et al., DOI 10.1016/j.oceaneng.2026.125850, combine berth/yard/shore power and uncertain vessel arrivals with adversarial hierarchical multi-agent SAC.

### Industry data and interoperability constraints

The World Bank / S&P Global CPPI 2025 is still the global vessel-time benchmark, not a cargo receiver-service benchmark. S&P reports its granular underlying dataset covers approximately 180,000 port calls annually at over 1,000 terminals in 500 ports, with vessel/operator timestamps and move counts. That dataset is commercial; any head-to-head benchmark requires licensed access and a transparent mapping of event boundaries. Public AIS is not a substitute for those call-level operational measurements.

StormGeo announced it acquired Portchain on 30 October 2026, explicitly positioning the combination around connecting voyage routing/speed decisions with current berth availability and terminal plans. This is evidence that voyage-to-berth coordination is an active, commercially integrated capability. The research differentiation should therefore be cross-vendor evidence, authority, revocation, historical replay, commitment completeness and independently adjudicated outcomes—not the claim that these systems do not coordinate.

DCSA's 2026 Standards Roadmap emphasizes moving from siloed spreadsheets/databases/manual re-entry toward interoperable APIs. Use DCSA's current Port Call and Track & Trace schemas/conformance materials as the event-interchange baseline; interoperability does not establish physical truth or receiver acceptance.

### How this changes our tournament

1. Match the tidal-port/bridge branch-and-price family on instances with berth segments, mooring orientation, tide windows and bridge capacities.
2. Match HA-QMIX with health degradation, maintenance bays, charging slots and stochastic AGV failures; publish at least 20 independent seeds as a research target.
3. Match GNN/MAPPO and RL-ALNS on identical buffer/AGV scenarios.
4. Match OPS/energy MILP and robust adversarial arrival planning on the same grid/berth scenarios.
5. Report CPPI-compatible vessel-time separately from terminal cargo and receiver service. Do not treat either as a proxy for the other.
6. Where commercial data is unavailable, publish the data gap and use open/synthetic instances only for the claims they support.

Sources: [Pang et al., tidal-port branch-and-price](https://doi.org/10.1016/j.trb.2026.103584); [Yang et al., HA-QMIX](https://doi.org/10.3934/jimo.2026198); [Li & Li, coordinated berth/QC/AGV](https://doi.org/10.3390/jmse14181714); [Li & Hu, OPS/energy dispatch](https://doi.org/10.1016/j.jclepro.2026.149105); [Wang et al., robust berth/yard/OPS](https://doi.org/10.1016/j.oceaneng.2026.125850); [CPPI 2025](https://www.worldbank.org/en/topic/transport/publication/cppi); [StormGeo–Portchain acquisition](https://stormgeo.com/insights/stormgeo-acquires-portchain-to-advance-just-in-time-voyage-optimization-at-scale); [DCSA Standards Roadmap 2026](https://dcsa.org/newsroom/dcsa-standards-roadmap-2026).


## Interval-valued clock uncertainty (v0.3 research increment)

The `src/clock_bounds.rs` module extends replay from scalar timestamps to conservative closed intervals on a caller-declared normalized time scale. A comparison `A <= B` is:
- definitely true when `A.latest <= B.earliest`;
- definitely false when `A.earliest > B.latest`;
- uncertain when the intervals overlap and neither relation is established.

Exact boundary equality is inclusive. The qualifier applies this to:
- physical valid-from / valid-until vs physical target;
- publication and actual retrieval vs decision cutoff;
- effective rights start/end vs decision cutoff.

A record is definitely admissible only when every comparison is established. Overlap produces `TEMPORALLY_UNCERTAIN`; a definite violation produces `DEFINITELY_INADMISSIBLE`. If one comparison is uncertain and another is definitely false, the result is inadmissible while retaining both reason codes.

The module has 12 Rust tests for equality, publication/retrieval overlap, definite late publication, effective-rights windows, physical validity, mixed uncertainty/violation, negative epoch values and invalid intervals. The frozen `fixtures/clock-cases.tsv` corpus has 12 named cases; `clock-bounds-audit` emits a deterministic receipt and `scripts/clock_reference.py` independently recomputes the interval ordering without importing Rust.

Run the full expanded validation from the repository root:

```sh
cargo fmt --manifest-path research/maritime-decision-time-replay/Cargo.toml -- --check
cargo test --locked --manifest-path research/maritime-decision-time-replay/Cargo.toml
cargo clippy --locked --manifest-path research/maritime-decision-time-replay/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path research/maritime-decision-time-replay/Cargo.toml --bin mdtr-audit > mdtr-receipt.tsv
cargo run --locked --manifest-path research/maritime-decision-time-replay/Cargo.toml --bin temporal-audit > mdtr-temporal-receipt.tsv
cargo run --locked --manifest-path research/maritime-decision-time-replay/Cargo.toml --bin clock-bounds-audit > mdtr-clock-receipt.tsv
python3 research/maritime-decision-time-replay/scripts/independent_reference.py --receipt mdtr-receipt.tsv --temporal-receipt mdtr-temporal-receipt.tsv
python3 research/maritime-decision-time-replay/scripts/clock_reference.py --receipt mdtr-clock-receipt.tsv
```

The supplied bounds are not proof that a source clock is correctly synchronized. The evidence envelope must preserve clock source, timezone/offset provenance, timestamp precision, calibration and derivation of earliest/latest bounds. This module assumes the intervals have already been normalized onto one declared time scale; it does not itself authenticate clocks or infer timezones.

The interval-clock v0.3 increment subsequently passed exact-head qualification; see the exact-head result below.


## Exact-head qualification record — v0.3 interval clocks

The dedicated `mdtr-exact-head` workflow passed on commit `029686724a07870d50051e07211735a50afa1472`:
- Formatting: pass
- Rust tests: 34 passed, 0 failed (12 base + 10 tri-temporal + 12 interval-clock)
- Strict Clippy: pass
- Decision receipt and independent reference: 14/14
- Tri-temporal receipt and independent reference: 13/13
- Interval-clock receipt and independent reference: 12/12
- All three deterministic receipts uploaded

The general repository CI matrix was still in progress at the time of this inspection; it is not represented as part of the dedicated research workflow pass.

The clock extension establishes only conservative ordering relative to the supplied intervals. It does not validate source-clock authenticity, timezone provenance, calibration or synchronization. The v0.3 claim is bounded to the synthetic interval corpus and implementation contract.
