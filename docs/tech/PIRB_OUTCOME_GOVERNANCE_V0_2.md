---
title: PIRB v0.2 — Locally Governed Outcome Metrics and Adaptive Review
version: 0.2-draft
status: proposal
authority: MeasurementOnly
scope: outcome-definition governance, metric versioning, contestability, distributional reporting, and safe review transitions
---

# PIRB v0.2 draft: outcome governance is part of the system under test

## Why this increment exists

PIRB v0.1 adds fail-closed response feasibility, multi-community spillover checks, and an adaptive stop-rule reference evaluator in [PR #1987](https://github.com/Luminous-Dynamics/symtropy/pull/1987). This v0.2 proposal isolates a different failure class: the evaluator can compute a metric perfectly while the metric itself has been defined by an unaccountable authority, excludes affected people, changes after outcomes are observed, or prevents people from challenging the result.

The benchmark should therefore test **the governance of measurement**, not merely the correctness of arithmetic. This is a proposal and contract draft, not an executable evaluator, approved study, consent receipt, or deployment authorization.

## Research basis

- UNDP's Independent Evaluation Office published updated impact-evaluation guidance on 19 March 2026. It explicitly distinguishes observing change from assessing whether a programme caused that change, and says a suitable method should match the question and evidence available: https://www.undp.org/evaluation/news/impact-evaluation-guidelines-published
- OECD's July 2026 evaluation guidance describes evaluation as a source of learning and accountability, including what worked, what did not, for whom, and why; it emphasizes clear objectives, independent and resourced functions, and participatory approaches tailored to context: https://www.oecd.org/en/publications/development-co-operation-tips-tools-insights-practices_be69e0cf-en/evaluating-development-co-operation_5602e1f7-en.html
- The OECD DAC evaluation criteria are complementary lenses, not a method that should be applied mechanically; their interpretation depends on the intervention, context, and stakeholders: https://www.oecd.org/en/publications/applying-evaluation-criteria-thoughtfully_543e84ed-en.html
- The Alliance for Peacebuilding's March 2025 standards argue that creating standards is insufficient without practical tools, application, and accountability: https://www.allianceforpeacebuilding.org/afp-publications/guiding-standards-for-peacebuilding
- The OECD recommendation on the humanitarian-development-peace nexus calls for conflict-sensitive action and assessing effects on conflict dynamics, social cohesion, exclusion, resilience, services, markets and local accountability, including unintended negative consequences: https://legalinstruments.oecd.org/public/doc/643/5d2573dc-8508-482c-97c6-d8319f21a345.htm

These sources motivate the proposal. They do not endorse PIRB or validate any Luminous Dynamics system.

## Paradigm shift

Treat every consequential outcome indicator as a **versioned, scoped, contestable contract** with:

1. A specific question, unit, denominator, population, geography, time window and missing-data policy.
2. An accountable authority with evidence of its mandate and an explicit scope of decision.
3. A record of consultation, dissent and challenges that does not force people to publish sensitive identities or testimony.
4. A rule for how aggregation, disaggregation, unknown coverage and small-cell privacy affect what can be claimed.
5. A history that preserves earlier definitions and results. Changes after observing outcomes create a new version and a disclosed post-hoc change; they never silently rewrite history.
6. A claim ceiling that distinguishes synthetic tests, historical replay, observational results and causal evaluation.

The schema does not prove any of those social or legal facts. It provides a structure that an independent review can inspect and a reference evaluator can falsify.

## Scope and prohibitions

PIRB v0.2 must not become a global peace score, a ranking of communities or people by propensity for violence, a surveillance or population-targeting engine, an automated aid-denial mechanism, or an authority to approve interventions. A schema-valid document does not prove legitimate representation, free consent, truthfulness of evidence, or safety.

Do not encode direct names, political affiliation, protected-group risk scores, private testimony, precise vulnerable-household locations, or exploitable service vulnerabilities in the shared metric definition. Use governed references and safely aggregated outputs instead. Small-cell suppression and scope restrictions require contextual privacy review; a single universal numeric threshold is not assumed.

## Proposed contract

The accompanying Draft 2020-12 schema, `peacebuilding-outcome-governance-v0.2.schema.json`, defines a metric record containing the indicator definition, population and geographic scope, denominator, missingness and aggregation policy, disaggregation/privacy policy, authority and mandate references, approval and dissent references, challenge/appeal route, version-control fields, claim ceiling, and governance-event history.

The synthetic example `peacebuilding-outcome-governance-v0.2.json` is deliberately fictional. It is a schema example, not a source of policy thresholds or empirical service data.

### Metric lifecycle

| Current state | Trigger / precondition | Required result |
|---|---|---|
| `DRAFT` | Definition lacks scoped authority or mandatory fields | `BLOCKED`; do not treat as a decision-ready metric |
| `PROPOSED` | Authority mandate, affected-scope definition, safe challenge route or privacy policy missing | `BLOCKED` |
| `PROPOSED` | Consultation/approval evidence is incomplete or contested | `REVIEW_REQUIRED`; no activation |
| `APPROVED` | Exact version, effective window, denominator, missingness and claim ceiling are frozen | May become `ACTIVE` only via a recorded authorized event |
| `ACTIVE` | A material affected group/scope is newly discovered and unassessed | `REVIEW_REQUIRED`; suspend unqualified aggregate reporting |
| `ACTIVE` | Metric changes after outcome observations begin | Preserve prior version/results; mark change post-hoc and require `REVIEW_REQUIRED` |
| `ACTIVE` | Appeal is unresolved, dissent was suppressed, or material evidence is contested | `REVIEW_REQUIRED`; do not silently close the challenge |
| `ACTIVE` | Mandatory rights/service floor breach, prohibited sensitive data, missing authority, or safety-critical evidence unknown/stale/contradictory | `BLOCKED` |
| `REVIEW_REQUIRED` | Authorized independent review resolves the reason, versions any changed definition, records evidence and preserves dissent | Return to `APPROVED` / `ACTIVE` only through an explicit authorized event |
| Any non-retired state | Legitimate authority suspends, supersedes or retires the indicator | Record the event and effective window; preserve prior versions and the reason |

This lifecycle is a proposed contract for review. It must not activate or halt a real intervention by itself. Any real operational stop action needs a separately authorized owner and safety protocol.

### Version-change rule

- Before outcomes are observed, a corrected definition may replace a draft while preserving the superseded revision.
- Once any outcome data are inspected, a consequential definition change creates a new metric version and requires a reason, changed-field summary, authority/review record, observation timestamp, and sensitivity analysis where meaningful.
- Prior values are not recomputed under the new definition without retaining the originally reported values and making the re-analysis explicit.
- Missing data, unassessed groups and unresolved contradictory evidence remain unknown/contested—not zero, absence of harm, consent, or success.

## Proposed acceptance targets

These are software-conformance targets, not field impact thresholds:

- **Definition completeness:** 100% of decision-ready indicators bind unit, denominator, population and geographic scope, effective window, missingness policy, aggregation rule, claim ceiling, and authority.
- **Authority:** zero activation when the mandate reference, scoped decision authority, approval record, or challenge route is missing.
- **Contestability:** every active metric has a usable challenge/appeal route; non-participation cannot be rewritten as assent.
- **Post-hoc integrity:** zero silent overwrite of a metric version after outcomes have been observed; 100% of post-observation changes are explicitly versioned and flagged.
- **Distributional coverage:** zero unqualified aggregate success claims when a declared group worsens or a material affected scope remains unassessed.
- **Privacy:** zero acceptance of fields prohibited by the declared protection policy; granular public output requires a context-specific disclosure review.
- **Review/stop behavior:** zero continuation in cases that require `BLOCKED` or `REVIEW_REQUIRED`; all transitions carry a reason code and evidence references.
- **Claim discipline:** zero causal claims from schema conformance, synthetic tests or before/after-only observations.
- **Reproducibility:** receipts bind schema/version, metric-definition digest, governance-event digest, source code, evaluator and toolchain; sensitive source data may be referenced by protected identifiers rather than publicly published.
- **Governance-event integrity:** duplicate event IDs, backward timestamps, future-version events, or an active metric without an approval-before-activation trace are rejected.
- **State/event agreement:** the declared metric status must agree with its latest state-changing governance event; a review-required challenge must not leave the metric silently active.
- **Independent tests:** each state transition has a nominal case, a violating mutation, and a missing/unknown-evidence mutation; bounded state-space checks use an independently structured oracle.

## Implementation sequence

1. Review the state machine and schema with peacebuilding evaluators, local/community partners, data-protection specialists, and relevant civic/service domain maintainers. No consultation is implied by this draft.
2. Implement a standalone Rust validator/evaluator that checks structure, version transitions, event ordering, approval preconditions, stale/post-hoc changes, challenge state and claim ceilings.
3. Add generated fixtures for incomplete authority, contested consent, missing affected communities, post-hoc metric changes, suppressed appeals, privacy leaks, and inconsistent governance histories.
4. Build a bounded independent oracle and mutation suite; require exact results under permutations of non-semantic event ordering but preserve chronology where order matters.
5. Only later propose a field pilot. Use the [field-evaluation protocol template](https://github.com/Luminous-Dynamics/symtropy/blob/research/peace-infrastructure-resilience-benchmark-v0-1-20261018/docs/tech/PIRB_FIELD_EVALUATION_PROTOCOL_TEMPLATE_V0_1.md) when it exists on the intended integration branch, and require local authority, independent review, predeclared outcomes, an appropriate evaluation design, data-protection controls, and stop/revise authority.

## Relationship to existing work

- v0.1 reference implementation: [Symtropy PR #1987](https://github.com/Luminous-Dynamics/symtropy/pull/1987)
- v0.2 research and acceptance agenda: [Symtropy issue #2033](https://github.com/Luminous-Dynamics/symtropy/issues/2033)
- Mycelix service commitment / consent integration: [Mycelix issue #5199](https://github.com/Luminous-Dynamics/mycelix/issues/5199)

Do not create competing Mycelix mediation or justice authority. The metric definition governs only how an outcome is measured; the relevant local authority, service domain, mediation/justice pathways, and independent evaluator retain their separate responsibilities.
