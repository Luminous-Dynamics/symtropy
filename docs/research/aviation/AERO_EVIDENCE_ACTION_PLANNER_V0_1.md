# AERO Evidence-Action Planner v0.1

**Status:** deterministic synthetic decision-analysis prototype; no independent domain adjudication or field performance results  
**Authority ceiling:** non-binding investigation recommendations only; no assurance mutation, engineering acceptance, physical-work authorization or certification credit

## 1. Purpose

The planner ranks candidate evidence-acquisition actions by their expected value to a declared decision. It does not rank by confidence, uncertainty alone or model fluency.

The question is:

> Which permitted, evidence-producing action has the greatest expected improvement in decision value after accounting for acquisition cost and delay?

The distinction matters. An observation can be highly informative while being irrelevant to the available decision. Such an action should not be recommended solely because it reduces entropy.

The approach follows established value-of-information analysis: value is measured by expected decision utility before and after observing the sample, under an explicit decision model. The literature distinguishes EVSI, EVPI, EVPPI and expected net benefit of sampling; model and risk assumptions should be exposed rather than hidden in a scalar confidence score.

References:
- [ISPOR Value of Information Analysis: Emerging Good Practices Task Force](https://pmc.ncbi.nlm.nih.gov/articles/PMC7373630/)
- [Abbas and Hazen, On the Value of Information Across Decision Problems (2025)](https://doi.org/10.1287/deca.2024.0187)
- [National Academies, Incorporating Uncertainty into Decision Making](https://www.ncbi.nlm.nih.gov/books/NBK200840/)

## 2. Implemented synthetic kernel

The Rust module `evidence_action_evsi_v0_1.rs` implements a bounded, exact-integer EVSI ranking:

```
prior world-state probabilities
        + decision alternatives
        + utility(decision, state)
        + P(outcome | state) for each action
        + owner permission and model calibration dispositions
        + direct and delay costs
        -> exact EVSI and net-benefit numerators
        -> deterministic action ranking or abstention
```

Probabilities are integer parts-per-million (1,000,000 = 1). Utilities and costs use an explicitly declared integer utility unit. The EVSI numerator is scaled by (10^{12}), the product of the two PPM denominators. The implementation uses checked `i128` arithmetic and emits exact decimal numerators as strings; it does not round intermediate probabilities or silently normalize a malformed table.

For action (a), states (s), outcomes (o), decisions (d), prior (p(s)), likelihood (p(o\mid s,a)), and utility (U(d,s)):

[
EVSI(a)=
\sum_o \max_d \sum_s p(s)p(o\mid s,a)U(d,s)
-
\max_d \sum_s p(s)U(d,s)
]

The first term and second term are converted to the common PPM-squared numerator scale before subtraction. Expected net benefit subtracts direct and delay cost at that same scale. This is an expected utility calculation, not a safety probability or engineering acceptance score.

Tie-breaking is deterministic: net benefit descending, EVSI descending, direct cost ascending, delay cost ascending, then canonical action ID.

## 3. Hard boundary: permission and calibration precede ranking

An action can be ranked only if its upstream dispositions are:

- `VERIFIED_PERMITTED` under the exact owner action-catalogue / authority workflow; and
- `QUALIFIED_FOR_DECLARED_PURPOSE` for the supplied outcome model.

Denied or unknown permission, unverified/out-of-domain/non-estimable outcome models, missing authorization receipt digests, malformed outcome tables and missing artifact contracts cannot be treated as low-cost candidates. They are excluded or cause the input to be rejected.

These fields are inputs from source-owner evaluators. This prototype does not authenticate their signatures, verify the authorization receipt, validate the probability model or create authority. Production integration must consume owner-verified outputs and preserve their exact receipts.

Each action names target obligation IDs and an expected artifact-contract digest. This prevents a recommendation from becoming an unconstrained prompt such as “run a test and see what happens.” The owner still controls the acquisition method, prerequisites, safety envelope, approvals and execution.

## 4. Authority and non-mutation contract

The planner copies the following through unchanged:

- claim ID and frozen disposition;
- decision-context digest;
- assurance-state digest;
- action-catalogue digest;
- decision-model digest;
- outcome-model digest.

Its output always declares:
- `assurance_state_mutated = false`;
- `physical_work_authorized = false`;
- `engineering_acceptance_granted = false`.

A positive net benefit means only that an action is worth considering under the declared decision model. It does not clear a hard gate, change a claim disposition or authorize execution. A hard-gate blocker remains until the separately authorized acquisition produces admissible owner evidence and the existing assurance path reevaluates it.

## 5. Synthetic example and current software tests

The published example is `examples/aero_evidence_action_planner_v0_1.json` and its structural schema is `schemas/aero_evidence_action_planner_v0_1.schema.json`.

The synthetic example has two possible world states, two decision alternatives and two evidence outcomes. It includes:
- one permitted and calibrated document-acquisition action;
- one action with unknown permission, which must be excluded;
- a decision-irrelevant utility control, where information gain must not be mistaken for decision value;
- a malformed-prior-probability control, which must fail rather than renormalize;
- a deny-all control, which must abstain rather than report success.

For the positive fixture, the exact EVSI numerator is `40000000000000`, with cost 1 and delay cost 2 utility units, producing a net-benefit numerator of `37000000000000`. This is an arithmetic fixture result only, not an estimate for a real engineering project.

## 6. Benchmark programme

Compare these policies on identical frozen claims, decision models, action catalogues and cost assumptions:

1. expert-selected next action;
2. static rules / fixed priority;
3. lowest-cost-first;
4. maximum expected information gain;
5. EVSI / expected net benefit of sampling;
6. LLM-only action ranking;
7. source-bound Symthaea planner with calibration, permission and abstention.

Report separately:
- realized decision regret against an independently adjudicated oracle policy;
- realized net benefit and acquisition cost;
- elapsed time / delay per decision-relevant blocker resolved;
- fraction of outputs admissible under the target evidence contract;
- inconclusive, redundant and dominated acquisition fraction;
- high-information / zero-decision-value false-priority rate;
- probability calibration and proper scoring by action family;
- expert disagreement, overrides and automation-bias indicators;
- worst critical stratum, numerator, denominator and uncertainty interval.

### Hard gates

- 0 recommendations for denied or unknown-permission actions.
- 0 recommendations using unverified, out-of-domain or non-estimable outcome models.
- 0 malformed probability tables accepted or silently normalized.
- 0 planner outputs that mutate the original assurance state or claim disposition.
- 0 physical/operational work authorizations from the planner.
- 100% exact agreement with an independent arithmetic oracle on the frozen synthetic corpus.
- 100% deterministic ranking for identical input commitments.
- 100% abstention when no qualified permitted action exists.
- Positive-control retention: a deny-all planner is not a pass.
- Explicit numerator/denominator, source episode count and confidence interval for any empirical utility claim.

Efficiency is optimized only after those gates pass.

## 7. Next research steps

1. Define the owner-issued action catalogue and authorization receipt interface. The catalogue must include owner, requester role, prerequisites, permitted method, operating envelope, stop conditions, expected artifact schema and evidence receipt requirements.
2. Develop probability calibration and model-credibility requirements for acquisition outcomes. Synthetic-known-truth tests are not enough for real-world likelihood estimates.
3. Add coupled-blocker / sequential acquisition planning, including the option to stop, request a document, escalate or report that no available action has positive value.
4. Add decision-model and utility sensitivity analysis. If the recommended action changes across plausible utility/loss assumptions, return that instability instead of a false-precision ranking.
5. Add value-of-perfect-information and value-of-partial-perfect-information diagnostics to identify whether the limiting uncertainty lies in a decision-relevant group of state variables.
6. Conduct blinded human-factors studies with expert, rules, information-gain and LLM baselines. Measure both usefulness and automation bias.
7. Integrate only after the source-bound adapter (#7881) provides stable claim, obligation, provenance and observer commitments.

## 8. Limitations and non-claims

This prototype does not choose or authorize physical tests, infer safety or airworthiness, establish model credibility, decide whether an engineering claim is true, or change the AERO disposition. It does not yet model correlated outcomes, sequential/conditional acquisition plans, distributionally robust probabilities, risk-averse utility or multi-party negotiation. Those require separately specified versions and owner/domain validation.

The current kernel demonstrates exact arithmetic and enforceable interface boundaries on synthetic cases. It is not a field-qualified acquisition planner.
