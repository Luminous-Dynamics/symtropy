# AeroWeft — Assurance Argument Red-Team and Counterexample Minimization

**Tracking issue:** #5721  
**Consumes:** Track W #5647 and Tracks P–V  
**Status:** synthetic-first seeded challenge harness  
**Claim ceiling:** coverage of declared assurance invariants only; not proof of hazard completeness, flight safety or authorization.

## Research question

How often can a domain-aware adversary falsify a supposedly review-ready assurance case, which claim/evidence dependencies are challenged, and what is the smallest replayable witness that demonstrates the violation?

Track W validates a structured assurance argument. Track X attacks its declared invariants, using the actual Track W data types and evaluator, and compares W with a separate finite invariant oracle. The initial challenge profile covers scope binding, typed evidence purposes, independent evidence groups, hazard-to-argument links, node validity and external authority decisions.

## Paradigm shift

Move from checking whether the case is structurally complete to actively trying to falsify its strongest claims.

A red-team result distinguishes:
- a seeded invariant violation that the independent oracle detects and W blocks;
- a violation the independent oracle detects but W allows through;
- a W block not predicted by the finite oracle (an overblocking/oracle-disagreement signal);
- no counterexample found under the declared attack set and budget;
- a mutation whose preconditions are invalid.

No counterexample found within budget is not proof that the assurance case is true. The finite oracle covers declared policy invariants only. It cannot prove the hazard universe complete or discover every physical hazard.

## Initial attack families

- Remove a required hazard-to-argument relation while preserving graph connectivity.
- Substitute Track R conflict-performance evidence for a Track T model-credibility predicate.
- Splice evidence onto a different aircraft identity.
- Reject/revoke an external authority decision.
- Expire a relied-on node before operation end.
- Remove evidence correlation identity so it cannot count as independent.
- Remove non-normative context links as a negative control; W should remain review-ready.

The generator also creates historical-evidence, node-validity, source-purpose, correlation and authority variants from the actual case dependency frontier.

## Independent oracle boundary

Track X's finite oracle independently checks:
- exact case/aircraft/operation scope;
- operation-window validity;
- typed source-to-purpose compatibility;
- policy-required claim and mitigation evidence purposes;
- qualified evidence status and independent correlation-group floors;
- required hazard presence, claim/strategy linkage and mitigation association;
- authority role/identity, accepted status, effective time, recorded/knowledge cutoff and operation validity;
- graph endpoint, duplicate-edge, cycle and root-connectivity invariants.

The oracle does not call Track W. It is a separately implemented finite policy oracle, not an independent physical or aviation-safety oracle. A confirmed counterexample means a declared invariant was violated and W failed to block it; it does not imply an aircraft is unsafe. A missed counterexample must be routed to the owner for repair. Conditional expert challenges and unknown hazards require separate human adjudication.

## Result model and replay

Every attack result binds:
- generator version and seed;
- case ID and policy digest;
- attack ID and mutated node/edge identities;
- independent oracle violations;
- Track W status or structural error;
- minimized witness;
- exact removed node/edge trace.

The minimizer greedily removes unrelated nodes and edges only when the exact same oracle-violation set and Track W outcome remain unchanged. It rejects reductions that introduce a new graph-structure failure or erase the original witness. The minimized case is directly replayable.

## Initial benchmark gates

Prospective targets:
- 100% deterministic replay for fixed generator version, seed, case and attack budget;
- zero invalid mutations classified as counterexamples;
- zero finite-oracle-confirmed seeded violations missed by W on the frozen suite;
- zero NO_COUNTEREXAMPLE_FOUND_WITHIN_BUDGET presented as safety or permission;
- every minimized witness preserves the exact oracle violation set and W outcome;
- report attack coverage by family, not one aggregate score;
- compare domain-aware mutation with random mutation and static Track W checks at equal budgets;
- report precision/recall separately for oracle-confirmed invariants and human-adjudicated conditional defeaters;
- scale 10 / 100 / 1,000 / 10,000 nodes and 10 / 100 / 1,000 / 10,000 candidates with p50/p95/p99 runtime, memory, witness rate, minimization ratio and adjudication burden after correctness gates pass.

## Industry/SOTA comparison

Track X composes rather than duplicates:
- Symtropy #4446 differential fuzzing and counterexample shrinking;
- Symtropy #5981 common-cause analysis;
- Symtropy #3749 analytic/manufactured numerical oracles;
- Track W's structured assurance argument;
- Track P causal receipt binding and Track Q evidence custody;
- Tracks R–U model/performance/applicability evidence;
- Track V experiment planning, which remains planning-only.

The research advantage sought is measurable assurance-argument attack coverage with exact replay and minimized counterexamples, not a claim to replace formal verification, independent safety assessment, fault-tree/FMEA analysis, expert review or competent-authority decisions.

## Claim ceiling

A passing Track X benchmark establishes only that this declared generator and finite oracle behaved reproducibly on the frozen attack profile and budget. It does not establish physical truth, completeness of hazards, aircraft safety, separation, certification, clearance or permission to operate.