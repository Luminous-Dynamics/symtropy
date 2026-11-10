# Symtropy perceptual budget policy — V0

A small, isolated Rust policy tool for choosing among **measured** rendering profiles under task-specific quality and resource constraints.

## Run

From this directory:

```sh
cargo test
cargo run -- fixtures/city-block-remote-inspection.json
```

The CLI reads one JSON manifest (or standard input) and emits a deterministic receipt. It never launches a renderer or changes simulation state.

## Policy

The requested task defines:
- task mode (cinematic, engineering inspection, safety inspection, or accessibility);
- maximum normalized spatial error and temporal error;
- minimum task-legibility bound;
- semantic channels that must remain visible (units, reference frame, uncertainty, freshness, evidence class, contrast metadata);
- protected asset IDs that may not be demoted;
- frozen-workload evidence requirement and optional blinded panel minimum;
- p95/p99 frame-time, peak VRAM and optional GPU energy/frame budgets. Remote/low-power is a resource objective, so an energy cap can be applied to an engineering or safety view without changing its task semantics.

Each candidate profile binds its scene capture, metric bundle and measurement receipt digests. Missing or unknown required information is rejection, not a default. No weighted score can compensate for a failed hard gate.

When energy is required, the lowest-energy admissible profile wins; ties break by p95 frame time, VRAM and stable profile ID. Without an energy cap, p95 frame time is the first cost dimension. The full admissible set remains in the receipt for Pareto analysis.

## Scope / claim ceiling

This is policy conformance over supplied finite measurement records only. It does not:
- calculate spatial/temporal perceptual metrics;
- validate the truth of frame-time, energy, VRAM or human-panel measurements;
- perform foveated rendering, LOD, asset protection or scene rendering;
- prove that a view is safe or comprehensible to a real operator;
- mutate physics, controls or authoritative world state.

Human preference is optional unless the task contract requires it. A panel record must bind its protocol and sample-set digests; its statistical method and evaluator protocol remain owned by the benchmark research programme (#4486).

## Bundled fixture status

`fixtures/city-block-remote-inspection.json` is **synthetic-only software test data**. Its digests are placeholders and its frame-time, VRAM and energy values are invented known-answer inputs. It is deliberately marked `synthetic_only` and `require_measured_workload = false`; it is not an engine benchmark or a claim about measured performance.

Actual benchmark runs must use capture/measurement receipts from #4486 and set `require_measured_workload = true`. A real measured-workload candidate must be classified `frozen_workload_measurement`, bind the exact scene and metric-bundle digests, and use the actual upstream receipt digest.
