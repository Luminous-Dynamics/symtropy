# Spacecraft unit-level conformal calibration v0.1

A standalone, dependency-free Rust calibrator/evaluator for correlated spacecraft pose observations. It consumes evaluator-derived per-frame SE(3) nonconformity scores in unsigned Q32.32 and calibrates them at a declared independent unit: TARGET, SESSION or SEQUENCE.

It composes with:
- #7782 / #7777: SE(3) Mahalanobis score convention and geometric prediction-region constructor.
- #7723: per-frame pose-region coverage, error and sharpness scorecard.
- #7786: split identity and translation-lineage auditor.
- #7715: synchronized frame identity.
- #7785 / #7800: domain and temporal-calibration research.

It does not estimate pose, calculate the per-frame nonconformity score, inspect imagery, or verify source bytes.

## Statistical method

For each scope and independent unit g, the calibration score is the maximum frame score:

\`S_g = max_i s_{g,i}\`.

If any frame is ABSTAINED, MISSING or INVALID_INPUT, the unit maximum is positive infinity. Such observations are not discarded from calibration.

For n calibration units and nominal coverage 1-alpha, the exact one-based split-conformal order-statistic rank is:

\`k = ceil((n + 1) * (1 - alpha))\`.

The implementation uses integer basis points for 50%, 80%, 90%, 95% and 99%, and integer Q32.32 scores. If k > n or the selected order statistic is infinite, there is no finite threshold; the result is not promotable.

The unit-level statement is marginal simultaneous coverage over a **new independent unit**: under exchangeability of calibration and future units inside the declared scope, every scored frame in the new unit is covered with probability at least the nominal level. It is not conditional per-target coverage, nor does it establish that declared sequences are actually independent.

## Matched frame-wise diagnostic

The evaluator also computes the conventional frame-wise quantile from all calibration frames. This is intentionally labelled a diagnostic: adjacent frames are often correlated, so treating them as independent calibration examples can invalidate the usual frame-level exchangeability argument.

Both thresholds are applied to the same test units. Reports include:
- frame-wise and unit-wise thresholds and exact ranks;
- test frames, scored frames, abstentions, missing and invalid records;
- conditional frame coverage (covered / scored);
- operational success (covered / all frames; abstentions/missing/invalid are failures);
- complete-unit simultaneous success count;
- units containing at least one silent miss;
- two-sided 95% Wilson intervals with independent units, never frames, as the trial count;
- separate promotion reasons for each nominal level.

The Wilson interval is an approximate score interval under independent, identically distributed unit outcomes; it is not a finite-sample distribution-free conformal guarantee. The code does not prove unit independence or exchangeability.

## Domain and horizon scope

A profile scope is keyed by:
- domain;
- acquisition condition;
- crop/truncation stratum;
- sensor modality;
- OOD group;
- calibration revision;
- fixed frame budget.

Every unit within a scope must contain exactly that many rows. This v0.1 restriction prevents silently applying a threshold calibrated on short sequences to longer horizons. Different frame budgets, sensor modes, illumination conditions or model/calibration revisions form separate scopes.

The unit ID must equal its declared target/session/sequence ID. Unit identities cannot cross CALIBRATION and TEST, including across scope buckets. Sample IDs must be unique. Malformed status/score combinations, empty metadata and incomplete units reject the entire manifest.

## Positive known-answer result

The frozen positive corpus contains:
- 30 calibration sequences;
- 35 held-out test sequences;
- 11 frames per sequence;
- one calibration-frame outlier with score 1,000 per sequence, other frames score 100;
- one test-frame score of 500 per sequence, other frames score 100.

At nominal 90%:
- the frame-wise calibration rank is 298 of 330 frames, so its threshold is 100;
- the unit-maximum calibration rank is 28 of 30 sequences, so its threshold is 1,000;
- the frame-wise threshold leaves one miss in every held-out sequence;
- the unit-wise threshold covers all 35 complete held-out sequences.

At nominal 95%, 35/35 complete sequences still do not make the Wilson lower bound reach 95%, so the held-out promotion gate correctly remains false. At nominal 99%, 30 calibration units cannot provide a finite split-conformal order statistic (rank 31), so the threshold is infinite/unavailable.

This is an authored synthetic fixture that verifies the algorithmic distinction. It is not measured spacecraft data.

## Reproduce

\`\`\`sh
cargo fmt --manifest-path tools/spacecraft-unit-conformal-calibration-bench/Cargo.toml -- --check
cargo test --locked --manifest-path tools/spacecraft-unit-conformal-calibration-bench/Cargo.toml
cargo clippy --locked --manifest-path tools/spacecraft-unit-conformal-calibration-bench/Cargo.toml --all-targets -- -D warnings

MANIFEST=tools/spacecraft-unit-conformal-calibration-bench/fixtures/sequence-conformal-positive-v0_1.tsv
DIGEST=$(sha256sum "$MANIFEST" | cut -d' ' -f1)
cargo run --quiet --locked --manifest-path tools/spacecraft-unit-conformal-calibration-bench/Cargo.toml -- "$MANIFEST" "$GIT_COMMIT" "$DIGEST"
python3 tools/spacecraft-unit-conformal-calibration-bench/tests/reference_oracle.py "$MANIFEST" "$GIT_COMMIT" "$DIGEST"
\`\`\`

The Python oracle is separately implemented, hashes the raw fixture bytes itself, and recomputes the full receipt. The workflow compares complete parsed JSON receipts and separately checks expected positive/negative outcomes.

## Profile integration

For a downstream profile builder, the unit-wise threshold, calibration unit count, order-statistic rank, independent-unit kind, frame budget and held-out complete-unit Wilson lower bound are the relevant fields. The calibration unit count—not frame count—is the finite-sample n.

The current SE(3) region consumer (#7782) accepts the quantile, sample count, order-statistic rank and held-out coverage lower bound. It will need a versioned profile extension to bind unit kind, unit-manifest digest, exact horizon/frame budget and unit-max score construction before a production profile can consume this output. Until that adapter exists, this crate emits research receipts and thresholds, not an authenticated deployable profile.

## Promotion target (not a result)

For every domain/horizon scope and nominal level:
- finite unit-max quantile exists;
- at least 30 independent held-out units are present;
- lower endpoint of the two-sided 95% Wilson interval for complete-unit coverage meets the nominal level;
- missing/abstained/invalid frames remain explicit failures;
- no score is pooled across unsupported domain or calibration revisions.

There is no weighted composite score. Coverage, simultaneous unit reliability, frame-wise availability and region sharpness are distinct. Region sharpness remains the responsibility of #7723.

## Claim ceiling

A passing synthetic benchmark establishes only the implementation's rank, grouping, arithmetic, receipt and held-out evaluation behavior on the frozen corpus. Its statistical interpretation requires exchangeable independent units inside the declared scope. The evaluator does not establish those assumptions, conditional coverage, unseen-target performance, physical sensor calibration, true orbital uncertainty or deterministic collision clearance. No result authorizes a maneuver.
