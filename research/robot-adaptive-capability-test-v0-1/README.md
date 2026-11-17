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
- Each item explicitly declares whether it contributes to the single capability axis. Gate-only safety controls default to excluded from the Rasch estimate; common anchors must measure the declared axis.
- A failed safety-critical gate or explicit unsafe violation stops the adaptive run.
- Unknown observations stop with EvidenceRequired; they are preserved as Unknown and never converted to binary failure or success.
- InvalidEvidence is rejected rather than admitted as a scored response.
- Declared minimum counts for content strata are enforced before precision stopping.
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
- Independently calibrated item difficulty, stratum, mandatory/anchor/safety-critical flags, and a per-item capability-axis scoring flag.
- Fixed prior, target posterior standard error, adaptive item budget and minimum stratum counts.
- Ordered evaluator outcomes: Success, Failure, Unknown, UnsafeViolation or InvalidEvidence. Only valid binary outcomes from items marked for the declared capability axis update Rasch theta.

The current implementation requires canonical lowercase 64-character SHA-256 identity strings. Digests are opaque bindings; this crate does not compute or authenticate them.

## Run

From this directory:

    cargo fmt --all -- --check
    cargo clippy --all-targets -- -D warnings
    cargo test --all-targets
    cargo run

No third-party dependencies are required.

## Tests

The synthetic suite covers mandatory ordering, common anchors, adaptive difficulty selection, exact tie-breaking, stratum quotas, precision and budget stops, bank exhaustion, budget/coverage interaction, bad IDs/digests, response chronology, replayable selection history, distinct axes and response-history effects. It also verifies that gate-only safety controls do not move the capability estimate, safety failures stop adaptation, Unknown requires new evidence, and inadmissible evidence is rejected.

All fixtures are synthetic known-answer development tests. They do not calibrate a real item bank, qualify a robot, establish a physical capability score or demonstrate trial-cost savings.

## Research limitations

- One-dimensional Rasch/1PL measurement can misrepresent multidimensional robot capability.
- Item calibration is external and must be independently estimated with uncertainty before native use.
- Binary outcomes discard partial task information.
- Adaptive selection can bias comparison; fixed confirmation is mandatory.
- The prior and item difficulty scale are model assumptions; theta is not a physical unit.
- Safety-critical items cannot be skipped based on posterior precision.
- Gate-only controls are deliberately excluded from a capability-axis estimate unless their item metadata explicitly declares otherwise.
- Small synthetic tests do not establish calibration invariance across robot morphologies or sites.
