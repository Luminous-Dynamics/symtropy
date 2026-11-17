# Robot Adaptive Capability Test v0.1

Parent: Symtropy #5752  
Challenge distribution owner: #5590  
Capability envelope owner: #4329  
Safety-vector owner: #4326  
Paired scorecard/replay owner: #2145  
Status: standalone synthetic-first Rust reference; exact-head runner qualification pending.

## Purpose

Select the next challenge item using expected Fisher information under a frozen one-parameter logistic Rasch calibration. The selector runs only on the evaluator side and measures one declared capability axis.

This is a measurement-efficiency experiment. It is not a robot controller, task generator, policy, acceptance authority, safety rating or universal robot score.

## Model

For latent capability theta and independently calibrated item difficulty b:

    P(success | theta, b) = 1 / (1 + exp(-(theta - b)))

The v0.1 implementation fixes item discrimination to one, uses a fixed Normal prior, and estimates theta by maximum a-posteriori Newton iteration. Item information is p(1-p). The selector chooses the eligible item with maximum information, with canonical item-ID tie-breaking.

Do not pool different capability axes into one theta. Do not calibrate or update item difficulty from the candidate currently being measured.

## Mandatory and fixed controls

- All safety-critical controls are mandatory and selected first.
- Common anchor items are mandatory and administered before adaptive items.
- Declared minimum counts for content strata are enforced before precision stopping.
- Unknown or invalid item/response/calibration identity is rejected.
- Budget exhaustion and bank exhaustion are explicit outcomes, not precision success.
- Every response sequence is replayed; a result is rejected if it does not match the selector's actual next item.

## Comparability rule

Adaptive item selection means different candidates may receive different items. Adaptive outcomes therefore are not a paired leaderboard comparison.

Use the selector to reduce redundant measurement burden and estimate capability precision. Any comparative improvement claim still requires:
1. common fixed anchor outcomes;
2. a separate blinded fixed confirmation bank with common items/seeds;
3. paired per-instance outcomes and uncertainty using #2145;
4. safety-vector checks using #4326;
5. empirical capability/tail evaluation using #4329;
6. a fresh challenge distribution and holdout integrity from #5590.

The selector's evaluator-side theta, item difficulty, expected information and future item order must never be exposed to the evaluated policy.

## Input contract

- Item bank ID and digest.
- Capability axis ID.
- Calibration ID and digest shared by bank and profile.
- Independently calibrated item difficulty, stratum, and mandatory/anchor/safety-critical flags.
- Fixed prior, target posterior standard error, adaptive item budget and minimum stratum counts.
- Ordered binary outcomes supplied by the independent task evaluator.

The current implementation requires canonical lowercase 64-character SHA-256 identity strings. Digests are opaque bindings; this crate does not compute or authenticate them.

## Run

From this directory:

    cargo fmt --all -- --check
    cargo clippy --all-targets -- -D warnings
    cargo test --all-targets
    cargo run

No third-party dependencies are required.

## Tests

The synthetic suite covers mandatory ordering, common anchors, adaptive difficulty selection, exact tie-breaking, stratum quotas, precision and budget stops, bank exhaustion, bad IDs/digests, response chronology, replayable selection history, distinct axes and response-history effects.

All fixtures are synthetic known-answer development tests. They do not calibrate a real item bank, qualify a robot, establish a physical capability score or demonstrate trial-cost savings.

## Research limitations

- One-dimensional Rasch/1PL measurement can misrepresent multidimensional robot capability.
- Item calibration is external and must be independently estimated with uncertainty before native use.
- Binary outcomes discard partial task information.
- Adaptive selection can bias comparison; fixed confirmation is mandatory.
- The prior and item difficulty scale are model assumptions; theta is not a physical unit.
- Safety-critical items cannot be skipped based on posterior precision.
- Small synthetic tests do not establish calibration invariance across robot morphologies or sites.
