# GeoFusion SSP 2.0.1 federation package v0.1

**Tracking:** [GEOFUSION-SSP-PACKAGE-001 / #6378](https://github.com/Luminous-Dynamics/symtropy/issues/6378)  
**Standards-layer owner:** [#1467](https://github.com/Luminous-Dynamics/symtropy/issues/1467)  
**Benchmark composition:** [#6224](https://github.com/Luminous-Dynamics/symtropy/pull/6224), [#6337](https://github.com/Luminous-Dynamics/symtropy/issues/6337), [#6304](https://github.com/Luminous-Dynamics/symtropy/issues/6304)  
**Status:** SSP package assembler and structural verifier; no external SSP tool round-trip executed.

## Purpose and boundaries

FMI describes each FMU's runtime interface. SSP describes the connected system structure and parameterization. This package declares the two-FMU GeoFusion thermal federation using the official SSP 2.0.1 XML schemas.

This is not another FMI importer/master, trace verifier or physical energy-contract engine:
- #6341 provides the source-built FMUs and FMPy importer/dispatch transcript.
- #6309/#6304 owns the FMI API exchange ledger.
- #6198 owns normalized FMI clock/step/retry/barrier semantics.
- #6154 owns physical carrier/boundary/source-interval balances.
- #6224 owns matched external master and system-structure comparison protocols.
- #1467 owns the broader FMI/SSP federation architecture.

## Declared SSP system

The default SSD uses SSP's required SSD format attribute `version="2.0"` and SSP release 2.0.1 schema files.

| Component | FMU resource URI | Connector | Kind | Type / unit |
|---|---|---|---|---|
| GeothermalSource | `resources/GeoFusion-GeothermalSource.fmu` | `thermalPowerOut` | output | Float64 / W |
| HeatExchangerBuffer | `resources/GeoFusion-HeatExchangerBuffer.fmu` | `thermalPowerIn` | input | Float64 / W |

The package contains exactly one connection: geothermal output `thermalPowerOut` → buffer input `thermalPowerIn`.

The SSP unit definition for W is (kg\,m^2\,s^{-3}), with unit factor 1 and offset 0. Because both connector units are W, no unit transformation is declared.

## Build a release package from actual FMU files

The assembler requires two real FMU ZIP files. It inspects the actual FMU bytes; it does not substitute fixtures or silently create fake executable artifacts.

```sh
python ssp_package.py build \
  --geothermal-fmu /path/to/GeoFusion-GeothermalSource.fmu \
  --heat-exchanger-fmu /path/to/GeoFusion-HeatExchangerBuffer.fmu \
  --output GeoFusion-Thermal-Federation.ssp

python ssp_package.py verify GeoFusion-Thermal-Federation.ssp

# Compare an SSP file exported by an external tool with the source SSD.
python ssp_package.py compare-ssd \
  SystemStructure.ssd \
  exported/SystemStructure.ssd

# Compare a complete SSP round-trip, including all uncompressed FMU members.
python ssp_package.py compare-ssp \
  GeoFusion-Thermal-Federation.ssp \
  exported/GeoFusion-Thermal-Federation.ssp
```

Comparison commands emit a JSON receipt and use exit code 0 for semantic/content preservation, 3 for a valid but different/rejected candidate, and 2 for invalid input. The SSD canonicalizer ignores XML whitespace, attribute ordering and presentation-only layout/geometry but compares the system name, component source URIs, connector names/kinds/types/units, connection endpoints and SI unit definitions. The SSP package comparator also checks every uncompressed file member inside each referenced FMU: ZIP recompression/timestamp changes are distinguished from changes to modelDescription, binaries or other FMU resources.

The build emits:
- `GeoFusion-Thermal-Federation.ssp`: deterministic SSP ZIP package.
- `GeoFusion-Thermal-Federation.ssp.sha256`: SHA-256 receipt for the complete package.
- `GeoFusion-Thermal-Federation.ssp.manifest.json`: external receipt with the package hash and per-FMU artifact identities.

Inside the SSP package:
- `SystemStructure.ssd`: default SSP system description.
- `resources/GeoFusion-GeothermalSource.fmu`.
- `resources/GeoFusion-HeatExchangerBuffer.fmu`.
- `resources/geofusion-ssp-evidence.json`: embedded provenance manifest containing each FMU package, modelDescription and selected binary digest, model identity, FMI version and capability flags.

The assembler preserves the original FMU ZIP bytes. It does not unpack/repack or rewrite an FMU.

## Qualification performed by the assembler/verifier

- Validate the SSD against the vendored official SSP 2.0.1 XSD.
- Inspect each FMU ZIP for unsafe/duplicate paths, modelDescription, FMI 3.0 Co-Simulation interface, expected modelIdentifier and selected `x86_64-linux` binary.
- Verify the declared Float64 variable's value reference, causality, variability and unit, plus the exact SI base-unit definition for W.
- Check component source URIs, connector type/kind/unit, one-to-one component membership, and the exact directed connection.
- Recompute package, modelDescription and binary SHA-256 digests from actual bytes and cross-check the embedded manifest.
- Reject DTDs/external entities, symlink members, path traversal, duplicate ZIP entries and non-relative component resource URIs.
- Use ZIP_STORED, sorted member order and fixed timestamps for deterministic SSP archive bytes for fixed input bytes.

The official SSP 2.0.1 XSD files and license are vendored under `third_party/SSP-2.0.1/`; exact upstream tag and blob SHAs are recorded in `UPSTREAM.md`. The schemas are licensed under BSD 2-Clause. The SSP specification text has a separate license; this repository vendors schema sources, not specification prose.

## Canonical round-trip comparison

The comparator is intended for the future import → export → cross-import campaign. It separates:
- **System-structure equivalence:** canonical component, connector, unit and connection graph; layout and XML serialization changes are ignored.
- **Referenced-FMU content preservation:** member-by-member SHA-256 of uncompressed FMU contents. Recompression is allowed, but any changed model description, platform binary or resource is reported.
- **Outer SSP archive identity:** retained as a receipt, but archive-byte differences alone are not treated as semantic graph changes.

It does not claim the tools can execute the contained FMUs. Tool versions, import/export warnings and exported package digests remain part of the external campaign receipt.

## Tests and claim ceiling

`tests/test_ssp_package.py` creates tiny **NON-EXECUTABLE, test-only FMU doubles** in temporary directories. They contain placeholder binary bytes; they are used solely to test packaging and the negative-control suite and are not committed as FMUs or accepted as real model evidence.

The 27-case test suite covers deterministic assembly, correct graph/unit semantics, wrong variable identity/type/causality/unit, missing binaries, model identifier mismatch, path traversal, duplicate archive entries, reversed/wrong connections, stale FMU bytes, symlink inputs, DTD/external-entity rejection, canonical graph comparison, layout-insensitive round-trip equivalence, FMU ZIP recompression, and platform-binary tampering.

Run:

```sh
python -m pip install --requirement requirements.txt
python -m unittest discover -s tests -v
```

A passing test suite qualifies the assembler/verifier logic against the declared synthetic package contract. It does not mean an external SSP-capable simulator has imported, executed, exported or round-tripped this SSP. That remains a separately recorded industry interoperability track. No FMI execution, master correctness, energy conservation, geothermal physics, safety, economics or physical plant qualification is claimed.
