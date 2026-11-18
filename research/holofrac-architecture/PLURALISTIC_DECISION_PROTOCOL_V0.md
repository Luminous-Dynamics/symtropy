# HoloFrac Consent-First Pluralistic Decision Protocol v0

**Status:** synthetic multi-stakeholder decision benchmark  
**Implementation:** research/holofrac-retrieval/src/pluralistic.rs  
**Executable:** research/holofrac-retrieval/src/bin/pluralistic_decision_sweep.rs  
**Workflow output:** pluralistic-decisions.csv  
**Related:** [#6011 — occupant agency](https://github.com/Luminous-Dynamics/symtropy/issues/6011), [#6007 — external qualification](https://github.com/Luminous-Dynamics/symtropy/issues/6007), [#5961 — budgeted experiment selection](https://github.com/Luminous-Dynamics/symtropy/issues/5961)

## 1. Purpose

Test a decision procedure in which multiple stakeholders can express distinct criteria, consent scope and hard minimums. It preserves Pareto alternatives and reports maximin/leximin comparisons without treating one aggregation rule as universally fair.

This is a deterministic synthetic fixture. It is not a real occupant-preference model, a social-choice legitimacy result, a legal consent mechanism or a privacy certification.

## 2. Decision model

Eight normalized outcome criteria are represented, each oriented so higher is better:
- thermal comfort;
- air quality;
- daylight;
- acoustic comfort;
- accessibility;
- privacy;
- energy efficiency;
- adaptability.

Each alternative carries an exact ID, its outcome vector in [0,1], and an independently evaluated hard-constraint result. An infeasible alternative is removed before Pareto ranking or social aggregation.

Each participant supplies explicit criterion weights summing to one and optional hard minimums (veto thresholds). These weights represent that participant's declared preferences only. The model does not infer preferences from demographics, cameras, biometrics, behavior or psychological attributes.

## 3. Consent and identity handling

The fixture distinguishes:
- **LocalOnly:** excluded from group aggregation. A separate participant-local decision execution path is not implemented in this fixture.
- **AnonymousAggregate:** may contribute only to a sufficiently large aggregate; stable participant ID is removed at the decision-snapshot boundary.
- **NamedAggregate:** participant ID and attributable preference outcomes may be returned because this scope explicitly permits named sharing.
- **NotGranted / Withdrawn:** cannot contribute to a group result.

Non-response is not consent. A missing profile is not assigned neutral weights. A withdrawn or out-of-scope profile cannot remain in a later aggregate snapshot.

The ledger retains source IDs and raw inputs for revision tracking inside the owning module. The group-decision API keeps raw snapshot profiles, weights, veto thresholds and exact cohort counts private. Individual satisfaction rows are emitted only for NamedAggregate participants. Anonymous vetoes collapse to criteria-only blockers: no individual identity, threshold, observed value or count is emitted.

This is output minimization at the module/API boundary. It is not an encrypted ledger, secure enclave, access-control system or protection against another part of the owning process reading the source ledger.

## 4. Minimum anonymous cohort and privacy limitations

The research fixture requires at least **five** anonymous participants before any anonymous profile can contribute to a shared group decision. Cohorts of one, two, three or four fail closed. Named participants do not count toward the anonymous cohort threshold.

A five-person minimum group size is a disclosure-control heuristic, not a mathematical guarantee of anonymity. It does not bound leakage from:
- repeated or adaptively chosen queries;
- differences between overlapping cohorts or successive snapshots;
- auxiliary data or known participant preferences;
- unique or rare preference signatures;
- aggregate means/minima and which criterion caused an anonymous veto;
- membership, inference or intersection attacks.

In particular, because the current evaluator accepts caller-supplied candidate outcome vectors and reports aggregate satisfaction summaries, an attacker with repeated access may be able to infer aspects of the cohort's preference distribution by submitting carefully chosen alternatives. The current fixture has no query authorization, rate limit, privacy budget or differential-privacy mechanism. This is a known open risk and requires the dedicated red-team work in the follow-on issue.

The benchmark must therefore be described as **identity-redacted, minimum-cohort aggregation**, not anonymous in the cryptographic or differential-privacy sense. A real deployment requires an explicit threat model and may need query governance, output coarsening, suppression, formally specified differential privacy or removal of shared preference output entirely.

## 5. Bitemporal preference and consent semantics

Every event has:
- effective time: when the preference/consent applies;
- recorded time: when the decision system became aware of that revision;
- a monotonically increasing per-stakeholder revision and exact prior-event ID.

A decision snapshot at cutoff t sees only events whose effective and recorded times are both at or before t. A late-recorded revocation cannot rewrite what an earlier decision knew. A new decision after the revocation becomes recorded must exclude the withdrawn profile.

Historical replay must preserve both preference state and knowledge cutoff. The fixture's time model is deterministic; real adapters still need trusted time/source provenance and the applicable organizational consent policy.

## 6. Decision methods

The same feasible alternatives and participant inputs are compared through:
- Pareto dominance on the original multi-criterion outcome vector;
- equal-participant mean satisfaction (comparison baseline only);
- maximin satisfaction;
- leximin on the sorted participant-satisfaction vector.

The leximin vector is retained internally for ranking, not exposed as a public per-person score list. Named participants may receive individual satisfaction rows within their explicit consent scope. Anonymous participants contribute to internal maximin/leximin and aggregate mean/minimum statistics without individual rows. Aggregate mean/minimum summaries remain potential side channels.

No ranking is a universal default. Decision authority and affected stakeholders must determine which method, veto rules, hard constraints and trade-offs are appropriate. A Pareto frontier is not consent or authorization.

## 7. Executable acceptance corpus

The module tests:
1. Pareto-frontier preservation for non-dominated alternatives.
2. A named participant's explicit privacy veto rejects an otherwise attractive alternative and retains attributable explanation.
3. Mean ranking can favor a majority while maximin/leximin favor a more balanced alternative.
4. LocalOnly and Withdrawn inputs are excluded.
5. Anonymous IDs and raw profile fields do not cross the decision-snapshot API boundary.
6. Anonymous per-person satisfaction rows are never emitted.
7. Cohorts of one through four fail closed; a five-person cohort is admitted.
8. Anonymous vetoes remain blocking but expose only affected criteria, not threshold, observed value, identity or count.
9. A forged/inconsistent anonymous profile count is rejected.
10. Late-recorded revocation does not leak backward into an earlier decision cutoff.
11. Future-effective preferences are not applied before their effective time.
12. Hard-infeasible alternatives never enter the Pareto set.
13. Invalid weights, veto thresholds and missing consent fail closed.

## 8. Metrics and prospective targets

- Pareto/frontier and decision-ranking agreement with an independent finite oracle: 100%.
- Hard-constraint or active veto violations admitted: zero.
- Anonymous stable IDs/raw weights/raw veto thresholds crossing the group-decision API boundary: zero.
- Per-person satisfaction rows emitted for AnonymousAggregate: zero.
- Anonymous cohorts below five included in group aggregation: zero.
- Anonymous veto count/numeric threshold/observed value exposed: zero.
- Named output rows outside explicit NamedAggregate consent: zero.
- Historical consent/preference cutoff disagreement with independent replay: zero.

The current executable does not yet test adversarial query differencing, membership inference, re-identification or formal differential privacy. Those are not silently counted as passing.

## 9. Red-team follow-on

The next privacy qualification increment should:
- generate overlapping cohorts and compare all aggregate outputs;
- submit adaptive candidate vectors to estimate whether aggregate weights/satisfaction can be reconstructed;
- attempt membership inference with auxiliary preferences;
- test unique/sparse profile combinations and criterion veto differencing;
- verify no public method returns raw profile vectors or exact anonymous cohort size;
- add query authorization, quotas or privacy accounting only after the threat model is defined;
- compare raw aggregate outputs against a formally specified privacy mechanism, where appropriate.

The benchmark must measure utility loss and privacy leakage jointly. Do not label a mechanism differentially private unless its adjacency relation, contribution bounds, epsilon/delta, composition accounting and implementation are formally specified and independently audited.

## 10. Claim ceiling

Passing this fixture establishes only deterministic consent filtering, API-level identity redaction, minimum-cohort suppression, veto and decision-ranking behavior for the finite synthetic corpus. It does not establish anonymity against re-identification, differential privacy, legal consent, ethics approval, real occupant satisfaction, democratic legitimacy or safe building control. Real pilots require informed consent, data minimization, accessible participation, explicit override and independent review by affected stakeholders.
