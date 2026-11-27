# Time-bounded maritime productive capability and offer lifecycle v0.1

Companion to MCSC #7686 and draft PR #7688. This is a composition contract. It does **not** implement a second provider registry, resource offer schema, capability assurance graph, booking system, job authority service, manufacturing process model or VVUQ engine.

## 1. The paradigm shift

A region does not possess a durable productive capability because a provider is listed, a machine exists, a certificate was once issued or a capacity number appears in a spreadsheet.

A regional capability is a time-bounded, configuration-bound option whose evidence and prerequisites remain valid at the decision cutoff and across the requested execution window.

The core state sequence is:

`OFFER_PUBLISHED
 -> REFERENCES_RESOLVED
 -> CAPABILITY_QUALIFIED
 -> CALENDAR_AVAILABLE
 -> CAPACITY_RESERVED
 -> JOB_AUTHORIZED
 -> JOB_EXECUTED
 -> INDEPENDENTLY_INSPECTED
 -> RECEIVER_ACCEPTED`

These states are not interchangeable and are not implied by one another.

- Published offer: a provider declares what it can offer.
- References resolved: resource, site, process, configuration, material, utilities, tooling and skill references resolve to canonical owner records.
- Capability qualified: exact evidence and VVUQ support the requested process/quality envelope.
- Calendar available: a dated slot exists inside the provider's valid time window.
- Capacity reserved: the slot/resource is actually committed under the owner protocol.
- Job authorized: the participant with authority approved the specific configuration and work package.
- Job executed: physical process records exist.
- Independently inspected: the output passed its applicable inspection at the matching revision.
- Receiver accepted: the intended receiver accepted the exact quantity/configuration.

No optimistic promotion is permitted. A failed transition remains at the last supported state; missing evidence produces UNKNOWN/NEEDS_EVIDENCE, not a fabricated pass.

## 2. Canonical owners

Consume and reference, do not duplicate:

- Mycelix [MFG-PROC-001B #6533](https://github.com/Luminous-Dynamics/mycelix/issues/6533): immutable, versioned `ManufacturingResourceOffer`, provider/site/resource, process/capability/configuration references, capacity/calendar declarations, issuer authority, validity and revocation. It explicitly distinguishes offer published, references resolved, capability qualified, calendar available, capacity reserved, job authorized and job executed.
- Symtropy [PROTECT-CAPABILITY-ASSURANCE-001 #5710](https://github.com/Luminous-Dynamics/symtropy/issues/5710): exact configuration-bound claims, evidence validity states, transitive change impact and minimum requalification actions. An affected claim becomes REVIEW_REQUIRED/REQUALIFICATION_REQUIRED, not silently qualified.
- Symtropy [MFG-TWIN-VVUQ-001 #3121](https://github.com/Luminous-Dynamics/symtropy/issues/3121): independent component/interface/composition/end-to-end VVUQ; do not infer composed credibility from individually qualified component twins.
- Symtropy #4433: shipyard welding process trace and configuration-bound inspection.
- Symtropy #3331: component digital thread, metrology/NDT, revision and as-built qualification.
- Mycelix #6773/#4814/#5762: productive-capability portfolio, physical resource/dependency facts and common-cause closure.
- Symtropy #3683/#7554: end-to-end service and canonical receiver-acceptance/protected-floor adjudication.
- Mycelix #6343: port-city PCC, energy, water, compute, heat and authority composition.

MCSC owns only the join between the capability lifecycle state and a declared receiver-service option. It does not own those upstream state machines.

## 3. Required decision-time join

For every portfolio option (p), resource offer (r), service obligation (s), and decision time (t_0), the adapter must resolve:

1. **Identity and configuration:** provider ID, resource/equipment ID, facility/site/zone ID, process/capability-profile ID, exact configuration digest, work-package/product revision and applicable material/part revision.
2. **Engineering capability:** process envelope, tolerances, material/grade applicability, tool/fixture compatibility, equipment state, qualified operator skills, environmental and utility prerequisites.
3. **Qualification evidence:** evidence IDs, method, measurement/uncertainty, independent assessor, result, calibration state, configuration scope, valid-from/valid-until and revocation/supersession.
4. **Time and capacity:** calendar source, capacity units and resource pool, available window, concurrency, setup/changeover, maintenance blackout, reservation and expiry.
5. **Authority and rights:** provider/issuer identity and signer authority, data-use rights, participant consent/contract, job-specific authorization, revocation state.
6. **Execution and outcome:** job/work-order identity, as-executed trace, process telemetry reference, inspection/NDT result, dispatch/custody/handoff, receiver acceptance and reconciliation.

All IDs/versions/digests must be owner-native references. No copied material properties, process temperatures, forces, electrical ratings or measurement values when canonical owner data exists.

## 4. Temporal semantics

Store distinct times:
- event/measurement time;
- first-available time;
- ingestion time;
- decision cutoff;
- offer valid-from and valid-until;
- evidence valid-from and valid-until;
- calendar slot start/end;
- reservation/commit time and expiry;
- job authorization time;
- execution start/end;
- inspection time;
- dispatch, handoff and receiver-acceptance time.

A retrospective correction or new certificate must not silently change what an earlier decision knew. Historical replay uses first-available evidence at the historical decision cutoff.

Qualification must cover the intended execution window, not merely be true at the instant the optimizer runs. If a calibration, operator competency, permit, tooling state or resource offer expires before completion, the option is not fully qualified for that window unless an explicitly authorized requalification action can complete before use.

An offer may be valid while the resource is busy. A capability may be qualified while the calendar is unavailable. A reservation may exist while authority to execute is revoked. Keep those conditions distinct.

## 5. Change-triggered invalidation and requalification

Consume #5710's owner-native dependency closure and change-impact report. Trigger review/requalification for relevant events such as:
- machine/tooling/fixture or safety-control configuration change;
- firmware/software or process recipe change;
- supplier, source mill, material grade/heat/lot or production site change;
- operator skill expiry, reassignment or competency change;
- inspection procedure, NDT equipment or calibration change;
- utility/environmental envelope or facility access change;
- damage, incident, major repair or extended shutdown;
- revoked, expired, superseded, disputed or contradicted evidence.

Do not invalidate every unrelated qualification: use typed dependency closure. Do not preserve an affected capability merely because one certificate remains individually in date. Conversely, a change may invalidate evidence without proving the physical resource is defective.

A requalification action must specify required test/inspection/calibration, acceptance criteria, independence, cost, duration, prerequisites and exact claims restored. Select a minimum defensible action set, not a cheapest action that leaves mandatory claims uncovered.

## 6. SOVQC extension: investment and renewal as a temporal option

The existing vector-valued `compare_service_options` compares two outcomes under the same frozen obligation. The next research extension should compare capability portfolios across a horizon, with each capability becoming usable only after its evidence-backed commissioning and qualification times.

For candidate investment/qualification package (c), baseline portfolio (P_0), service demand (s_t), and exogenous scenario (omega):

`DeltaY_c(t, omega) = Y(P0 + c, s_t, omega) - Y(P0, s_t, omega)`

Keep separate dimensions for:
- accepted/on-time service and unresolved units;
- capability qualification start, expiry and renewal latency;
- time to restore minimum viable service and full service;
- whole-life cost including capex, training, qualification, inspection, standby, rework, obsolescence and renewal;
- emissions, energy, water, material yield and waste;
- regional value/skills actually retained after qualification;
- supplier concentration and common-cause exposure.

A staged capability investment is not current capacity before qualification. The system must be allowed to return: imports win, no-build wins, centralization wins, the capability arrives too late, or its renewal costs outweigh its service option value.

Do not sum these dimensions into a scalar unless a specific decision authority supplies a valid, transparent preference model. Publish the vector and Pareto set.

## 7. Benchmark protocol

### Exact micro-corpus (minimum 15 new lifecycle cases)

1. Offer is published but canonical resource ID does not resolve: NOT_QUALIFIED.
2. Resource ID resolves but configuration digest mismatches qualification evidence: REQUALIFICATION_REQUIRED.
3. Process/capability profile is qualified but material grade is outside its declared envelope: NOT_QUALIFIED.
4. Machine is qualified but operator competency is expired: NOT_QUALIFIED.
5. Calibration expires before the job's execution end: reject or require a completed requalification action.
6. Offer is valid but calendar unavailable: not schedulable.
7. Calendar available but capacity is not reserved: not a committed plan.
8. Reservation exists but job authorization is revoked: no execution authority.
9. Provider withdraws/changes an offer after planning but before reservation: reconcile/replan; do not execute the stale offer.
10. A tool/firmware change affects one typed claim: invalidate the exact dependency closure, not unrelated capabilities.
11. Supplier changes a material source lot: recheck material/process qualification and custody.
12. NDT equipment changes after calibration: requalify applicable measurement claims.
13. Training starts but operator competency is not yet demonstrated: no capacity credit.
14. Workforce/skill is qualified for only a partial shift: time-window capacity is bounded to that interval.
15. New qualification is completed only after the receiver deadline: no current-service credit.
16. Evidence is revoked retrospectively but first became available after the original decision: preserve historical decision-time replay and mark current state revoked.
17. Resource suffers damage/incident: review claims without equating unknown evidence with physical failure.
18. A minimal requalification set restores three affected claims; unrelated claims stay valid.
19. Two providers share the same mill, grid, corridor or inspection dependency: common-cause cut remains visible.
20. Duplicate offer ID with changed digest: conflict, not idempotent overwrite.

### Targets (proposals, not results)

- Zero stale/revoked/wrong-revision/unresolved capability offers admitted to an executable plan.
- 100% exact agreement with #5710's independent change-impact reference on tiny graphs.
- 100% exact agreement with #6533's owner state machine on frozen offer lifecycle cases.
- Zero promotion of declared capacity into bookable/reserved capacity without the owner receipt.
- Zero promotion of skill enrollment into qualified workforce capacity.
- Requalification closure precision/recall measured separately; minimize false invalidation and false retention.
- Report time-to-detect, time-to-requalify, evidence acquisition cost, service lost during requalification and avoided loss from maintaining alternate paths.
- Held-out facility, process, supplier and common-cause topology families before performance claims.

## 8. Claim ceiling

A source-qualified offer is a provider claim, not a physical fact. A qualified capability is a scoped evidence statement, not a reservation. A reservation is not execution authority. Execution is not accepted output. Receiver acceptance is not proof of causal regional benefit.

Only owner-native evidence, rights, authority and exact time/configuration binding can promote those states. A passing synthetic integration validates only the declared state-transition contract and its test fixtures.
