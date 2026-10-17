# CIV-INFRA Measurement Compatibility Contract v0.1

**Status:** draft / synthetic software contract  
**Implementation:** \`tools/civ-infra-measurement-compatibility/\`  
**Parent research issue:** [CIV-INFRA-OBS-001 #1800](https://github.com/Luminous-Dynamics/symtropy/issues/1800)  
**Related baseline:** [Decision-impact planner PR #1799](https://github.com/Luminous-Dynamics/symtropy/pull/1799) and [primitive-derived evaluator PR #1784](https://github.com/Luminous-Dynamics/symtropy/pull/1784).

## 1. Paradigm shift: from partitions to compatibility

A caller-authored partition can be perfectly complete yet scientifically unjustified: it can assert a different observation for every state even when the measurement resolution and uncertainty make those states indistinguishable. R2 checks the partition's bookkeeping; it cannot establish measurement realism.

R3 instead derives a relation between declared states and possible outcome bins from typed state intervals, a versioned transform, and output ranges. One state can be compatible with several outcomes, especially when bins overlap or state uncertainty crosses a boundary. Outcome buckets therefore are not assumed to be mutually exclusive or equiprobable.

This prototype demonstrates that principle with a small whitelist of deterministic transforms. It is not a general measurement-model language.

## 2. Input and numeric contract

- Each numeric interval uses signed integer fixed-point ticks; one million ticks equal one declared unit.
- Supported units are meter, millimeter, and dimensionless. Meter/millimeter values are normalized into nanometer ticks for exact cross-unit comparison; dimensionless values retain micro-unit ticks.
- Intervals and outcome bins are closed: \([a,b]\). Boundary equality is possible in both adjacent bins.
- The model declares an exact version, model reference, optional SHA-256, input unit and applicability domain.
- Supported transforms are identity and a non-negative dimensionless square. The square transform rounds endpoints outward (lower downward, upper upward) so numeric quantization does not silently shrink the possible-output interval.
- A state interval not wholly within the model domain is flagged and treated as compatible with every outcome; it is not clipped to look more informative.
- Outcome bins may overlap. Predicted output not covered by any bin is reported as a support gap.
- An observation that matches no outcome bin is out-of-model-support. Even if an observation falls in a bin, if no declared state envelope supports it, the prior state set is also preserved and the model/support mismatch is flagged.
- If a supported observation is compatible with more than one state, all such states remain. No probabilities or expected value of information are manufactured.

## 3. Evidence semantics

Model and upstream-report references are preserved in the report. Supplied SHA-256 strings are checked for syntax, not retrieved from an artifact store or verified against bytes. The exact input JSON byte hash is computed by the CLI and bound to the report.

Consequently, v0.1 does **not** establish model authenticity, sensor calibration, observation-model correctness, model validation or engineering truth. \`model_artifact_ref\` is traceability metadata, not proof. A later artifact adapter must retrieve the exact referenced bytes, verify the hash and scope, and record verified/unverified status before the output can claim source-backed model provenance.

## 4. Generated software benchmark

Frozen fixture: four synthetic states, three outcome bins, meter/millimeter conversions, overlapping intervals, and a supported observation that leaves two states possible.

Generated suite: 100 deterministic seeds × 20 states × 5 bins = 10,000 state/outcome evaluations. A separate direct interval-overlap calculation checks the complete compatibility-edge set, not only its count; serialized reruns are compared exactly. CI stores each full input, its SHA-256, expected edge pairs/count and full report in an artifact.

Acceptance targets:

1. Zero false singleton classifications on the frozen/generated compatibility oracle.
2. 100% rejection of malformed versions, unknown fields, reversed intervals, duplicate IDs, incompatible dimensions and malformed supplied digests.
3. Partly or wholly out-of-domain state envelopes retain compatibility with every outcome and are never clipped.
4. 100% preservation of the prior state set for unsupported observations, including observations that fall in a generic output bin but outside every state-specific predicted envelope.
5. Exact meter/millimeter equivalence and order-invariant semantic reports.
6. Exact deterministic replay for the same bytes, rule version and toolchain.
7. No compatibility work above 100,000 state/outcome combinations in v0.1; higher-throughput evaluation should use a separately reviewed streaming/compact-receipt design rather than merely raising the allocation cap.
8. Report generated world count, edge count, source commit, exact input digest, artifact checksum and wall-clock/memory measurements before asserting scale.

These criteria establish implementation consistency against the declared interval contract only. They do not qualify a physical measurement system.

## 5. Research anchors and design rationale

- BIPM/JCGM GUM-6:2020, *Guide to the expression of uncertainty in measurement — Part 6: Developing and using measurement models*. It concerns the development and adequacy assessment of measurement models: https://doi.org/10.59161/JCGMGUM-6-2020
- BIPM/JCGM 100:2008/Amd.1:2026, *Nonlinearity in measurement models*. The 2026 amendment makes nonlinear propagation a current metrology concern; this v0.1 prototype limits nonlinearity to one exact, bounded transform rather than silently approximating arbitrary functions: https://doi.org/10.59161/PPDI3267
- NIST Advanced Manufacturing Series 100-85 (5 October 2026), *On Methods of Uncertainty Quantification in Computational Modeling and Simulation: Harmonizing Modeling and Metrology Practices*. It recommends risk-based credibility work scaled by model influence and decision consequence: https://doi.org/10.6028/NIST.AMS.100-85

These references motivate model-domain and uncertainty discipline. They do not validate this software or its synthetic examples.

## 6. Out of scope / next stages

Not yet supported: open endpoints, arbitrary formulas, nonlinear correlated multivariate uncertainty, censoring/detection limits, saturation, missingness mechanisms, time-varying calibration, likelihood distributions, calibrated probability updates, adaptive multi-step experiment selection, and solver-specific physical evidence.

Next stages should add a typed, verified artifact resolver; property/differential tests for conservative nonlinear interval bounds; a separate measurement calibration corpus with held-out model families; then adapters for solver-backed engineering evidence such as the SWMM qualification contract in #1764. Probability-based value of information is a distinct later version requiring defensible priors, observation likelihoods and held-out calibration.

No safety, conformance, permitting, hydraulic validity, sensor accuracy or real-world economic benefit is established by a software pass.
