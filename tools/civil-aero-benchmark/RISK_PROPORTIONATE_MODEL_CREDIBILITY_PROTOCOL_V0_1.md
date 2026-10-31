# Risk-proportionate model credibility requirements protocol v0.1

## Purpose and evidence ceiling

A model should not receive the same assurance effort regardless of how it is used. A low-influence exploratory surrogate and a model that controls a high-consequence decision need different evidence. This protocol adds an explicit, reviewable policy for scaling model credibility requirements according to model influence and decision consequence.

The motivating external framework is NIST Advanced Manufacturing Series 100-85 (published October 5, 2026). It describes a risk-based credibility assessment approach in which VVUQ effort is scaled according to model risk, determined by model influence and decision consequence. It also distinguishes model creation, logic and transparency perspectives. The Luminous matrix below is our own declared project policy inspired by that approach; it is not the NIST formula, an ASME/JCGM requirement, a regulatory risk classification or an airworthiness standard.

Reference: Srinivasan, V. (2026), On Methods of Uncertainty Quantification in Computational Modeling and Simulation: Harmonizing Modeling and Metrology Practices, NIST AMS 100-85, https://doi.org/10.6028/NIST.AMS.100-85

## Classification inputs

Each assessment records model identifier and intended use; model creation profile (mechanistic/physics-based, empirical/data-driven or hybrid); logic transparency (inspectable, partially inspectable or opaque); artifact access (independent replay available, restricted or unavailable); model influence (advisory, material or controlling); decision consequence (limited, significant or severe); rationales for both influence and consequence; and a versioned classification reference.

Influence and consequence are qualitative judgments anchored to the stated intended use. They are not probabilities and are not combined into a numeric expected-loss score. Missing rationales or rubric identity fail closed.

## Luminous internal risk matrix v0.1

| Model influence / consequence | Limited | Significant | Severe |
|---|---|---|---|
| Advisory | Screening | Low | Moderate |
| Material | Low | Moderate | High |
| Controlling | Moderate | High | Critical |

This matrix is a declared policy choice. A project owner must justify the chosen cells for the actual intended use, and an independent reviewer should challenge the classification when the model controls a consequential decision.

## Required assurance controls

All tiers retain a baseline: explicit intended-use/applicability domain, assumptions/input lineage, exact model/run identity, failure modes/abstention policy, and independent control review.

Model creation changes the technical evidence profile:
- Mechanistic: code verification, iterative convergence, conservation/consistency, spatial/temporal convergence as applicable, and model-form sensitivity.
- Empirical: data-source/license provenance, leakage-safe train/validation/test separation, predictive calibration, applicability-domain and out-of-distribution detection.
- Hybrid: the union of mechanistic and empirical controls plus coupling/interface verification.

Logic and artifact access add controls:
- Partially inspectable logic requires independent interface and decision-trace audit.
- Opaque logic requires independent black-box output reproduction.
- Restricted artifacts require approved third-party artifact audit.
- Unavailable decisive artifacts prohibit independent verification/validation claims and prohibit admission as verified design evidence.

Risk tier adds requirements:
- Screening: explicit screening-only label and no design release/manufacturing/safety claim.
- Low: input sensitivity screen; no design release or safety certification from this assessment.
- Moderate: experimental comparison against independent measurements and uncertainty, plus a decision uncertainty budget.
- High: moderate controls plus blind validation holdout and independent reproduction.
- Critical: high controls plus physical test or justified alternative evidence route, independent safety case/authorization, and fail-safe human override. The software cannot authorize flight, manufacturing release or safety-critical operation.

The output is strictly requirements_assessment_only_no_evidence_completion_or_release_claim. It lists what is required; it does not mark any requirement as completed. A separate artifact-bound verifier must evaluate the evidence for each control.

## Implementation and negative controls

src/risk_credibility.rs implements the qualitative matrix and returns deterministic, sorted control requirements. Its synthetic demo uses three invented profiles to demonstrate Screening, High and Critical outcomes. The unit tests verify matrix monotonicity, distinct mechanistic/empirical/hybrid assurance requirements, and explicit restrictions for an opaque critical model whose artifacts are unavailable. The demo is not an assessment of any real model.

## Benchmark targets

1. 100% of assessments must retain intended use, influence/consequence rationales and rubric version.
2. 100% of critical contexts must request independent reproduction, physical test or a justified equivalent, independent safety case and fail-safe human override.
3. 0% of outputs may claim assurance completion or release authorization.
4. Test the matrix against a curated panel reviewed by at least two independent engineers; report inter-rater agreement and every adjudicated disagreement.
5. Add counterfactual tests: raising influence or consequence must never lower the tier or remove a required control.
6. Track false-low classifications and missing evidence controls in a blinded review set.

The goal is not to maximize the number of controls. It is to spend assurance effort where model influence and decision consequence make error most costly, while maintaining a fail-closed evidence ceiling.

## References

- NIST AMS 100-85 (2026): https://doi.org/10.6028/NIST.AMS.100-85
- NASA CFD verification assessment: https://www.grc.nasa.gov/www/wind/valid/tutorial/verassess.html
- NIST IR 8298 (2020), industrial CFD VVUQ: https://nvlpubs.nist.gov/nistpubs/ir/2020/NIST.IR.8298.pdf
