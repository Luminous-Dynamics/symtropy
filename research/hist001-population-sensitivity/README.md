# HIST-001 population-estimate sensitivity

A small, dependency-free Rust calculator for the **published derived population estimates** in Table 2 of:

Huffman, T.N. & Woodborne, S. (2021). *New AMS dates for the Middle Iron Age in the Mapungubwe landscape*. South African Journal of Science, 117(3/4), Article 8980. https://doi.org/10.17159/sajs.2021/8980

This is not a demographic reconstruction, census, population simulator, or corrected historical estimate. It tests the sensitivity of the authors' printed Table 2 calculations to two explicit assumptions:

- persons assigned per homestead;
- active duration of a homestead.

## Base calculation

For ordinary phase rows, the paper's stated rule is represented as:

`general_population = homesteads × persons_per_homestead ÷ (phase_years / homestead_use_years)`

For the K2 row, the paper separately treats the first 25 years as an initial smaller-capital period. The reference implementation preserves that temporal split:

- initial interval general population = total homestead-persons × 25 / 150;
- later interval general population = remaining homestead-persons ÷ (125 / homestead_use_years).

At the published assumptions of 50 persons per homestead and 50 years of homestead use, the calculator should reproduce the published general-population values to within one person (minor fractional results are retained rather than silently rounded).

Capital populations are kept as separate fixed values from the printed table; they are not recalculated by this utility.

## Run

```sh
cargo test --manifest-path research/hist001-population-sensitivity/Cargo.toml
cargo run --manifest-path research/hist001-population-sensitivity/Cargo.toml
```

The binary emits CSV for a small predeclared sweep:
- persons per homestead: 40, 50, 60;
- homestead use duration: 40, 50, 60 years.

## Interpretation boundaries

- Table 2 values are author-derived estimates, not observed population counts.
- The source's assumed 50 people and 50-year duration are not measured as universal constants for each settlement.
- The K2 25/125-year split is preserved as the authors' published partition.
- Capital population values are not varied; therefore total-population sensitivity is conditional on holding that capital estimate fixed.
- The calculation does not model survey intensity, site preservation, missing settlements, overlapping occupation, household size variation, migration, or uncertainty in phase boundaries.
- The result is a sensitivity analysis of the published estimator, not evidence that one alternative scenario is historically more accurate.

The data and source claims remain in the CivBench HIST-001 source register and Table 2 transcription. Exact source snapshot hashes remain unavailable; second-person transcription review is still required.
