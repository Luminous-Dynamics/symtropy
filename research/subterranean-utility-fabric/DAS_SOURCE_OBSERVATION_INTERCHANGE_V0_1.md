# Vendor-Neutral DAS Source-Observation Envelope v0.1

**Workstream:** DAS-SOURCE-INTERCHANGE-QUAL-001 (#5004)  
**Owners:** Symtropy #4972 and #5004; Sol-Atlas #755 for spatial representation; #5003 for retention  
**Status:** schema and synthetic fixture proposal; not a vendor adapter or API implementation

## 1. Design rule

The interchange envelope describes a source observation and references the exact acquired waveform artifact. It does not normalize raw waveform bytes into a new universal DAS signal format.

Vendor-native source artifacts remain immutable when available and permitted. Each adapter records what the source actually supplied. Missing metadata uses explicit unavailable/withheld/unknown states rather than invented defaults.

## 2. Scope

The envelope carries:
- source artifact URI, media type, byte length, SHA-256 and acquisition/binding status;
- SensorThings-aligned sensor host, sensor, datastream and observed-property references;
- exact vendor/model/firmware/interrogator configuration and optical-path identity;
- time interval, result time, clock source and synchronization quality;
- signal phenomenon and unit semantics;
- a typed sample axis: optical range, vendor channel index or calibrated physical fibre chainage;
- fibre identity/revision/endpoints and route revision when known;
- raw/normalized/processed/feature stage and parent artifact digests;
- quality, availability, epistemic, coverage, rights and disclosure states.

## 3. Deliberate non-equivalences

- Optical range is not physical fibre chainage.
- Physical fibre chainage is not route station.
- Route station is not geographic position.
- Geographic position is not owner asset attribution.
- Channel pitch is not spatial resolution.
- Gauge length is not channel pitch.
- Strain, strain-rate, phase, amplitude, temperature and electrical current are separate observed properties with separate units and processing semantics.
- A content digest detects byte changes relative to an acquired reference; it does not authenticate the publisher or prove physical truth.

A route/geographic transform is a separate Sol-Atlas/Symtropy calibration result, not an implicit field in this raw source envelope.

## 4. Raw versus derived

Raw vendor-native observations have no parent processing artifact. Normalized/processed/feature derivatives require one or more exact parent digests and a transformation parameter digest. A derivative is never presented as raw.

Any downsampling, filtering, denoising, windowing, normalization, compression or feature extraction creates a derivative with lineage. Original source data are not overwritten.

## 5. Relationship to standards

- SensorThings API 2.0 provides a reusable observation access/data model. This profile is a local source-artifact envelope; it is not a SensorThings conformance result.
- OMS/SOSA semantics describe observed property, procedure, time and feature-of-interest relationships.
- MUDDI describes underground feature/network concepts; the MUDDI Core A.1–A.4 conformance effort remains separately incomplete.
- PROV-O or equivalent records the source/derivation activity.
- HDF5, Zarr or vendor-native formats are candidate waveform object containers, not mandatory universal payload encodings.

## 6. Validation layers

1. JSON Schema 2020-12 structural validation.
2. Semantic validator for units, axis kind, property compatibility and raw/derived parent rules.
3. Byte verification for source/derivative digests after bytes are acquired.
4. Authorization and disclosure policy enforcement.
5. Vendor-specific adapter conformance to original source documentation.
6. Independent field or lab qualification of any physical interpretation.

No one layer substitutes for another.

## 7. Claim ceiling

The schema and synthetic example are an interchange proposal. They do not prove that any vendor supports this envelope, that source bytes have been acquired, that the digest represents real bytes, or that any physical event has been detected.
