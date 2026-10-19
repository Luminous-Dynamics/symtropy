# EV Session Audit v0.1

Standalone Rust quality audit and descriptive-statistics tool for EV charging-session JSON exports with ACN-Data-style fields. This is not a charging controller or scheduler benchmark.

## Research data sources

- **ACN-Data (Caltech/PowerFlex)** documents `connectionTime`, `disconnectTime`, `doneChargingTime`, `kWhDelivered`, `siteID` and other session fields. Access to the REST API requires registration; use data only under the provider's terms. https://ev.caltech.edu/dataset
- A static research repository has site/session and time-series exports from 2018–2020; inspect its README and license before reuse. https://github.com/tongxin-li/ACN-Data-Static
- The NLR heavy-duty depot dataset has 81 **modeled** 15-minute load profiles covering fleet types, fleet sizes and charging strategies. These are not individual measured sessions and require a separate adapter. https://data.nlr.gov/submissions/162
- NLR depot-based vehicle operations give background on medium/heavy-duty dwell and duty cycles. They are not a directly interchangeable charging-session corpus. https://data.nlr.gov/submissions/231

## Run

```sh
cargo test --manifest-path tools/ev-session-audit/Cargo.toml --all-targets
cargo run --quiet --manifest-path tools/ev-session-audit/Cargo.toml -- \
  tools/ev-session-audit/fixtures/acn_style_synthetic_v0_1.json \
  --dataset-id synthetic-fixture-v0.1 \
  --source-uri https://ev.caltech.edu/dataset \
  --evidence-class synthetic
```

The source URL and evidence class are declared by the caller and not authenticated. The example URL does not make the synthetic fixture provider data.

Preserve the unmodified source export and record its SHA-256, retrieval date, license/access terms, exact site/time filters and export version. Do not commit restricted or raw session records.

## Outputs

Reports total and per-site valid session counts, delivered energy total and distribution, connection dwell duration, active charging duration, post-charge occupancy, and invalid-row indexes/reasons. P50 is the median; P95 uses nearest rank. Missing done-charging times remain unavailable and are counted, not imputed.

Input row payloads, source row IDs, session IDs, user IDs and optional user-entered request fields are not copied to the report. Invalid rows are not silently dropped; valid plus invalid rows reconcile to total input rows. Site IDs are emitted to permit aggregation, so map/remove them before publication when appropriate.

## Limits

- RFC 1123 GMT/UTC timestamps are accepted. Other offsets are rejected rather than guessed.
- Metadata provenance is caller-declared and not independently authenticated.
- Workplace sessions cannot establish heavy-duty fleet or industrial depot performance.
- Session data alone do not reveal feeder capacity, EV acceptance curves, SOC, battery state of health, true charger availability, mission feasibility or causality.
- This is not a protocol certification, metrology audit, electrochemistry model, full life-cycle assessment or evidence that a scheduling policy is superior.
