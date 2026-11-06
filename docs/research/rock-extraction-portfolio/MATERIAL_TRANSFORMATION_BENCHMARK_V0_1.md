# Material transformation benchmark v0.1

**Status:** stage-resolved measurement contract and conservative flow-ledger evaluator. Synthetic fixtures only. No mine attrition rate, transport response or productivity improvement has been measured.

## 1. Why this benchmark exists

The cutter PSD is not necessarily the PSD entering the capture system, arriving at the cyclone, leaving the filter or reaching accepted discharge. Size distributions can change through selective capture, segregation, deposition, particle breakage/attrition, moisture changes, sample bias and time misalignment.

Relevant research:

- Zhou et al. (2026), *Particuology* 114, 270–282, DOI 10.1016/j.partic.2026.04.017, model non-spherical coarse coal breakage in a single pneumatic bend using CFD–DEM and a Tavares breakage model. Sphericity affected impact energy and breakage behaviour; the study explored oscillatory flow to reduce collisions/power loss. This is coal and a modeled single bend, not mine-scale hard-rock transport. https://doi.org/10.1016/j.partic.2026.04.017
- A 2025 *Results in Engineering* particle-driven CSP conveyance-loop study, DOI 10.1016/j.rineng.2025.108532, reports 1,000-hour olivine conveying tests with attrition and erosion measurements, including d10/d50/d90 and fines changes under different solids/air ratios. It demonstrates why distribution changes need measurement over the circuit; its material and plant context are not hard-rock excavation. https://doi.org/10.1016/j.rineng.2025.108532
- Bradley's Wolfson Centre review describes the coupled mechanisms of particle attrition and pipeline erosion, with strong material dependence and sensitivity to impacts, particle shape/size and pipe geometry. https://shapa.co.uk/technical-advice/attrition-and-wear-in-pneumatic-conveying/
- Raju & Esterhuyse (2026), *Mining, Metallurgy & Exploration*, report a seven-month South African underground hard-rock undercutting/vacuum trial where removal, hose friction, distance, density and residual fines constrained sustained cutting. https://doi.org/10.1007/s42461-026-01722-w

These studies motivate the measurements; none directly validates an attrition or fragmentation model for the intended mine.

## 2. Paradigm shift

Replace a single cutter PSD and a single end-of-line mass balance with a **stage-resolved material-stream ledger**.

At each physical boundary, measure:
- dry-solid mass-flow interval;
- mass-flow interval by exact particle-size bin;
- station, location, installed configuration and stream identity;
- synchronized sample window and timing uncertainty;
- instrument, calibration and source data provenance.

Represent branches explicitly: floor residual, hose deposits, separator coarse product, filter fines, rejected output and accepted discharge. Do not leave missing branches as an implicit zero.

A stream can be split or merged, but its identity must be traceable through a directed acyclic topology. Each physical stream is produced and consumed at most once in the declared ledger.

## 3. Executable v0.1 implementation

The standalone crate `symtropy-material-transformation-bench` validates a child case against the exact #3876 parent case and interval.

- Exact parent case/interval/trial/site/material cohort/machine configuration/transport configuration identity.
- Exact parent PSD bin grid; final bin remains open-ended.
- Distinct physical stations and stream roles.
- Independent total dry-solid flow and per-bin dry-solid flow intervals.
- Conservative sum of per-bin intervals must overlap the independent stream-total interval.
- Stream source and calibration provenance.
- Synchronized observation clocks and time uncertainty.
- Input/output topology with split streams, no double counting and acyclicity.
- Residence-time interval checked against input/output sample-window midpoint displacement, expanded only by explicit time uncertainty.
- Transition mass-flow closure classified as WITHIN_UNCERTAINTY, DEFINITE_DEFICIT or DEFINITE_SURPLUS.
- Per-size-bin output-minus-input flow changes are reported as intervals.

No root-sum-square uncertainty aggregation is assumed. Interval bounds are conservatively added. No missing stream is zero-filled.

## 4. What a PSD shift does—and does not—mean

If the coarse fraction falls and the fine fraction rises downstream, this is an observed distribution shift. It does **not** prove particle breakage.

Competing explanations include:
- genuine particle breakage or abrasion;
- size-selective capture or separation;
- segregation/dune deposition;
- unmeasured residual or filter/dust branches;
- different rock cohorts or feed rates;
- sampling and imaging biases;
- time-window misalignment or residence-time variation.

Causal attrition attribution needs synchronized matched cohorts and independently sampled/weighted material at adjacent boundaries, while accounting for every branch. If particle-level breakage lineage or a transition matrix is claimed, it needs a separately identifiable model and sufficient physical evidence. Two histograms alone do not identify the particle transition matrix.

## 5. Benchmark comparison

- B0: cutter-discharge PSD only.
- B1: stage PSD snapshots, no stream flow closure.
- B2: stage PSD plus synchronized per-stream dry-solid flow.
- B3: B2 plus split branches (residual, deposit, separator coarse, filter fines, reject).
- B4: B3 plus size-conditioned morphology from #3910.
- B5: non-spherical CFD–DEM/attrition model, calibrated on independent data and evaluated against held-out stage transitions.
- B6: Symtropy/Symthaea hybrid model with physical, signal and material context.

Hold out full material cohorts, campaigns, installed path configurations and sites. Do not randomly split adjacent windows or particles from the same sample.

## 6. Metrics

- stream total flow closure and bin-flow reconciliation;
- mass deficit/surplus intervals by physical transition;
- per-bin mass-flow changes and interval classification;
- stage-wise fine fraction and distribution changes, with declared weighting basis;
- d10/d50/d80/d90 only when sampling resolution and interpolation methodology support them;
- sampling repeatability and calibration uncertainty;
- residual/deposit/fines/reject branch accounting;
- pressure drop and flow response on held-out paths;
- erosion and attrition rates only when physically sampled, independently weighed and separated from other losses.

Do not report one pooled "fragmentation accuracy" when stage-wise errors differ.

## 7. Research hypotheses (not achieved)

- H1: stage-resolved flow and PSD identifies where material-flow loss and size-distribution changes occur, rather than attributing all loss to the cutter or blower.
- H2: morphology-conditioned transition models outperform PSD-only models on held-out paths after controlling for material cohort, density, moisture and independent friction parameters.
- H3: bend/path-specific sampling shows where attrition and wear are coupled for the intended rock cohort.
- H4: full stream accounting reduces unexplained dry-solid mass residual without hiding uncertainty or inventing unmeasured branches.

No fixed percent improvement target is appropriate before pilot variance and sampling repeatability are measured. Preregister effect size and sample count for each physical cohort and path configuration.

## 8. Adversarial controls

- nested or overlapping PSD bins;
- gaps, open-ended-bin errors and parent-boundary mismatch;
- total stream flow inconsistent with binned flow;
- duplicated/unknown stream and station identifiers;
- stream role bound to wrong physical station;
- non-independent or duplicated raw sample source;
- stream mass counted twice at a split/merge;
- cyclic stream topology;
- input and output windows outside the declared residence-time interval;
- clock mismatch and unbounded timing uncertainty;
- branch omission masking a deficit;
- one material cohort compared against another;
- particle number fractions treated as mass fractions;
- model PSD output promoted to physical truth;
- PSD change automatically labelled breakage/attrition.

100% rejection of planted structural violations is a contract gate, not physical-performance qualification.

## 9. Integration and ownership

- #3864/#3876 own parent PSD, cutter generation, capacities and baseline mass balance.
- #3891/#3910 own size-conditioned morphology evidence.
- #3877/#3913 own synchronized event-level pneumatic sensor evidence.
- #3888/#3890 own spatial residual/support-footprint geometry.
- #3812/#3814 own accepted output and full-cycle metrology.
- #3825 owns energy/resource accounting.
- #3853 owns strategy eligibility and Pareto comparisons.
- #3925 owns stage-resolved stream topology, PSD flow and conservative transition closure.

## 10. Non-claims

This is a read-only evidence contract. It does not simulate attrition, infer particle-level trajectories, set airflow/pressure/velocity, qualify a separator or hose, predict mine-scale breakage, or authorize machine operation. Synthetic fixtures are conformance data only. Physical claims require calibrated sample systems, synchronized flow metrology, independent weighing, residence-time characterization and prospective validation.
