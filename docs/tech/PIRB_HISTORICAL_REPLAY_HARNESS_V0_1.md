---
title: PIRB Historical Replay Harness
version: 0.1
status: research-stage contract
authority: MeasurementOnly
---

# PIRB Historical Replay Harness v0.1

## Purpose

Evaluate the historical operating characteristics of a warning-to-response process without conflating software conformance with field effectiveness. The harness scores recorded alerts against separately recorded, later-available labels and reports the parts that remain unknown. It does not create alerts, learn a predictor, recommend an intervention, or authorize a live action.

This is a separate increment from the synthetic PIRB reference evaluator (v0.1), outcome-governance contract (v0.2), and exact-head governance receipt. It consumes aggregate historical records; it does not replace Mycelix authority, consent, mediation, Justice, or Commons service records.

## Unit of analysis and time model

One input record is one opaque community-period observation. There are no person or household rows. A record contains:

- A decision timestamp and zero or more evidence references, each with its own availability timestamp and verification state.
- An optional recorded alert issuance timestamp.
- An outcome label of harm, no harm, or unknown, plus the time the outcome occurred (for harm) and the time the label became available.
- A response status and, where asserted, completion time, independent verification time, and evidence reference.
- A flag indicating whether the declared scope was assessed.

The decision time is the information cutoff. Every evidence item used by that record must have been available at or before it; otherwise the dataset is rejected for look-ahead leakage. Outcome labels are evaluation targets, not decision inputs, and must become available after the decision. Unknown labels have no outcome timestamps and never count as no-harm.

The dataset declares a time window with exclusive boundaries:

1. Training period: evaluation start through (but not including) training end.
2. Calibration period: training end through (but not including) calibration end.
3. Held-out test period: calibration end through (but not including) test end.

The boundaries must be strictly ordered, the test end cannot be later than the evaluation-as-of time, and each record must fall into exactly one period. The tool does not train or tune a model inside these splits; they are chronological reporting windows, not a claim that temporal generalization has been proven.

## Metric semantics

For each split, report totals, known/unknown label coverage, alerts with unknown outcomes, unassessed scope, evidence-insufficient records, alerts issued, verified response completions, confusion counts, and warning lead time.

- **Timely true positive:** alert issuance precedes the recorded harm.
- **Missed harm:** a harm label with no alert or with an alert issued at/after the harm.
- **Late alert:** an alert at/after harm; it is separately counted and still contributes to missed harms.
- **False alert:** an alert in a known no-harm observation window.
- **True negative:** no alert in a known no-harm observation window.
- **Timely-warning precision:** timely true positives divided by all alerts with known outcomes, including late alerts in the denominator.
- **Recall:** timely true positives divided by all known harm cases.
- **False-alert burden:** false alerts per 100 community-period records whose outcomes are known.
- **Median warning lead time:** median elapsed seconds between timely alert issuance and harm in true-positive records only.
- **Verified response completion:** records that assert independently verified completion divided by all records with an alert. It is a process measure, not evidence the response prevented harm.

A result of zero alerts, zero known outcomes, or zero known harm cases yields a null rate rather than a fabricated zero. The receipt explicitly marks probability calibration unavailable because this contract accepts binary alerts and no predicted probability.

## Distribution, missing coverage, and privacy

Metrics are calculated from known outcome labels only; unknown labels are counted and excluded from the confusion matrix rather than imputed as negative. The report gives overall held-out test metrics plus per-community test errors for groups at or above the dataset's declared minimum known-outcome denominator. Small groups are suppressed, and the count of suppressed groups remains visible. That threshold must be selected by the responsible data-governance process; it is not a universal privacy guarantee.

Community identifiers must be opaque group references, and publication must not contain individual, household, sensitive testimony, or precise vulnerable-location data. Even input digests can leak information about small or guessable datasets. Public CI uses only the checked-in synthetic fixture. Do not upload protected real-data inputs or their raw SHA-256 receipts into public CI or public artifacts.

## Response lifecycle and evidence limits

A claimed independently verified completion requires an alert, completion after alert issuance, verification at/after completion, both timestamps within the dataset's evaluation-as-of boundary, and a non-empty independent evidence reference. The evaluator validates fields and chronology. It does not authenticate the issuer, prove the verifier's independence, or establish that a delivered response improved safety.

A warning is not a dispatch; a dispatch is not completed service; a completed response is not proof of prevented harm. Conflicting and stale evidence are counted as evidence-insufficient, not silently upgraded.

## Governance for historical data

For `authorized_historical` input, the dataset must cite its accountable authority, approved evaluation protocol, privacy review, and data steward, with non-synthetic references. The software only checks those fields' form. The data controller and relevant local authorities must independently establish legal basis, consent/authorization, purpose limitation, safeguarding, retention, access controls, and an appropriate evaluation design before the data are processed.

The receipt retains a digest of the exact input. For sensitive data, keep the input and receipt in the authorized environment. A public hash can be a confirmation oracle when someone can guess the underlying data. A public-facing release should go through independent privacy and disclosure review and publish only approved aggregates or an appropriate governed commitment.

## Exact-head receipt and claim ceiling

The deterministic JSON receipt binds the input bytes, checked-in schema, dependency-lock bytes, evaluator source commit (when supplied), Rust/Cargo version strings, time boundaries, coverage, and metrics. The CI workflow checks the actual commit against the event SHA and independently compares the input, schema, and lock digests. A local run marked `UNPINNED_LOCAL_RUN` is not an exact-head audit.

Permitted claim: the specified dataset passed structural and semantic validation, and the stated descriptive metrics were computed over the supplied historical records with declared splits and missingness.

Not established: authenticity of authorization; truth or representativeness of records; community consent or legitimacy; effectiveness or safety of any intervention; causal impact on violence or peace; deployment readiness; or permission to automate a live response.

## Acceptance targets

- Zero accepted evidence timestamps later than decision time.
- Zero split-boundary overlap or record misassignment.
- Zero unknown outcome labels silently recoded as no harm.
- Zero verified-response status without the required chronology and independent evidence reference.
- Exact match of input, schema, and dependency-lock digests in a passing exact-head receipt.
- All required confusion and response counts independently recomputable from the aggregate fixture.
- Small-community test results suppressed at the declared threshold.
- No live decision authority and no causal-effect claim emitted by the tool.

The included fixture is synthetic and only checks evaluator plumbing. It is not a field dataset, alert benchmark result, or policy recommendation.
