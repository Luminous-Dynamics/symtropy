# Published benchmark input-availability audit

Last checked: 22 October 2026.

Primary source: Wang, Yin, and Khan, “Spatial-Scheduling Strategy for Irregularly Shaped Blocks Under Disruption in Shipbuilding,” *IET Collaborative Intelligent Manufacturing* (2026), https://doi.org/10.1049/cim2.70066.

## What is publicly available on the article page

- **Table 1 — Case families:** 40-, 50-, and 60-block families, with Cases 1–18 and different numbers of sub-cycles. Case9 is the 50-block / five-sub-cycle family.
- **Tables 2–4 — Aggregate sensitivity and representation results:** source-reported parameter sweeps for nectar-source count and iteration count, plus comparison of initial, simplified and regularized shape representations.
- **Table 6 — Aggregate disrupted-case results:** Case9_50_0 through Case9_50_5; PA, IA, GA and DE summaries for yard utilization (`Aur`), algorithm time (`Act`) and vehicle time (`Vt`), and counts of workstation (`D1`) and vehicle (`D2`) disruptions.
- **Appendix A1 — Vehicle fleet:** published nominal vehicle types / counts and related weight dimensions.
- **Appendix A2 — Shape library:** 19 block-shape coordinate records across initial, simplified and regularized representations. These can seed a future geometry-parser conformance set only after source/license terms are checked; this file does not redistribute their coordinate table.
- **Reported solver environment:** Python implementation on Windows 11, Intel Core i7 at 2.40 GHz and 32 GB RAM.
- **Algorithm parameters described in the article:** weighted-goal criterion with time and space weights each 0.5; the paper reports 15 repetitions per case and discusses its two-stage ABC / ALNS / disruption-recovery process.

See the publisher article sections 5.1–5.4, Table 6, Appendix A and its Data Availability Statement: https://ietresearch.onlinelibrary.wiley.com/doi/10.1049/cim2.70066.

## What remains essential for a faithful Case9_50_4 reproduction

The published aggregate table is not the case input. The data availability statement says: “The experimental data and the simulation results that support the findings of this study are available from the corresponding author upon request.” The following must be recovered from the paper's linked data or supplied through an authorized request, not guessed from the summary:

- The exact 50-block assignment to geometry IDs and the complete per-block polygon vertices in the coordinate convention used by the algorithm.
- The yard aisle geometry / effective usable-yard definition and any placement transformations or rotations used for the 50-block case.
- Block-to-workstation assignment, weights, transportation durations / start times, full five-sub-cycle membership and exact short-term plans.
- The individual workstation and vehicle disruptions constituting Case9_50_4, their event timing and effect on operation status.
- Vehicle-to-task assignment, travel-time inputs, availability and refuelling data, plus any other arrays consumed by the implementation.
- The exact fitness/objective normalization, the stopping/evaluation budget and configuration for every compared method.
- The 15 per-run values (or raw logs) for PA, IA, GA and DE; reported table aggregates alone cannot reproduce distributions or infer random seeds.
- Any exact baseline source implementation needed to reproduce the comparator algorithms, plus licensing/permission for redistribution or public derived artifacts.

## Reproduction decision rule

**Current status: target identified; full instance not yet reproduced.** We can test an independent parser and geometry routines on the publicly visible shape examples, and store Table 6 aggregates as reference numbers. We cannot reproduce Case9_50_4 until the case-specific source data and protocol are available.

If any required component stays unavailable, label the run `non-comparable`, publish a missing-input record, and move to clearly labelled synthetic robustness cases. Never treat guessed assignments, synthetic layouts, or a plausible match to the table as a reproduction.

## Next benchmark tasks

1. Create a licensed, source-attributed geometry-parser fixture pack that records table row ID, representation variant, units, polygon validity, computed shoelace area, and difference from the article's listed area.
2. Request the complete supplementary data and run outputs from the corresponding author, explicitly asking permission for public redistribution of non-sensitive derived instances.
3. Freeze a normalized instance only after all required fields and transformations are sourced. Verify coordinate winding, non-self-intersection, positive area, yard bounds and unit consistency independently.
4. Reproduce the 15-run protocol and report every run, including timeouts and infeasible candidates, before comparing against the reported aggregate values.
5. Test the added operational semantics—delayed clearance, finite fleet, loading/unloading and departure-time-dependent travel—as labelled ablations separate from the headline reproduction.

## Aggregate reference versus instance truth

The checked-in aggregate summary file preserves only numerical results from Table 6. It is not a source dataset, does not claim to implement PA/IA/GA/DE, and must never be used as substitute geometry or a ground-truth per-run trace.
