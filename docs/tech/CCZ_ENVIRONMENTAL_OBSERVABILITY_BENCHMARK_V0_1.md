# CCZ Environmental Observability Benchmark v0.1

**Status:** synthetic contract benchmark; implementation PR is a draft. No physical ocean model is implemented or validated.
**Issue:** https://github.com/Luminous-Dynamics/symtropy/issues/2011
**Cross-project reuse:** Symtropy environmental observation context [#1240](https://github.com/Luminous-Dynamics/symtropy/issues/1240), Environment Observatory [#1245](https://github.com/Luminous-Dynamics/symtropy/issues/1245), coarse ocean circulation/tracer work [#465](https://github.com/Luminous-Dynamics/symtropy/issues/465), maritime deterministic corpus [#1612](https://github.com/Luminous-Dynamics/symtropy/issues/1612), source-pinned maritime receipt [#1642](https://github.com/Luminous-Dynamics/symtropy/pull/1642), and the [CCZ Commons Atlas](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/165).

## Paradigm shift: claims are scoped, multidimensional and falsifiable

A sensor's failure to detect an effect is not proof that no effect exists. “No impact detected within the declared scope” is an observation-bound statement, not “no impact,” “safe,” or “recovered.” Missing baselines, incomparable controls, stale calibration, unknown detection limits, inadequate coverage, or missing follow-up must stay visible.

The benchmark deliberately does **not** compress results into one overall safety score. It returns separate dimensions:

1. **Evidence integrity:** source snapshot identities and sample identities must be internally consistent.
2. **Comparability:** declared baseline, reference control, methods, detection limits, calibration and spatial coverage must support the question.
3. **Impact result:** detected evidence or non-detection, each limited to the declared spatial scope.
4. **Recovery result:** separate from impact detection. Sufficient recovery follow-up is a distinct claim from short-term recolonization or lack of a detected difference.
5. **Cumulative exposure:** separate completeness outcomes for declared sound, light and sediment exposure channels.

A system can therefore report direct-track impact evidence while honestly saying that cumulative fleet-wide exposure remains indeterminate. Conversely, complete exposure accounting is not proof of ecological safety.

## Published research anchors

These are falsification-design anchors, not universal thresholds or interchangeable ecosystems:

- Valls Domedel et al., *Nature Ecology & Evolution* 10, 318–329 (2026), assessed two years of pre-trial observations and sampling two months after the 2022 eastern Pacific commercial-scale test. They reported 37% lower macrofauna density and 32% lower species richness directly within tracks, with significant natural background variation. In the area affected by sediment plumes, the authors found no abundance change but observed altered dominance relationships; sample-size-independent accumulation measures gave a more nuanced result. The benchmark therefore requires baselines, comparable control sites and taxonomic/method metadata rather than treating a single diversity metric as complete truth. [Paper](https://doi.org/10.1038/s41559-025-02911-4).
- Gazis et al., *Nature Communications* (2025), reported current-mediated dispersion and sediment redeposition in a benthic plume monitoring experiment. This motivates boundary-truncation, changing-current and spatial-blind-spot scenarios, not a universal “safe distance.” [Paper](https://www.nature.com/articles/s41467-025-56311-0).
- Glover et al., *Current Biology* (2026), reviews nodule fields, seamounts and hydrothermal systems, and distinguishes resilience/recolonization from biodiversity-loss risk. Those habitats and disturbance mechanisms are not interchangeable; this benchmark must not infer one universal impact model or threshold. [Review](https://doi.org/10.1016/j.cub.2026.03.014).

## Frozen synthetic corpus

The corpus in [CCZ_ENVIRONMENTAL_OBSERVABILITY_CORPUS_V0_1.json](CCZ_ENVIRONMENTAL_OBSERVABILITY_CORPUS_V0_1.json) contains 19 named, adversarial cases. All source/sample identities and evidence states are explicitly synthetic; no real mine site, species census, field measurement or authorization is encoded.

It includes:
- Valid direct-track impact and far-field impact evidence without scope expansion.
- A properly bounded non-detection that cannot be promoted to “no harm.”
- Missing baseline, incomparable controls/methods, stale calibration, unknown detection limits and unknown spatial coverage.
- A plume at the monitoring boundary with incomplete spatial coverage.
- Recovery claimed with insufficient follow-up.
- Cumulative sound/light/sediment exposure that is incomplete independently of the local impact outcome.
- Mismatched source snapshots and duplicate sample identities.
- Ambiguous signal, empty sample set and multi-fault combinations.

## Benchmark targets (goals, not measured results)

| Gate | Target |
|---|---|
| Frozen semantic corpus | 100% of 18 cases match the exact declared multidimensional outcome and reason codes |
| Single-fault mutation | 100% of mutations removing a required evidence condition downgrade the relevant dimension or invalidate the evidence |
| False certainty | Zero cases convert non-detection into absence/no harm, or insufficient follow-up into recovery |
| Snapshot integrity | Zero mismatched source snapshots or duplicate sample IDs accepted as valid evidence |
| Scope discipline | 100% of result records retain direct-track/near-field/far-field/regional scope; no spatial extrapolation is implied |
| Exposure coverage | Incomplete/unknown required cumulative channels remain indeterminate and cannot be offset by a local non-detection |
| Provenance | 100% of receipts bind the exact input bytes by SHA-256, the source commit, suite/schema/tool version and per-case expected/actual dispositions |
| Determinism | Identical corpus bytes, source commit and tool version produce byte-identical normalized receipts |
| Claim boundary | No numeric environmental safety threshold or operational authorization is emitted |

These are finite-corpus software gates, not proof that actual ocean monitoring is complete, accurate or safe.

## Execution

From the repository root:

```sh
cargo test -p ccz-observability-bench --locked
cargo run --quiet -p ccz-observability-bench --locked -- \
  --corpus docs/tech/CCZ_ENVIRONMENTAL_OBSERVABILITY_CORPUS_V0_1.json \
  --source-commit "$(git rev-parse HEAD)" \
  --receipt target/ccz-observability-receipt.json
```

The CI workflow pins Rust 1.96.0 and runs only this bounded verifier. A queued run is not evidence of a pass. A receipt validates fixture classification only; it does not verify source authenticity, license compliance, model fidelity, ecological outcomes, legal status or operating authority.

## Future integration

- Feed source-captured measurements only through the existing source receipt/provenance architecture; do not create a parallel environmental source store.
- Before accepting a physical plume model, specify its spatial/temporal domain, units, conservation properties, calibration, uncertainty, reference datasets and known failure envelope. Verify it against measured reference cases independently.
- Keep current-driven plume transport, suspended matter, deposition, sensor drift and cumulative fleet exposure as separate observable channels.
- Require baseline/reference designs and recovery follow-up appropriate to the particular habitat and disturbance mechanism; no hard-coded duration is presumed sufficient.
- For any physical mission, environmental observability supports human/regulatory decisions but does not itself authorize autonomous contact, recovery or extraction.
