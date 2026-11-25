# ErgoStream passive gap/pattern baseline v0.1

**Implementation:** `src/acquisition.rs`  
**Policy name:** `passive-gap-pattern-v0`

## Algorithm

This is a deliberately small, inspectable, passive structure-extraction baseline. It receives only the same public `PolicyViewV0` exposed to a candidate learner:

1. Validate schema version, available-prefix length, contiguous frame sequence and strictly increasing ticks.
2. Compute the smallest and largest observed inter-frame tick gaps.
3. If there is no separation between those two values, abstain and return an empty snapshot.
4. Otherwise set the segmentation threshold to their midpoint and close a segment whenever an inter-frame gap exceeds it.
5. Treat segments as candidate units and cluster their exact observed sequences of (channel ID, symbol) pairs.
6. Treat every adjacent pair of units as a motif and cluster by the corresponding pair of unit-cluster IDs.
7. Do not close the last segment unless the caller explicitly marks end-of-stream.

The implementation uses an ordered map so cluster assignment is deterministic. No generator seed, latent unit label, relation label, hidden evaluator state, downstream question or task reward is read.

## What it tests

- Whether a very simple timing heuristic is sufficient for the initial two-scale stream generator.
- Whether exact observed-pattern clustering recovers recurrence under an alphabet permutation.
- Whether the baseline refrains from inventing segmentation when the timing channel provides no separable gap scale.
- Whether the baseline treats a still-open tail as incomplete rather than as a completed unit.

## Limitations

The algorithm uses global min/max gap statistics from the supplied prefix and assumes two timing scales. It is not a robust event detector, an active acquisition policy, a continuous-time model or a cross-domain transfer algorithm. It may fail under clock jitter, dropped frames, multiple event durations, unrelated long pauses, irregular sampling or a world where temporal gaps do not mark unit boundaries. Those are required challenge conditions for future generator families.

Perfect recovery on the current reference generator would establish only that this simple baseline solves that narrow generator. It is not evidence of general structural discovery. It is included precisely so the next learner must beat a simple, transparent comparator under stronger matched-marginal and false-analogy controls.

## Reproduction

```sh
cargo fmt --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml -- --check
cargo test --locked --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml
cargo clippy --locked --manifest-path research/ergo-stream-world-contract/v0.1/Cargo.toml --all-targets -- -D warnings
```
