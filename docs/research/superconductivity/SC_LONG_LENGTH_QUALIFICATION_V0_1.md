# SC-LONG-LENGTH-001 — Long-length REBCO conductor spatial qualification (v0.1)

**Status:** research contract / schema proposal  
**Parent:** Symtropy #6522  
**Issue:** https://github.com/Luminous-Dynamics/symtropy/issues/6550  
**Evidence boundary:** initial fixtures are synthetic; no commercial tape data or manufacturing capability is asserted.

## Why this layer exists

A reel-average critical current can conceal a weak segment, clustered defects, edge degradation, process drift, or a long unmeasured interval. Coupon measurements and a single short-sample result cannot establish that a long production reel meets a continuous-length application requirement.

The 2026 roadmap by Noe et al. reports, within its stated commercial scope, typical REBCO engineering-current-density ranges of 30–60 kA/cm² at 77 K and 150–300 kA/cm² at 20 K, a typical stated cost range of €80–100 per kA·m at 77 K self-field, and a need for improved throughput, yield, longer reel-to-reel runs and in-situ characterization. It describes approximately 1 km as a present upper-run scale in the discussion of future length increases—not as a universal limit for every manufacturer. Source: [Noe et al., Superconductor Science and Technology 39 (2026) 023501](https://doi.org/10.1088/1361-6668/ae1f20).

Those published values are context, not a supplier guarantee, a current market census, or an acceptance profile. Every comparison must pin its source version and reproduce its actual temperature, field, field angle, geometry, current criterion and cost boundary.

## Industry comparison anchors (source-pinned; not universal thresholds)

The comparison must be product- and condition-specific. Current public manufacturer specifications illustrate why generic averages are insufficient:

- SuperPower's 2G HTS specification lists widths 2–12 mm, standard piece lengths of 200–500 m with some specifications above 900 m, and 4 mm self-field critical-current ranges of 120–160 A for AP tape and 80–120 A for HM tape at 77 K using a 1 μV/cm transport criterion. Its page also reports example axial tensile strength of 550 MPa for a specified substrate/stabilizer configuration and a soldered lap-joint resistivity near 50 nΩ·cm² at 77 K. These values are configuration-dependent manufacturer specifications, not a universal product family pass threshold: https://superpower-inc.com/specification.aspx
- Fujikura's product table lists FESC-SCH04 (4 mm width, 0.11 mm total thickness, 50 μm substrate, 20 μm stabilizing layer, artificial pins) at 130 A at 77 K self-field and 640 A at 20 K / 5 T. Fujikura explicitly labels the 20 K / 5 T value a reference value that does not guarantee actual performance: https://www.fujikura.co.jp/en/products/superconductors/superconductingwire-reel/superconductingwire-reel.html
- A 2026 translation-gap review reports estimated routine piece lengths / yields of >20 km / >98% for NbTi, >10 km / >95% for Nb₃Sn, and 0.3–2 km / 80–90% for REBCO. Treat these as the review's estimates under its own definitions, not a universal audited census. The manufacturer-specific SuperPower values above do not have exactly the same product / length definition, reinforcing the need to pin each benchmark record: https://www.sciencedirect.com/science/article/pii/S2949822826015558

A useful independent product-performance cohort is the 2026 REBCO anisotropy study of three 4 mm tapes from SST, SuperPower and Faraday Factory Japan / SuperOx. It reports sample-specific Ic at 77 K self-field and at 20 K / 19 T with the field-angle convention explicitly stated. It is especially relevant for building an orientation-aware comparison cohort, but the samples were received in 2023 and the measured values must not be represented as current vendor-wide specifications: https://doi.org/10.1088/1361-6668/ae940e

The initial benchmark should preserve model, architecture, sample, source batch, substrate, REBCO thickness, stabilizer, field orientation, criterion and test lab. It must not average or rank across mismatched conditions.

## Condition-profile-specific spatial coverage

The follow-on correction in [SC-IC-CONDITION-PROFILES-001 #6601](https://github.com/Luminous-Dynamics/symtropy/issues/6601) partitions every Ic coverage union by a deterministic condition-profile identity. The identity is computed from the exact operating-condition fields and measurement unit, including temperature, field magnitude/vector/orientation, current criterion, test current, electric-field criterion, strain and frequency where present. Geometry is retained on each source observation but is not part of the operating-condition identity because transport and non-contact proxy methods can have different measurement geometries while targeting the same operating condition.

The auditor requires at least temperature, field magnitude or vector, field orientation or vector, current criterion, geometry and unit for any measured/inferred Ic observation that contributes to a coverage profile. Direct transport, calibrated proxy, inferred, calibration-unqualified and proxy-unqualified intervals are unioned separately *within each condition profile*. The report no longer emits one reel-wide Ic coverage fraction across unlike conditions.

This matters because 77 K self-field Ic and 20 K / 5 T Ic are different engineering operating points. Their spatial footprints cannot be combined into one qualified coverage number. Explicitly unmeasured length remains a separate unscoped spatial quantity when no test-condition tuple is declared.

The synthetic negative controls duplicate the same physical footprint at 77 K and 20 K and vary the current criterion. They require separate profile IDs and separate interval unions. This validates condition partitioning only; it does not interpolate Ic between temperatures or fields, infer an application envelope, or authorize product acceptance.

## Industry observability comparator: direct transport vs calibrated proxy

SuperPower's public process description states that it measures transport critical current at 3 m intervals along conductor and also uses a calibrated non-contact magnetization system ("TapeStar") to produce a lengthwise Ic trace. The same page illustrates a transport measurement on approximately 630 m of 4 mm tape and a TapeStar trace of that same length. This is unusually useful because it documents a real industrial metrology architecture, rather than merely a product headline: https://www.superpower-inc.com/Technology.aspx

SuperPower states that direct transport Ic is measured at 3 m intervals. Preserve that as a sampling-spacing fact only: it does not prove the unmeasured space between sample locations was directly tested. In every record, the physical measurement footprint and sample-center spacing are separate fields; a gap between footprints remains unmeasured unless a qualifying scan actually covered it. The TapeStar trace also requires its actual spatial footprint, resolution, calibration revision and registration uncertainty to be recorded, rather than assuming the entire reel was observed because a plot is drawn across its length.

The v0.1 auditor therefore separates:
- `direct_transport`: a transport-current measurement used for the direct-Ic spatial coverage metric;
- `noncontact_magnetization_proxy`: a calibrated indirect Ic estimate, reported separately and never counted as direct transport coverage;
- `structural_inspection`: e.g. defect / surface inspection, not an Ic measurement;
- `model_inference` / `other`: non-direct evidence that must not be silently promoted to direct Ic.

The benchmark should compare direct transport sampling and calibrated proxy traces on common reel coordinates, using the direct measurements as matched validation anchors. Required outputs include proxy bias and uncertainty versus direct transport, false-negative probability for local low-Ic segments, spatial resolution / blind-zone effects, registration error, drift across calibration revisions, and inspection cost per accepted kA·m. A denser proxy map may improve coverage, but it is not more authoritative merely because it is spatially dense.

SuperPower's published specification lists 200–500 m pieces with some above 900 m depending on specification; its 77 K self-field values are stated by width and tape type, with a 1 μV/cm transport criterion. Fujikura's FESC-SCH04 page lists 130 A at 77 K self-field and 640 A at 20 K / 5 T for its specified 4 mm artificial-pin tape, while explicitly warning that the 20 K / 5 T figure is a reference value and not a performance guarantee. These are manufacturer specifications and product-specific comparators, not matched cross-vendor measurements or universal acceptance thresholds:
- https://superpower-inc.com/specification.aspx
- https://www.fujikura.co.jp/en/products/superconductors/superconductingwire-reel/superconductingwire-reel.html

A separate 2026 IEEE Transactions on Applied Superconductivity study compares electrical, thermal and mechanical performance across four industrial REBCO manufacturers and compares two production-year batches from one manufacturer. It supports a second benchmark principle: compare actual measured product cohorts and batch changes under common test methods, not only manufacturer specification sheets. DOI: https://doi.org/10.1109/TASC.2026.3672426

## Paradigm shift

Replace:

`reel -> average Ic -> pass/fail`

with:

`reel genealogy + spatial coordinates + calibrated measurements + process history + application profile -> coverage-aware tail-risk and accepted-product qualification`

The object of qualification is a **spatially located conductor segment**, not a nominal formula or average reel. The same spatial identity must be retained as material is cut, split, spliced, joined, cabled and incorporated into a coil.

## Contract and coordinate rules

The companion JSON Schema is `docs/research/schemas/rebco_long_length_segment_v0_1.schema.json`.

Every observation binds:
- parent reel, segment, lot, campaign and process-run identities;
- measurement basis: direct transport, non-contact magnetization proxy, structural inspection, model inference or other declared method;
- a declared reel datum, positive longitudinal direction, width datum and face / side;
- a bounded longitudinal and widthwise measurement footprint;
- property, value / interval, unit, evidence state and measurement method;
- temperature, magnetic-field magnitude and orientation, current criterion and geometry when relevant;
- instrument, calibration revision, timestamp and measurement uncertainty;
- raw artifact SHA-256 and source rights / review state.

Coordinate transforms, reel reversal and winding direction must be explicit. A map generated by interpolation is a derived estimate, not a direct measurement. The system must preserve unsampled areas as unknown. No map renderer or downstream projection may turn absence of a reading into a pass.

### Evidence states

- `measured`: a value is directly observed under the declared method and conditions.
- `not_detected`: the method did not detect the target above a declared detection limit; this is not equivalent to zero.
- `unmeasured`: no valid observation exists for this region / property.
- `inferred`: a model-derived estimate with model identity and uncertainty.
- `disputed`: the source or result is contested and cannot silently satisfy a hard gate.
- `not_applicable`: the property is outside this record's declared scope.

Evidence state, measurement authority and product acceptance are separate dimensions. A valid hash preserves bytes; it does not establish scientific truth, calibration, physical sample identity, data rights or product conformity.

## Spatial evaluation

Report direct transport coverage and calibrated proxy coverage separately. Report all statistics with the sampling design, spatial resolution and dependence assumptions:
- observed coverage by length and width;
- length-weighted mean, median, p01, p05, p10 and minimum;
- contiguous under-specification length and number of failing runs;
- spatial autocorrelation / correlation length when estimable;
- fraction of length meeting a frozen product requirement;
- accepted cut lengths under declared minimum-length, splice and joint rules;
- uncertainty and unobserved coverage separately.

Adjacent readings are correlated and cannot be treated as independent replicates. If the measurement design does not support a tail estimate, return insufficient evidence rather than manufacturing a precise lower percentile. A direct-measurement coverage interval contributes to qualified coverage only when the measurement system explicitly records `calibration_valid_at_measurement=true`; missing or false calibration status remains observed-but-unqualified.

## Application-conditioned product acceptance

A segment only satisfies an application profile when the profile's conditions are met. At minimum, a critical-current comparison binds:
- temperature;
- field vector / magnitude / orientation;
- current criterion (for example, a stated electric-field criterion);
- conductor width, thickness and full cross-section convention;
- strain / mechanical state where relevant;
- test method, uncertainty and sample / segment identity.

Keep superconducting-layer `Jc` separate from full-conductor engineering `Je`. Do not transfer 77 K self-field values into a 20 K high-field use case. Do not use a bulk specimen's value as a tape or cable value.

## Yield-adjusted industrial metric

For an explicitly frozen application profile:

`accepted_kA_m = sum(usable_current_kA(segment, profile) * accepted_length_m(segment))`

`cost_per_accepted_kA_m = total_declared_manufacturing_and_qualification_cost / accepted_kA_m`

The cost boundary must state whether it includes substrate, precursor, energy, deposition, stabilizer, inspection, scrap, rework, joints / splices, capital depreciation, transport and cryogenic / device operating costs. Do not compare a 77 K self-field conductor quote with a high-field system lifecycle cost.

Report accepted kA·m, qualified length, yield, scrap, rework, joint count and cost components separately. Never compress an unqualified / unknown cost or performance term into an optimistic scalar.

## Leakage-resistant benchmark

Hold out complete reels for reel-level generalization. Hold out parent lots, manufacturing runs, campaigns and time periods in separate evaluations. A manufacturing-line or facility transfer is a separate higher-bar task.

For spatial interpolation, freeze withheld contiguous blocks and guard bands; nearby points must not leak across partitions. Any preprocessing, feature selection, calibration and threshold selection must be fit inside the training partition.

Compare at equal input / compute / inspection budgets:
1. reel mean / lot mean;
2. linear spatial trend and local-window interpolation;
3. spatial-statistics / Gaussian-process baseline;
4. process-only tabular model;
5. process + microstructure + spatial model;
6. uncertainty-aware adaptive inspection / abstention policy;
7. exact synthetic oracle.

Score:
- error and interval coverage for Ic / Jc / Je maps;
- defect detection precision, recall and false-negative rate;
- observed coverage and uncertainty in unmeasured regions;
- lower-tail calibration and contiguous failing-run prediction;
- accepted kA·m and saleable-length yield under the same frozen profile;
- performance at fixed inspection budget;
- cost per accepted kA·m with identical boundaries.

## Required negative controls

The benchmark must catch:
- a single weak segment hidden by a good reel mean;
- equal defect counts with different clustering / maximum contiguous length;
- unmeasured intervals silently marked Pass;
- centreline-only inspection missing width-edge defects;
- calibration drift appearing as a process improvement;
- coordinate offset or reversed reel direction;
- adjacent train/test samples from the same reel;
- an inline proxy passing when the later cryogenic test fails;
- a segment passing at 77 K self-field but failing its declared application field / temperature;
- rework / split / splice genealogy that drops or double-counts parent length.

## Initial acceptance gates

Synthetic contract gates:
- invalid coordinate frames, identities, bounds and reel-parent joins are rejected;
- `unmeasured`, `not_detected`, `measured`, `inferred` and `disputed` remain distinct;
- all constructed reel / lot / campaign leakage is detected;
- missing coverage cannot satisfy a required acceptance region;
- replay of identical inputs is deterministic and digest-bound.

Physical / model targets must be preregistered only after a rights-cleared baseline cohort and measurement-system capability study exist. Then compare prospective accepted kA·m and cost per accepted kA·m against matched incumbents.

## Ownership

- Symtropy #6522 owns the general REBCO process-to-microstructure-to-conductor twin.
- This issue (#6550) owns spatially registered long-length evidence, tail-risk and yield-adjusted qualification.
- Symthaea #11736 owns conductor-to-coil / cable physics and protection.
- Symthaea #12651 owns translation-gap and application readiness.
- Symthaea #12671 owns protocol / execution / realized phase semantics.
- Sol-Atlas #975 consumes source-backed facility and industrial capability references.
- Mycelix #6426 owns the scientific-event interoperability profile.

## Claim ceiling

A passing schema or synthetic auditor establishes only the declared data and evaluation semantics. It does not establish commercial REBCO performance, real production yield, industrial superiority, sensor adequacy, supplier capacity, process capability, manufacturing authorization or a new superconducting material.
