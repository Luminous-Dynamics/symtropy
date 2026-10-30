# FED-RES-001: Federated Defensive Resilience Benchmark

**Status:** design proposal for review. This branch includes the initial reference safety kernel; the full scenario corpus and performance benchmark are not implemented. No performance claim is implied.  
**Scope:** synthetic civil-protection and critical-service resilience only.  
**Existing work to compose (not replace):** [Mycelix CIV-RES-000 (#2006)](https://github.com/Luminous-Dynamics/mycelix/issues/2006), [LEX-NET-Q1 (#1057)](https://github.com/Luminous-Dynamics/mycelix/issues/1057), [LEX-NET-021 (#1117)](https://github.com/Luminous-Dynamics/mycelix/issues/1117), [CIV-IMPACT-001 (#5046)](https://github.com/Luminous-Dynamics/mycelix/issues/5046), [BANK-RES-001 (#5302)](https://github.com/Luminous-Dynamics/mycelix/issues/5302), [LOG-COMMONS-S0-001 (#4901)](https://github.com/Luminous-Dynamics/mycelix/issues/4901), [Symtropy BANK-RES-SIM-001 (#2330)](https://github.com/Luminous-Dynamics/symtropy/issues/2330), [Sol-Atlas BANK-RES-GEO-001 (#204)](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/204), [Symthaea BANK-RES-DECISION-001 (#8206)](https://github.com/Luminous-Dynamics/symthaea/issues/8206), and [Symtropy QUAL-002 (#1280)](https://github.com/Luminous-Dynamics/symtropy/issues/1280).

## 1. Research question

Can independently governed organizations coordinate verified incident response and restore essential services faster than credible baselines, while preserving local autonomy and preventing forged, stale, contradictory, or unauthorized information from becoming authoritative?

The proposed **Federated Defensive Resilience Benchmark (FDRB)** is a composition and end-to-end outcome profile over those existing workstreams, not a competing federation protocol, authority model, service simulator, or general composed-stack value benchmark. It tests whether qualified federation/evidence inputs and operational recovery plans produce better critical-service outcomes under reproducible disruptions.

The candidate stack may compose:

- **Mycelix:** participant identity and credentials, policy-scoped authorization, evidence provenance, correction/revocation history, and auditable decisions.
- **Symthaea:** read-only analysis, competing hypotheses, anomaly prioritization, uncertainty and calibration receipts. A model output is evidence or a recommendation, never authority.
- **Symtropy:** deterministic disruption scenarios, dependency/resource dynamics, recovery rollouts, paired baselines, and counterfactual comparison.
- **Sol-Atlas:** source-aware asset, lifeline, location, and capacity context. It must preserve source freshness and uncertainty and must not expose protected detail outside policy.

### Existing work ownership and anti-duplication boundary

The repositories already contain most of the semantic and domain-specific pieces. This tranche must compose them rather than create a second owner:

| Concern | Canonical work owner | FDRB role |
|---|---|---|
| Institutional authority, privacy planes, civic evidence and contestability | Mycelix CIV-RES-000 (#2006) and its child work | Consume only qualified, profile-bound outputs |
| Cross-sovereign identity, recognition, revocation, policy conflict and equivocation semantics | Mycelix LEX-NET-Q1 (#1057) plus LEX-NET-021/022/024/025/031/034 | Import dispositions and replay their negative controls; do not implement a competing resolver |
| Critical-operation pilot, dependency inventory and practical pilot gates | Mycelix BANK-RES-001 (#5302) | Provide an operational measurement profile only where the pilot applies |
| Incremental value of the composed four-project stack, baselines and ablations | Mycelix CIV-IMPACT-001 (#5046) | Reuse paired comparisons, ablation discipline and pre-registration; do not create a second value leaderboard |
| Offline logistics scenario kernel and independent verifier | Mycelix LOG-COMMONS-S0-001 (#4901) | Reuse it for logistics-specific fixtures rather than writing another twin |
| Critical-service disruption/recovery simulation | Symtropy BANK-RES-SIM-001 (#2330) | Extend its existing scenario family and outcome receipts |
| Qualified site/lifeline evidence | Sol-Atlas BANK-RES-GEO-001 (#204) | Consume source-/freshness-qualified dependency inputs |
| Mitigation, uncertainty and abstention evaluation | Symthaea BANK-RES-DECISION-001 (#8206) | Consume model recommendations as proposals; measure incremental benefit |

Issue existence and draft documentation do not imply that any cross-repository component is implemented or qualified. Before adding a new scenario, schema, oracle, or validator, review the current owner and reuse its contract wherever semantics match.

The small Rust kernel in this directory is a consumer-side regression guard for decisions at the simulation boundary. It is not a replacement authority engine, an independent federation implementation, or proof that Mycelix's upstream rules have passed qualification. Once exact upstream receipts/contracts are available to the adapter, move shared semantics into conformance tests and remove divergent duplicate rules.

This benchmark does not require these components to be integrated before the first synthetic fixture can be implemented. Component boundaries and adapter contracts should be tested independently before composition.

## 2. Core hypothesis and paradigm shift

The paradigm shift is **proof-carrying coordination rather than trust-by-dashboard**. Every consequential shared state transition should carry enough evidence for an independent verifier to reconstruct:

1. who or what asserted the event;
2. which policy and trust profile applied;
3. what evidence and source versions were available at the time;
4. whether the evidence was current, stale, disputed, revoked, or incomplete;
5. which organization retained decision authority;
6. what was recommended, what was authorized locally, what actually changed, and how the effect was verified.

A valid signature proves integrity/origin under its key assumptions; it does not prove truth, currentness, institutional authority, or non-equivocation. A majority vote, aggregate reputation score, or high model confidence must not fill those gaps.

The network should be **fail-closed for privilege and authoritative shared claims, but graceful-degrading for local essential service**. A communications partition may pause cross-organization coordination; it must not silently grant new authority or unnecessarily disable local emergency operations.

## 3. Non-goals and safety boundaries

FDRB is restricted to synthetic civil protection and continuity-of-service scenarios, such as electricity, water, telecommunications, healthcare logistics, municipal services, and disaster recovery. It does not model attack execution, weapon systems, target selection, force deployment, or autonomous engagement.

It does not establish legal authority, institutional legitimacy, production readiness, regulatory compliance, or real-world security. No benchmark result authorizes deployment or the use of force. Any live pilot requires participant agreements, lawful authority, independent security/privacy review, operator training, and a rollback plan.

## 4. Benchmark worlds and reproducibility

Use generated fixtures only; no real personal information, classified data, production telemetry, live threat indicators, or operational secrets.

Initial scale points:

| Profile | Participants | Service/dependency nodes | Purpose |
|---|---:|---:|---|
| Local | 5 | 20 | Debuggable minimal federation |
| Regional | 12 | 100 | Cross-organization recovery and realistic dependency graphs |
| Extended | 25 | 500 | Scaling and correlated-failure stress |

A participant represents an independently governed organization or service operator, not a military unit. The dependency graph identifies which services rely on other services or resources. Each event has a stable ID, source identity, event time, observation time, policy/profile version, evidence references, and an explicit evidence state.

Freeze the initial corpus at **12 scenario families × 30 deterministic seeds = 360 paired scenario runs per baseline**, plus zero-severity and no-fault controls. Record the exact seeds, corpus digest, scenario generator revision, candidate commit SHA, toolchain, configuration, and verifier version. Add scenario families only by versioning the corpus; never silently change old expected outcomes.

The simulation and verifier must run offline and deterministically. A result should be reproducible from committed fixtures and a locked dependency graph without hosted CI or external services.

## 5. Required end-to-end scenario families

The federation and evidence adversarial cases are already owned by Mycelix LEX-NET-Q1 (#1057) and its referenced theorems. FDRB should import their independently qualified dispositions or run their existing frozen fixtures as prerequisites; it must not invent a competing answer for whether a credential, history, claim, or cross-sovereign authorization is valid.

FDRB's novel unit of evaluation is **operational consequence and recovery quality after a qualified event/disposition enters the service model**:

1. **Single service outage:** compare feasible recovery plans after one essential node becomes unavailable.
2. **Partial degradation:** model a service below minimum capacity without incorrectly marking it fully available.
3. **Common-cause failure:** shared power, telecom, supplier, or transport loss removes several apparently redundant alternatives.
4. **Dependency inventory gap:** omit, stale-date, or mark a critical dependency unknown; measure false assurance and abstention.
5. **Provider substitution:** evaluate a proposed replacement only when capacity, compatibility, time and evidence constraints support it.
6. **Resource-constrained recovery:** allocate scarce crews, spares, transport, energy, or supplies; reject infeasible plans.
7. **Recovery route failure:** a proposed recovery path becomes unavailable partway through the plan.
8. **Partitioned organizations:** each participant preserves local essential functions; the global recovery plan marks cross-org prerequisites as pending rather than inventing capacity.
9. **Safe reconnection:** after a partition, the model handles duplicated, delayed and conflicting updates using upstream Mycelix dispositions and measures reconciliation delay.
10. **Uncertain incident severity:** compare plans under uncertain severity and cost; measure sensitivity, robustness and the quality of abstention.
11. **Incorrect analytical recommendation:** inject a confidently wrong Symthaea proposal and verify it cannot bypass resource feasibility, upstream evidence disposition or human/local authorization.
12. **Held-out topology and demand:** measure transfer to previously unseen dependency graphs, demand profiles and correlated disruptions without tuning on the holdout.

Each operational scenario must bind an independent oracle for service state, resource conservation, feasible recovery, and expected service outcomes. It must not re-adjudicate upstream identity, legal, trust, evidence-currentness or authority semantics. Reuse the LOG-COMMONS-S0 verifier for logistics fixtures and BANK-RES-SIM-001 for critical-service scenarios where those contracts match.

## 6. Fair baselines

Run each scenario and seed against the same initial state, event stream, allowed information, resource budget, and recovery objective. The exact experimental arms are subordinate to Mycelix CIV-IMPACT-001 (#5046) and the chosen vertical's existing owner; they are not a competing value methodology.

- **B0 — incumbent/manual or simple rules:** the declared current workflow or strongest simple graph/rules heuristic, with human-review and handoff assumptions fixed before measurement.
- **B1 — centralized coordination:** a centralized broker with conventional authenticated messaging and a shared incident record.
- **B2 — upstream federation + simple recovery:** qualified Mycelix evidence/authorization dispositions (when available for the tested profile) feed a simple rules/graph recovery planner.
- **C1 — upstream federation + Symtropy recovery:** same inputs and upstream dispositions, with Symtropy service/resource simulation and recovery planning.
- **C2 — composed analysis arm:** C1 plus qualified Sol-Atlas infrastructure context and Symthaea non-authoritative recommendations.
- **Ablations:** where feasible, remove one component at a time to measure incremental contribution, as required by CIV-IMPACT-001.

These are **candidate comparison arms**, not claims that any named vendor or deployed system has been reproduced. If an upstream Mycelix disposition is not independently qualified, mark the relevant scenario as blocked/unqualified; do not synthesize a replacement authority decision inside Symtropy. Any simplifications and missing upstream results must be explicit in the report.

Use identical scenario inputs, information permissions, resource budgets, and decision horizons. The candidate must not see evaluator-only labels or holdout truth. Include a centralized baseline because decentralization is not automatically safer, faster, or more available.

## 7. Metrics

Report raw counts and uncertainty alongside aggregate scores. Do not collapse safety and speed into one weighted leaderboard number.

### Safety and evidence integrity — hard gates

- Unauthorized authority-bearing transitions: **0**.
- Model-only authorization or promotion of a recommendation to an authoritative finding: **0**.
- Stale, revoked, or contradictory evidence accepted as verified current truth without a qualifying rule: **0**.
- Unauthorized protected fields disclosed: **0**.
- Consequential transitions missing required actor/issuer, policy/profile version, evidence references, event identity, and verification status: **0**.
- Conflicting signed branches silently discarded: **0**.
- Duplicate/replayed delivery producing a duplicate consequential effect: **0**.

A single violation fails the candidate for that corpus revision, regardless of performance.

### Operational and resilience measures

- **Detection quality:** precision, recall, false positives per 1,000 benign events, and detection delay against the independent oracle.
- **Verified information latency:** median and p95 time from first eligible observation to verified cross-participant receipt.
- **Recovery:** median and p95 time to restore each essential service to its declared minimum capacity; capacity-weighted service-hours lost; resource feasibility violations.
- **Partition behaviour:** percentage of local essential functions retained, time to detect divergent federation views, and time to safely resume permitted exchange after recovery.
- **Evidence quality:** proportion of claims correctly labelled verified, disputed, stale, revoked, unknown, or corrected.
- **Privacy:** number and sensitivity of fields disclosed beyond recipient capability; aggregate reconstruction tests in later versions.
- **Analysis quality:** Brier score/calibration error where valid probabilities are produced, selective risk when the model abstains, and performance under distribution shift.
- **Cost:** CPU/memory, network bytes, storage growth, and coordination overhead per verified incident.
- **Interoperability:** successful exchange and correct semantic preservation across versioned participant profiles, including unsupported-feature and downgrade cases.

Report results by scenario, severity, participant count, baseline, and seed. Never average away a critical safety failure.

## 8. Provisional qualification gates

These are proposed gates, not measured results or universal service-level guarantees. The primary outcome and minimum practically meaningful improvement must be selected with the target-domain owner and preregistered before confirmatory outcomes are observed. Mycelix CIV-IMPACT-001 (#5046) owns the composed-stack incremental-value methodology, and BANK-RES-001 (#5302) explicitly requires domain-specific thresholds rather than an invented universal percentage.

1. **Safety:** meet the applicable upstream Mycelix negative-control gates and operational invariants on every scored run. A safety violation cannot be offset by a performance improvement.
2. **Correctness under uncertainty:** 100% of oracle-labelled stale, revoked, unsupported, or contradictory upstream evidence remains non-authoritative unless a separately qualified profile explicitly permits a bounded consequence.
3. **Local resilience:** 100% of partition tests preserve configured local essential functions; a network dependency must not silently expand authority or make unavailable local capacity appear available.
4. **Pre-registered practical effect:** the domain owner chooses a primary service outcome and minimum practical effect before the confirmatory phase. Do not use a universal percentage target across cyber, logistics, energy and other domains.
5. **Fully loaded cost:** include data preparation, integration, compute, human review, failure handling and recovery—not only inference latency.
6. **Incremental analytical benefit:** at a fixed false-positive and resource budget, the analysis-enabled arm should improve a preregistered service outcome over the same stack without Symthaea; otherwise report no demonstrated incremental benefit.
7. **Generalization:** report held-out scenario seeds and at least one held-out dependency topology. No tuning on the held-out set.
8. **Accounting completeness:** every scheduled run produces an explicit success, failure, invalid, or indeterminate disposition. No silent exclusion of unfavorable results.

If any gate fails, publish the failure and its cause. Do not relax safety gates to make a performance result look good. A zero-violation result over a finite synthetic corpus is evidence only about that corpus, not proof of zero real-world risk.

## 9. Execution plan

**Stage A — spec and semantic freeze.** Define typed event/evidence states, scenario schema, oracle contract, seed handling, baseline assumptions, and output format. No network access and no production data.

**Stage B — independent reference model (initial slice included).** A dependency-free standalone Rust reference kernel and unit tests are included under `benchmarks/fed-res-001/`. The tests cover local recovery during partition, blocked federation publication during partition, model non-authority, revoked credentials, disputed evidence, corroboration, disclosure, replay, untrusted observations, and non-authoritative analysis. This kernel checks fixture-declared fields only; a separately implemented oracle must still validate source identity/independence, event ground truth, expected state transitions, and recovery outcomes.

**Stage C — synthetic event/recovery simulator.** Add dependency graphs, resource budgets, disruption schedules, paired baseline runs, and deterministic reports. Keep the first benchmark discrete-event and graph-based; do not force it into rigid-body physics where that adds no evidentiary value.

**Stage D — cross-project adapters.** Map Mycelix receipts/policies to the scenario contract, Symthaea outputs to non-authoritative recommendations with immutable input-cut references, and Sol-Atlas assets to source/freshness-qualified dependency inputs. Reject lossy adapters.

**Stage E — independent qualification.** Run exact-head offline tests; verify corpus hashes; test at least one negative control per safety invariant; report source/static checks separately from executed tests. Hosted runner status is not a substitute for local evidence, and queued jobs are not passes.

**Stage F — shadow-mode pilot decision.** Only after synthetic qualification, independent security review, governance agreements, privacy review, operator exercise, and rollback planning. Begin with low-risk service continuity; do not include sensitive safety-reporting or privileged operational control in v1.

## 10. Advancement criteria and publication

Every report should include the exact commit SHA, corpus digest, environment, command, seed coverage, raw per-run results, verifier output, baseline assumptions, missing cases, failures, and the exact statement supported by the evidence.

Publish negative results and all safety-gate failures. A benchmark PASS means only that the frozen implementation passed the frozen synthetic corpus. It does not prove absence of vulnerabilities, readiness for production, legal compliance, or superiority to an untested vendor.

## 11. Standards to reuse

- [NIST SP 800-207 — Zero Trust Architecture](https://csrc.nist.gov/pubs/sp/800/207/final)
- [NIST Cybersecurity Framework 2.0](https://www.nist.gov/cyberframework)
- [OASIS STIX/TAXII 2.1](https://www.oasis-open.org/standards/)
- [FIRST Traffic Light Protocol 2.0](https://www.first.org/tlp/)
- [CISA Cross-Sector Cybersecurity Performance Goals 2.0](https://www.cisa.gov/cross-sector-cybersecurity-performance-goals)
- [MITRE ATT&CK Evaluations](https://attackevals.mitre.org/enterprise/er8)
- [NATO Federated Mission Networking framework](https://coi.nato.int/FMNPublic/SitePages/Home.aspx)

The intent is standards-compatible integration, not a new proprietary replacement for these efforts.
