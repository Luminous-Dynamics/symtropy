# Measurement qualification benchmark v0.1

**Status:** synthetic reference-material and resolution-aware sampling evaluator. No physical scanner, sample stream, reference artifact or field cohort has been qualified.

## 1. Research basis and what is actually new

### Resolution-limited sample-size research (November 2026)

Sasakura & Kimura, *Powder Technology* 483, article 122851 (November 2026), DOI 10.1016/j.powtec.2026.122851, present a framework for sample size in large-scale particle-image analysis based on descriptor stability and image resolution limits. The indexed abstract highlights a Last-Exit rule intended to suppress false convergence and Monte Carlo inverse analysis for sample-size determination.

Source: https://doi.org/10.1016/j.powtec.2026.122851

The paper is adjacent to our morphology sampling problem. The complete paper has not been independently inspected here; v0.1 does **not** claim to reproduce its Last-Exit method or numerical results. We implement a separate, explicit reference-and-quantile qualification contract.

### Controlling metrology / particle measurement references

- ISO 13322-2:2021 covers dynamic image analysis and instrument qualification using particulate reference materials, including motion blur, depth of field and particle orientation. https://www.iso.org/standard/72566.html
- ISO 9276-6:2008 was reviewed and confirmed current in 2022. It states that averaging shape over all particles is ineffective and calls for distributions of particle characteristics in addition to size distributions. https://committee.iso.org/standard/39389.html
- BIPM/JCGM lists JCGM 100:2008 and its 2026 Amendment 1 on nonlinearity in measurement models, along with JCGM 101 on Monte Carlo propagation of distributions and JCGM 106 on measurement uncertainty in conformity assessment. https://www.bipm.org/en/web/guest/publications/guides
- NIST AMS 100-85, published 5 October 2026, advocates risk-scaled VVUQ harmonized with metrology and explicitly discusses distribution propagation, uncertainty and digital-twin credibility. https://doi.org/10.6028/NIST.AMS.100-85

## 2. Paradigm shift

A large particle count is not itself evidence of a qualified measurement.

Three independent questions must be answered:
1. Can the instrument resolve the physical feature and descriptor value being claimed?
2. Does it reproduce certified reference values across sessions, operators and orientations?
3. Does the independent sampling design support the particular p05/p50/p95 quantile precision being reported?

A high sample count cannot compensate for unresolved features, duplicated particles, correlated frames, censoring, selection bias or an unrepresentative size stratum.

## 3. Executable v0.1 contract

The crate `symtropy-measurement-qualification-bench` validates a child of #3910.

### Reference checks

Each certified reference check binds:
- descriptor type, 2D/3D basis and versioned descriptor definition;
- certified value and standard uncertainty;
- feature dimension and certificate reference;
- calibration identity matching the instrument;
- repeated scans with unique replicate IDs and declared session, operator and orientation.

v0.1 requires at least three replicates spanning at least two sessions, operators and orientations. It computes a deliberately conservative error envelope:

`absolute(mean measured - certified value) + certified standard uncertainty + maximum replicate deviation + half descriptor-resolution quantum`

The envelope must not exceed the experiment owner's predeclared maximum absolute error.

This envelope is a bounded engineering acceptance check. It is **not** a complete GUM uncertainty calculation and it is not a substitute for a calibrated reference certificate.

### Sampling and quantile adequacy

Each sample set binds to:
- exact parent morphology case, parent interval, PSD bin, descriptor basis and definition version;
- randomized selection within the parent size stratum;
- one particle per independently declared acquisition block;
- descriptor domain, resolution quantum and minimum resolvable feature;
- predeclared quantile probabilities, confidence levels and maximum interval widths.

Left/right censored observations remain censored. They are not converted to zero, the resolution floor, or an exact value.

For n independent observations and population quantile p, let B be the number of observations at or below the true p-quantile. Then B follows Binomial(n,p). For order statistics X_(r) and X_(s), the distribution-free interval coverage is:

`P(r <= B <= s-1)`

v0.1 enumerates admissible rank pairs, selects the narrowest observed interval whose exact binomial coverage reaches the target confidence, and compares the interval width with the declared maximum. Domain limits are only used where explicitly supplied and physically justified.

All p05/p50/p95 targets must pass for the sample set to pass. A large sample can still fail if the observed interval is wider than the required precision.

### Independence caveat

The exact binomial guarantee is conditional on the independent random-sampling premise. v0.1 rejects repeated acquisition-block identifiers and requires one particle per declared independent block. It does not correct:
- hidden clustering across frames, trays, batches or source aliquots;
- sampling selection bias or incomplete stream capture;
- nonstationary geology/process conditions;
- finite-population sampling effects;
- censored data or left/right truncation;
- measurement error in the descriptor values themselves.

A declared unique block ID does not prove physical independence. The sampling protocol still requires independent review.

Cluster-aware bootstrap, finite-population corrections, sequential sample-size planning and Last-Exit-like stability rules are future work.

## 4. Synthetic golden fixture

The bundled fixture contains:
- one synthetic 2D dynamic-image instrument;
- one synthetic circularity reference artifact;
- six repeat scans across three sessions, three operators and six orientations;
- 48 fabricated particle observations in one parent PSD bin;
- p05, p50 and p95 targets at 90% confidence.

Independent arithmetic replay of the fixture yields a conservative reference error envelope of 0.0125 against a declared 0.04 tolerance. The three quantile intervals meet their deliberately broad synthetic precision limits. The result remains `SYNTHETIC_ONLY_NOT_PHYSICAL_QUALIFICATION`.

Tightening the p05 interval-width limit to 0.01 causes a sample-adequacy failure at the same sample count. That is the intended behaviour: adequacy depends on the desired precision and observed spread, not a universal fixed N.

## 5. Benchmark tournament and acceptance

- B0: fixed particle count and sample mean only.
- B1: repeated scan / reference artifact checks.
- B2: resolution-aware descriptor-domain checks.
- B3: exact order-statistic p05/p50/p95 interval adequacy.
- B4: cluster-aware resampling for realistic dynamic-image batches (future).
- B5: sequential sample-size planning with pilot variability and resolution limits (future).
- B6: Monte Carlo propagation of nonlinear image/geometry measurement models with declared correlations and calibration distributions (future).

Software acceptance:
- 100% rejection of planted duplicate particle/block, wrong parent bin, invalid descriptor definition, under-resolution and censor-as-zero controls.
- Exact rank-coverage calculations agree with independently computed golden vectors.
- Insufficient precision is reported as failure, not hidden by more decimal places.
- Repeatability/reference bias and quantile sampling precision remain separate reported dimensions.

Physical acceptance requires a measurement-system study, certified references, blinded replicate scans, independent sample preparation and a predeclared uncertainty budget. No universal tolerance is imposed by this crate.

## 6. Integration

- #3910 owns morphology descriptor distributions and parent PSD identity.
- #3927 owns stage-resolved material stream mass/PSD evidence.
- #3934 owns the instrumented hard-rock pilot plan.
- #3917 can use qualified costs/utilities to prioritize additional samples or reference scans.
- #3961 owns reference-material and sampling adequacy checks only.

## 7. Non-claims

This is not ISO certification, instrument certification, a complete JCGM GUM calculation, a cluster-aware sample-design engine, proof of physical IID sampling, a field measurement result or mine performance qualification. A PASS is limited to the declared reference artifacts, descriptor definition, instrument, resolution range and sampling premise.
