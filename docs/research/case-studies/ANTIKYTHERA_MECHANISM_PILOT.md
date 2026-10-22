# Pilot case study: Antikythera Mechanism reconstruction dispute

**Status:** Research seed; not an adjudicated historical reconstruction  
**Purpose:** Stress-test evidence links, uncertainty, chronology, competing reconstructions, and rights metadata  
**Data fixture:** `crates/domains/humanity-history-core/fixtures/antikythera-research-seed.json`

## Why this is a useful engineering pilot

This is selected for its epistemic stress-test value, not as the first or most important human advance. The case crosses mathematics, astronomical cycles, bronze gearing, inscription interpretation, and modern imaging. The evidence is fragmentary, and published accounts contain meaningfully different interpretations of the front calendar ring.

The seed records nine claims, seven source records (including the input-data DOI), two explicit challenge relations, 30 quantitative estimates, four point-vs-interval probes, thirteen paired posterior sensitivity deltas, one comparison group with explicit assessment states, and one planned (not executed) reproduction run. No source paper or media files are bundled. The actual Harvard Dataverse measurements have not yet been downloaded or hashed. It includes:
- A 365-hole movable calendar-ring interpretation in a 2008 reconstruction;
- A 354-day lunar-calendar proposal from later hole-spacing analysis;
- A 2024 study's uncertainty-qualified hole-count estimates;
- A 2025 paper's candidate kinematic designs under a 354-division assumption;
- The published use of Babylonian arithmetic-progression cycles in eclipse prediction.

It deliberately **does not** infer a complete direct cultural-transmission route merely because Babylonian cycles appear in a Greek mechanism. The connection between knowledge traditions deserves its own evidence, actors, places, translation or travel records, and uncertainty. A timeline ordering alone is not a transmission claim.

## Research sources and how they are used

1. **Thoeni, Budiselic, and Ramsey (2019), Harvard Dataverse:** [Replication Data for the 2020 lunar-calendar paper](https://dataverse.harvard.edu/dataset.xhtml?persistentId=doi:10.7910/DVN/VJGLVS). The 2024 ring-count analysis states it is based entirely on these measured hole-position data. This pilot records the DOI as the intended raw-data source, but its file inventory, exact license, download, and content digests are still unverified; no raw files are redistributed.\n2. **Freeth et al. (2006), Nature:** [Decoding the ancient Greek astronomical calculator](https://doi.org/10.1038/nature05357). The abstract dates the device toward the end of the second century BCE and describes eclipse predictions based on Babylonian arithmetic-progression cycles. It also acknowledges fragmentary evidence and controversial functions.
3. **Freeth et al. (2008), Nature:** [Calendars with Olympiad display and eclipse prediction](https://doi.org/10.1038/nature07130). Its supplementary description presents a 365-hole movable calendar scale.
4. **Freeth et al. (2021), Scientific Reports:** [A Model of the Cosmos](https://doi.org/10.1038/s41598-021-84310-w). This paper proposes a specific epicyclic front mechanism consistent with its reading of the surviving inscriptions and spatial constraints. UCL Discovery identifies the article as CC BY 4.0; third-party figures still need their own credit-line review.
5. **Budiselic et al. (2020/2021), British Horological Institute:** [Evidence of a Lunar Calendar](https://bhi.co.uk/wp-content/uploads/2020/12/BHI-Antikythera-Mechanism-Evidence-of-a-Lunar-Calendar.pdf). This analysis challenges the assumed 365-day ring and proposes 354 days using measurements of surviving holes.
6. **Woan and Bayley (2024), Horological Journal:** [An improved calendar ring hole-count](https://eprints.gla.ac.uk/329337/). Its abstract reports a 68% credible estimate of 355.24 holes (+1.39/-1.36) from all measured points, and 354.08 (+1.47/-1.41) after excluding holes adjacent to fractures. The authors state that 365 holes are not plausible under their model assumptions.
7. **Lin (2025), Results in Engineering:** [New reconstruction designs](https://doi.org/10.1016/j.rineng.2025.108632). The abstract describes three candidate date-subsystem designs under a 354-division lunar-ring interpretation. The exact reuse license has not been normalized in this seed; only citation metadata is stored.

Bibliographic links are not equivalent to bundling the paper or its imagery. The fixture's rights records preserve this distinction: the 2021 article has a verified CC BY 4.0 record; unknown rights remain unknown; no paper or media files are redistributed.

## Structured numerical estimates and explicit model alternatives

The fixture now has four structured `QuantitativeEstimate` records:

| Estimate | Value | Uncertainty / interpretation |
|---|---:|---|
| `antikythera:estimate-ring-365` | 365 holes | Parameter of the cited 2008 reconstruction; not a direct count of the full surviving ring |
| `antikythera:estimate-ring-354-proposal` | 354 holes | Parameter implied by the 2020 lunar-calendar proposal; not a direct count |
| `antikythera:estimate-woan-bayley-all-points` | 355.24 holes | 68% credible interval [353.88, 356.63], all measured data |
| `antikythera:estimate-woan-bayley-exclude-fractures` | 354.08 holes | 68% credible interval [352.67, 355.55], excluding holes adjacent to fractures |

The last two are sensitivity estimates from related measurements and a shared source, not independent replications. Their assumptions are recorded separately. Values/interval endpoints remain decimal strings so serialization does not silently round the source-reported precision.

The seed defines three separate reconstruction models and one model-comparison group. Its explicit decision rule is to report axes separately instead of synthesizing a winner. Initial axes include fit to measured hole spacing, sensitivity to fracture handling, calendar-interpretation consistency, mechanical feasibility, dependence on missing geometry, and evidence provenance.

The seed now records all 26 source-reported discrete posterior probability values from Table 2 of Woan and Bayley (2024), covering N=349–361 under full and trimmed data:

| Integer hole count | Full data | Trimmed data | Delta (trimmed − full) |
|---:|---:|---:|---:|
| 349 | 0.0000 | 0.0006 | +0.0006 |
| 350 | 0.0002 | 0.0055 | +0.0053 |
| 351 | 0.0027 | 0.0293 | +0.0266 |
| 352 | 0.0187 | 0.0978 | +0.0791 |
| 353 | 0.0776 | 0.2056 | +0.1280 |
| 354 | 0.1917 | 0.2714 | +0.0797 |
| 355 | 0.2823 | 0.2251 | −0.0572 |
| 356 | 0.2479 | 0.1173 | −0.1306 |
| 357 | 0.1298 | 0.0384 | −0.0914 |
| 358 | 0.0405 | 0.0079 | −0.0326 |
| 359 | 0.0075 | 0.0010 | −0.0065 |
| 360 | 0.0008 | 0.0001 | −0.0007 |
| 361 | 0.0001 | 0.0000 | −0.0001 |

The full-data column sums to 0.9998 at displayed precision, while the trimmed column sums to 1.0000. The 0.0002 full-column difference is consistent with rounding each displayed probability to four decimals; the source values are preserved without renormalization. The paired deltas subtract source values rounded to four decimal places; they are not newly computed posterior probabilities.

The paper describes its trimmed set as excluding short segments S5/S6 and the first and last holes of S1/S2/S3/S7 (holes 2, 23, 24, 32, 33, 69, 76, and 81). These are sensitivity variants of related measurements, not independent replications.

The seed additionally declares four point-vs-interval probes. The analysis calculates only the candidate value's location relative to the reported interval and the arithmetic distance to the nearest bound:
- 365 is 8.37 holes above the all-measurement interval [353.88, 356.63].
- 365 is 9.45 holes above the fracture-excluded interval [352.67, 355.55].
- 354 is within both intervals.

Those distances are deterministic arithmetic, not statistical model scores. In particular, an interval's stated 68% coverage must not be converted into a "32% rejection probability", and related sensitivity estimates must not be treated as independent replicates.

The comparison contains four explicit model/axis assessments:
- **Source-reported:** the 2024 authors' conclusion that 365 holes are not plausible under their assumptions, plus their two sensitivity estimates. This is attributed to the paper, not independently re-fit here.
- **Source-reported:** the 2025 authors' statement that three candidate designs are mechanically feasible; the candidates have not been locally recreated.
- **Not evaluated:** common mechanical feasibility for the 365-hole candidate. This is explicitly recorded as unassessed, not treated as evidence against it.

Every assessment must name a declared comparison axis and a model in the comparison group. Future `independently_reproduced` results require a versioned validation artifact; a citation alone is insufficient.

This is only a typed literature comparison. We have not locally recreated any gear train, refit the published statistical model, or verified any paper's numerical output.

## Next stage: independently reproduce the posterior

The [posterior replication protocol](ANTIKYTHERA_POSTERIOR_REPLICATION_PROTOCOL.md) specifies the raw-data manifest, 22-parameter source model, Rust-first implementation lane, synthetic recovery tests, source-result tolerances, and convergence gates. The published result remains source-reported until that protocol is executed against the licensed, hashed measurement files.

## Validation and adversarial tests

Run from the Symtropy workspace root:

```sh
cargo test -p humanity-history-core
cargo run -p humanity-history-core --bin humanity-history-validate -- crates/domains/humanity-history-core/fixtures/antikythera-research-seed.json
cargo run -p humanity-history-core --bin humanity-history-validate -- analyze-sensitivity crates/domains/humanity-history-core/fixtures/antikythera-research-seed.json
```

The first command should run the core's unit suite; the second should emit JSON with `valid: true` and zero issues. These are **expected outputs**, not test results claimed by this research seed. The code must be compiled and run in a Rust-enabled environment before acceptance.

Required mutation controls for the case study:

1. Remove one referenced source from `sources`: validation must emit `missing_source_reference`.
2. Change the only support link on the 2024 claim to `challenges`: validation must emit `missing_evidence_for_claim`; a challenge must never satisfy a positive-support gate.
3. Reverse the construction interval's endpoints: validation must emit `invalid_time_interval`.
4. Change the 2024-to-2008 challenge edge's evidence relation from `supports` to `challenges`: validation must emit `missing_evidence_for_relation`; the edge is documented by evidence that the challenge was made, not by a citation that merely opposes it.
5. Change the schema version to an unsupported value: validation must emit `unsupported_schema_version`.
6. Add a distributable asset with `unknown` rights: validation must fail closed with `undistributable_rights`.

Interpretation-level checks, which the structural validator cannot yet automate:
- The user can open both 365- and 354-division branches from the same dispute node.
- Every model-specific quantitative result displays the assumptions and uncertainty from the relevant paper.
- The interface distinguishes a published claim, a proposed reconstruction, and the underlying physical observation.
- A simulated gear train is never presented as proof that the ancient maker used that exact train.
- No source claims that temporal precedence by itself establishes historical influence.

## Acceptance criteria for this pilot

- Structural validator returns zero issues on the unmodified seed.
- Each mutation above fails with the expected error code and no unrelated error.
- Both competing ring interpretations remain queryable; neither is silently deleted or averaged into one answer.
- Citation locators and reuse-status decisions are inspectable.
- A later visualization links gear/dial elements to claim IDs and shows each reconstruction's assumptions.
- An independent reviewer can reproduce the result from the pinned dataset version without a live model or online service.

## Coverage limitation

This pilot is not a representative sample of global human history and must not become the default template for what counts as a major advance. A subsequent case study should be chosen using published-source access, local or community review, multilingual evidence, rights constraints, and clear significance criteria—while broadening the corpus beyond Mediterranean mechanical astronomy.
