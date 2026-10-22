# CIV-INFRA Bridge/Tunnel Decision-Impact Case v0.1

## Purpose

This fixture applies the existing CIV-INFRA decision-impact planner (PR #1799) to the bridge/tunnel qualification program (Symtropy #1921). It is a deliberately small integration example, not a new optimizer or civil solver.

The central research question is: **which investigation most reduces uncertainty about the current design choice, under an explicit finite set of possible worlds?** This is decision-impact ranking, not expected Value of Information. No outcome probabilities or likelihoods are implied.

## Frozen synthetic world

The three candidate corridor concepts are a surface detour, bridge alternative and tunnel alternative. Four fictional states produce four distinct candidate decision signatures:

1. Stable, dry ground: all alternatives pass only the modeled synthetic checks.
2. Bridge-scour risk: the bridge candidate is infeasible under the declared state.
3. Tunnel-groundwater risk: the tunnel candidate is infeasible under the declared state.
4. Unresolved deep geology: bridge and tunnel remain unknown, not safe by default.

These state signatures are fixture inputs, not predictions of a real site. The action-to-outcome mappings are also declared synthetic partitions; the fixture does not establish that a real borehole or scour survey can discriminate these states.

## Hand-checkable ranking

There are four distinct decision signatures, hence 6 unordered discordant state pairs before investigation.

| Action | Outcome group sizes | Worst remaining signature classes | Worst remaining discordant pairs | Discordant pairs separated |
|---|---:|---:|---:|---:|
| tunnel-groundwater-borehole | 2 + 2 | 2 | 1 | 4 |
| bridge-scour-survey | 1 + 3 | 3 | 3 | 3 |
| desktop-review-no-discrimination | 4 | 4 | 6 | 0 |

The planner's declared lexicographic policy therefore ranks the tunnel investigation first for decision impact, the bridge survey second, and the non-discriminating desktop review last. Its estimated cost and duration remain separate fields: no unsupported exchange rate between decision ambiguity, money and elapsed time is invented. The ranking does **not** mean the tunnel borehole is the best field action in a real project; site-specific scope, cost, safety, permissions, measurement validity and outcome-model evidence are not qualified here.

The integration test freezes the expected ordering and key metrics. A negative control deletes one state's membership from a declared outcome map and requires the planner to reject the incomplete mapping rather than assume a convenient result.

## Paradigm shift

Use measurement planning to ask what could change the decision before spending effort on data that merely looks informative. Then raise the bar on evidence: a real investigation can influence engineering decisions only when its measurement model, site applicability, units, uncertainties, provenance and outcome-support limits are qualified.

Keep the layers separate:

- **Symtropy:** finite scenario evaluation and decision-impact computation using the existing planner.
- **Symthaea:** proposes investigations, identifies assumptions and challenges alternatives; it cannot invent the survey's measurement performance.
- **Sol-Atlas:** projects the approved, versioned corridor and investigation provenance spatially; rendering does not establish physical feasibility.
- **Mycelix:** preserves owner, reviewer, procurement and approval evidence; a computed ranking is not authorization to drill, close, inspect less, or build.

## Benchmark targets

- Exact agreement with the hand-derived action ordering and metrics for this fixture.
- Zero accepted mappings with missing, duplicated, undeclared or overlapping state membership.
- Deterministic replay for the same fixture bytes and evaluator version.
- Preserve unknown bridge/tunnel status in the unresolved geology state.
- Do not generate probabilities or expected-VoI fields.
- Report fixture and claim scope as synthetic; do not claim a physical survey-performance result.

## Research anchors

- European Commission JRC, [Reliability background of the Eurocodes](https://eurocodes.jrc.ec.europa.eu/publications/reliability-background-eurocodes): uncertainty, time-dependence, deterioration, inspection and reliability over service life.
- European Commission JRC, [Major concepts of the EN Eurocodes](https://eurocodes.jrc.ec.europa.eu/en-eurocodes-about-en-eurocodes/major-concepts-en-eurocodes): safety, serviceability, robustness, durability, working life and quality assurance.
- Nasim et al., 2025, [A demonstration of a digital twin framework for structural health monitoring: Application to bridge infrastructures](https://doi.org/10.1016/j.iintel.2025.100184): a field-oriented digital-twin example combining physics-based modeling and structural monitoring. Structural-condition monitoring remains a different validation claim from design of new structures.

## Limits

This is a software integration fixture. All states, costs, durations and action-outcome mappings are synthetic. No bridge load, tunnel lining, groundwater, scour, track-bridge interaction or geotechnical equations are evaluated. No measurement likelihood, calibrated sensor model, engineering safety limit, legal standard, permit, construction release or operational authority is established. A passing test establishes only that the planner behaves as specified on this finite input.
