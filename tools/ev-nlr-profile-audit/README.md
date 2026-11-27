# EV NLR Profile Archive Audit v0.1

A standalone Rust validator and matched-output comparator for the National Laboratory of the Rockies (NLR) public 15-minute heavy-duty depot-charging profile archive.

## Source and scope

- NLR data catalog: https://data.nlr.gov/submissions/162
- Dataset DOI: 10.7799/1787031
- Upstream generator repository: https://github.com/NatLabRockies/hdev-depot-charging-2021
- Pinned upstream source revision: 61530df51f6cd7d64f014ebbf8714cef24c8ca70
- Original study: Borlaug et al., *Heavy-Duty Truck Electrification and the Impacts of Depot Charging on Electricity Distribution Systems*, Nature Energy (2021), DOI https://doi.org/10.1038/s41560-021-00855-0
- Upstream source license: BSD 3-Clause. This adapter does not vendor the NLR source code or archive data.

The catalog describes 81 modeled 24-hour profiles at 15-minute resolution:

- three fleet vocations: beverage delivery, warehouse delivery and food delivery;
- fleet sizes 10, 50 and 100;
- charging strategies immediate 100 kW, delayed 100 kW and minimum power;
- profile summaries average, minimum-peak and maximum-peak.

Each profile must contain exactly 96 intervals. Expected filenames follow the upstream generator convention:

    <fleet>_<size>vehs_<avg-prof|min-prof|peak-prof>_<immediate|delayed|min_power>_15min.csv

The expected CSV header is time,avg_power_kw.

**Important:** These are modeled aggregate load profiles generated from published operating schedules and declared assumptions. They are not 81 measured charging sessions, measured battery trajectories, or direct substation observations. The reference archive is accepted as operator-published data; this tool validates structure and identity, not publisher authenticity or physical truth.

Report schema: tools/ev-nlr-profile-audit/schemas/ev_nlr_profile_audit_report_v0_1.schema.json.

## What the auditor does

1. Requires the complete 3 × 3 × 3 × 3 = 81 profile matrix. Missing and unexpected CSVs are errors.
2. Requires the 96 exact timestamps from 00:00 through 23:45 in 15-minute increments, with no missing, duplicate or shifted interval.
3. Rejects malformed CSV columns, non-finite power and negative power.
4. Computes a deterministic SHA-256 for each raw CSV and a sorted archive-content digest over filenames and per-file hashes.
5. Reports minimum, mean, peak and daily integral (kWh) per profile.
6. Verifies that the average profile peak does not exceed the selected maximum-peak profile for the same fleet, size and strategy. It does **not** require the selected minimum-peak profile to be pointwise below the average profile: the NLR minimum profile is the sampled day with minimum maximum demand, not an interval-by-interval lower envelope.
7. Optionally compares a candidate archive against the reference using 7,776 matched interval values:
   - MAE and RMSE in kW;
   - P95 absolute interval error;
   - mean signed bias;
   - daily energy difference per profile;
   - signed and absolute net whole-archive daily-energy error;
   - sum of absolute per-profile daily-energy errors (so opposing errors cannot hide);
   - maximum absolute profile peak-demand error.

All candidate files are revalidated against the same expected file set and time grid before metrics are computed.

## Tests

The unit suite builds deterministic temporary archives containing all 81 profiles and checks:
- the 81 × 96 profile/interval contract;
- exact zero error for identical candidate/reference archives;
- exact 0.5 kW MAE and bias for a candidate with a constant +0.5 kW offset;
- a positive/negative cross-profile bias control proving that net daily-energy error can cancel while the sum of per-profile absolute errors remains non-zero;
- rejection of timestamp drift, missing profiles, negative and non-finite power;
- rejection when the average profile's peak exceeds the maximum-sample profile peak.

## Run

    cargo fmt --manifest-path tools/ev-nlr-profile-audit/Cargo.toml --all -- --check
    cargo test --manifest-path tools/ev-nlr-profile-audit/Cargo.toml --all-targets
    cargo clippy --manifest-path tools/ev-nlr-profile-audit/Cargo.toml --all-targets -- -D warnings

    cargo run --quiet --manifest-path tools/ev-nlr-profile-audit/Cargo.toml -- \
      --reference-dir /path/to/extracted/nlr/15min-profiles \
      --source-revision 61530df51f6cd7d64f014ebbf8714cef24c8ca70 \
      --source-archive-sha256 VERIFIED_ZIP_SHA256 \
      --output nlr-reference-audit.json

To compare a candidate profile archive:

    cargo run --quiet --manifest-path tools/ev-nlr-profile-audit/Cargo.toml -- \
      --reference-dir /path/to/extracted/nlr/15min-profiles \
      --candidate-dir /path/to/candidate/15min-profiles \
      --candidate-id symtropy-depot-model-v0.1 \
      --candidate-evidence-class simulated \
      --source-revision 61530df51f6cd7d64f014ebbf8714cef24c8ca70 \
      --source-archive-sha256 VERIFIED_ZIP_SHA256 \
      --output nlr-candidate-comparison.json

Candidate evidence classes in v0.1 are restricted to estimate or simulated. Metered or field observations need a separate evidence contract with source identity, measurement boundary, calibration, timestamp, meter point and provenance before being admitted here.

## Interpretation and promotion

A successful audit means that the extracted archive matches the expected public profile contract and its internal summary checks. A comparison report means that the candidate's 15-minute values have been compared to those published profile values. It does not prove that the candidate used identical operating days, seeds, resampling, vehicle-energy assumptions, dwell windows or generator semantics.

A genuine **replication** must use the same pinned operation-day inputs, schedule segments, resampling with replacement, number of samples, random-seed derivation, 1.8 kWh/mile consumption assumption, 100 kW constant-power immediate/delayed strategies, minimum-power rule, middle-day extraction and 15-minute averaging. The reference archive does not contain all sample-level random draws needed to infer these settings from profile outputs alone. A candidate with unmatched workload or settings is a cross-model comparison, not a strict replication.

Report both the archive-validation result and the comparison metrics. Keep the 81 individual profile scores; a single global MAE can hide failure on one vocation, fleet size or charging strategy. Next qualification tiers should add:
- matched stratified metrics by fleet/vocation, fleet size, strategy and profile kind;
- energy-integral and peak-demand error alongside interval error;
- robustness to source-row, seed, and time-grid perturbation;
- held-out vocation and fleet-size transfer tests;
- a separate measured-site validation track with meter and feeder evidence.

No claim is made here about measured NLR charging outcomes, physical charger performance, distribution-grid feasibility, national fleet performance or South African transfer. NLR schedules and depot profiles are a U.S. research reference, not a local deployment dataset.
