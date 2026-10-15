# Maritime Source Capture Runbook v0.1

**Purpose:** make one small real maritime dataset capture reproducible without confusing a declared digest with source authenticity, semantic correctness, licence permission, or operational admission.

**Status at 2026-10-15:** source-selection research is complete enough to name a first candidate, but no raw UNCTADstat data bytes were captured by this execution. Do not claim a source-byte hash, committed real-data fixture, or successful real-source verification until the steps below are completed against an actual downloaded file.

## First candidate: UNCTADstat PLSCI

Use the official [Port Liner Shipping Connectivity Index, quarterly (PLSCI)](https://unctadstat.unctad.org/datacentre/reportInfo/US.PLSCI), table identifier `US.PLSCI`, as the first discovery candidate.

Pin these semantics alongside the artifact:

- Spatial level: port, not country, terminal, berth, vessel, or individual port call.
- Temporal frequency: quarterly. The available observations are not complete for every port and period.
- Indicator: `Index (average Q1 2023 = 100)`; it is an index, not a count, capacity, probability, or physical measurement.
- Methodology vintage: the series was rescaled in 2024 so that 100 represents the average port connectivity in Q1 2023. Do not silently compare this with the prior scale.
- Source basis: shipping-line schedules and network components. It is suitable for comparative network baselines and labelled synthetic scenarios, not live berth availability, current vessel position, proof of an individual port call, cargo custody/delivery, route safety, or navigation advice.
- Reuse terms: UNCTAD's [Data Hub terms](https://unctadstat.unctad.org/EN/About.html) and [FAQ](https://unctadstat.unctad.org/EN/FAQ.html) state CC BY 3.0 IGO and allow redistribution of data/metadata with source attribution. Preserve the exact terms page/file reviewed and its digest in the capture record. This does not constitute legal advice or override any dataset-specific notice.

The South African slice should be modest: select only the desired quarters and port rows through the official table's own export controls. Save the selection/filter specification with the capture. Do not copy values from a secondary display as a substitute for the official export. Preserve port labels and publisher codes exactly as supplied; a name is not a validated UN/LOCODE mapping.

### Retrieval limitation recorded

On 2026-10-15, the official PLSCI metadata and UNCTAD Data Hub terms were reviewable, but the data-table page returned 403 to the available retrieval channel and the direct CSV/API representation was not accessible through that channel. An official UNECE production UN/LOCODE download was also attempted as a fallback, but the ZIP could not be materialized in this execution. Consequently **no raw table or registry bytes, digest, or real port mapping is claimed here**. Repeat the capture from a permitted environment using the source's official download UI or documented endpoint.

## Capture procedure

1. Open the official PLSCI table and record the visible table ID, selected dimensions/filters, indicator label, latest included quarter, methodological notes, and source/coverage notes. Download using the official UI or documented endpoint. Do not guess a hidden export URL.
2. Preserve the downloaded file without parsing, re-encoding, trimming, re-sorting, or line-ending normalization. Keep raw bytes separate from any filtered, cleaned, or normalized derivative. If the export itself is filtered server-side, record every filter and treat that exact response as the captured artifact.
3. Save request/response evidence privately: exact requested URI, final effective URI, redirect chain, retrieval start/end in UTC, HTTP status, reported content type, reported transfer size, tool/version, and the server headers actually returned. Do not publish bearer tokens, cookies, credentials, or expiring signed URLs. If the original request evidence contains secrets, retain it in a restricted location and publish a sanitized URI plus an explicit redaction note.
4. Capture the specific terms/metadata pages reviewed as separate files where permitted. Record stable page URI, retrieval UTC, and SHA-256 of each saved file. The licence name/URI alone is not proof that the correct terms text was reviewed.
5. Compute byte length and SHA-256 of the exact downloaded payload using an independent utility. For example, on a GNU/Linux machine, after setting `ARTIFACT_URL` to the exact HTTPS URL exposed by the official download control:

   ```sh
   set -euo pipefail
   umask 077
   capture_dir='private-maritime-captures/unctad-plsci-2026-10-15'
   mkdir -p "$capture_dir"
   date -u +'%Y-%m-%dT%H:%M:%SZ' > "$capture_dir/retrieval-started-at-utc.txt"
   curl --proto '=https' --proto-redir '=https' --location --max-redirs 5 \
     --fail --show-error --silent \
     --dump-header "$capture_dir/response-headers.txt" \
     --output "$capture_dir/source-artifact.bin" \
     --write-out 'effective_url=%{url_effective}\nhttp_code=%{http_code}\ncontent_type=%{content_type}\nsize_download=%{size_download}\n' \
     "$ARTIFACT_URL" > "$capture_dir/transfer-summary.txt"
   date -u +'%Y-%m-%dT%H:%M:%SZ' > "$capture_dir/retrieval-completed-at-utc.txt"
   wc -c < "$capture_dir/source-artifact.bin" > "$capture_dir/byte-length.txt"
   sha256sum "$capture_dir/source-artifact.bin" > "$capture_dir/sha256.txt"
   ```

   Do not add `--compressed`, transform the saved payload, or hash a parsed representation. Verify that the HTTP status is successful and that the final content type and downloaded file match the expected export before proceeding. A successful transfer of an HTML error page is not a data capture.

6. Create a v0.1 data receipt and source manifest with the exact observed values—not manually transcribed guesses. The receipt must identify UNCTAD as publisher, table `US.PLSCI`, the official source and request URIs, retrieval timestamp, HTTP status/content type, exact file byte length/digest, indicator and methodology semantics, observed period/coverage, attribution, terms review evidence and restricted claim ceiling. A locally assigned capture identifier must be labelled as such; do not present it as an official UNCTAD revision.
7. Run the Rust receipt verifier against the saved file and compare its SHA-256/length with the independent command output. Keep the original payload immutable. If building a crosswalk, run `--verify-port-crosswalk-snapshots` against the exact source export and a separately captured UN/LOCODE **production** release file, not the continuously changing pre-release.
8. Review port identity separately. A successful digest match proves byte equality only for the bytes presented. It does not resolve port/terminal granularity, authenticate the publisher beyond captured transport/release evidence, validate the statistical meaning, or grant operational authority. Unproven port names remain ambiguous/unresolved; missing rows stay missing, not zero.
9. Run exact-head formatting, unit tests, positive CLI checks, negative digest-mismatch controls, and Clippy. Bind evidence to the tested commit SHA. Queued, missing, cancelled, or stale-head runs are not passes.

## Suggested public capture manifest

Keep the manifest small and free of credentials. At minimum record:

- capture ID and manifest schema version;
- publisher, table/series ID, stable source page URI and exact export URI (unless it contains a secret);
- dataset semantics, dimensions, applied filters, indicator units/scale, methodology vintage, temporal coverage and known missingness/coverage limitations;
- retrieved-at UTC (plus start/end if available), retrieval method/tool version, HTTP status, effective URI, content type, byte length, SHA-256 algorithm and digest;
- references/digests for the saved terms and metadata files, reviewer identity, review time, licence, attribution text, and any redaction note;
- exact receipt path/hash, verification command, tested source commit, and final status;
- claim ceiling: `NotAdmitted`; comparative network context/synthetic-scenario input only. No navigation, berth availability, vessel movement, port-call, custody, delivery, safety or operational claims.

The dedicated [PLSCI profile schema](./MARITIME_PLSCI_PROFILE_V0_1.schema.json) and Rust guard now enforce this series' semantics separately from the generic receipt: port-level, quarterly, index points, benchmark period `2023-Q1` with value `100`, methodology vintage `rescaled-2024-six-component`, and an explicit six-component definition. The generic receipt should use product/domain `other` with unit `dimensionless`; the profile gives the unit its index-specific meaning. A receipt-bound synthetic example and 20-case hostile corpus are in `crates/tools/maritime-data-receipt/tests/fixtures/synthetic-plsci-profile.json` and `plsci-profile-corpus-v0.1.json`. Verify them with:

```sh
cargo run --locked -p maritime-data-receipt -- \\
  --validate-macro-connectivity-profile \\
  crates/tools/maritime-data-receipt/tests/fixtures/synthetic-plsci-profile.json \\
  crates/tools/maritime-data-receipt/tests/fixtures/synthetic-plsci-receipt.json
```

The sample is invented test data only. The current profile validates metadata and exact receipt/artifact bindings; it does **not** parse the source export, validate every row/value, authenticate the publisher, establish a location mapping, or admit operational use. Do not weaken the contract to fit a source file. If a future source release changes scale, methodology, spatial level, frequency, or estimate conventions, add a versioned profile and explicit migration rather than mutating the pinned vintage.

## Acceptance gate

A real-capture milestone is complete only when all are true:

- the exact source file is present and its origin/download route and applied filters are recorded;
- byte length and digest independently reproduce from the preserved file;
- source and terms/metadata snapshots have evidence and attribution;
- a receipt verifier passes against the same file and explicitly remains `NotAdmitted`;
- changed bytes and mismatched declared digests fail closed;
- the source's statistical scope, methodology and port-level coverage are preserved;
- any port-to-UN/LOCODE relation has independent evidence and unresolved/ambiguous states are preserved;
- all reported CI evidence is tied to the exact tested head SHA.

No real values or digest should be checked into the repository solely to make a fixture appear complete. A real source capture can be retained outside the public repository until exact terms, scope and publication choices have been reviewed.
