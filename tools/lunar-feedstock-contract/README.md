# Process-aware lunar feedstock qualification

**Status:** reference schema and synthetic validator fixture; no physical beneficiation or reactor-feed campaign has been performed.

This work treats feedstock as a traceable stream through multiple unit operations. An assay made at a magnetic/electrostatic separator outlet does not qualify the material at the reactor, printer or sintering tool inlet. Transfer can lose mass, change PSD, segregate mineral phases, fragment agglutinates, accumulate dust/fines or alter electrostatic state.

## Paradigm shift: process-qualified feedstock, not generic grade

The design contract is:

`feedstock identity + process boundary + stream lineage + measured quality + uncertainty + mass/energy closure`

The downstream process must be named before the feed specification is evaluated. A mineral distribution suitable for a hot-press route may not be suitable for microwave sintering or selective laser melting. An anorthite-rich product is not automatically a universal ISRU feedstock.

The acceptance profile can include numeric `property_constraints` with a minimum/maximum interval, unit, rationale and exact test-condition ID (temperature, atmosphere/pressure, measurement method or process-relevant state). The reactor-inlet sample must contain a matching `property_measurement`. A matching property name is not sufficient when the test condition differs. The interval must lie wholly inside the constraint; missing or incompatible conditions are unknown, and boundary overlap is indeterminate.

### 2026 source anchors (bounded claims)

1. Coffman et al., *Ground and parabolic flight testing of a magnetic and electrostatic separation system for lunar regolith beneficiation*, Acta Astronautica (2026), DOI [10.1016/j.actaastro.2026.09.032](https://doi.org/10.1016/j.actaastro.2026.09.032). The paper reports a magnetic plus electrostatic-sieve sequence and four output fractions. For one tested L2W60-g814 mare simulant, the reported plagioclase content increased from 35.5 wt% to 70.6 wt% in the 20–200 μm recovered product. This is a source-specific material/apparatus result, not a general 70.6% separator guarantee, universal recovery rate, or downstream reactor qualification.
2. Wang et al., *Multi-Criteria Evaluation of Lunar Regolith Minerals for In-Situ Thermal Manufacturing and Feedstock Beneficiation*, Engineering (2026), DOI [10.1016/j.eng.2026.09.011](https://doi.org/10.1016/j.eng.2026.09.011). The paper evaluates mineral properties for distinct manufacturing routes; the reported dominant selection criteria differ by process (thermal conductivity for hot-press sintering, dielectric loss for microwave sintering, and solar absorptivity for selective laser melting). These rankings are scoped to its characterized mineral/property set and must not be treated as universal feed specifications.
3. Nakagawa & Kioka, *Segregation in Granular Flow Under Lunar Conditions*, Fluids 11(11), 279 (2026), DOI [10.3390/fluids11110279](https://doi.org/10.3390/fluids11110279). The review highlights that size/mineral segregation and agglutinate breakage can change the material state during transfer; a separator-outlet grade is not evidence of the reactor-inlet grade unless the intermediate chain is measured.
4. ESA MoLES3 integrated robotic feedstock preparation campaign, Nature (2026), DOI [10.1038/s44453-026-00051-y](https://doi.org/10.1038/s44453-026-00051-y). It provides a useful end-to-end analogue separating excavation, transfer and beneficiation. Its measured transfer-loss and energy boundaries are specific to its test configuration, not universal lunar factors.

## Data contract

`feedstock-run.schema.json` uses Draft 2020-12 and records:

- **Acceptance profile:** named downstream process, boundary (reactor inlet, printer feed, sintering feed, etc.), phase-specific mass-fraction intervals, target particle-size range and required property artifacts.
- **Stage events:** timestamped unit operations with explicit input/output stream IDs, equipment profile, settings artifact, energy, mass closure tolerance/residual/status.
- **Material streams:** stream identity, parent streams, created-by event, measured mass and disposition (candidate product, reject, recycle, loss, storage or reactor feed).
- **Quality samples:** the exact stream and sampling point, sampling window, assay method, phase fractions, size bins, target-size fraction, numeric process-property measurements, thermal/charge/shape evidence artifacts and uncertainty budget.
- **Final acceptance:** the final stream and quality sample, evaluated specifications, required-measurement completeness, mass-closure completeness and a bounded decision.

Artifacts are content-addressed with SHA-256. Measurement status distinguishes measured, published, estimated, synthetic and unknown. Missing is not zero.

## Acceptance semantics

For a lower-bound specification (x ge L), with reported estimate (hat x) and expanded uncertainty interval ([hat x-u,hat x+u]):

- **In spec:** the entire interval is on the accepted side ((hat x-u ge L)).
- **Out of spec:** the entire interval is below the bound ((hat x+u < L)).
- **Indeterminate:** the interval overlaps the specification boundary.
- **Unknown:** a required measurement or assay is absent.

For upper-bound requirements the inequalities reverse. For two-sided specifications the entire interval must lie inside the allowed region to qualify. A central value barely above a threshold is not enough.

The same logic is applied to the *reactor-inlet* sample. Separator-outlet assays are retained for measuring quality drift, but cannot satisfy final acceptance.

## Stream and mass closure

Every unit operation declares input and output stream IDs. The validator recomputes:

[
m_{residual} = sum m_{input} - sum m_{output}
]

It combines input/output measurement uncertainty and checks the conservative residual interval against the declared stage tolerance. The upstream event must precede the descendant stream's creating event. Every output stream must identify the event that created it. Initial material can be an externally supplied source stream, but its upstream history remains explicitly unknown rather than fabricated.

A future physical campaign should additionally reconcile elemental/mineral-phase mass across all products, rejects, dust/fines, recycle and retained inventories—not merely total mass.

## Current synthetic example

`examples/synthetic-process-chain-run.json` demonstrates:

`100 kg raw stream → magnetic split → electrostatic product/reject split → transfer → 59 kg reactor-inlet stream + 1 kg transfer loss`

Two distinct assays exist:
- separator-outlet sample: 70.6 wt% plagioclase;
- reactor-inlet sample: 72 wt% ± 1 wt% plagioclase, with 95% target-size fraction in 20–200 μm.

The final decision is tied to the latter sample only. All masses, assays, settings and energies are arithmetic fixtures. They do not represent an operated separator or physical lunar material.

## Controller and beneficiation tournament

The benchmark should compare on identical characterized batches:

1. raw feed/no beneficiation;
2. magnetic-only;
3. electrostatic-only;
4. size screening only;
5. magnetic → electrostatic;
6. screening → magnetic → electrostatic;
7. process-sequenced beneficiation selected for a named downstream process;
8. each of the above with measured transfer, storage, metering and reactor-inlet losses.

Report independent metrics, not a weighted score that can hide trade-offs:
- phase-specific enrichment and recovery;
- target PSD/shape pass fraction;
- accepted in-spec kg at separator outlet and reactor inlet;
- rejected, recycled, retained, dust/fines and transfer-loss mass;
- phase/elemental closure residual and uncertainty;
- energy per accepted kg and process time;
- segregation, PSD drift, agglutinate breakage, contamination and charge change;
- property coverage for the selected downstream process;
- assay interval coverage, worst material-batch error and unknown/abstention rate.

The competitive claim to pursue is not merely higher enrichment. It is **more process-qualified feedstock delivered at the actual downstream boundary per kWh and per mission-hour**, while preserving recovery, controlling losses and making uncertainty visible.

### Process-specific property examples

The framework intentionally stores property requirements with the process and measurement conditions:
- Hot-press sintering can require thermal-conductivity measurements at the relevant temperature, pressure and porosity range.
- Microwave sintering can require dielectric loss and permittivity at the relevant frequency and temperature.
- Selective laser melting can require solar/laser-band absorptivity, emissivity, melt/thermal response and relevant particle-shape/PSD evidence.
- Hydrogen reduction or electrolysis should define mineral-phase, liberation, volatile, particle-size and contaminant limits for the actual chosen chemistry and reactor; a generic mineral ranking does not supply those thresholds.

These are examples of *which evidence dimensions may matter*, not numeric specifications. Actual acceptance bounds must come from the downstream process design and validated operating envelope. The current synthetic anorthite example has an empty property-constraint list and does not claim a hot-press, microwave, SLM or electrolysis feedstock qualification.

## Proposed research targets (not achieved)

- Reproduce the source-specific 35.5→70.6 wt% plagioclase result under the same L2W60-g814 material and test conditions within assay uncertainty.
- Improve accepted target-mineral kg per kWh by at least 15% relative to the strongest matched single-stage baseline without violating a pre-registered recovery non-inferiority bound.
- Preserve the named process acceptance interval from separator outlet through storage and metering to reactor inlet.
- Predict reactor-inlet phase fractions and PSD on entirely held-out material batches with 90–95% interval coverage.
- Reduce cumulative off-spec mass with adaptive remixing/sequencing without unacceptable increases in energy, fines, dust, contamination or agglutinate breakage.

These are falsifiable hypotheses requiring source-matched assays, calibrated instruments, independent material batches and preregistered constraints. They are not present results.

## Run the regression guard

```sh
bash scripts/check-lunar-feedstock-contract.sh
```

It validates the synthetic chain, SHA-256 artifact references, stream lineage, phase/size distributions, per-stage mass closure and uncertainty-aware reactor-inlet acceptance. Negative controls reject an outlet-assay substitution, tampered mass, uncertainty-boundary overlap mislabeled in-spec, inconsistent size fraction and altered artifact digest.

The guard is a focused semantic regression test, not a general JSON Schema engine. The schema should also be validated by a Draft 2020-12 implementation in downstream data tooling.

## Integration and authority

- Process-aware feedstock: [#6124](https://github.com/Luminous-Dynamics/symtropy/issues/6124)
- Granular flow and transfer: [#6041](https://github.com/Luminous-Dynamics/symtropy/issues/6041)
- Dense cohesive segregation: [#6070](https://github.com/Luminous-Dynamics/symtropy/issues/6070)
- Vibration / metering control: [#6090](https://github.com/Luminous-Dynamics/symtropy/issues/6090)
- Material provenance/property domains: [#6004](https://github.com/Luminous-Dynamics/symtropy/issues/6004)
- As-built and disposition closure: [#6006](https://github.com/Luminous-Dynamics/symtropy/issues/6006)

Symtropy records the stream ledger, assays, events, energy, uncertainty and replay. Specialist mineral-processing solvers retain their physics authority. Symthaea can propose sequences and experiment choices only inside independently validated envelopes. Provenance/custody systems can preserve data lineage but cannot establish physical assay truth.

## Claim ceiling

This branch adds a schema, semantic validator and synthetic process-chain fixture. No magnetic/electrostatic separation, XRD, PSD, thermal property assay, physical transfer or reactor-inlet campaign was run. No process-specific feedstock has been physically qualified.

## Paired quality-drift analysis

The Rust crate in this directory compares assay samples across process stages:

```sh
cargo fmt --manifest-path tools/lunar-feedstock-contract/Cargo.toml -- --check
cargo test --locked --manifest-path tools/lunar-feedstock-contract/Cargo.toml
cargo clippy --locked --manifest-path tools/lunar-feedstock-contract/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path tools/lunar-feedstock-contract/Cargo.toml -- \
  tools/lunar-feedstock-contract/examples/synthetic-process-chain-run.json \
  Q-SEP-001 Q-REACTOR-001
```

For each mineral phase it reports:

[
\Delta x = x_{to}-x_{from},\qquad
u_{\Delta x}^{worst}=u_{to}+u_{from}
]

The linear bound is deliberate: it is the conservative upper bound for the uncertainty of a difference when covariance/correlation between the two assays is not known. A root-sum-square would assume independent errors and can overstate confidence if the assays share calibration standards or systematic effects. A future covariance-aware run may use a documented covariance artifact; absent that, use the linear bound.

A change is labelled resolved increase/decrease only when the one-standard-uncertainty interval excludes zero. Otherwise it is indeterminate. Unknown measurements remain unknown, and values are never subtracted across different or missing units. This is a descriptive paired-assay comparison, not a causal test that transfer or segregation caused the change.

PSD bins are compared only when the source and destination assays use matching bin edges. Different partitions return `incompatible_bin_partitions`; the program does not interpolate or pretend that unmatched bins are equivalent. The report separately compares the process target-size fraction and preserves each bin's own interval.

Linux CI runs formatting, unit tests, Clippy and the synthetic separator-to-reactor example. The synthetic pair's plagioclase shift and unchanged target-size fraction are both indeterminate under the conservative linear bound; no correlation structure is assumed. This is only a regression fixture, not a physical process finding.
