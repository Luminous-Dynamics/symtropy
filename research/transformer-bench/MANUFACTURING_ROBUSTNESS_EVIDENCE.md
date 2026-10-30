# Manufacturing robustness evidence envelope

**Status:** schema and validation contract with synthetic metadata only. It does not recompute robustness scores, authenticate evidence bytes, approve experiments or authorize release.

## Why this layer exists

The manufacturing acceptance screen establishes route/test-plan/calibration eligibility. Symthaea then ranks eligible candidates using candidate-specific measurement noise and finite model-discrepancy scenarios.

A candidate may nevertheless be statistically informative but unrepresentative of production, or may amplify confidence in a misspecified model. Symtropy carries the evidence lineage for two independent audit values: production representativeness and error-amplification risk.

## Input and output

Run:

```sh
cargo run --locked --manifest-path research/transformer-bench/Cargo.toml --bin manufacturing-robustness-evidence -- \
  research/transformer-bench/fixtures/manufacturing-robustness-evidence-synthetic.csv \
  /tmp/symthaea-robustness-candidates.csv
```

The validator requires:
- product/serial/lot identity and a frozen production-reference population;
- reference-population and candidate-descriptor SHA-256 fields;
- representativeness method, score, validation digest and at least five independent holdout lots;
- explicit group disjointness for reference and amplification audits;
- a frozen discrepancy-scenario set, amplification method/score/audit digest and at least five independent holdout lots;
- different model version identifiers for acquisition and amplification audit;
- positive measurement standard uncertainty and comparable cost units within the input file;
- an explicit eligibility state and a decision-receipt digest, cross-checked against the manufacturing-acceptance report for the same product ID.

It emits candidate ID, representativeness, amplification risk, measurement standard uncertainty, acquisition cost, cost unit, and the hard eligibility state for a Symthaea adapter. It reads the `manufacturing-acceptance` output and rejects an ELIGIBLE claim when that product is blocked or absent from the report. Only an independently accepted product with an ELIGIBLE candidate state maps to `eligible=true`; INELIGIBLE and UNKNOWN remain blocked.

## Important limits

This envelope validator does **not** calculate representativeness or amplification risk. It validates shape, bounds, and declared independence metadata. Separate frozen validation processes must compute these scores and produce the referenced evidence artifacts. Artifact SHA-256 syntax is checked, not the bytes themselves; use the evidence-receipt tool against a controlled artifact manifest and separately verify signer/authorization provenance.

The fixture is synthetic. The deliberately contrasting scores (0.95 representativeness / 0.05 risk versus 0.08 / 0.91) exist only to exercise rank reversal. They are not transformer factory statistics. Five holdout lots is a contract smoke-test minimum, not a statistically sufficient sample size for industrial qualification. Different model version strings do not independently prove auditor independence; inspect code, data lineage, configuration and digests.

## Required benchmark

Compare random, variance, shared-noise EIG, heteroscedastic EIG, finite-scenario worst-case EIG and the representative/de-amplifying proxy at matched candidate pools and cost budgets. Hold out product family, supplier, material lot, station, operating regime and campaign. Report worst-group decision regret, decision-relevant recovery, false-safe rate, interval coverage, error amplification, OOD detection, cost-to-decision, ineligible-selection attempts, and abstention. Report risk-coverage and cost-to-decision curves, not only mean accuracy.

## Research context

AISTATS 2026 research warns that information-maximizing designs can amplify error under model misspecification, and proposes acquisition balancing representativeness, informativeness and de-amplification. Symthaea's proxy is not a reproduction of R-IDeA:
- https://proceedings.mlr.press/v300/tang26d.html
- https://proceedings.mlr.press/v300/callahan26a.html

NISTIR 8620 (2026) identifies interoperability, VVUQ and cybersecurity as important manufacturing digital-twin challenges:
- https://doi.org/10.6028/NIST.IR.8620

This evidence contract does not establish NIST or ISO conformity.
