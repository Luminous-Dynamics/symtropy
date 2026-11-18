# HoloFrac Consent-First Pluralistic Decision Protocol v0

**Status:** synthetic multi-stakeholder decision benchmark  
**Implementation:** research/holofrac-retrieval/src/pluralistic.rs  
**Executable:** research/holofrac-retrieval/src/bin/pluralistic_decision_sweep.rs  
**Workflow output:** pluralistic-decisions.csv  
**Related:** [#6011 — consent-first pluralistic building decisions](https://github.com/Luminous-Dynamics/symtropy/issues/6011)

## 1. Purpose

Test whether HoloFrac can support multiple explicitly expressed stakeholder preferences without hiding disagreement inside one opaque utility score.

This first fixture compares candidate design/operation outcome vectors under explicit participant-supplied weights and hard veto thresholds. It reports the Pareto frontier, per-stakeholder satisfaction, equal-participant mean, maximin ranking and leximin ranking separately. No aggregation method is declared universally fair.

The fixture is entirely synthetic. Its "stakeholders" are synthetic roles/IDs, not real occupants or demographic inferences.

## 2. Preference and consent event contract

Each preference event binds:
- event ID and internal stakeholder ID;
- contiguous preference revision and exact previous-event ID;
- effective time and recorded/knowledge time;
- consent scope and status;
- eight explicit criterion weights;
- optional per-criterion hard minimum/veto thresholds.

Criteria in v0:
- thermal comfort;
- air quality;
- daylight;
- acoustic comfort;
- accessibility;
- privacy;
- energy efficiency;
- adaptability.

All outcome values are normalized to [0,1] with higher values better. Cost-like criteria must be transformed into a clearly declared benefit measure before being passed to this fixture.

Consent scopes:
- LocalOnly: may be used only in participant-local processing; excluded from shared aggregate decisions.
- AnonymousAggregate: may contribute to shared aggregate results, but stakeholder identity is omitted from shared per-stakeholder output.
- NamedAggregate: may contribute to named/shared evaluation only with that explicit authorization.

Consent statuses:
- NotGranted: cannot affect a group result; carries no preference weights/vetoes.
- Granted: may contribute only within the declared scope.
- Withdrawn: cannot affect a future group result; its withdrawal is a new append-only event and carries no preference weights/vetoes.

A late-recorded withdrawal is not allowed to rewrite what an earlier decision knew. Historical replay filters on recorded time; the later withdrawal only affects snapshots after it becomes known.

## 3. Decision semantics

- Symtropy hard feasibility is represented separately from preference.
- A candidate failing hard feasibility is always rejected.
- Any active, aggregate-consenting stakeholder's explicit veto threshold can reject a candidate.
- Participant satisfaction is calculated from that participant's own explicit weights only.
- Group mean satisfaction is reported as a comparator, not the default answer.
- Maximin ranks by the least-satisfied active participant.
- Leximin compares the sorted satisfaction vector from worst-served upward.
- Pareto dominance is evaluated on the original outcome vector, not a scalarized utility.
- Anonymous participants have no identity in shared assessment rows or veto outputs.
- Non-response is not consent; local-only and withdrawn profiles are excluded from group aggregation.

## 4. Benchmarked methods

The current executable compares:
- Pareto frontier;
- equal-participant mean satisfaction;
- maximin satisfaction;
- leximin satisfaction;
- explicit hard-constraint and individual-veto rejection.

Follow-on comparisons should include declared weighted-sum policy, Nash social welfare, voting/approval rules, quorum and delegated authority, sensitivity to weight perturbation, strategic preference reports, missing/unequal participation and preference drift. Every method has assumptions and failure modes; no one method should be labelled "fair" without a stated normative basis and affected-party review.

## 5. Acceptance targets

- 100% Pareto frontier agreement with an independent brute-force oracle on frozen finite candidate sets.
- Zero hard-infeasible candidates admitted to the Pareto set or rankings.
- Zero explicit veto violations admitted.
- 100% of excluded profile classes accounted for (local-only, not granted, withdrawn).
- Zero use of withdrawn/out-of-scope profiles after their revocation is recorded.
- Zero stakeholder IDs in anonymous output rows.
- Historical preference/consent snapshots must match the exact events recorded by the decision cutoff.
- Report each stakeholder's satisfaction, vetoes and unmet criteria; never rely on group average alone.

## 6. Adversarial extensions

Add tests for:
- late-recorded consent grant/revocation;
- absent/no response and invalid/incomplete profiles;
- contradictory and overlapping hard vetoes;
- one small group with strong preferences versus a large group with weak preferences;
- all aggregation policies selecting different alternatives;
- preference weight perturbation and strategic reporting;
- different automation scopes by service (HVAC, windows, personal devices);
- accessibility and privacy floors;
- anonymity leaks and re-identification attempts;
- manual override, local-only mode and loss of connectivity.

## 7. Claim ceiling

Passing establishes decision-procedure correctness for the synthetic profile/candidate corpus only. It does not establish actual occupant satisfaction, democratic legitimacy, universal fairness, privacy protection in a deployment, legal compliance, or permission to collect personal data. Real pilots require informed consent, data minimization, accessible participation, appropriate ethics review and independent review by affected people.
