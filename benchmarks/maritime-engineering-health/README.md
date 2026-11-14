# Maritime engineering-health evaluator

Standalone offline Rust evaluator for the synthetic corpus at
benchmarks/maritime/engineering-health-corpus-v0.1.json.

## Run

From the repository root:

~~~sh
cargo fmt --manifest-path benchmarks/maritime-engineering-health/Cargo.toml -- --check
cargo test --locked --all-targets --manifest-path benchmarks/maritime-engineering-health/Cargo.toml
cargo clippy --locked --manifest-path benchmarks/maritime-engineering-health/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path benchmarks/maritime-engineering-health/Cargo.toml -- benchmarks/maritime/engineering-health-corpus-v0.1.json
~~~

The CLI emits the SHA-256 of the exact input bytes, an expected-versus-actual
result for each case, and an asset/configuration-bound capability-impact projection.
Evidence admission computes age from case decision_time_ms minus source
observed_at_ms, rejects future-dated observations, compares the computed age
with max_age_ms, and requires a monotonic-and-UTC-bound clock. A source cannot
be admitted merely because it carries a quality label of "current".

`decision_time_ms` and `observed_at_ms` are milliseconds on the declared
vessel UTC-mapped clock domain. `after_ms` is the ordered scenario-tape offset;
it is not subtracted from the UTC timestamp. UTC-only, conflicted or unknown
clock quality is not sufficient for current decision evidence.
The projection is read-only: it cannot authorize a transition or clear a restriction.
"No restriction indicated" applies only to the tested engineering-health evidence;
it is not a vessel-wide ODD pass. The CLI exits non-zero for any mismatch.

## Claim ceiling

A PASS proves only that this reference evaluator reproduces the frozen synthetic
contract outcomes. It does not establish machine diagnosis accuracy, machinery
health, safety, physical behavior, restoration authorization, class/statutory
compliance, MASS Code compliance or permission to operate a vessel. The crate has
no network, OT, controller, protection-system, actuator or equipment effects.

## Aalto marine-engine dataset audit

A separate Rust CLI inventories and audits an already-extracted copy of the public
Aalto Marine Engine Fault Dataset. It is intentionally not an online downloader
and does not extract untrusted ZIP paths.

First verify/extract the licensed archive in a controlled temporary directory,
then run:

~~~sh
cargo run --locked \
  --manifest-path benchmarks/maritime-engineering-health/Cargo.toml \
  --bin marine_engine_dataset_audit -- \
  /path/to/extracted-dataset-root \
  benchmarks/maritime/marine-engine-fault-dataset-v1.manifest.json \
  --archive /path/to/Marine_Engine_Fault_Data_v1.zip \
  --output /path/to/marine-engine-dataset-audit-v0.1.json
~~~

The dataset root may be the extracted parent directory or the
Marine_Engine_Fault_Data directory itself. The CLI:
- rejects path traversal and caps each input member at 128 MiB;
- computes SHA-256 for every inventoried file and for the archive when supplied;
- attempts publisher-MD5 verification using md5sum, md5 or OpenSSL when available;
- parses scenario/baseline CSV files with a strict UTF-8 RFC-4180-style reader;
- validates the three header rows and 73-column scenario / 70-column baseline layouts;
- records row counts, binary anomaly labels, 0-to-1 onset transitions, both anomaly-only
  injector-nozzle files, relative-time backsteps/duplicates and per-channel missing cells;
- hashes and UTF-8 checks the metadata files, and generically parses metadata CSVs;
- emits a file-level split-manifest template that keeps every fault recording unassigned and marks the single baseline recording as reference_only.

Absolute timestamp empties and duplicates are reported separately; the publisher MD5 and ZIP signature are hard integrity gates. Extracted member bytes are hashed individually, but the tool does not prove that a specific extracted directory was produced from the supplied archive unless the extraction process is independently recorded.

The tool does not infer root-cause labels from the binary anomaly-state column,
impute missing measurements, silently reorder time-series, or assign fault recordings
to train/validation/test partitions. The baseline file remains reference_only. The dataset-index and variable-dictionary semantics still require
human reconciliation after payload inspection. An incomplete audit exits non-zero.
The report schema is
`docs/tech/MARITIME_ENGINE_DATASET_AUDIT_REPORT_V0_1.schema.json`. The report includes warnings for duplicate timestamps, incomplete header metadata and the fact that the auditor does not prove the extracted tree came from the supplied ZIP.

The tool's `content_audited_pending_independent_review` state means all declared
members were present, the archive had a recognized ZIP signature, its MD5 matched
the publisher's value, and the specified structural checks completed. It still
does not cryptographically bind the extracted directory to that archive; preserve
the controlled extraction record separately. It does not
mean that the archive is an operationally representative fleet dataset, that
the published labels are physically adjudicated, or that an independent marine
engineer has accepted the interpretation.

## Programme roadmap and additional tracks

- Unified architecture and benchmark gates: `docs/tech/MARITIME_ENGINEERING_INTELLIGENCE_ROADMAP_V0_1.md`.
- Transfer and continual learning: Symtropy issue #4510.
- Data trust and evidence-preserving reconstruction: Symtropy issue #4535.
- RUL and maintenance decision utility: `docs/tech/MARITIME_PROGNOSTICS_BENCHMARK_V0_1.md`, forecast contract schema `docs/tech/MARITIME_RUL_FORECAST_V0_1.schema.json`, issue #4545.
- Narrative causal risk from incident/maintenance records: issue #4548.
