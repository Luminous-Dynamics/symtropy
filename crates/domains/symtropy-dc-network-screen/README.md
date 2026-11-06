# Symtropy DC Network Screen

A deterministic DC power-flow screening layer for locational data-centre flexibility. It bridges signed load-reduction trajectories to a frozen network topology and monitored branch limits.

## Sign convention

Positive non-slack bus injection means net generation or demand reduction. A data-centre load reduction of 30 MW is represented as a +30 MW injection at its PCC bus. The slack bus balances the sum of all non-slack injections.

## v0.1 capability

- Frozen model/topology IDs and digests with an effective timestamp.
- Exactly one slack bus; duplicate buses/branches and unknown bus references rejected.
- Positive per-unit branch reactance, caller-supplied normal and emergency ratings.
- Chronological baseline and flexible injection profiles with a signed per-bus load-reduction map.
- Explicit topology maximum-age and numerical tolerances; the snapshot must remain within its freshness contract through the interval end.
- DC susceptance matrix assembled from in-service branches; Gaussian elimination with partial pivoting.
- Baseline and flexible signed branch flows, branch flow deltas and absolute-magnitude relief.
- Normal-rating checks for the intact topology.
- Explicit N-1 outage list; each outage is recomputed. Singular or islanded outage topologies are recorded as not feasible, not silently dropped.
- Emergency-rating checks for the post-outage network.
- Exact chronology, bus mapping, slack-bus convention, topology age and contingency-set checks.

## Explicit limits

This is active-power DC screening. It assumes a linearized network and caller-supplied reactances/ratings. It does not calculate voltage magnitude, reactive power, losses, transformer taps/phase shifts, frequency response, fault current, protection, AC load flow, RMS/EMT dynamics, HIL, feeder hosting capacity, market accreditation or ELCC. It is not a utility interconnection opinion. Positive DC screening is not AC feasibility.

The first comparator should be hand-computed three-bus networks. The next stage is independent AC solver comparison on identical topology/injections/limits, then feeder models and operator-authorized network models.

## Run

```sh
cargo test -p symtropy-dc-network-screen
cargo clippy -p symtropy-dc-network-screen --all-targets -- -D warnings
```

Energy is MW × hours = MWh (no additional 1,000 divisor). All tests are synthetic known truth. No operator network model or live grid data is included.

## Maximum feasible dispatch fraction

DC branch flows are affine in a scalar action fraction λ when the injection pattern is scaled from the supplied baseline to the requested flexibility vector. For every in-service branch, the evaluator solves the exact one-dimensional interval satisfying −rating ≤ F_baseline + λ(F_full − F_baseline) ≤ rating, intersects the branch intervals and then intersects required N-1 contingencies. The result reports the minimum and maximum feasible action fraction plus the corresponding maximum network-deliverable MW for the requested load-reduction vector.

This is not an optimizer and does not assume monotonicity. The feasible range can begin above zero when the baseline overload is relieved only after some response. If no common fraction satisfies all required constraints, the result returns no feasible fraction. Optional contingency diagnostics do not reduce the basecase deliverability result unless N-1 is a declared contract requirement.

## Per-unit base

The frozen network model requires an explicit positive `base_mva`. Bus injections are accepted in MW and converted to per-unit by dividing by this base before solving the reduced bus-susceptance matrix. Branch flows are converted back to MW by multiplying the per-unit flow by the same base. The base is retained in the top-level report. The known-truth suite checks that changing the base from 1 MVA to 1,000 MVA preserves the physical MW results.

## PCC identity and action fraction

The flexibility identity pins `pcc_bus_id`. Every non-empty load-reduction action map must apply only at that bus, and the bus must exist in the frozen topology and must not be the slack bus. This is a single-site v0.1 adapter; multi-site portfolios require a separately bound asset-to-bus manifest.

The action-fraction result is an interval `[lambda_min, lambda_max]` in [0, 1]. This matters when the baseline network is already overloaded: a nonzero minimum dispatch may be required to relieve the overload before the network state becomes feasible. The maximum fraction is not assumed to be one, and a lack of any common feasible fraction is reported explicitly.
