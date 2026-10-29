# FED-RES-001: Federated Defensive Resilience Benchmark

**Status:** design proposal for review. This branch includes the initial reference safety kernel; the full scenario corpus and performance benchmark are not implemented. No performance claim is implied.  
**Scope:** synthetic civil-protection and critical-service resilience only.  
**Parent/composition references:** [Mycelix CIV-RES-000 (#2006)](https://github.com/Luminous-Dynamics/mycelix/issues/2006), [Mycelix LEX-NET-021 (#1117)](https://github.com/Luminous-Dynamics/mycelix/issues/1117), [Symtropy BANK-RES-SIM-001 (#2330)](https://github.com/Luminous-Dynamics/symtropy/issues/2330), and [Symtropy QUAL-002 (#1280)](https://github.com/Luminous-Dynamics/symtropy/issues/1280).

## 1. Research question

Can independently governed organizations coordinate verified incident response and restore essential services faster than credible baselines, while preserving local autonomy and preventing forged, stale, contradictory, or unauthorized information from becoming authoritative?

The proposed **Federated Defensive Resilience Benchmark (FDRB)** tests this as a reproducible systems property, not as a claim about any one model or protocol. It is intended to qualify a civilian resilience coordination substrate across communications outages, evidence conflicts, compromised participants, resource dependencies, and recovery decisions.

The candidate stack may compose:

- **Mycelix:** participant identity and credentials, policy-scoped authorization, evidence provenance, correction/revocation history, and auditable decisions.
- **Symthaea:** read-only analysis, competing hypotheses, anomaly prioritization, uncertainty and calibration receipts. A model output is evidence or a recommendation, never authority.
- **Symtropy:** deterministic disruption scenarios, dependency/resource dynamics, recovery rollouts, paired baselines, and counterfactual comparison.
- **Sol-Atlas:** source-aware asset, lifeline, location, and capacity context. It must preserve source freshness and uncertainty and must not expose protected detail outside policy.

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

## 5. Required adversarial scenario families

1. **Benign outage:** distinguish an equipment/service failure from a malicious event; preserve uncertainty while the cause is unknown.
2. **Fabricated report:** inject a signed but unsupported or misattributed claim; it must not become a verified incident merely because it is authenticated.
3. **Duplicate and reordered messages:** verify idempotency and stable event lineage when offline queues replay data.
4. **Network partition and merge:** let each participant retain its local state; detect divergent views and reconcile only under explicit policy.
5. **Revocation during partition:** an issuer or credential becomes revoked while a participant is offline. The old credential must not gain wider authority because currentness cannot be checked.
6. **Conflicting signed histories:** two validly signed but incompatible statements appear for the same subject/context. Preserve the conflict; do not use last-write-wins or model preference to declare a winner.
7. **Compromised intermediary:** a relay delays, duplicates, drops, or rewrites transport metadata. End-to-end content commitments and recipient acceptance must remain distinct from transport receipts.
8. **Model confidently wrong:** provide a model recommendation inconsistent with ground-truth fixtures. It must remain non-authoritative, with uncertainty and source lineage visible.
9. **Disclosure boundary:** request an output that contains fields outside the recipient's capability. The independent verifier must detect the disclosure even if the request is signed.
10. **Common-cause dependency failure:** multiple nominally redundant services share a power, communications, supplier, or transport dependency. Count actual independent capacity, not provider/node count.
11. **Scarce recovery resources:** compare recovery schedules under limited crews, equipment, energy, or supplies; ensure infeasible plans are not reported as successful.
12. **Stale or corrected source:** supersede an observation or model/input snapshot. Preserve the historic record while preventing stale evidence from being represented as current.

Every scenario needs an independently authored oracle that states the permitted state transitions, expected evidence dispositions, known ground truth, and non-negotiable safety invariants. Scenario generation code must not also serve as the only oracle.

## 6. Fair baselines

Run each scenario and seed against the same initial state, event stream, available information, resource budget, and recovery objective.

- **B0 — manual coordination proxy:** explicit notification delay, human review time, and service-desk handoff assumptions, all declared before measurement.
- **B1 — centralized broker:** a single coordinator with conventional authenticated messaging and a shared incident record.
- **B2 — signed federation:** peer exchange with signatures and deduplication but without the candidate's full evidence-state, conflict, currentness, and authorization semantics.
- **C1 — candidate federation:** explicit evidence states, policy-scoped disclosure, revocation/currentness handling, and independent verification; analytical recommendations remain non-authoritative.
- **C2 — candidate + analysis/simulation:** add Symthaea prioritization and Symtropy/Sol-Atlas recovery context, but compare only recommendations against the same oracle and resource constraints.

These are **research baselines**, not claims that any named vendor or deployed system has been reproduced. All simplifying assumptions must be published. Include a centralized baseline because decentralization is not automatically safer, faster, or more available.

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

## 8. Provisional targets

These are proposed research gates, not measured results or universal service-level guarantees. Revisit them only through a documented corpus/methodology revision.

1. **Safety:** meet every zero-violation gate above on all 360 paired runs and their fault controls.
2. **Correctness under uncertainty:** 100% of oracle-labelled stale, revoked, unsupported, or contradictory evidence remains non-authoritative unless a separately qualified profile explicitly permits a bounded consequence.
3. **Local resilience:** 100% of partition tests preserve the configured local emergency functions; no network dependency may silently expand a participant's authority.
4. **Recovery improvement:** at least 30% lower p95 verified-information-to-recovery-plan time than B0, with equivalent ground-truth coverage and no increase in safety violations.
5. **Healthy-network overhead:** target no more than 10% additional median verified-information latency versus B1 in the no-fault control, while retaining evidence and policy properties absent from the baseline.
6. **Analysis benefit:** at a fixed false-positive budget, C2 should improve verified incident prioritization or recovery outcomes over C1; if it does not, do not claim that adding Symthaea improves the system.
7. **Generalization:** maintain reported results on held-out scenario seeds and at least one held-out dependency topology. No tuning on the held-out set.

If any target proves unrealistic, publish the gap and its cause. Do not relax the safety gates to make a performance result look good.

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
