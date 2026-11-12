# Symtropy Maritime Source-Mix and Negative-Transfer Bench

P0.1 deterministic, read-only evaluator for a matched multi-source transfer tournament. It validates source eligibility, task/taxonomy compatibility, source-weight normalization, duplicate-source independence, target-label sealing and matched run budgets. For the entropy-weighted run it computes per-source leave-one-out deltas across separate operational metrics; it does not collapse them into one composite score.

## Run

```sh
cargo test --manifest-path crates/domains/symtropy-maritime-source-mix-bench/Cargo.toml
cargo run --manifest-path crates/domains/symtropy-maritime-source-mix-bench/Cargo.toml -- benchmarks/maritime-source-mix/source-mix-case-v0.1.json --source-commit <40-hex> --case-sha256 <64-hex>
```

## P0.1 tournament

The frozen contract requires four matched comparators:

1. NO_TRANSFER;
2. BEST_SINGLE_SOURCE, chosen on validation only;
3. UNIFORM_MULTI_SOURCE;
4. ENTROPY_WEIGHTED_MULTI_SOURCE, with source weights selected from source-train and unlabeled target evidence only.

All runs bind the same target split, seed, label budget and compute-budget identity. Preprocessing is fit on source training only. Target-test labels are sealed and cannot select source weights, normalization, thresholds or the model.

Sources retain exact data digest, engine/domain identity, measured/simulated/derived/reconstructed role, evidence state, label state, fault-taxonomy compatibility, quality and calibration. A source with incompatible taxonomy, weak labels, rejected/stale quality, expired calibration or duplicate-copy identity cannot receive a transfer weight. A duplicated source ID is not a new independent source merely because it is listed twice.

## Source contribution and negative transfer

The full entropy-weighted result is compared with a leave-one-source-out result for every selected source. The receipt reports five deltas separately:

- macro recall (higher is better);
- worst-fault recall (higher is better);
- false alarms per valid engine-hour (lower is better);
- expected calibration error (lower is better);
- unknown-fault false-known rate (lower is better).

A source is flagged as exhibiting negative transfer on one or more declared metrics when removal improves recall or reduces false alarms/calibration error/false-known rate. This is a diagnostic of the frozen ablation, not a causal proof that the source is universally harmful. No single aggregate utility score is emitted.

## Frozen synthetic case

The example contains two measured-role source domains, one simulated-role domain, one duplicate copy, one near-but-taxonomy-incompatible source and one weak-label source. The dynamic weighting fixture deliberately assigns 0.20 weight to a simulated source whose removal improves all five target metrics. The evaluator must identify that source as negative-transfer under this particular synthetic case while retaining the per-metric trade-offs.

Expected synthetic result:
- four matched comparator methods;
- three selected entropy-weighted sources, with weights summing to one;
- one negative-transfer source, `src-simulated-adversarial`;
- two excluded source candidates (duplicate copy, incompatible taxonomy and weak labels are reported with separate reasons).

All numbers, source digests, labels and domain identities are fabricated known-truth conformance values. No marine engine performance, transfer accuracy, fleet generalization or operational capability is claimed.

## Research baseline

Shi et al., “Multi-source data fusion for marine four-stroke diesel engine fault diagnosis: An adaptive weight transfer learning framework,” *Energy Conversion and Management* 353 (1 April 2026), DOI 10.1016/j.enconman.2026.121194. The paper describes a shared-specialized representation, per-source/target discriminators, discriminator-posterior-entropy weighting and semi-supervised regularization, evaluated on an 8V396 testbed at 25%, 50% and 75% load with measured and simulated source domains. It reports 83.64% average accuracy for its pure real-measured scenarios and 82.37% for real/simulation hybrid scenarios. These are reported paper results, not reproduced here and not directly comparable to the Aalto MU323DGSC dataset. The exact entropy-to-weight mapping and per-task split must be recovered before replication.

## Claim ceiling

P0.1 qualifies only the deterministic handling of this source-mix case, exact source manifests, method runs, target split and declared metric calculations. It does not train EW-MDAN, establish source utility on physical engine data, prove cross-engine transfer, or authorize maintenance, mode changes or machinery control.
