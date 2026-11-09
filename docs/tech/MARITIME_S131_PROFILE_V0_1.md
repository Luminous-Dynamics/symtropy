# IHO S-131 harbour-infrastructure profile adapter v0.1

**Status:** metadata-contract extension and synthetic tests only. This document does not decode S-131 payloads, validate an IHO product, establish a port's physical state, or authorize navigation, berth allocation, construction, or operations.

## Why S-131 belongs in this stack

The IHO Geospatial Information Registry identifies S-131 as **Marine Harbour Infrastructure**. Edition 2.0.0 is the first operational edition, dated 1 September 2026, based on S-100 Edition 5.2.0. The registry records the signed Feature Catalogue `Signed_131_FC_2.0.0` (uploaded 2 October 2026) and signed Portrayal Catalogue `Signed_131_PC_2.0.0` (uploaded 9 October 2026).

Authoritative registry: https://registry.iho.int/productspec/view.do?category=product_ID&domainS=ALL&idx=233&product_ID=S-131&searchValue=&statusS=5

IHO description: S-131 datasets describe port-facility layout and port-service availability. They are intended to supplement ENC and support berth-to-berth navigation and voyage planning; they do not carry the same geographic detail as ENC. S-131 therefore complements S-101 rather than replacing it.

## Adapter mapping in this profile

| Profile field | S-131 mapping |
|---|---|
| `product.code` | `S-131` |
| `product.s100_edition` | `5.2.0` for the current first operational edition; future profiles must pin the applicable registry value |
| `product.specification_edition` | `2.0.0` for the current first operational edition |
| `product.feature_catalogue_revision` | Exact signed Feature Catalogue revision identifier |
| `product.portrayal_catalogue_revision` | Exact signed Portrayal Catalogue revision identifier |
| receipt product/domain | `port_facility` / `port_facility` |
| vertical reference | Optional at the profile-envelope level; any feature-specific vertical semantics remain source-native and must be represented by an explicitly reviewed profile |
| measurement units | Only declared physical attribute units/counts: `m`, `cm`, `mm`, `ft`, `in`, `count` |
| service/capability values | Preserve native enumerations and feature semantics; do not turn code lists into synthetic units |

The profile pins Feature Catalogue and Portrayal Catalogue revision identifiers, but v0.1 does not yet bind the raw catalogue ZIP bytes to independent artifact receipts. That remains a required follow-on before claiming byte-level catalogue provenance or validating signed catalogue authenticity.

## Important semantic separation

S-131's port-service availability information is not equivalent to:
- a live berth reservation or berth occupancy feed;
- a confirmed port-call or terminal operating commitment;
- a real-time shore-power capacity reservation;
- a vessel-specific berth compatibility or manoeuvring assessment;
- official ENC navigation data, S-129 under-keel clearance, or a navigation clearance;
- engineering acceptance or permission to construct/modify a facility.

Keep product publication time, dataset epoch, feature validity, retrieval time and assessment time distinct. Preserve the source-native meaning of each feature/attribute and never infer that missing service information means the service is unavailable—or that a depicted service is currently deliverable.

## Frozen synthetic tests

The S-100 v0.1 corpus now contains a synthetic S-131 base profile and negative controls for:
1. valid metadata-only profile with no mandatory vertical reference or validity window;
2. mutable Feature Catalogue revision;
3. current-velocity units incorrectly assigned to a port-facility profile;
4. missing horizontal CRS;
5. public source with unreviewed terms;
6. attempted navigation-authority escalation.

The verifier still returns `METADATA_CONTRACT_VALID_NOT_ADMITTED` for a structurally valid profile. Receipt binding checks the exact generic receipt and raw artifact bytes; the verifier does not interpret S-131 feature content.

## Next qualification steps

1. Retrieve the exact S-131 Edition 2.0.0 Product Specification, signed Feature Catalogue, signed Portrayal Catalogue, application schema and any published S-158:131 validation-check artifact from the IHO registry.
2. Record acquisition permission, publisher/source identity, exact raw bytes, SHA-256, file length, signature-verification method and trust-root/revocation context for each artifact. A filename or registry label alone is not a byte receipt.
3. Freeze official or IHO-published conformance fixtures only after their terms and redistribution rights are reviewed; until then use locally generated synthetic metadata tests.
4. Add a parser/validator adapter as a separate owner with schema/product-version pinning, feature-catalogue linkage, portrayal-catalogue linkage, code-list validation and S-158:131 checks. Do not silently extend the metadata verifier into a payload decoder.
5. Bind Sol-Atlas projections to raw S-131 feature identity, feature-level temporal state, coverage, coordinate reference, catalogue version and reversible transformation lineage.
6. Keep port-call / terminal capacity / berth commitment from DCSA/PCS/operator sources as separate owner-native evidence. Join only through explicit, versioned facility crosswalks with unresolved states preserved.

## Claim ceiling

A passing profile check establishes only metadata-contract consistency and exact binding to the declared receipt/artifact. It does not establish IHO conformance, signed-catalogue authenticity, source accuracy, current service availability, berth compatibility, navigation suitability, port security, engineering safety, regulatory approval, or physical-operation authority.
