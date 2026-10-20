# Adversarial furnishing geometry v0.4

**Status:** draft, synthetic-only. This is an oracle qualification slice—not a furniture solver, accessibility certification, or building-code compliance claim.

## Why this work exists

A sparse-grid benchmark can demonstrate one favorable case, but does not expose numerical tolerance bugs, invalid geometry, task-envelope conflicts, or high-collision workloads. v0.4 adds exact known-truth tests and a small workload matrix before trusting broader performance claims.

## Geometry cases added

- Strictly touching axis-aligned edges are not treated as physical overlap.
- Overlap smaller than the oracle's explicit 1e-9 m tolerance is treated as clear; overlap exceeding that tolerance is detected.
- Non-finite rectangles are rejected by geometry validity checks and cannot spuriously overlap.
- Item-local functional envelopes are transformed correctly at 0, 90, 180 and 270 degrees.
- Unsupported angles (for example 45 degrees) return no transformed envelope instead of silently falling back to unrotated geometry.
- Empty spatial indexes return no candidates, and a 16-rectangle all-overlap case returns all 120 unique pairs.

The 1e-9 tolerance is a floating-point predicate tolerance, not a human clearance margin. It must never be described as practical construction tolerance or acceptable physical spacing.

## Stress benchmark matrix

The new dependency-free Rust benchmark uses a frozen, in-memory synthetic corpus and reports p50/p95 at 25, 100, 1,000 and 10,000 items for:

| Workload | Geometry truth | Expected functional decision |
|---|---|---|
| Sparse grid | Disjoint furniture footprints and task envelopes | pass; candidate only |
| Tight functional clearance | Furniture footprints do not collide, but next row enters the front task envelope | fail for blocked requirements; unrelated rows may pass |
| Dense footprint overlap | Adjacent grid footprints overlap; exact collision-pair count is known from grid-neighbor geometry | functional conflicts remain blocked; emit every true pair |

The harness checks that the BVH emits exactly the known expected physical-overlap pair count for the dense workload and that there are no unrelated findings. This makes false-negative/extra-pair behavior a hard gate, instead of reporting just elapsed time. Corpus construction is outside the timed region; the full oracle audit is inside.

## Targets and comparisons

These are proposed gates, not achieved results:

- Zero missed physical overlap pairs and zero false pairs against the grid-derived known truth.
- Zero false-safe decisions in the frozen synthetic test corpus.
- Deterministic pair and diagnostic ordering.
- Record p50/p95 separately for sparse, tight-clearance and dense-overlap workloads at each size tier.
- For sparse and clearance-conflict scenes, provisional p95 targets: under 100 ms at 10,000 items on the named reference machine.
- Dense overlap can legitimately be output-bound: with 10,000 rectangles, there are up to 49,995,000 unique unordered pairs. Do not promise sub-100 ms when the correct output contains tens of millions of collisions; report pair count, throughput, wall time, and peak memory.
- No geometry index or speedup is allowed to downgrade fail/indeterminate to pass.

## Comparison plan

Compare at least three implementations on the same source cases and machine: the v0.2 all-pairs oracle, v0.3 BVH/indexed implementation, and an independent brute-force reference used as the collision truth. Record source SHA, toolchain, OS/CPU/RAM, input digest, iteration count, expected/observed collision counts, p50/p95 and peak memory. A hosted CI run is a regression sentinel; stable hardware and repeated runs are required for a general performance claim.

## Exact-head stress result — run 39481555651

The focused [Furnishing Layout Audit run #39481555651](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39481555651) passed at source SHA `a6fadac278a7005c61ff92626ed41e600a3fdb40`: 26 Rust tests passed, formatting and strict Clippy passed, and every sparse/tight/dense stress assertion matched expected collision counts with zero unexpected findings. The machine and per-tier outputs are archived in [the run receipt](evidence/FURNISHING_GEOMETRY_STRESS_CI_RUN_39481555651.json).

| Workload | 25 items p50/p95 | 100 p50/p95 | 1,000 p50/p95 | 10,000 items |
|---|---:|---:|---:|---:|
| Sparse grid | 0.029 / 0.041 ms | 0.141 / 0.189 ms | 2.322 / 2.445 ms | 43.149 ms (single sample) |
| Tight functional clearance | 0.028 / 0.044 ms | 0.139 / 0.164 ms | 2.226 / 2.293 ms | 43.080 ms (single sample) |
| Dense footprint overlap | 0.035 / 0.047 ms | 0.175 / 0.199 ms | 2.656 / 2.773 ms | 49.088 ms (single sample; 39,402 expected and observed collision pairs) |

The 10,000-item measurements use one sample, so they are not p50/p95 estimates. The runner reported Ubuntu 24.04, Rust 1.96.0, four visible vCPUs on an AMD EPYC 9V45 host and 15 GiB memory. These are diagnostic hosted-runner observations, not a production SLA. The matched v0.2 baseline remains in progress; no speedup ratio is claimed.

## Claim boundary

These cases still use axis-aligned 2D rectangles. They do not model door swing arcs, moving drawers beyond their declared rectangular envelopes, 3D objects, uncertainty in measured dimensions, wheelchair swept-path geometry, reach ranges, transfer spaces, egress, accessibility standards or building code. Those should be added as separate, sourced and independently reviewable task models. Unknown required evidence must block, not be guessed.
