# Protective Capability Assurance E5 — continuous qualification and requalification

**Snapshot:** 16 November 2026  
**Owner:** PROTECT-CAPABILITY-ASSURANCE-001 #5710  
**Branch:** `research/protect-capability-assurance-e5-v0-1`  
**Parent:** E4 typed semantic projections #5643 / PR #5708  
**Status:** synthetic capability graph, JSON Schema, Rust impact/requalification oracle and 23 tests written; exact-head Rust execution pending

## 1. Paradigm shift

E5 moves from a static protective-systems risk register to a **continuous capability-assurance graph**.

A certificate, inspection or test report can remain historically correct while no longer supporting a claim about the current system configuration. Replacing a UPS, changing a filter, updating firmware, changing a supplier or allowing calibration to expire can invalidate some claims without invalidating every capability in the facility.

E5 makes each capability claim explicit about:
- the assets/configuration it depends on;
- the supporting evidence IDs and their scope;
- evidence validity intervals and source limitations;
- upstream capability dependencies;
- the required requalification methods;
- the minimum independence level;
- mandatory safety floors.

A change event triggers deterministic impact propagation along declared dependency edges. The system identifies exactly which claims must be reviewed or requalified and proposes an admissible action set. It does not execute work, certify a real installation or promote a capability to qualified.

**Important distinction:** loss of evidence currency is not proof of physical failure. A claim becomes unqualified for the new context until it is re-established; the old evidence remains part of the historical record.

## 2. Synthetic scenario

Fixture: `fixtures/protect-capability-assurance-e5-v0.1/scenario.json`

The scenario contains four claims:
- emergency power continuity;
- air-handling/filtration capability;
- independent communications;
- integrated shelter continuity.

Two changes are introduced:
1. an UPS configuration revision changes from rev-A to rev-B, while its supporting load-test evidence remains scoped to rev-A;
2. a filter-flow calibration reaches its exclusive validity boundary on 16 November 2026.

The expected impact closure is:
- power: directly affected by the UPS configuration change;
- air: directly affected by expired filter evidence;
- shelter: affected transitively because it depends on power and air;
- communications: unaffected and remains qualified.

This fixture tests both precise invalidation and the avoidance of unnecessary broad requalification.

## 3. Evidence state semantics

Effective evidence state is calculated for a named review date and asset configuration:

| State | Meaning |
|---|---|
| `VALID` | Evidence is within its validity interval and matches the current asset configuration |
| `EXPIRED` | The exclusive `valid_until` boundary has been reached |
| `SUPERSEDED` | The evidence's asset configuration digest does not match the current asset |
| `CONFLICTED` | Evidence is known to be contradictory |
| `REVOKED` | Evidence has been withdrawn by an authority |
| `UNKNOWN` | Applicability or trust cannot be established |
| `NOT_EXECUTED` | A required test or operation has not occurred |
| `NOT_APPLICABLE` | The evidence does not apply to this claim/context |
| `NOT_YET_VALID` | The evidence validity interval has not started |

E5 never converts any of the non-valid states into a pass. A valid historical record remains retrievable but cannot silently qualify a changed configuration.

## 4. Change impact and claim states

The Rust oracle:
1. validates the graph/schema and exact references;
2. rejects duplicate IDs, dangling dependencies and claim cycles;
3. validates that configuration-change events match the current asset digest;
4. evaluates evidence against the scenario `as_of` date and asset configuration;
5. seeds affected claims from changed assets and invalid evidence;
6. computes transitive closure only over explicit claim-dependency edges;
7. keeps unrelated claims qualified;
8. returns a requalification plan without executing any action.

Current states include `QUALIFIED`, `REQUALIFICATION_REQUIRED` and `NOT_QUALIFIED`. The report distinguishes the reason for invalidation, the prior status and the computed current status.

## 5. Exact requalification planner

The frozen catalog contains six actions:
- an independent UPS load test;
- an independent filter-flow test;
- an independent shelter drill;
- an integrated independent refuge requalification action;
- a cheap self-certified suite that fails the independence floor;
- a cheap visual walkthrough that fails the required methods.

The optimizer enumerates admissible action subsets (bounded at 20 actions) and minimizes the tuple:
1. total integer cost;
2. number of actions;
3. lexicographically sorted action IDs for deterministic tie-breaking.

Mandatory methods and independence constraints are hard admissibility conditions, not weighted preferences. The integrated action covers all three affected claims for **$2,800**, beating the three separate admissible actions at **$3,500**. The self-certified and visual actions are rejected even though they are cheaper.

The plan status is `MINIMUM_COST_ADMISSIBLE_PLAN_AVAILABLE_NOT_EXECUTED`. It is not a work order, purchase authorization, safety certificate or instruction to perform a test.

## 6. Industry baseline and standards mapping

E5 is designed to complement, not replace, established frameworks:

- **NIST SP 800-160 Vol. 2 Rev. 1** describes cyber-resiliency engineering for systems that anticipate, withstand, recover from and adapt to adverse conditions. E5 operationalizes a bounded change/evidence/claim portion of that lifecycle. [NIST publication](https://csrc.nist.gov/pubs/sp/800/160/v2/r1/final)
- **ISO 22301:2019**, with **Amendment 1:2024**, defines business continuity management requirements and adds climate-action context. E5 can connect continuity claims to live dependency and evidence state, but is not a BCMS or certification. [ISO 22301](https://www.iso.org/standard/75106.html)
- **IEC 62443-2-1:2024** sets asset-owner security-program requirements for operational IACS, recognises long-lived legacy environments and introduces a maturity model. E5 can track compensating controls and evidence scope by configuration, but does not certify IEC 62443 compliance. [IEC catalogue](https://webstore.iec.ch/en/publication/62883)
- **NIST SP 800-34 Rev. 1** provides contingency-planning guidance. E5's graph can bind recovery claims to assets, evidence and dependencies; it does not replace continuity planning. [NIST publication](https://csrc.nist.gov/pubs/sp/800/34/r1/upd1/final)

These standards are frameworks/normative baselines, not executable benchmarks that can be directly replaced by a small Rust fixture. A fair comparison must use matched operational scenarios and qualified reviewers.

## 7. Benchmarks

| Metric | Prospective target |
|---|---:|
| Impact-closure recall vs frozen gold graph | 100% |
| Under-invalidation of affected claims | 0 |
| Unrelated-claim invalidation | 0 |
| Configuration digest mismatch admitted as qualified | 0 |
| Expired/unknown/conflicted/revoked evidence promoted | 0 |
| Safety floor overridden by cost | 0 |
| Requalification action coverage | 100% |
| Infeasible requalification plan accepted | 0 |
| Deterministic replay and plan reproduction | 100% |
| Validity boundary correctness | 100% on date-boundary corpus |
| Decision time and reviewer effort | p50/p95 vs manual baseline |
| Recovery/continuity service-hours preserved | measured under scenarios with uncertainty |

Benchmark families should include single and compound changes, common-cause dependencies, supplier and firmware changes, expired calibration, configuration drift, rollback/correction events, stale certificates, contradictory evidence, independent test requirements and infeasible requalification plans.

## 8. Verification status and claim ceiling

- Synthetic fixture, strict Draft 2020-12 schema, Rust oracle and 23 tests are committed.
- E5 is registered as `protect-capability-assurance-e5` in the isolated verifier workflow.
- Local environment does not contain `cargo`, `rustc` or `rustfmt`; no local compilation/test pass is claimed.
- Exact-head formatting, Clippy, compilation, tests and executable output require hosted CI.
- The gate emits a plan only. `executed_actions=false` and `qualification_promoted=false`.
- No live facility, external provider, private data, supplier credential or physical action is accessed.
