# Symtropy DC Network Screen

A deterministic DC power-flow screening layer for locational data-centre flexibility. It bridges signed load-reduction trajectories to a frozen network topology and monitored branch limits.

## Sign convention

Positive non-slack bus injection means net generation or demand reduction. A data-centre load reduction of 30 MW is represented as a +30 MW injection at its PCC bus. The slack bus balances the sum of all non-slack injections.

## v0.1 capability

- Frozen model/topology IDs and digests with an effective timestamp.
- Exactly one slack bus; duplicate buses/branches and unknown bus references rejected.
- Positive per-unit branch reactance, caller-supplied normal and emergency ratings.
- Chronological baseline and flexible injection profiles with a signed per-bus load-reduction map.
- Explicit topology maximum-age and numerical tolerances.
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

All tests are synthetic known truth. No operator network model or live grid data is included.
