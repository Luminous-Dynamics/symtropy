# Environmental data acquisition fallbacks and source-use limits (2026)

**Status:** operational protocol; no complete Gobabeb source download is claimed by this document.
**Current constraint observed in the authoring shell:** DNS resolution failed for both `doi.pangaea.de` and `ftp.bsrn.awi.de`. The official PANGAEA landing page and its HTML preview were accessible through a separate research-browser route. This difference is a transport limitation, not evidence that the dataset is absent.

## 1. Do not confuse the accessible preview with the source file

The PANGAEA Gobabeb June 2025 record is:
- DOI: https://doi.org/10.1594/PANGAEA.983811
- Landing page: https://doi.pangaea.de/10.1594/PANGAEA.983811
- Publisher HTML preview: https://doi.pangaea.de/10.1594/PANGAEA.983811?format=html
- Publisher tab-delimited export: https://doi.pangaea.de/10.1594/PANGAEA.983811?format=textfile
- Publisher-listed other version / station-to-archive reference: `ftp://ftp.bsrn.awi.de/gob/gob0625.dat.gz`

The published HTML preview explicitly says it shows only the first 2,000 rows. It is useful for checking variable inventory, the displayed column order, station metadata and example values, but it is **not** a complete month and must not be treated as a complete raw source artifact or used to calculate monthly validation metrics.

The landing page documents 15 variables, including SWD, DIR, DIF, LWD, SWU, LWU, 2 m temperature, relative humidity, and station pressure. It notes fog/stratus days 4, 25 and 26 June 2025. Those annotations can inform a predeclared event-stratified evaluation, but only after timestamp conventions and a suitable event definition are reviewed.

## 2. Network/DNS fallback runbook

Try these routes in order, without relabeling a failure as an absent dataset:

1. **Primary machine download:** retrieve the full publisher TSV at the exact monthly DOI using PANGAEA's documented `?format=textfile` export. Keep the response bytes unchanged and hash them.
2. **HTTP content negotiation:** PANGAEA documents requesting `Accept: text/tab-separated-values` against the DOI URL. This is a protocol alternative, not a guarantee that an isolated runtime can resolve or reach the endpoint.
3. **Separate networked browser / workstation:** open the exact monthly PANGAEA record and use “Download dataset as tab-delimited text” to save the full source file. Check the downloaded filename, byte size and beginning/end of file. Do not save the “View dataset as HTML” preview as if it were the complete table.
4. **Offline source registration:** after the operator has acquired the complete text export, use [register_local_pangaea_source.sh](scripts/register_local_pangaea_source.sh) to preserve a copy, calculate its SHA-256 and create a manifest without network access. The manifest intentionally says the local source origin was operator-asserted, not independently authenticated. This proves byte integrity after registration, not how the bytes were originally obtained.
5. **Managed egress environment:** if all client endpoints are unreachable, run the network acquisition from an approved machine or managed runner with functioning DNS and access to PANGAEA; transfer the immutable source artifact and manifest back through a controlled channel. Do not bake the source data into a CI log or embed large raw files in Git.
6. **BSRN archive endpoint:** use the station-to-archive file only where the user has authorized access and understands the BSRN logical-record format. The direct FTP archive may require a read account. It is not a drop-in replacement for the PANGAEA TSV, and the two formats require separate parsers and provenance.

If the complete text export cannot be acquired through any allowed route, stop at the metadata/preview stage. Do not fabricate missing rows or infer full-month coverage from the preview.

## 3. Offline source-registration contract

`register_local_pangaea_source.sh DOI LICENSE_LABEL EVIDENCE_CLASS SOURCE_FILE OUT_DIR`:
- requires an exact PANGAEA DOI shape, explicit operator-reviewed license label and explicit evidence class;
- performs a minimal PANGAEA-metaheader / timestamp-shaped-row plausibility check;
- copies the original supplied bytes and writes a SHA-256/byte-count manifest;
- records `acquisition_method=offline_local_file_registration` and `source_origin_assertion=operator_asserted_not_independently_authenticated`;
- leaves the status at `source_downloaded_not_ingested`.

The helper is intentionally not a scientific parser. It does not authenticate the DOI-to-file relationship, check the full schema/columns, confirm timezone or integration semantics, interpret quality flags, normalize values, or validate physical plausibility. The test suite uses a synthetic fixture; it is not evidence that a real PANGAEA export has passed.

After registration, inspect the entire real metaheader and the exact table header before writing or running a source-specific adapter. Preserve the raw file. Write normalized output to a separate artifact with its own transformation revision, configuration digest, SHA-256, row count and missingness report.

## 4. Dataset-use rules are engineering constraints

The publisher's June 2025 page identifies the **Baseline Surface Radiation Network License 1.0 (BSRN-1.0)** and asks users to quote the dataset citation. BSRN's official release guidelines say:
- BSRN data are provided to external users for bona fide research at no cost and the station/dataset must be explicitly acknowledged with the corresponding dataset citation;
- direct commercial use (selling the data) is forbidden;
- WRMC data from the archive must not be passed to third parties without WRMC agreement, subject to the published-data exception; and
- when BSRN data are included in a wider release under that exception, they must be identified as BSRN data and information on obtaining the original source must be supplied.

Sources:
- Exact June 2025 dataset and its cited license: https://doi.pangaea.de/10.1594/PANGAEA.983811
- BSRN data-release conditions: https://bsrn.awi.de/data/conditions-of-data-release/
- PANGAEA text-export and content-negotiation guidance: https://wiki.pangaea.de/wiki/Data_Access_and_Reuse
- PANGAEA text metaheader documentation: https://wiki.pangaea.de/wiki/Metaheader

Do not assume that open source code licenses grant the right to redistribute measurement data or use them commercially. Keep the dataset-specific source artifact external to this repository unless the data-release terms and project use have been checked. Before a commercial deployment or redistribution, obtain the required permission or choose a data source whose license supports the intended use.

## 5. Evidence ceiling

The current branch contains acquisition contracts and synthetic tests. A successful local registration would mean only that the supplied bytes were copied and hashed consistently. It would not prove their origin. No complete June 2025 BSRN TSV, source-specific parser result, station-versus-POWER score, or commercial reuse permission is claimed.
