# Crossed measurement-system analysis (Gage R&R) v0.1

**Status:** balanced crossed ANOVA diagnostic with synthetic fixtures. It is not a calibration certificate, accreditation statement, product acceptance, or automatic permission to use an instrument on production.

## Why this track exists

The Symtropy → Symthaea experiment path needs candidate-specific observation noise. That uncertainty must come from a validated measurement process, not from prediction disagreement or a guessed instrument datasheet value. A repeatable, traceable Gage R&R study provides an estimate of repeatability and reproducibility for a declared measurand, instrument, operator population and study design.

This first implementation uses a balanced, crossed, random-effects two-way ANOVA with repeated measurements. It is deliberately narrow: one instrument, one measurand/unit, multiple parts, multiple operators, and the same number of repeats in every part/operator cell. It is a study diagnostic, not a complete MSA package or a direct implementation of the AIAG MSA manual.

## Run

    cargo run --locked --manifest-path research/transformer-bench/Cargo.toml --bin measurement-system-msa --       research/transformer-bench/fixtures/measurement-system-msa-low-noise-synthetic.csv       research/transformer-bench/fixtures/measurement-system-msa-spec-synthetic.csv       research/transformer-bench/fixtures/measurement-system-msa-calibration-synthetic.csv       /tmp/measurement-system-msa.csv

The study-spec file declares minimum parts, operators and repeats; it does not automatically make the sample design adequate. The observation table must contain exactly one unique sample for every part/operator/repeat cell, contiguous repeats from 1, one controlled measurand and unit, one instrument, and canonical UTC timestamps. The calibration record must be current at every measurement timestamp and match the instrument and calibration class.

## Variance model

For a balanced crossed study with p parts, o operators and r repeats per cell, the method-of-moments estimates use the mean squares from the two-way random-effects ANOVA:

    variance_repeatability = MS_error
    variance_part_operator = max((MS_interaction - MS_error)/r, 0)
    variance_operator = max((MS_operator - MS_interaction)/(p*r), 0)
    variance_part = max((MS_part - MS_interaction)/(o*r), 0)

The report separates repeatability, part×operator interaction, operator reproducibility, part-to-part variation and total Gage R&R. Negative method-of-moments variance components are truncated to zero; the report preserves the components so an engineer can see what drives the total. A production study should use a documented sampling plan appropriate to the process and the intended measurement use.

The candidate-noise output is the square root of repeatability + interaction + operator variance. It can be exported as a study-specific measurement-noise estimate only when the study design, measurement-system adequacy, intended use and approval have all been independently reviewed. An MSA result is not itself the external eligibility receipt used by Symthaea.

## Guidance metrics

The binary reports:
- %Study Variation: 100 × GRR standard deviation / total standard deviation.
- %Tolerance: 100 × 6 × GRR standard deviation / tolerance width.
- Number of distinct categories: floor(1.41 × part standard deviation / GRR standard deviation).
- Repeatability, interaction, operator, reproducibility and part-to-part standard deviations separately.

The labels below 10%, 10–30%, above 30% and NDC at least 5 are AIAG-oriented guidance summaries, not universal legal limits, automatic release verdicts, or proof that a measurement system is fit for every purpose. The applicable purchaser/quality plan and intended decision determine acceptance. The report deliberately separates %Study Variation, %Tolerance and NDC instead of compressing them into one pass/fail score.

## Frozen synthetic controls

- measurement-system-msa-low-noise-synthetic.csv: 10 parts × 3 operators × 3 repeats, low measurement variation relative to part-to-part variation.
- measurement-system-msa-high-grr-synthetic.csv: same design with larger repeatability and operator variation; challenges the high-GRR band and low NDC.
- measurement-system-msa-incomplete-negative.csv: one missing part/operator cell; analysis must reject the design.
- measurement-system-msa-observations-expired-negative.csv with measurement-system-msa-calibration-expired-negative.csv: expired calibration; report must block, not emit a candidate-noise estimate.

All data are synthetic, not real factory measurements or proof of actual instrument performance.

## Important exclusions / next work

This v0.1 does not yet include crossed-study confidence intervals, nested designs, non-normal measurement errors, destructive testing, multiple instruments, multiple characteristics per study, stability-over-time studies, bias/linearity studies, uncertainty budgets traceable to calibration certificates, operator population random-effects diagnostics, or Gage R&R acceptance decision by a quality authority. It also syntax-checks certificate SHA-256 rather than authenticating bytes or verifying signatures. Real deployment requires independent artifact receipts and validated calibration evidence.

Next benchmarks:
1. Compare each variance component against hand-calculated and independently pinned statistical-package references.
2. Simulate fixed-seed variance-component grids (repeatability/operator/interaction/part variance) and measure recovery bias and confidence-interval coverage once interval estimates are implemented.
3. Verify classifications on frozen low-GRR, high-repeatability, high-operator-shift, interaction, low-NDC, unbalanced and expired-calibration challenges.
4. Use multiple real, independent studies across machines/operators/lots to assess external validity; do not split repeated observations from one study across train/test.
5. Test whether resulting observation-noise inputs improve Symthaea acquisition under equal-cost and equal-query budgets without increasing abstention failures, false-safe outcomes or invalid actions.

## Industry/research references

- Minitab's summary of AIAG-oriented Gage R&R acceptance guidance: https://support.minitab.com/en-us/minitab/help-and-how-to/quality-and-process-improvement/measurement-system-analysis/supporting-topics/gage-r-r-and-wheeler-s-emp-studies/is-my-measurement-system-acceptable/
- JCGM 106:2012, measurement uncertainty in conformity assessment: https://www.bipm.org/en/doi/10.59161/jcgm106-2012
- ISO 22514-2:2026, time-dependent process capability/performance: https://www.iso.org/standard/88883.html

Minitab is a summary source for common AIAG guidance thresholds; consult the controlled MSA manual and applicable purchaser requirements for actual acceptance criteria.
