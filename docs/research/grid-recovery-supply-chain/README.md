# Grid Recovery Supply-Chain Portfolio v0.1

**Status:** research contract / synthetic fixture  
**Issue:** [GRID-RECOVERY-SUPPLY-CHAIN-001 #6237](https://github.com/Luminous-Dynamics/symtropy/issues/6237)  
**Parent:** [GRID-ASSET-RECOVERY-001 #6119](https://github.com/Luminous-Dynamics/symtropy/issues/6119)  
**Programme:** [POWER-FABRIC-BENCH-001 #5241](https://github.com/Luminous-Dynamics/symtropy/issues/5241)  
**Snapshot:** 2026-11-19

## 1. Research question

Can a portfolio restoration planner coordinate multiple simultaneous incidents and capital projects when they compete for the same qualified manufacturing, material, testing, transport, port, vessel and specialist-workforce capacity?

The unit of analysis is no longer one repair task or one asset. It is the **shared delivery portfolio**.

## 2. Industry evidence and limits

ENTSO-E's 2026 assessment (participating European TSO scope) reports:
- typical procurement lead time of at least 36 months for 245–420 kV transformers;
- highly customised phase-shifting transformers with lead times above four years;
- leading European HVDC cable suppliers reported booked through 2030;
- simultaneous constraints in raw materials, factories, cable-laying vessels, port infrastructure and installation crews, with three-to-five-year-or-more lead times;
- nine of twelve workforce-survey submissions identifying skilled-workforce availability as a key challenge.

These observations motivate the resource model, but do **not** provide per-supplier lead-time distributions, guaranteed delivery dates or South African/global procurement distributions. Do not create a probability distribution from the report's aggregate statements.

Primary source: https://eepublicdownloads.blob.core.windows.net/public-cdn-container/clean-documents/Publications/Position%20papers%20and%20reports/2026/entso-e_report_investment_supply_chain_assessment_2026_261030.pdf

## 3. v0.1 contract

This initial contract describes:
- qualified suppliers and the exact asset families/interfaces for which evidence exists;
- finite capacity resources (factory lines, test bays, ports, vessels, heavy-haul corridors, specialist crews);
- bounded capacity windows and calendars;
- lead-time claims with explicit evidence basis;
- project demand and service priority;
- held/confirmed/proposed reservations;
- common-cause disruptions affecting multiple shared resources;
- content-addressed source artifacts and provenance.

A reservation is not created by a scenario. The v0.1 contract is read-only; it does not place a purchase order, reserve a factory slot, pay a supplier or dispatch transport.

## 4. Lead-time epistemics

Every lead-time statement must carry a basis:
- `contractual_quote`: a dated, source-backed quote or contractual promise;
- `observed_distribution`: a distribution derived from identified historical deliveries, with sample count and population boundaries;
- `expert_interval`: an explicitly attributed interval, not a probability distribution;
- `scenario_delay`: a synthetic or declared stress-case delay;
- `right_censored_backlog`: evidence that bookings extend through a date, without a promised delivery date;
- `unknown`: insufficient evidence.

A supplier reported as booked through 2030 is not promising delivery on 1 January 2031. A right-censored backlog is a lower-bound observation on existing commitments, not a lead-time sample.

Probabilities remain absent from v0.1. Add them only after obtaining a defensible, asset-class- and geography-specific historical population, censoring treatment, calibration method and independent review.

## 5. Shared capacity and reservation semantics

Each physical/logistical resource has:
- canonical identity and owner/operator;
- resource type and native unit;
- qualified capacity and capacity windows;
- source/evidence references and validity;
- common-cause/failure-domain references.

Reservation states:
- `proposed`: request only; does not consume or guarantee capacity;
- `held`: capacity temporarily held under an external, source-backed reference;
- `confirmed`: a source-backed reservation;
- `released`: reservation explicitly released;
- `fulfilled`: capacity use completed;
- `conflicted`: inconsistent claims require reconciliation;
- `unknown`: state cannot be established.

Only `held` and `confirmed` consume capacity in the v0.1 synthetic checker. A proposed reservation does not prove availability. An expired/stale hold must be reconciled before reuse.

No alias may create a second identity for the same physical vessel, factory line, port slot or crew. The v0.1 schema stores canonical resource IDs; real identity resolution and alias reconciliation need an application-level verifier.

## 6. Portfolio fixture

The synthetic portfolio has two simultaneous projects competing for one transformer factory line:
- Project A has a confirmed factory slot for 245–420 kV transformer production in the first window.
- Project B proposes the same slot, but it is not treated as committed or available.
- The supplier's separate backlog statement is right-censored through 2030-12-31; it does not claim a promised delivery date.

A negative control changes Project B's proposed booking to confirmed. The fixture validator must detect over-allocation of the single factory line.

Additional synthetic resources model factory test capacity, a port berth and a cable-laying vessel. A compound disruption references multiple resources under one shared cause group to prevent an independence assumption.

All identities, capacities, windows, project priorities, reservations, disruptions and evidence records in this fixture are synthetic.

## 7. Benchmark policies and outcomes

Compare:
1. **Reactive procurement:** procure after a failure with no reserved spare/slot.
2. **Strategic spares:** pre-position a profile-qualified spare under an explicit ownership/maintenance policy.
3. **Regional pooled inventory:** share inventory with explicit reservation, priority and withdrawal rules.
4. **Reserved capacity:** hold factory/test/transport capacity in advance.
5. **Supplier diversification:** use multiple independently qualified suppliers where evidence supports interchangeability.
6. **Portfolio allocation:** allocate constrained slots across incidents under a transparent, declared service-priority policy.

Report a metric vector:
- time to qualified spare, production, factory test, dispatch, site delivery, commissioning and restored service;
- native service deficit by beneficiary and event;
- p50/p95/CVaR95 only when probability inputs are justified;
- number of conflicting/overbooked reservations;
- supplier concentration and common-cause exposure;
- lead-time slack and right-censoring;
- inventory carrying, obsolescence, reservation and expedite costs;
- displaced projects and service floors;
- feasibility, bounds, runtime and allocation stability.

Use a lexicographic objective: technical/legal qualification and hard safety constraints first; minimum essential-service floors second; total service loss and tail restoration third; cost/inventory efficiency afterward.

## 8. Adversarial acceptance

- Proposed bookings never count as confirmed capacity.
- Duplicate reservation identities are rejected.
- Confirmed/held capacity cannot exceed the resource's capacity window.
- A booking beyond a supplier's right-censored backlog date is not inferred to be available.
- A right-censored backlog claim cannot contain a promised delivery date.
- Unknown supplier qualification cannot be treated as compatible.
- Expired or stale capacity evidence requires reconciliation.
- Common-cause events affecting several resources retain one shared cause and correlation-group identity.
- Missing resource capacity is unknown, never infinite.
- Portfolio source-record counts reconcile exactly.
- Small instances agree with an independent enumerator.
- Search-budget exhaustion is INCONCLUSIVE, not infeasible or optimal.

## 9. Relationship to current work

- #6119 owns the event's selected recovery plan and initial procurement path.
- #6213 owns resumable work and commitment-safe replanning.
- #6235 maps recovery work to independently modelled/measured service trajectories.
- #6237 owns portfolio-level supplier and specialist-capacity contention.
- #6214 owns the four-scope federation for household/community/municipal/metropolitan coordination.

Do not duplicate the recovery scheduler, CivDyn service cascade (#3894/#3896), resource stock-flow ledger, rights/route gate graph (#6148), or specialist electrical/water/telecom solvers.

## 10. Qualification boundary

This is a schema and synthetic contract fixture. No production procurement integration, supplier booking, real supplier capacity, South African lead-time distribution, physical logistics, field performance, commercial forecast or SOTA result is established. No contract, purchase order, payment, reservation or dispatch action is authorized.


## Qualification-aware supplier selection

Supplier slot capacity and supplier qualification are independent checks. A supplier may expose a free factory window but remain unqualified for the required asset family or interface profile. In v0.1, only a `qualified` record with non-expired validity can support a held/confirmed reservation for that project's asset family. A `conditional`, `expired`, `not_qualified` or `unknown` profile cannot be treated as a compatible confirmed source.

The expanded synthetic fixture includes a second supplier with unknown qualification and a proposed factory option. It is visible for future investigation but is not eligible for a held/confirmed assignment. A negative control promotes it to confirmed and must fail qualification semantics, even though that supplier has an otherwise free capacity window.
