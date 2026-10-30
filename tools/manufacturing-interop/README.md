# MTConnect / QIF semantic mapping reference

Standalone Rust reference for the parser-independent typed mapping layer between MTConnect machine observations, QIF inspection results and explicit manufacturing cycle/part relationships.

## Standards boundary

- MTConnect Version 2.8 is the only admitted version in v0.1. The normative MTConnect model is SysML/XMI; official XML schemas are published separately.
- QIF 3.0 / ISO 23952:2020 is the only admitted QIF version in v0.1.
- ISO 23247-5:2026 provides digital-thread lifecycle guidance.
- ISO 23247-6:2026 provides integrated, unified and federated digital-twin composition.

This code does not parse XML or validate XSDs. An upstream parser must provide a document receipt declaring the exact version, namespace, schema bundle digest and a passed XSD-validation result. This kernel consumes that receipt and validates its structure. The project must later add a pinned maintained XML parser plus official schema bundles before claiming document conformance.

## Source-preserving model

MTConnect Sample, Event, Condition and Asset records remain distinct enum variants. Native data-item/component IDs, sequence numbers, timestamps, units and source locators are retained. Complete unfiltered streams report sequence gaps; filtered streams do not incorrectly infer loss from absent sequence numbers. Duplicates, conflicting duplicate sequences and timestamp regression are recorded and block the qualified join.

QIF results retain part/lot/operation, measurement plan, characteristic/feature, specification revision, instrument resource, actual value, original unit, timestamp and source locator. The mapper rejects non-measured results for the scalar measurement path. It does not derive uncertainty or a posterior distribution.

A join requires a separate verified association receipt that binds both source document digests, machine cycle, QIF result, part/lot/operation and characteristic identity, with a validity interval. No fuzzy match or nearest-time join exists.

## Unit handling

A unit transform emits a derived value but retains the original numerical value, original unit and transform revision. The mapper never replaces the QIF source actual value with a normalized value. Unit dictionaries and standardized conversion policy are future adapter work; no general unit ontology is created here.

## Build and run

    rustc --edition=2024 -D warnings --test tools/manufacturing-interop/interop.rs -o /tmp/mfg-interop-tests
    /tmp/mfg-interop-tests

    rustc --edition=2024 -D warnings tools/manufacturing-interop/interop.rs -o /tmp/mfg-interop
    /tmp/mfg-interop --verify
    /tmp/mfg-interop --csv

## Claim ceiling

The tests validate source-preserving typed mapping and explicit join rules against synthetic records. They do not prove an upstream MTConnect or QIF parser conforms to an official schema, validate a real vendor document, authenticate the association receipt or qualify the physical measurement. The output is evidence input to separate metrology/conformity decision logic, not a product-release authorization.
