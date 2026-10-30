# AMB2025-07 TAM/SCR reference scorer

Standalone Rust reference for the NIST AM-Bench 2025 challenge CHAL-AMB2025-07-PCRTAM. It is a strict observable/metric layer, not a thermal physics solver.

## Scope and source boundary

NIST's published AMB2025-06/07 results describe the challenge outputs and report aggregate mean, median and standard deviation for:
- time above melt (TAM), defined as time above the midpoint between solidus and liquidus;
- solid cooling rate (SCR), immediately following complete solidification below solidus.

The challenge results describe an average effective emissivity of 0.642 for converting radiant temperature to emissivity-corrected temperature. The scorer assumes that any supplied TAM/SCR input values have already been calculated with the correct NIST measurement operator and calibration; it does not derive temperatures from radiance.

NIST reports that TAM values below 0.5 ms were excluded, while SCR values above 1e6 °C/s were excluded due to spatter/vapor-plume artefacts. TAM is described as fairly normally distributed; SCR is non-normal and poorly summarized by mean/std alone. Therefore this reference keeps q05/q25/q75/q95 as well as mean, median and population standard deviation. It never clamps outliers and never converts missing measurements to zero.

Official references:
- NIST result and aggregate tables: https://www.nist.gov/document/amb2025-06and-07challengeresults
- Challenge definitions: https://www.nist.gov/ambench/am-bench-2025-measurements-and-challenge-problems
- Challenge award information: https://www.nist.gov/ambench/am-bench-2025-challenge-submissions-and-awards
- NIST PDR DOI for AMB2025-06/07 calibration data: https://doi.org/10.18434/mds2-3707
- NIST PDR optical microscopy measurement-results record: https://data.nist.gov/od/id/mds2-4103

## Frozen summary targets

The built-in reference table contains NIST's four AMB2025-07 groups:
- 1 mm / 0.75 ms
- 1 mm / 5.0 ms
- 5 mm / 0.75 ms
- 5 mm / 5.0 ms

TAM is reported in ms. SCR is reported in units of 1e5 °C/s. Published N datapoints are retained with their published scaling and rounding. They are not assumed to be exact raw counts.

## Strict raw-sample CSV contract

Input header, exactly:

    sample_id,repeat_id,pad_width_mm,turnaround_ms,tam_s,scr_c_per_s

- sample_id must be unique and non-empty.
- repeat_id must be one of the three NIST run IDs for the selected turnaround setting.
- pad_width_mm must be 1 or 5.
- turnaround_ms must be 0.75 or 5.0.
- tam_s and scr_c_per_s may independently be blank for missing values.
- Quotes, inconsistent field counts and non-finite values reject.
- Each of the four pad/turnaround groups must contain all three NIST repeat IDs.

The scorer filters the two metrics independently, pools the three repeats at the sample level, and computes:
- accepted sample counts and excluded/missing counts;
- mean, median, population standard deviation;
- q05/q25/q75/q95 using a linear Type-7 quantile convention;
- absolute differences against the rounded published NIST aggregate summaries.

The official public summary does not disclose whether its standard deviation is sample or population standard deviation; population standard deviation is chosen and explicitly reported here. Exact raw-data reproduction remains unclaimed until this estimator is compared with the admitted raw NIST files.

## Build and run

    rustc --edition=2024 -D warnings --test tools/ambench-2025/amb2025_07_tam_scr.rs -o /tmp/amb07-tests
    /tmp/amb07-tests

    rustc --edition=2024 -D warnings tools/ambench-2025/amb2025_07_tam_scr.rs -o /tmp/amb07
    /tmp/amb07 --verify
    /tmp/amb07 --reference
    /tmp/amb07 --score /path/to/admitted_samples.csv

## Interpretation

A campaign with all four groups and all three repeat identities can be scored even when statistics do not match NIST. The output explicitly reports DO_NOT_MATCH rather than hiding error. It does not compress TAM and SCR into one arbitrary cross-unit number. A later raw-data extension may add empirical CDF / Wasserstein-1 / energy-distance comparison, spatial-map errors and confidence intervals, but must preserve the independent groups and metric semantics.

## Claim ceiling

This tool validates metric, filtering, pooling and summary-report semantics. It does not validate thermal physics, emissivity, pyrometry, data rights, the measurement system or the actual model. It does not claim that Symtropy matches NIST or beats a challenge winner without an admitted raw corpus and matched blind comparison.
