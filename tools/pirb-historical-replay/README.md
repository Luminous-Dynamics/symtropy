# PIRB historical replay

A standalone Rust evaluator for aggregate-only retrospective replay of previously recorded alerts and response outcomes. The tool does not train a model, predict conflict, contact people, select interventions, or authorize operational action.

## Run

From the repository root:

    cargo test --manifest-path tools/pirb-historical-replay/Cargo.toml
    cargo run --manifest-path tools/pirb-historical-replay/Cargo.toml -- --input docs/fixtures/pirb-historical-replay-v0.1.json
    cargo run --manifest-path tools/pirb-historical-replay/Cargo.toml -- --input docs/fixtures/pirb-historical-replay-v0.1.json --source-commit "$(git rev-parse HEAD)" --rustc-version "$(rustc --version)" --cargo-version "$(cargo --version)"

The local example emits `UNPINNED_LOCAL_RUN` unless a commit SHA is supplied. Such a receipt is useful for exploration but is not an exact-head receipt. The dedicated workflow checks out and verifies the exact tested SHA, independently recomputes the input/schema/lock digests, and archives the synthetic input with the receipt.

## Input contract and fail-closed checks

The Draft 2020-12 schema and Rust semantic validator reject unknown fields, including person-level identifiers; duplicate case and community-period observations; misordered or overlapping temporal boundaries; records outside the declared window; future-dated evidence relative to the decision; alerts before decisions; invalid outcome chronology; and claimed verified responses without ordered completion/verification timestamps and an independent evidence reference.

Every record represents one opaque aggregate community-period, not a person or household. Feature evidence has its own availability time and must have been available at or before the decision time. Outcome labels are kept separate from decision-time evidence and must have become available after the decision. Each dataset declares one fixed outcome-follow-up horizon: harm must occur inside it, and a no-harm label requires the full horizon to have elapsed. Unknown outcomes stay unknown; they are never recoded as no harm. Unavailable, stale, and conflicting evidence remain visible in coverage indicators instead of being silently upgraded to verified.

## Metrics

Results are reported separately for the strictly ordered training, calibration, and held-out test time windows. The evaluator does not fit or tune a model in any of these windows; the split is used to prevent random shuffling and to make the primary test period explicit.

Metrics include known-outcome coverage, unknown outcomes, alerts with unknown outcomes, timely-warning precision, recall, false alerts per 100 known community-periods, median lead time for timely alerts, late alerts, evidence-insufficient records, unassessed scope, and independently verified response completion. This input records binary alerts, not probabilities, so probability calibration is explicitly unavailable. Per-community errors are emitted only when the governed minimum subgroup denominator is met; the number of suppressed communities remains visible.

Definitions are operational and deliberately narrow: an alert before a recorded harm is timely; an alert at or after that harm is counted as a late alert and a missed harm; an alert in a no-harm observation window is a false alert. A verified response means the input claims independently verified completion and includes the required chronology/reference. The evaluator checks structure and chronology, not whether the evidence source is truthful or genuinely independent.

## Data authorization, privacy, and claim ceiling

The input must be synthetic or appropriately authorized historical evidence. For an authorized-historical input, the contract requires non-synthetic authority, protocol, privacy-review, and stewardship references. These are assertions to support an audit trail; the software cannot authenticate the authority, validate consent, or confer permission to use data.

Do not commit real field records to this public repository or upload a receipt from protected data to a public workflow. A digest of a small or guessable dataset can itself reveal information through confirmation attacks. Run sensitive evaluations in the authorized data controller's environment and control both input and receipt access; publish only a reviewed aggregate report or a governed commitment when appropriate. A minimum cell-size threshold is not, by itself, a complete privacy guarantee.

A passing receipt establishes input/schema conformance and descriptive retrospective replay only. It does not establish community legitimacy or consent, data truth, safety, causal attribution, reduced violence, or deployment readiness. This is a measurement-only research tool, not a global peace score, person-level violence-risk score, targeting system, or automated aid/rights decision mechanism.
