# AERO Assurance Benchmark Protocol v0.2

**Status:** prospective preregistration; not executed  
**Registry:** `aero-assurance-metrics-v0.2`  
**Supersedes:** metric definitions in v0.1 while preserving v0.1 as an immutable historical registry  
**Scope ceiling:** engineering-claim assurance software, evidence binding and decision reproducibility; no physical qualification, airworthiness, certification or model VVUQ authority

## 1. Why v0.2 exists

The original 17-metric registry froze numerator, denominator, population, inclusions, exclusions and stratification. The next gap was statistical identifiability: it did not explicitly record the sampling unit, independence unit or confidence-interval method for each metric.

That gap creates a common benchmark failure mode: one root-cause fault can generate hundreds of near-identical rows, which are then incorrectly reported as hundreds of independent tests. v0.2 makes the unit of analysis and inference method part of the machine-readable contract.

It also adds 11 metrics for the new claim-specification and claim-to-model-metric binding work. The original 17 remain present; this is a versioned expansion of the existing registry, not a competing scoreboard.

## 2. Added metrics

The 11 added metrics are:

1. `claim_specification_false_accept_rate`
2. `valid_claim_specification_retention_rate`
3. `claim_context_binding_miss_rate`
4. `semantic_digest_mismatch_detection_rate`
5. `model_metric_mapping_mismatch_acceptance_rate`
6. `claim_to_metric_semantic_oracle_agreement_rate`
7. `claim_metric_required_stratum_coverage_miss_rate`
8. `claim_metric_receipt_manifest_bypass_rate`
9. `claim_metric_stale_owner_record_acceptance_rate`
10. `receiver_first_seen_receipt_bypass_rate`
11. `independent_metric_mapping_reviewer_disagreement_rate`

Every metric includes an explicit numerator, denominator, population, inclusions, exclusions, stratification, sampling unit, independence unit, interval method, comparator arms and gating class.

The target on each safety metric is a prospective target, not a result. False accept and false rejection remain separate. The valid-specification-retention metric is needed to prevent reject-all from passing. Human disagreement is report-only and must retain dissent.

## 3. Independent adjudication

Each challenge case requires at least two independent reviewers. For claim-to-model metric mapping, reviewers must be competent in the physical discipline and requirements/assurance methods. Record reviewer qualifications, conflict declarations, initial independent decisions, rationale, confidence and dissent.

The oracle must be independently authored from raw challenge facts. It must not call the implementation under test, reuse its helper functions, copy its generated reason-code table or derive expected outputs from a test run.

Unresolved cases remain unresolved. They are excluded from a specific false-accept denominator only under a documented preregistered rule, and their counts and reasons remain public.

## 4. Partitions and leakage control

- **Public development:** available for implementation development; not qualification evidence.
- **Exposed challenge:** frozen adversarial set for debugging and regression; not a blind holdout.
- **Blinded holdout:** independently authored and withheld until the implementation and protocol are frozen.

Partition whole root-cause families and engineering campaigns. Do not place variants of one unit mismatch, missing receipt, trust revocation or supplier correction across both development and holdout and then count them as independent.

## 5. Statistical interpretation

For adverse hard-gate rates, use a one-sided 95% exact Clopper-Pearson upper confidence bound. If zero failures occur in n independent units:

[
p_{upper}=1-0.05^{1/n}
]

The prospective sample plan is 300 independent root-cause families for an initial screen and 2,995 for a qualification-level 0.1% upper bound under the simple independent Bernoulli model. These are independent families, not 300/2,995 JSON rows.

When cases share an initiating fault, resource, model, supplier, trust transition or engineering campaign, cluster them. Report cluster counts and use cluster-aware resampling or hierarchical analysis. Do not use an unclustered binomial interval when its independence assumption is plainly false.

For paired review-time and cost comparisons, use matched tasks and cluster bootstrap over claim families, preserving AERO/comparator pairing. Publish distributions and component costs, not only a single ratio.

## 6. Matched industry comparison

Benchmark the same frozen claims, source documents, evidence cutoffs, trust snapshots, model profiles, risk classifications and reviewer tasks across:
- expert human engineering review;
- structured requirements/assurance checklist;
- PLM/QMS-based workflow;
- single-model LLM-assisted review;
- retrieval-augmented LLM review;
- multi-agent LLM review;
- AERO claim and evidence assurance.

Do not assign a product name to a comparator without freezing product version, modules, configuration, training, data connectors and human workflow. IBM DOORS Next, PTC Windchill and Ansys ModelCenter are examples of systems with relevant requirements, PLM and analysis-workflow functions; their actual configurations and capabilities must be recorded during a real comparison, not inferred from product names.

## 7. Non-negotiable safety order

1. Establish zero observed critical false admissions on independently adjudicated negative root-cause families.
2. Establish complete affected-claim invalidation under qualified dependency observation.
3. Only then optimize unnecessary requalification and review economics.
4. Preserve false rejection, abstention, reviewer disagreement and unresolved outcomes separately.
5. Never combine safety, coverage, latency and economics into one compensatory score.

A system that rejects every claim fails valid-claim retention and coverage. A system that accepts invalid claims fails the hard-gate metrics. Neither can win by optimizing one aggregate number.

## 8. Relationship to standards and existing infrastructure

AERO is an assurance/composition layer, not a replacement for:
- ISO 23247-5:2026 (manufacturing digital thread across the product lifecycle) or ISO 23247-6:2026 (integrated, unified and federated digital-twin composition/interoperability);
- NIST IR 8536 (manufacturing traceability meta-framework, verifiable provenance and selective disclosure);
- NASA-STD-7009B and NASA-HDBK-7009B (model-and-simulation credibility practices and implementation guidance);
- ASME V&V 10-2019 (R2025) for computational solid-mechanics verification and validation;
- SAE ARP4754B for aircraft and systems development, requirements validation and design implementation verification;
- existing requirements/PLM/QMS, metrology and model-owner workflows.

The new differentiator being benchmarked is the claim-level semantic binding across those existing systems, including the exact owner metric, population, stratum, predicate, uncertainty semantics and historical receiver-time receipts.

## 9. Registry files and current status

- Schema: `schemas/aero_assurance_benchmark_metrics_v0_2.schema.json`
- Registry: `examples/aero_assurance_benchmark_metrics_v0_2.json`
- v0.1 remains preserved and independently validated as the prior registry.
- v0.2 is `PROSPECTIVE_NOT_EXECUTED`.
- Case adjudication remains `NOT_INDEPENDENTLY_ADJUDICATED`.
- CI validates schema and census; it does not execute benchmark trials or establish superiority over industry baselines.
