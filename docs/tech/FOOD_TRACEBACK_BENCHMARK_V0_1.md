# Food-Safety Traceback Benchmark v0.1

**Status:** synthetic-first research contract; not an operational tool.
**Issue:** [FOOD-TRACEBACK-001](https://github.com/Luminous-Dynamics/symtropy/issues/2173)
**Fixture:** [FOOD_TRACEBACK_V0_1.json](../../benchmarks/fixtures/FOOD_TRACEBACK_V0_1.json)

## 1. Research question

Can an evidence-bounded traceback system preserve affected-product coverage while reducing unnecessary recall burden, without turning incomplete supply-chain records or pathogen-profile similarity into unsupported causal claims?

The benchmark evaluates a decision-support pipeline—not the truth of an outbreak investigation and not a substitute for a public-health authority.

General procurement or medicine-supply simulation usually measures delivery, cost, availability and service continuity. Foodborne traceback adds an asymmetric, safety-critical decision: too-narrow containment can leave affected product in circulation; an indiscriminately broad recall can destroy unrelated product and falsely implicate suppliers.

## 2. Existing practice and evidence boundary

- **CDC FDOSS/NORS** reports outbreak-level information such as dates, locations, illnesses, implicated foods, settings and pathogens: https://www.cdc.gov/nors/about/fdoss.html and https://www.cdc.gov/nors/data/index.html
- **FDA GenomeTrakr** uses whole-genome sequencing across clinical, food and environmental isolates to support outbreak detection and traceback: https://www.fda.gov/Food/FoodScienceResearch/WholeGenomeSequencingProgramWGS/. A genomic match is pathogen-relatedness evidence; it does not alone establish that a supplier's lot moved through a particular facility or caused a case.
- **FDA Food Traceability Rule** identifies Critical Tracking Events and Key Data Elements for certain foods. Check the live FDA page for current policy and dates: https://www.fda.gov/food/food-safety-modernization-act-fsma/fsma-final-rule-requirements-additional-traceability-records-certain-foods

These public resources do not, by themselves, give us an openly labelled, end-to-end graph of ingredients, transformations, shipments, retail lots and consumer exposure with complete causal ground truth. V0.1 therefore uses fully synthetic records and explicit truth labels. Do not mix public outbreak observations into the fixture and call inferred links ground truth.

## 3. Paradigm shift: from a graph query to an auditable decision envelope

A conventional query may return every upstream node reachable from an outbreak-associated lot. That can overstate what the records actually establish.

The proposed system returns three distinct objects:

1. **Containment candidates:** product lots to hold or investigate under a declared coverage rule.
2. **Source hypotheses:** possible upstream sources, with supporting and contradicting record IDs and explicit uncertainty.
3. **Evidence gaps:** missing handoffs, stale records, contradictory custody claims, unresolved identifiers and the next evidence that could separate hypotheses.

The system must be able to abstain on source attribution while preserving a justified containment action. It must explain how results change when a record is removed, expires or conflicts with another record. A plausible narrative is not evidence.

## 4. Standards interoperability boundary

V0.1's synthetic JSON is a benchmark fixture schema, **not** a GS1 EPCIS-conformant event feed. Do not imply standards conformance from passing this validator.

The next interoperability increment should compose with Mycelix [LEX-NET-015](https://github.com/Luminous-Dynamics/mycelix/issues/1110), which already defines a cross-standard supply-chain traceability bridge and names [GS1 EPCIS/CBV 2.0](https://www.gs1.org/standards/epcis) as a mapping target. GS1 describes EPCIS events as timestamped records of what happened to objects, where, and in what business context, including aggregation and transformation flows. Reuse that owner instead of inventing a parallel GS1 adapter or traceability ledger.

The adapter should preserve the source standard/profile version and original evidence commitment; explicitly retain unmapped, missing and contradictory fields; and never invent a handoff to make a path look complete. Map relevant EPCIS event types (including object, aggregation and transformation events) and applicable FDA Critical Tracking Events / Key Data Elements as separate, versioned conformance profiles. Keep these claims distinct: event syntax conformance, issuer identity, observation content, custody interpretation, product conformity and any regulatory interpretation. None implies the others.

## 5. Fixture design

FOOD_TRACEBACK_V0_1.json is synthetic and frozen at schema version 0.1.0. It contains five hand-auditable cases:

| Case | Stressor | Required behavior |
|---|---|---|
| CLEAN-001 | Complete custody path; one implicated and one unrelated finished lot | Reconstruct the path, retain the affected lot, and exclude the known unrelated lot under the declared complete-coverage assumption |
| MIX-SPLIT-002 | Two ingredient lots merge into a mix that feeds two finished lots | Propagate exposure through the actual transformation and split; do not assume one input lot maps to one output lot; preserve source ambiguity and do not force exclusion of FG-B where evidence cannot distinguish the inputs |
| MISSING-HANDOFF-003 | The ingredient-to-mix custody edge is unknown while an outbreak-associated finished lot is known | Preserve containment of the observed affected lot; mark the upstream source unresolved; do not claim complete trace coverage |
| CONFLICTED-CUSTODY-004 | Two incompatible upstream custody claims refer to one finished lot | Preserve the conflict, contain the affected lot, and abstain on conclusive source attribution |
| GENOMIC-NO-ROUTE-005 | A retail sample shares a pathogen profile with a supplier sample, but no custody route connects that supplier to the affected lot | Keep genomic association distinct from material flow; do not assert a supply path or causal source from similarity alone |

The fixture records are event- and evidence-scoped. A verified custody record, a verified sample-origin record, and a screening-only pathogen-profile comparison have different semantics. Sample-origin metadata identifies what was sampled; it is not a material-flow edge. A profile match can show relatedness without proving that the sampled ingredient caused the outbreak. Unknown and conflicted states are first-class values, not missing values to coerce into verified or false.

All entity IDs, quantities, dates, product lots, sample identifiers and truth labels are invented. No real supplier, product, illness cluster or facility is represented.

## 6. Comparison methods

Use the same input records, candidate-set definitions and evaluation denominators for all methods.

- **B0 — full-information oracle:** computes reachability and affected output lots using complete hidden truth. This is an evaluation upper bound, not a deployable method; its truth access must never leak into other methods.
- **B1 — naive reachable-upstream union:** implemented first baseline; returns upstream ingredient candidates and downstream finished lots while ignoring edge status. It measures the cost of treating graph presence as sufficient.
- **B2 — association-only sample/profile heuristic (implemented negative control):** maps a pathogen-profile match back to sample-origin ingredient lots without requiring a material-flow route. It is deliberately adversarial: its output is labelled association-only, must never be asserted as custody/cause, and is scored for false source attribution. A future temporal/geospatial comparator needs qualified event times and locations; v0.1 does not invent coordinates and should consume qualified spatial context from Sol-Atlas.
- **B3 — evidence-bounded envelope:** implemented reference method; reports verified source paths separately from possible source candidates built from material-flow edges with all declared statuses. It can abstain when no material route is observed and reports ambiguity when multiple candidate sources remain. Sample-origin records are excluded from material-flow reachability. It is a deterministic graph baseline, not a learned or validated causal inference system.

A standalone deterministic Rust evaluator implements B1, B2 and B3. B1 and B3 report source candidates and containment metrics; B2 deliberately demonstrates a false source attribution when a profile match exists without a custody route. Sample-origin records are excluded from material-flow reachability. Future GS1/Sol-Atlas adapters must preserve the event/provenance boundary rather than creating a second supply-chain ledger, genomic classifier, refrigeration simulator or regulator workflow.

## 7. Evaluation: never collapse safety into one score

Report per case, per stressor and as a macro-average. Show denominators and unresolved counts.

**Containment quality**
- Affected-lot recall: ground-truth affected finished lots retained divided by all ground-truth affected finished lots.
- Residual exposure: count and declared synthetic units of affected lots not contained.
- Unnecessary-recall burden: count and declared synthetic units of known unaffected lots included in containment.
- Coverage-claim validity: whether a method claiming complete trace/containment has all required path evidence under the fixture's coverage contract.

**Source attribution**
- Candidate source precision and recall, keeping set-valued hypotheses separate from a single selected source.
- Abstention quality: whether the method withholds a conclusive source claim when evidence cannot distinguish competing explanations.
- Brier score, log loss and reliability plots only if a method returns meaningful probabilities. Natural-language confidence is not a calibrated probability.
- Evidence support: fraction of asserted source-path edges that resolve to admissible record IDs; count unsupported promotions from unknown, conflicted or stale evidence.

**Cost and robustness**
- Candidate-generation wall time and evidence-request count, separately from investigator time. Do not claim hours saved before a practitioner study.
- Permute input order and require canonical semantic output and serialization to remain identical.
- Remove records, stale records, conflict records or an entire source channel in controlled ablations. Report changes in coverage, containment burden and source abstention.
- Keep health consequences, economic loss, destruction cost and false-attribution harm as separate dimensions. Any later composite cost must expose weights and sensitivity analysis.

The central visualization is a Pareto frontier between residual affected exposure and unnecessary-recall burden. A scalar score must not conceal an unsafe miss. The current receipt reports both dimensions per case; it does not yet fit or rank a global operating point.

## 8. Fail-closed invariants

1. **Missing is not negative.** Missing records do not show that a lot is clean, a transfer did not happen, or exposure was zero.
2. **Association is not custody.** Pathogen similarity, co-location, time overlap, co-purchase, sample-origin metadata or shared parentage are not individually a verified material-flow edge.
3. **Unknown/conflict remains explicit.** A missing, stale or conflicting transfer cannot be silently promoted to verified. Sample-origin records must be typed and cannot be traversed as lot-to-lot material movement.
4. **Containment and attribution are separate.** An affected finished lot can warrant containment even when its source is unknown.
5. **No unsupported complete-coverage claim.** If required handoffs are unresolved, disclose that the trace and containment scope may be incomplete.
6. **Evidence is inspectable.** Every candidate and ruled-out alternative retains supporting, contradicting and missing evidence references.
7. **Identity and units are strict.** Unknown lot identity, quantity basis, timezone or event order blocks claims that depend on resolving it.
8. **Deterministic replay.** Identical canonical inputs and configuration produce byte-identical canonical reports.
9. **Authority boundary.** The evaluator cannot declare food safe, publish a supplier accusation, authorize product release or initiate a real-world recall.

## 9. Acceptance gates

These are proposed requirements, not results:

- 100% exact agreement with hand-computed outputs for complete toy-case reachability and lot propagation.
- 100% preservation of unknown/conflicted status in designated stress cases.
- Zero source-causation claims from pathogen similarity, time overlap or graph reachability alone.
- Zero reports claiming complete trace/containment when required handoffs remain unresolved.
- 100% case accounting against declared denominators; no silent omission of false positives, unresolved cases or uncontained affected lots.
- Byte-identical canonical output under record/entity-order permutations, with explicit unit coverage for baseline metric invariance.
- No SOTA or industry-superiority claim until comparison against sourced, rights-cleared data and a practitioner-reviewed reference workflow under matched conditions.

A fixture validator should reject duplicate IDs, dangling references, invalid evidence status, malformed timestamps, non-positive or unitless quantities where quantities are compared, unknown schema versions and mutated frozen truth labels.

## 10. Follow-on research

Only after the synthetic oracle is reliable:

1. Build adapters for public outbreak aggregates and pathogen-genomic references, preserving distinct evidentiary meaning and source/license metadata.
2. Seek a governed, de-identified lot-trace dataset or tabletop exercise from food-safety practitioners. Define reference labels, disagreement adjudication and retention before use.
3. Compare with traceability software and investigators' current process using the same records, clock start, scope and independent scoring.
4. Test whether decision-sensitive evidence acquisition reduces unresolved hypotheses without delaying a necessary precautionary hold.
5. Consider a South African case study only with local authority and partner input, rights-cleared records, and no unsupported claims about local incidence or supplier culpability.

## 11. Claim ceiling

A passing synthetic benchmark establishes only that the evaluator respects its declared invariants on these small synthetic cases. It does not establish outbreak-detection capability, correct real-world source attribution, fewer illnesses, regulatory compliance, operational savings, or superiority over existing industry systems.