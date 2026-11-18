# AERO-HAPS-HANDOVER-001 — group handover, service continuity and fail-closed authority transfer

**Status:** synthetic-first deterministic protocol benchmark v0.1  
**Tracker:** [#6113](https://github.com/Luminous-Dynamics/symtropy/issues/6113)  
**Fleet owner:** [AERO-HAPS-FLEET-AVAIL-001 / PR #6074](https://github.com/Luminous-Dynamics/symtropy/pull/6074)  
**Network owner:** [AERO-HAPS-NET-ORCH-001 / PR #5948](https://github.com/Luminous-Dynamics/symtropy/pull/5948)  
**Claim ceiling:** deterministic cohort admission and authority accounting only.

## 1. Why handover is a primary service metric

A fleet can have enough airborne platforms and still fail users during handover. In a HAPS/terrestrial/LEO system, aircraft availability is not equal to user continuity. The target can reject some users because of aggregate bitrate, access restrictions or priority; some users may complete while others fail; backhaul may disappear during preparation; and a lost completion acknowledgement must not be treated as a successful state transition.

ITU-T Y.3212 §7.3.4 specifies group handover in which the source forwards UE group context and the target responds with **confirmed and failed UE subsets**. The target's confirmation depends on restrictions, aggregate bitrate capacity, access priority and backhaul information. This is a better contract than a binary “handover succeeded” flag. [ITU-T Y.3212](https://www.itu.int/rec/dologin_pub.asp?id=T-REC-Y.3212-202409-I%21%21PDF-E&lang=e&type=items).

The closest direct research baseline identified is Suwalka, Ivanova and Jain, *Semi-Markov-based analytical framework for efficient and reliable inter-HAPS handoff management in communication networks*, *Computer Networks* 283 (June 2026), DOI 10.1016/j.comnet.2026.112316. It uses five handover states: proper connection, initiation, preparation, execution and completion; it models non-exponential preparation/execution sojourns and evaluates steady-state probabilities, queue length, packets and delay. It compares with M/M/1/N. This is a published analytical baseline to reproduce, not an operational handover SLA. [DOI 10.1016/j.comnet.2026.112316](https://doi.org/10.1016/j.comnet.2026.112316).

## 2. Paradigm shift: cohort continuity plus explicit authority transfer

This benchmark separates four things that are often collapsed:
1. **Admission:** which users can the target actually accept under UE and aggregate bitrate limits?
2. **Execution:** which admitted users finish all handover phases before their service deadline?
3. **Authority:** which node is authoritative for each user after the protocol event?
4. **Accounting:** every offered UE must be counted exactly once as completed, rejected, rolled back or unresolved.

The source remains authoritative until a valid target completion acknowledgement is received. A lost acknowledgement is **unresolved**, not success. It must trigger reconciliation while preserving source authority. A duplicate acknowledgement is idempotent and cannot create an additional commit. For safety-critical drone control, late/uncertain transfer uses a local safe mode rather than store-and-forward.

The initial deterministic state path is:

`proper connection → initiation → preparation → execution → completion acknowledgement`

The implementation injects failures at initiation, preparation, execution and completion acknowledgement, plus stale-context rejection and backhaul unavailability. It deliberately does not claim to implement the full five-state semi-Markov distribution model.

## 3. Initial machine-readable artifacts

- [haps_group_handover_case_v1.schema.json](schemas/haps_group_handover_case_v1.schema.json)
- [synthetic_haps_group_handover_case_v1.json](examples/synthetic_haps_group_handover_case_v1.json)
- [Standalone Rust checker](../../../../tools/haps-group-handover)
- [Dedicated GitHub Actions workflow](../../../../.github/workflows/haps-group-handover.yml)

The fixture has three synthetic cohorts:
- Drone control: 12 safety-critical users, 100 kbps/user, 100 ms handover deadline, stateful, local-safe-mode fallback.
- Emergency alert: 40 users, 50 kbps/user, 250 ms deadline.
- Public broadband: 100 users, 250 kbps/user, 500 ms deadline.

The HAPS target has a synthetic 80-user-equivalent limit and 8,000 kbps aggregate bitrate limit. Priority rank is lower-is-higher; ties use ascending cohort ID. Admission is partial by UE using integer bitrate arithmetic.

## 4. Independent deterministic result

The first independent replay matches all five expected scenario oracles:

| Scenario | Offered | Admitted | Completed | Rejected | Rolled back | Unresolved | Safety local fallback |
|---|---:|---:|---:|---:|---:|---:|---:|
| Nominal partial group admission | 152 | 71 | 71 | 81 | 0 | 0 | 0 |
| Backhaul unavailable | 152 | 0 | 0 | 152 | 0 | 0 | 12 |
| Execution failure | 152 | 71 | 0 | 81 | 71 | 0 | 12 |
| Completion acknowledgement lost | 152 | 71 | 0 | 81 | 0 | 71 | 12 |
| Safety-critical deadline overrun | 152 | 71 | 59 | 81 | 12 | 0 | 12 |

In the nominal scenario, the target admits all 12 drone-control users and 40 emergency users, then 19 broadband users. The remaining 81 broadband users are rejected by the target limits. All 71 admitted users complete in 95 ms; the duplicate completion acknowledgement is ignored. There are zero authority overlaps.

In the execution-failure case, all 71 admitted users roll back and source authority remains. In the acknowledgement-loss case, all 71 are unresolved; no target authority is granted and reconciliation is required. In the 120 ms scenario, the 12 drone-control users exceed their 100 ms handover limit and are rolled back into local safe mode; the emergency and broadband cohorts complete.

These are synthetic scheduling/protocol-accounting figures, not measured handover, network, radio or operator performance.

## 5. Matched baselines and next research increment

Reproduce:
1. Break-before-make and make-before-break static group handover.
2. Priority-aware admission with target UE/aggregate bitrate constraints.
3. Earliest-deadline and maximum-aggregate-bitrate admission.
4. The Suwalka et al. 2026 five-state Semi-Markov model, including non-exponential phase duration distributions.
5. The paper's M/M/1/N comparison under matched assumptions.
6. ITU-T Y.3212 group admission/confirmed-failed subset behavior.
7. A fail-closed protocol with prepare/commit/rollback, idempotent acknowledgement and explicit reconciliation.

All methods must use the same cohort traces, bitrate distributions, user priorities, target capacity, phase distributions, backhaul loss process and source/target availability. Hold out full traffic campaigns, network topologies and failure regimes.

Research targets, not achievements:
- 100% of offered users accounted for as completed, rejected, rolled back or unresolved.
- Zero source release before target completion acknowledgement.
- Zero duplicate-authority states and zero duplicate commits after repeated acknowledgements.
- Zero safety-critical users transferred after their deadline; they must enter the declared local safe mode.
- Report p50/p95/p99 initiation, preparation, execution, completion and total interruption times separately by service class.
- Once the Semi-Markov baseline is reproduced, target at least 10% lower p95 interruption or 10% higher accepted critical traffic on held-out traces, without changing capacity, radio, forecast or compute budgets.

## 6. Validation and claim ceiling

The checked-in synthetic fixture was independently replayed and all five expected cohort/aggregate receipts matched. The standalone Rust crate has 12 authored unit tests for admission, tie-break, backhaul failure, execution rollback, lost acknowledgement, deadline overrun, fallback, idempotency, accounting and authority partition. These tests have **not** been executed locally; the dedicated workflow must establish formatting, Clippy, compilation and test results.

No full JSON Schema validator, RF/SINR model, packet simulator, mobility model, stochastic or Semi-Markov distribution, live subscriber trace, spectrum authorization, live radio service, drone command authority, flight test or operating permission is included.

## 7. Ownership and composition

Compose:
- fleet availability and maintenance: [PR #6074](https://github.com/Luminous-Dynamics/symtropy/pull/6074);
- traffic/beam allocation: [PR #5948](https://github.com/Luminous-Dynamics/symtropy/pull/5948);
- edge placement: [issue #5989](https://github.com/Luminous-Dynamics/symtropy/issues/5989);
- HAPS service contract and cross-layer architecture: [issue #5885](https://github.com/Luminous-Dynamics/symtropy/issues/5885);
- mission electro-thermal and battery qualification: [issue #11411](https://github.com/Luminous-Dynamics/symthaea/issues/11411) and [issue #3903](https://github.com/Luminous-Dynamics/symtropy/issues/3903).

This work owns handover cohort state, authority transfer and per-user accounting. It does not create a second radio, mobility, flight, battery or Semi-Markov reliability solver.
