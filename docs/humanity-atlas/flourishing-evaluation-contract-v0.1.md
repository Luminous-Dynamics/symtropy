# Human Flourishing Evaluation Contract v0.1

**Status:** research contract; proposed, not yet implemented or validated  
**Scope:** scenario comparison in the Humanity Atlas across Symtropy, Symthaea, Mycelix, and downstream decision-support consumers  
**Decision boundary:** this contract evaluates evidence about possible outcomes. It does not confer legal authority, certify a policy, or authorize live control of infrastructure.

## 1. Purpose

The Humanity Atlas should help people compare interventions by their consequences for human lives, communities, and the conditions that sustain future generations. It must not turn “flourishing” into one opaque scalar that can hide deprivation, unequal impacts, rights violations, or ecological damage behind gains elsewhere.

The primary output is a **versioned outcome vector with uncertainty, subgroup views, hard-constraint results, and a readable trade-off record**. A composite score may be reported only as an explicitly optional view whose dimensions, weights, ownership, and sensitivity are disclosed. It must never override a hard constraint or stand in for the full outcome vector.

This is a proposed evaluation layer. It complements—but does not replace—the existing scenario/run contract, model-credibility process, domain-specific engineering checks, or independent authority receipts.

## 2. Normative and evidence principles

1. **Plural outcomes, no hidden substitution.** Report material conditions, quality of life, relationships and agency, environmental conditions, and future resilience separately. Improvements in one dimension do not silently cancel severe harm in another.
2. **Floors before rankings.** Specify mandatory safety, rights, basic-service, and environmental constraints before comparing candidates. A known violation makes a candidate infeasible; an unknown or unmeasured constraint is unresolved, never a pass.
3. **Distribution is part of the outcome.** Report population-level results alongside relevant geography and affected-group results, including tails and minimum-service coverage where measurable. A better mean must not conceal a worse outcome for a disadvantaged community.
4. **Objective and lived experience are complementary.** Administrative records, calibrated sensors, engineering measurements, and environmental data do not replace people's own accounts of their lives. Subjective measures do not replace physical safety or service measurements either.
5. **Current well-being and future resources are distinct.** Report near-term effects separately from effects on ecological systems, maintenance backlogs, resilience, skills, fiscal capacity, and future exposure. Do not trade away future viability by silently discounting it.
6. **Uncertainty stays visible and typed.** Missing data are not zero; proxies are not direct measurements; simulation output is not observed reality; correlation is not causal attribution. Every known estimate must state whether uncertainty is quantified. A quantified interval must identify its interpretation (coverage interval, plausible bounds, scenario envelope, sensitivity range, or measurement tolerance); a coverage interval must state its coverage probability and method. A point estimate with unquantified uncertainty cannot clear a hard constraint. Show ranges, assumptions, data gaps, and sensitivity to plausible model choices.
7. **Affected people retain agency.** Publish the purpose, measures, limits, and known conflicts. Provide channels for correction, appeal, refusal where appropriate, and human review. Do not infer consent or preferences from telemetry.
8. **Scale in proportion to evidence and reversibility.** Use simulation for exploration, independent verification for model claims, and monitored bounded pilots for real-world learning. Keep safe fallback and accountable human authority for consequential operations.

These principles follow the measurement lesson in established human-development and well-being work: a composite index is useful for focus but cannot represent the whole construct, and well-being should include both objective and subjective outcomes, their distribution, and sustainability over time. See [OECD's Well-being and Progress framework](https://www.oecd.org/en/topics/sub-issues/measuring-well-being-and-progress.html), the [OECD 2025 Guidelines on Measuring Subjective Well-being](https://www.oecd.org/en/publications/oecd-guidelines-on-measuring-subjective-well-being-2025-update_9203632a-en.html), and UNDP's explanation of [what the HDI does and does not measure](https://hdr.undp.org/content/human-development-index-what-it-and-what-it-not).

## 3. Minimum outcome vector

Every assessment must document which dimensions are in scope, why omissions are acceptable for the intended use, and which outcomes are mandatory constraints. The following is a starter catalogue, not a claim that every domain already has reliable data.

| Dimension | Example outcomes (choose domain-valid measures) | Common failure to guard against |
|---|---|---|
| Essential material conditions | Housing adequacy and cost burden; reliable access to safe water, electricity, food, and essential medicine | Measuring supply or capacity rather than reliable, affordable access |
| Health and safety | Preventable harm, exposure, service continuity, occupational and public safety | Optimizing average performance while increasing severe or concentrated harms |
| Time, access, and capability | Time spent obtaining essential services; travel/accessibility; education and usable skills; ability to pursue chosen goals | Counting service availability as proof that people can actually use it |
| Subjective well-being | Life evaluation, affect, and meaning/purpose using validated instruments and appropriate sampling | Treating a single sentiment signal as a population's preferences |
| Social and civic agency | Ability to participate, contest decisions, maintain relationships, and access remedies | Equating engagement volume or approval scores with consent |
| Distribution and equity | Coverage, cost, reliability, harms, and benefits across relevant locations and groups | Reporting only aggregate means; unsafe re-identification in small groups |
| Environmental integrity | Water-system and ecosystem impacts, air quality, land impacts, lifecycle emissions and resource use | Moving impacts outside the modeled boundary or into the future |
| Future resilience | Maintenance and replacement needs, redundancy, climate exposure, supply dependencies, skills, and long-run affordability | Short-term gains that create unpriced fragility or deferred liabilities |

Each metric needs a stable identifier, definition, unit, direction of improvement, population/geographic scope, time window, source and transformation provenance, uncertainty/missingness description, subgroup applicability, privacy classification, and whether it is a hard constraint or an outcome for comparison. A measure without sufficient definition or provenance remains exploratory.

## 4. Decision procedure

### Stage A — Register the decision

Record the decision question, decision owner, affected population, jurisdiction, time horizon, alternatives including the current-policy baseline, intended use, excluded effects, legal/rights constraints, and who can challenge the assumptions. The model must not decide the normative weights on behalf of affected people.

### Stage B — Qualify the model for its intended use

Apply the Humanity Atlas model-credibility procedure: identify the model and version; verify implementation; compare against analytical or independently accepted reference cases where available; evaluate historical behavior without leakage from the evaluation period; document calibration, parameter and structural sensitivity; expose uncertainty and domain limits. A model can be fit for one bounded exploratory question and unfit for forecasting or control.

### Stage C — Establish baseline and counterfactuals

Use comparable temporal and geographic boundaries for every alternative. Bind every source artifact to its exact bytes and record data vintage, transformations, and known gaps. Distinguish observed inputs from modeled quantities. Use held-out periods or locations where the intended claim requires generalization. Preserve failed, incomplete, and skipped runs rather than silently dropping them.

### Stage D — Apply feasibility constraints before preference ranking

Evaluate every mandatory constraint as PASS, FAIL, or UNKNOWN, with evidence references and uncertainty. UNKNOWN does not pass. A known violation of a non-compensable constraint makes an option ineligible regardless of any aggregate benefit score. Constraints and thresholds must be declared before seeing the ranking, with the accountable source of each threshold recorded. A hard constraint must reference a digest-bound artifact whose machine-readable role is `pre-registration` and whose declared capture/declaration chronology is consistent with the analysis; a self-reported timestamp alone is not enough to qualify the constraint. Artifact hashing establishes byte identity, not trusted time or publisher authenticity. Artifact roles are machine-readable and separate from human-readable purpose descriptions.

### Stage E — Compare feasible options without hiding trade-offs

Report effect estimates and uncertainty for every in-scope outcome and subgroup. Identify Pareto-dominated options when the evidence supports that conclusion; show the non-dominated set and make unresolved trade-offs legible. If a decision authority elects to use weights, publish the weight source and rationale, show the full unweighted vector beside the score, and test whether rankings reverse over defensible weights. A ranking that changes materially under plausible weights or assumptions is **preference-/assumption-sensitive**, not a robust winner.

### Stage F — Independent review and bounded learning

Before a real-world pilot, publish the intended-use statement, predeclared endpoints, monitoring plan, stop conditions, privacy controls, safe fallback, and reviewer conflicts. Involve affected people and domain professionals. A pilot should be reversible where practicable, monitored against a pre-pilot baseline, and designed to detect harm as well as benefit. Expansion requires a separate decision with its own evidence review.

## 5. Evidence levels must not be conflated

Keep execution status, model credibility, and real-world outcome evidence as separate fields. The following proposed evaluation labels describe the strongest kind of support for a particular claim; they do not replace the existing run-status vocabulary:

- SIMULATION_ONLY: outcomes are generated by a model; no claim of real-world effectiveness follows.
- HINDCAST_EVALUATED: predictions were compared with withheld historical observations for a stated domain and period; this does not alone prove prospective accuracy.
- OBSERVATIONALLY_MONITORED: measurements from a real deployment or environment were collected with defined provenance; attribution may remain uncertain.
- CAUSALLY_EVALUATED: a stated causal design and assumptions support an effect estimate for a specified population and period; generalization remains bounded.
- INSUFFICIENT_EVIDENCE: the data, validation, coverage, or design do not support the proposed claim.

These are not interchangeable and should not form an automatic ladder: a causal estimate can still be narrow or biased, and a strong hindcast can still fail after conditions change. Failed, incomplete, or unqualified runs cannot support a positive outcome claim. Hashes establish byte identity, not publisher authenticity, measurement validity, or scientific truth.

## 6. First applied vertical slice: essential-service reliability

A promising pilot candidate is **decision support for essential-service reliability** (for example, water or electricity), beginning with retrospective analysis and simulation rather than autonomous operational control.

A minimum comparison could report:
- service availability and outage duration/frequency;
- safety and quality failures, including severity rather than just counts;
- affordability and household/service-user burden;
- geographic coverage and distribution of interruptions;
- time-to-detection, repair, and restoration;
- lifecycle cost, maintenance backlog, energy/resource use, and environmental constraints;
- uncertainty, missing telemetry, and the number/coverage of affected people.

Sensor telemetry and satellite products can be inputs only after their semantics, calibration, coverage, temporal alignment, and failure modes are documented. Remote sensing may contextualize environmental or land-surface changes; it is not direct proof that a particular household has safe water or reliable electricity. An absent sensor reading must remain missing, not be imputed as normal service. Any recommendations affecting real infrastructure require an accountable operator, an independent safety envelope, and a tested fallback.

The first deliverable should compare a small set of transparent alternatives against the same baseline and report where evidence is too weak to rank them. Success is not a single composite number; it is a defensible, inspectable decision record that helps people improve service without concealing who bears the cost.

## 7. Acceptance criteria for a future implementation

A machine-readable implementation of this contract should not qualify an assessment unless:

- all mandatory metrics and constraints are registered before candidate results are evaluated;
- each reported value is linked to source artifacts, transformations, units, scope, time window, and uncertainty or explicit unknowns;
- all alternatives share comparable baseline boundaries and evaluation windows;
- subgroup coverage and privacy rules are checked; all alternatives use the same declared subgroup identifiers, and missing coverage is represented explicitly as unknown rather than omitted;
- no unknown hard constraint is treated as passed and no composite score overrides a failed constraint;
- every hard constraint binds a structured, digest-verified pre-registration record whose metric, comparator, thresholds, scope, non-compensable flag, and declaration timestamp exactly match the assessment; that record cannot be reused as outcome evidence;
- invoking the semantic checker without verifying local artifact bytes cannot qualify a hard constraint; reports state whether bytes were verified;
- the full outcome vector remains available whenever a composite score is displayed;
- sensitivity analysis flags material rank reversals;
- evidence level and run status remain separate, with no promotion from a failed or incomplete run;
- the decision record preserves dissent, limitations, approval authority, and stop/fallback conditions;
- validation tests include adversarial cases for missing values, subgroup suppression, stale data, mismatched units, unsupported causal claims, hidden hard-constraint failures, re-hashed but semantically mismatched pre-registration artifacts, path traversal/control characters, and attempts to overwrite assessment/evidence files with reports.

## 8. Research references

- OECD, [Measuring well-being and progress](https://www.oecd.org/en/topics/sub-issues/measuring-well-being-and-progress.html): multidimensional current outcomes, inequality, and resources needed to sustain well-being.
- OECD (2025), [Guidelines on Measuring Subjective Well-being](https://www.oecd.org/en/publications/oecd-guidelines-on-measuring-subjective-well-being-2025-update_9203632a-en.html): life evaluation, affect, and eudaimonia as distinct, complementary components.
- UNDP, [The Human Development Index — what it is and what it is not](https://hdr.undp.org/content/human-development-index-what-it-and-what-it-not): composite indices are useful but limited and should be supplemented by a broader dashboard.
- NIST, [Measurement Uncertainty](https://www.nist.gov/itl/sed/topic-areas/measurement-uncertainty) and [TN 1297: Reporting Uncertainty](https://www.nist.gov/pml/nist-technical-note-1297/nist-tn-1297-7-reporting-uncertainty): uncertainty should be described with the method used; probability/coverage interpretations need a stated basis.
- W3C, [PROV Model Primer](https://www.w3.org/TR/prov-primer/): provenance records can represent the entities, activities, and agents involved in creating and transforming data.
- Research Object Crate, [RO-Crate 1.2 data entities](https://github.com/ResearchObject/ro-crate/blob/main/docs/_specification/1.2/data-entities.md): represent source and derived files as connected, typed data entities.
- OECD, [Managing climate risks — decision-making under uncertainty](https://www.oecd.org/en/publications/managing-climate-risks-facing-up-to-losses-and-damages_55ea1cc9-en/full-report/component-8.html): robust decision-making stress-tests candidate plans against multiple plausible futures rather than treating one forecast as the only future.

**Non-claim:** this document proposes an evaluation protocol. It does not demonstrate that any candidate intervention improves flourishing, validate any simulator, establish an empirical effect, or authorize deployment.
