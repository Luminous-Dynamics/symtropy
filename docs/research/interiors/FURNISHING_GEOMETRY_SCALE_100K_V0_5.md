# Indexed furnishing oracle: 100k scale probe v0.5

**Status:** proposed scale extension. The numbers are diagnostic local-oracle timing, not production SLA, physical-safety, accessibility or usability evidence.

## Motivation

The same-host A/B run #39481984586 compared the v0.2 brute-force oracle to the pinned v0.3 indexed oracle on identical stress inputs. At 10,000 items, the single-sample local-audit ratios were approximately 701x sparse, 500x tight functional clearance and 457x dense footprint overlap, with matching expected collision counts. The source receipt and exact run are archived in [baseline PR #2392](https://github.com/Luminous-Dynamics/symtropy/pull/2392). These 10k values are single samples, not p95 estimates.

Rather than run a potentially hour-long brute-force scan at 25k/50k/100k, this candidate-only probe measures the indexed oracle's next scale boundary. It keeps the same validator and synthetic known-truth structure while increasing sparse item/task count.

## Protocol

- Sizes: 10,000 (3 iterations), 25,000 (5), 50,000 (1), 100,000 (1).
- Workload: sparse, room-local axis-aligned rectangles with one mandatory service-access envelope per item.
- Timed region: audit_functional_envelopes over a preconstructed JSON document; corpus construction is excluded.
- Hard assertion: every generated item has the expected pass outcome and the oracle emits zero findings. Any finding fails the benchmark rather than becoming a slower result.
- The benchmark uses the nearest-rank empirical quantile (ceil(n*p)-1). Report p50/p95 only for the tiers with repeated samples. The 50k and 100k timings must be labelled single-sample latency; the 25k tier has five samples to make its p95 less fragile than a two-sample "p95."
- The workflow records exact source SHA, Rust version, OS, CPU and memory. Peak memory is not yet measured.
- Initial run #39483177535 measured 100,000 items at 760.021 ms in one sample. This is below the proposed 1 s target as a point observation, but it does not demonstrate the p95 gate; the exact-head receipt is archived at [FURNISHING_GEOMETRY_SCALE_100K_CI_RUN_39483177535.json](evidence/FURNISHING_GEOMETRY_SCALE_100K_CI_RUN_39483177535.json).

## Proposed gates

- Zero oracle findings on all four scale tiers.
- Deterministic output/order.
- p95 under 1 second at 100k on the declared reference machine as an initial engineering target, not a user-facing SLA.
- Record memory peak before promoting that target into a supported capacity guarantee.
- Do not state matched speedup above 10k; the baseline was not run at 25k/50k/100k.

## Limitations

This probe uses synthetic sparse 2D geometry; it does not measure highly clustered/irregular rooms, many true collisions, 3D transforms, geometry uncertainty, renderer/UI time, DHT, or solver search. It is intended to locate the next engineering boundary, not certify accessibility, building code, physical safety or resident outcomes.
