# EV factory scenario-family robustness frontier v0.1

**Issue:** [EV-FACTORY-ROBUST-001 #2439](https://github.com/Luminous-Dynamics/symtropy/issues/2439)  
**Implementation:** `tools/ev-passenger-cost-frontier/src/bin/ev_factory_robustness_frontier.rs`  
**Status:** analysis across synthetic simulator regimes; no measured factory is represented

## Paradigm shift: uncertainty has more than one level

Twenty seeds estimate random variation under one fixed simulator specification. They do not show that the result survives a different product mix, supplier cadence, changeover burden, repair pattern or finite-buffer topology. Robust design should compare **scenario families** as distinct model specifications instead of pooling every output into one misleading giant sample.

This tool reads five or more versioned DES reports, computes a SHA-256 for each exact input file, verifies a shared ordered seed list and matches per-seed outcomes, then evaluates interventions family-by-family.

## Initial synthetic profile suite

The CI protocol derives all five profile fixtures from the same base catalog and retains the same 20 seeds:

| Family | Perturbation | What it tests |
|---|---|---|
| `nominal` | Original source fixture | Reference design |
| `supplier-starvation` | Longer, more variable inbound inter-arrival time and smaller raw-material buffer | Is a downstream capacity addition still useful when material flow is limiting? |
| `variant-setup` | More variants, smaller batches and higher changeover penalties | Does parallel capacity interact with mixed-model sequence and setups? |
| `reliability-disruption` | Lower synthetic MTBF and longer repair duration | Does the investment ranking survive more frequent/longer station outages? |
| `finite-buffer-stress` | Smaller source and interstation buffers | Are recommendations sensitive to blocking and starvation from limited WIP buffers? |

These parameter changes are *stress-test knobs*, not sourced empirical estimates of a typical plant or a probability distribution over real factories. Each family remains a separate assumption set.

## Outputs and aggregation

For each station intervention and each family, the report includes the per-seed paired gain distribution, mean and 95% Student-`t` interval, exact source digest, family-specific best point-estimate intervention, and regret against that family’s best point estimate.

For each station across families, it reports:
- Mean of the family-specific mean gains (equal weight per declared family).
- Worst-family mean gain and minimum lower confidence bound across families.
- Number of families where the gain interval is entirely above the internal practical margin.
- Mean and maximum regret relative to each family's best point-estimate intervention.
- A three-objective Pareto indicator over mean gain, worst-family mean gain and maximum regret.

It also compares every pair of interventions **within each family**, using paired seed-level differences of their baseline-relative gains. A “robust best” is emitted only if one intervention clears the practical margin against every other candidate in every family. If profiles disagree or leave a pair unresolved, the tool abstains.

No averaging over family means is used to pretend that family specifications are IID observations. Random-seed variability is reported within a family; differences between family assumptions are kept visible. Family weights are equal by design in this initial benchmark, not estimates of how often regimes occur in industry.

## Run

After generating one report per family:

```sh
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml \
  --bin ev_factory_robustness_frontier -- \
  nominal=ev-report-nominal.json \
  supplier-starvation=ev-report-supplier-starvation.json \
  variant-setup=ev-report-variant-setup.json \
  reliability-disruption=ev-report-reliability-disruption.json \
  finite-buffer-stress=ev-report-finite-buffer-stress.json
```

The tool fails closed if any report is not synthetic, has failed simulator invariants, has duplicate/missing seeds, mismatched station/intervention sets, missing per-seed outcomes, incorrect output-minus-baseline arithmetic or reported aggregate means inconsistent with its per-seed rows.

## Proposed gates and external comparison

| Gate | Initial target | Why |
|---|---|---|
| Family inputs | At least five named scenarios, same exact seed list | Avoid optimizing one nominal point only |
| Reproducibility | Same profile files/seed list => byte-identical reports and digests | Permit independent review |
| Pairwise intervention comparison | Paired interval over the difference in paired gains | Preserve common-seed dependence |
| Cross-family winner | Candidate must beat all alternatives in every declared family by the pre-set margin | Fail closed under constraint migration |
| Regret and Pareto | Publish family-specific regret plus mean/worst-family metrics | Reveal trade-offs hidden by a single average |
| Field validity | Rights-cleared measured traces, frozen holdouts and independent VVUQ | Not available in this synthetic prototype |
| Purchase/safety release | Separate economics and approvals | Always false in this tool |

This direction aligns with public manufacturing-DT work that combines simulation with operating scenarios and reconfigurable line design. Wang et al. (2026) study simulation/emulator support for reconfigurable production-line decisions: https://doi.org/10.1007/s10479-025-06667-9. Keshvarinia et al. (2025) evaluate alternative responses to stochastic material-supply disruption using a manufacturing DES: https://doi.org/10.1016/j.dajour.2025.100646. A 2025 *Results in Engineering* paper considers DT-based operational strategies under unexpected plant scenarios: https://doi.org/10.1016/j.rineng.2025.106761. These papers motivate scenario-aware analysis; they do not establish that this prototype is better than their methods or commercial factory-planning tools.

NIST's 2026 manufacturing digital-twin report treats interoperability, verification/validation/uncertainty quantification, cybersecurity and workforce readiness as important dimensions beyond simulator reproducibility: https://doi.org/10.6028/NIST.IR.8620.

## Limits

- The practical margin of 0.05 good units/hour is an internal benchmark value, not an industry standard.
- The family suite changes only selected knobs supported by the current synthetic simulator. It does not yet model shift calendars, planned maintenance, common-cause failures, order due dates, energy tariffs, labor availability, inventory cash or correlated multi-week supplier outages.
- The robust-best test is deliberately conservative and may abstain often. That is preferable to declaring a winner when it is not robust.
- Even a unique robust-best result is conditional on the selected stress families and this simulator's structure; it is not a real factory's causal treatment effect or investment recommendation.
- No factory MES/PLC data was collected. No suppliers, manufacturers, plants, integrators, landlords or agencies were contacted, and no quotes were requested.

The tool never authorizes factory release, product-quality release, battery/HV safety or local compliance.
