# Ocean Observation Allocation Benchmark

A focused Rust benchmark for **bias-aware, non-contact marine observation planning**. The corpus is synthetic and deliberately small enough for exhaustive comparison against an exact subset oracle.

## Run

From the repository root with Rust installed:

    cargo test -p ocean-observation-allocation-bench --locked
    cargo run --quiet -p ocean-observation-allocation-bench --locked -- --corpus docs/tech/OCEAN_OBSERVATION_ALLOCATION_CORPUS_V0_1.json --source-commit "$(git rev-parse HEAD)" --receipt target/ocean-observation-allocation-receipt.json

The tool emits a deterministic receipt bound to the exact corpus bytes by SHA-256, Git source commit, schema/suite/tool version, seed, case count, policy reports and invariant outcomes. CI uses a path-filtered workflow and does not need the full Bevy application test suite. The focused workflow uploads the machine-readable receipt as a downloadable 14-day artifact and prints per-policy coverage, information score, effort, separate exposure totals, regret and selected candidate IDs to the job log.

## Compared policies

- **Random fixed seed:** transparent deterministic weak baseline.
- **Largest coverage gap:** favours high-priority, uncertain strata that are not yet represented.
- **Raw record count:** deliberate anti-baseline that can reinforce easy-access sampling.
- **Effort-aware stratified:** prioritizes uncovered strata while discounting historically over-sampled strata and higher action effort.
- **Uncertainty-reduction greedy:** maximizes declared marginal information score per effort.
- **Exact small-case oracle:** exhaustively enumerates eligible subsets under the same budget limits to bound avoidable regret.

The declared information score is a benchmark objective, not a biological truth score. Output also reports weighted stratum coverage, raw uncertainty reduction, effort, each exposure channel, uncovered strata and regret. Sound, light and sediment are independent hard limits, not terms that can trade against one another in a single "safety" score.

## Fail-closed eligibility

A candidate is not selectable unless it is non-contact, its data terms have been reviewed, its detection limits are declared, and all exposure channels are known. Unknown exposure is not zero. Contact actions and unresolved/restricted sources remain represented in the fixture as negative controls, but no policy may select them.

Historical record count and effort are sampling metadata. The tool has no species-presence/absence conclusion field; no records are not evidence of no species. The synthetic targets are not an empirical biodiversity inventory, an ocean physics model, ecological damage assessment, environmental approval, vehicle actuation permission or mineral-extraction optimizer.

## Benchmark goals

- Deterministic replay for the same corpus, seed, tool version and source commit.
- Zero selected actions that fail contact, rights, detection-limit, exposure-known or independent budget gates.
- In the frozen access-biased case, effort-aware stratification covers more declared-priority weight than the raw-record-count baseline.
- Exact-oracle regret is non-negative and zero for the oracle itself.
- Report worst uncovered strata and exposure channels rather than a single environmental safety score.
- The score/coverage trade-off is a simulation benchmark result only; real sites require independently reviewed data, methods and constraints.
