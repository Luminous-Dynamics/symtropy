# MTConnect / QIF semantic mapping reference

Standalone Rust reference for the parser-independent typed mapping layer between MTConnect machine observations, QIF inspection results and explicit manufacturing cycle/part relationships.

## Standards boundary

- MTConnect Version 2.8 is the only admitted version in v0.1. The normative MTConnect model is SysML/XMI; official XML schemas are published separately.
- QIF 3.0 / ISO 23952:2020 is the only admitted QIF version in v0.1.
- ISO 23247-5:2026 provides digital-thread lifecycle guidance.
- ISO 23247-6:2026 provides integrated, unified and federated digital-twin composition.

This code does not parse XML or validate XSDs. An upstream parser must provide a document receipt declaring the exact version, namespace, schema bundle digest and a passed XSD-validation result. This kernel consumes that receipt and validates its structure. The project must later add a pinned maintained XML parser plus official schema bundles before claiming document conformance.

## Source-preserving model

MTConnect Samples, Events and Conditions remain distinct stream-observation variants. MTConnect Assets are mapped as separate top-level asset records, never coerced into stream observations. Native data-item/component IDs, sequence numbers, original device timestamps, mapped reference timestamps, reference-clock identity, clock-map receipt digest, clock uncertainty, units and source locators are retained. The source document digest, source URI, schema version/namespace/bundle digest, parser identity/version, validation authority/receipt digest, and opaque extension manifest are echoed in the mapping receipt. Complete unfiltered streams report sequence gaps; filtered streams do not incorrectly infer loss from absent sequence numbers. Duplicates, conflicting duplicate sequences and timestamp regression are recorded and block the qualified join.

QIF results retain part/lot/operation, measurement plan, characteristic/feature, specification revision, instrument resource, actual value, original unit, timestamp and source locator. The mapper rejects non-measured results for the scalar measurement path. It does not derive uncertainty or a posterior distribution.

A join requires a separate verified association receipt that binds the MTConnect source document digest, the MES cycle source document digest, the QIF source document digest, machine cycle, QIF result, part/lot/operation, characteristic identity and common reference-clock identity, with an effective interval. The association must be valid at both the decision time and the QIF measurement time. Machine observations, cycle record and QIF result must share the declared reference clock; the cycle's mapping receipt must match observations in the cycle window, and combined clock uncertainty must stay within the admitted bound. No fuzzy match or nearest-time join exists. The separate joined evidence CSV carries the original and reference measurement timestamps, both mapping digests, uncertainty, association authority and validity, source locators, and the derived unit conversion without overwriting the original.

## Unit handling

A unit transform emits a derived value but retains the original numerical value, original unit and transform revision. The mapper never replaces the QIF source actual value with a normalized value. Unit dictionaries and standardized conversion policy are future adapter work; no general unit ontology is created here.

## Build and run

    rustc --edition=2024 -D warnings --test tools/manufacturing-interop/interop.rs -o /tmp/mfg-interop-tests
    /tmp/mfg-interop-tests

    rustc --edition=2024 -D warnings tools/manufacturing-interop/interop.rs -o /tmp/mfg-interop
    /tmp/mfg-interop --verify
    /tmp/mfg-interop --csv
    /tmp/mfg-interop --assets-csv
    /tmp/mfg-interop --joined-csv

## Claim ceiling

The tests validate source-preserving typed mapping and explicit join rules against synthetic records. Three frozen CSV artifacts capture the MTConnect mapped observation stream, separate top-level Asset records, and the composed QIF-to-cycle evidence receipt. Assets are not coerced into stream observations. They do not prove an upstream MTConnect or QIF parser conforms to an official schema, validate a real vendor document, authenticate the association receipt or qualify the physical measurement. The output is evidence input to separate metrology/conformity decision logic, not a product-release authorization.
