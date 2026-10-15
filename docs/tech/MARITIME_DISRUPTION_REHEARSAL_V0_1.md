# Maritime Disruption Rehearsal Contract v0.1

**Status:** executable contract evaluator + synthetic design fixture; not a physical simulation, validated model, operational procedure, or safety case.  
**Layer:** orchestration contract over existing Symtropy deterministic-scenario facilities; not a second simulator.  
**Parent:** [MARITIME-SIM-001 #1612](https://github.com/Luminous-Dynamics/symtropy/issues/1612), [MARITIME-TIME-001 #1650](https://github.com/Luminous-Dynamics/symtropy/issues/1650), [forecast applicability PR #1651](https://github.com/Luminous-Dynamics/symtropy/pull/1651).

## 1. Paradigm shift

A forecast validator answers a narrow question: does this versioned forecast's declared time envelope cover the requested assessment and target times under a named policy? It does not answer what to do.

The next useful system object is a **decision rehearsal**: a frozen set of plausible disturbances, explicit candidate plans, hard constraints, evidence envelopes, and per-case outcomes. It asks which plans remain feasible across the cases we chose to examine, what becomes unknown when evidence expires, and what new observation would distinguish the remaining options.

This changes the unit of work from a validated feed to an auditable comparison:

- **Forecast-centric → decision-centric.** Inputs are evaluated in relation to a specific scenario target and plan, not declared globally “good.”
- **Single prediction → uncertainty envelope.** Compare named scenarios and correlated/common-cause disturbances. Do not pretend one forecast is the future or assign ungrounded probabilities.
- **Optimise a score → satisfy hard constraints, then compare trade-offs.** Safety/feasibility constraints cannot be compensated for by time or cost improvements. Do not manufacture a single “best” or “safest” score.
- **Snapshot dashboard → replayable counterfactual.** Every plan comparison binds the same scenario revision, model/capability profile, evidence revisions, event order and policy. A changed input creates a distinct run.
- **Digital twin as authority → digital twin as bounded evidence.** A simulation can show consequences inside declared model scope; it cannot create physical truth, authorization, bookings, custody, port calls or delivery events.
- **Unknown as a first-class result.** Stale, conflicting, absent, unsupported or insufficiently sourced inputs produce `INDETERMINATE` or `UNSUPPORTED`, not zero, success, a clear map, or a silently substituted default.

This fits the IMO Compendium's direction toward interoperable maritime exchange with shared meanings, not a replacement proprietary vocabulary. The IMO Facilitation Committee approved a Strategy on Maritime Digitalization at FAL 50 (23–27 March 2026); the Committee's 31 March 2026 notice says it will be sent to other IMO committees for further review before submission to the Assembly in 2027. This is an approved committee strategy, not yet an adopted Assembly instrument. The contract also maps to UNCTAD's preparation / response / recovery resilience framing. Its stepwise method stresses hazard identification, vulnerability/impact assessment, response measures, prioritization and learning from implementation.

## 2. Boundaries and owners

| Project | Owns | Explicitly does not own |
|---|---|---|
| Symtropy | Frozen scenario, supported model/capability declarations, deterministic run, per-constraint outcomes and run receipt | Forecast truth, source publisher authentication, operational permission |
| Symthaea | Compare candidate plans using declared constraints, uncertainties and trade-offs; explain dominance and unresolved dependencies | Changing model admission, inventing missing physics, dispatching a vessel |
| Mycelix | Simulated intents and proposed allocations as distinct record types; later actual reservation/custody/delivery events only from their authoritative issuers | Converting a forecast or plan into a real reservation, custody transfer or delivery |
| Sol Atlas | Read-only map/time/lineage projection, including unknowns, conflicts, validity and evidence status | Promoting visual status to navigation, berth, engineering or execution authority |

Reuse existing run-receipt, provenance, identity, event, and rendering structures. This is a cross-project contract and thin vertical slice, not a new runtime, trust score, ontology, or digital-twin subsystem.

## 3. Normative decision semantics

Every outcome is a per-constraint result, not a single safety score.

- `FEASIBLE_WITHIN_DECLARED_MODEL`: supplied inputs and declared supported models satisfy the named constraint for this one run/case only.
- `INFEASIBLE`: a declared hard constraint is violated under the supplied case.
- `INDETERMINATE`: missing, stale, conflicted or uncertain evidence prevents a justified conclusion.
- `UNSUPPORTED`: the requested physical or semantic calculation is outside the admitted model/capability set.
- `NOT_EVALUATED`: a downstream check was not run; never infer success.

These statuses do **not** denote real-world safety or operational authorization. For all prototype results, the admission state remains `NotAdmitted`; navigation, berth availability, physical-truth and physical-execution authority remain false. A temporally applicable forecast is only a possible analytical input. Model credibility, uncertainty adequacy, source terms, domain qualification and independent review are separate gates.

A hard constraint is never averaged into an objective. The system may report independent synthetic trade-offs (e.g., delay, resource use, recovery time) only with units, assumptions and provenance. It must not collapse them into a weighted total unless a separately versioned, justified decision policy explicitly asks for that comparison, and even then hard constraints remain non-compensable.

## 4. Frozen reference rehearsal

See [MARITIME_DISRUPTION_REHEARSAL_V0_1.json](./MARITIME_DISRUPTION_REHEARSAL_V0_1.json). It defines an invented abstract terminal/corridor; it contains no real facility geometry, real port schedule, actual vessel, real reservation, or source forecast values.

Candidate plans are deliberately conditional:
1. continue the original simulated slot;
2. hold in a designated *abstract* waiting resource and reassess;
3. prepare an alternate-terminal option without booking or reserving it.

The synthetic case set perturbs the baseline independently and in combination: an adverse declared water-level envelope, loss of shore-power availability, forecast invalidation, and a common-cause disruption. The case set is a test of reasoning contracts, not a claim about real probabilities or physical dynamics. No hydrodynamics, under-keel-clearance, vessel energy, or terminal-capacity physics is invented by this fixture; relevant conditions are supplied as abstract, labelled inputs.

### Required comparison behaviour

1. Evaluate every hard constraint for every plan/case independently.
2. Preserve the constraint ID, value/unit or state, input evidence references, model support status and explanation.
3. If a constraint depends on expired or invalidated forecast evidence, degrade the dependent result to `INDETERMINATE`; do not carry forward its previous green state.
4. If a selected case violates a hard constraint, mark that plan/case `INFEASIBLE` within this rehearsal.
5. Report an alternate-terminal option as merely prepared while capacity/authority is unconfirmed. Do not create a Mycelix reservation or imply acceptance.
6. Report the robust candidate set across the declared case family, but do not call the set “safe.” A plan is *robustly feasible within this rehearsal* only if all required hard constraints are supported and satisfied in every required case; otherwise explain exactly which cases/constraints block it.
7. If all candidate plans are infeasible or indeterminate, report that plainly. Never force a winner.

## 5. Minimum run receipt

The eventual executable vertical slice should bind a canonical run receipt to:

- scenario ID, schema/version and exact scenario bytes digest;
- exact Symtropy source commit/tree, engine and capability profile;
- input artifact receipt IDs/digests, dataset/product/revision identities, terms-review state and temporal coverage;
- explicit decision time, target times, time zone/canonical timestamp rules, selected policy IDs and expiry/freshness outcomes;
- initial state, parameter/configuration digest, deterministic seed, timestep/numerical policy and stable entity IDs where applicable;
- ordered event/fault schedule and the selected scenario-set revision;
- candidate-plan IDs and the exact per-constraint results and separate metrics;
- warnings, conflicts, unsupported effects, missing inputs and completion status;
- canonical output digest, independent-verifier version/result, replay comparison and postflight immutability evidence;
- claim ceiling and the explicit statement `NotAdmitted`.

A digest binds bytes; it does not by itself authenticate a publisher, establish a licence, validate a scientific value, or prove that a model is correct.

## 6. Cross-project output contract

Emit a read-only comparison bundle with these conceptual collections:

- `evidence[]`: source identity, exact revision/digest, capture/issue/init/valid/assessment times as distinct fields, terms state, uncertainty and use limits;
- `cases[]`: named disturbances, provenance, dependencies/correlation declarations and whether a case is required or exploratory;
- `plans[]`: plan intent and preconditions; no operational side effects;
- `constraint_results[]`: plan × case × constraint × status, value/unit when available, evidence refs and reason code;
- `tradeoffs[]`: separate labelled quantities and assumptions, never a magic composite;
- `decision_summary`: robustly feasible-within-model set, blockers, unknowns and evidence that could reduce uncertainty;
- `run_receipt`: exact inputs/output identity and replay claims;
- `authority`: all physical/operational authority booleans false; simulated intent explicitly distinguished from actual events.

Symthaea consumes this bundle as analysis-only input. Mycelix may preserve a proposed action as a `simulated_intent`/`proposed_allocation` type, with no coercion into actual reservation/custody/delivery event types. Sol Atlas projects the same bundle and lets each result be traced back through plan → constraint result → evidence → source receipt. A view refresh must not mutate or promote the underlying record.

## 7. Adversarial and metamorphic tests to implement

Freeze these expectations before implementing the Rust reducer and an independent oracle:

1. Identical pinned inputs and declared deterministic profile replay to identical canonical output.
2. Changing one byte of a bound source, policy, scenario or configuration invalidates the prior run binding.
3. Retrieval of an unchanged stale run does not refresh its issue time.
4. Issue time after decision time yields indeterminate (or a hard invalidity where schema rules require it); target after valid coverage is not applicable.
5. An endpoint exactly at each documented inclusive/exclusive temporal boundary follows the versioned policy; no accidental `>` / `>=` ambiguity.
6. Removing one required input cannot improve any dependent result (unknown-preserving monotonicity).
7. Marking a source stale, withdrawn or conflicting cannot improve a dependent result.
8. Duplicating correlated evidence does not count as independent corroboration.
9. Adding an adverse required case cannot improve the robustly-feasible candidate set.
10. A single hard-constraint violation cannot be compensated by lower cost or shorter delay.
11. An unsupported physics request returns `UNSUPPORTED`, rather than a fabricated numeric result.
12. A change to scenario, policy or model-capability version makes any previous comparison stale.
13. `simulated_intent` cannot be serialized as a confirmed booking, port call, custody or delivery event.
14. Atlas rendering, labels, colours, filtering or selection cannot mutate the run receipt or admission state.
15. Output claiming operational/navigation authority fails the independent verifier even if all other fields are well-formed.

The independent oracle must not call the production reducer, timestamp conversion or outcome classifier. The corpus must include positive, negative, boundary, missing, stale, contradictory, unsupported and attempted-authority-escalation controls.

## 8. Research-aligned integration points

- IMO, [Maritime Single Window digitalization](https://www.imo.org/en/ourwork/facilitation/pages/maritimesinglewindow-default.aspx) and [FAL 50 digitalization strategy update, 31 March 2026](https://www.imo.org/en/mediacentre/pressbriefings/pages/facilitation-committee-approves-digitalization-strategy-cyber-security-measures.aspx): reuse shared semantics, interoperability and data-governance principles; do not represent the committee-approved strategy as already adopted by the Assembly.
- IMO, [Compendium on Facilitation and Electronic Business](https://www.imo.org/en/ourwork/facilitation/pages/imocompendium.aspx): reference model for shared maritime meanings and port-call/business data exchange.
- UNCTAD, [Building Capacity to Manage Risks and Enhance Resilience: A Guidebook for Ports](https://unctad.org/publication/building-capacity-manage-risks-and-enhance-resilience-guidebook-ports): organize rehearsals around preparedness, response and recovery.
- UNCTAD, [Review of Maritime Transport 2025](https://unctad.org/publication/review-maritime-transport-2025): current structural context for disruption, connectivity and maritime resilience; it is not a substitute for source capture or route-level operational input.
- IHO, [S-100 Standards in Force](https://iho.int/en/s-100-standards-in-force-0): re-check the applicable edition at implementation; the page last updated 3 July 2026 lists S-104, S-111 and S-129 at editions 2.0.0 in force from 1 January 2026.
- Copernicus Marine, [product IDs versus dataset IDs](https://help.marine.copernicus.eu/en/articles/4703373-differences-between-copernicus-marine-products-and-datasets-and-their-identifiers): keep parent products and downloadable dataset identifiers distinct.

## 9. Delivery sequence and honest completion criteria

1. Freeze the contract, fixture and authority constraints (included in this PR).
2. Add a **narrow executable Rust contract evaluator** to the existing `maritime-data-receipt` tool, invoked with `--evaluate-disruption-rehearsal <scenario-json-path>`. It derives outcomes from the declared abstract input states, cross-checks the frozen expected dispositions, rejects unknown fields/states and authority escalation, and emits a digest-bound report. It does not execute physical simulation.
3. Add a **separate test-only oracle** using a different JSON parser and an independent disposition classifier. The frozen five-case / three-plan fixture now has fifteen result combinations; tests also cover deterministic report bytes, unknown states, unknown fields and attempted authority escalation.
4. Run the exact-head workflow with rustfmt, the full crate tests, the rehearsal CLI, all existing maritime CLI verifiers and Clippy. A queued run is not a pass; inspect the exact candidate head and all jobs.
5. Only after the contract evaluator passes should the next increment connect to the existing deterministic scenario engine. Inspect the engine capability census first; do not create a second simulator or invent hydrodynamics, under-keel clearance, energy or terminal physics.
6. Bind an executable scenario run receipt and replay comparison to exact source, configuration, policy, evidence and output digests. Then connect Symthaea analysis, Mycelix simulated-intent records and Sol Atlas's read-only projection through explicit versioned fields.
7. Capture one real official source artifact only after exact bytes, retrieval evidence, product/dataset IDs, selected scope, terms snapshot and attribution are accessible and independently verified. Until then all scenario outcomes remain synthetic.

**Current limitations:** the new contract evaluator and tests have been committed to this PR branch, but they have not yet been reported as passing an exact-head Rust workflow. The forecast-applicability PR #1651 and its queued workflow remain separate gates. The new evaluator validates only the frozen abstraction contract; it is not a physics simulation or end-to-end project integration. Real-source capture remains open.
