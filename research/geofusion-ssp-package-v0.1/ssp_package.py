#!/usr/bin/env python3
"""Build and verify the GeoFusion two-FMU SSP 2.0.1 package.

The package assembler accepts real FMU file bytes. Unit tests construct tiny
non-executable FMU ZIP doubles in temporary directories; those are never
committed or accepted as plant evidence.
"""
from __future__ import annotations

import argparse
from dataclasses import dataclass
import hashlib
import io
import json
import math
import os
from pathlib import Path, PurePosixPath
import stat
import sys
import tempfile
import zipfile

from lxml import etree

SSP_RELEASE = "2.0.1"
SSD_VERSION = "2.0"
FMI_STANDARD_RELEASE = "3.0.2"
PLATFORM = "x86_64-linux"
SSD_NS = "http://ssp-standard.org/SSP1/SystemStructureDescription"
SSC_NS = "http://ssp-standard.org/SSP1/SystemStructureCommon"
NS = {"ssd": SSD_NS, "ssc": SSC_NS}

ROOT = Path(__file__).resolve().parent
SCHEMA_PATH = ROOT / "third_party" / "SSP-2.0.1" / "schema" / "SystemStructureDescription.xsd"

@dataclass(frozen=True)
class ComponentProfile:
    name: str
    source_uri: str
    model_identifier: str
    variable_name: str
    value_reference: int
    causality: str

PROFILES = (
    ComponentProfile(
        name="GeothermalSource",
        source_uri="resources/GeoFusion-GeothermalSource.fmu",
        model_identifier="GeothermalSource",
        variable_name="thermalPowerOut",
        value_reference=11,
        causality="output",
    ),
    ComponentProfile(
        name="HeatExchangerBuffer",
        source_uri="resources/GeoFusion-HeatExchangerBuffer.fmu",
        model_identifier="HeatExchangerBuffer",
        variable_name="thermalPowerIn",
        value_reference=21,
        causality="input",
    ),
)

class QualificationError(ValueError):
    """Raised when an SSP or FMU package violates the declared profile."""

def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()

def _secure_xml(data: bytes, label: str):
    parser = etree.XMLParser(
        resolve_entities=False,
        no_network=True,
        load_dtd=False,
        huge_tree=False,
        remove_comments=False,
    )
    try:
        root = etree.fromstring(data, parser)
    except etree.XMLSyntaxError as error:
        raise QualificationError(f"{label}: malformed XML: {error}") from error
    if root.getroottree().docinfo.doctype:
        raise QualificationError(f"{label}: DTD declarations are not permitted")
    return root

def _safe_archive_entries(archive: zipfile.ZipFile, label: str) -> dict[str, zipfile.ZipInfo]:
    entries: dict[str, zipfile.ZipInfo] = {}
    for info in archive.infolist():
        name = info.filename
        if not name or "\\" in name or name.startswith("/"):
            raise QualificationError(f"{label}: unsafe ZIP member path {name!r}")
        path = PurePosixPath(name)
        if any(part in ("", ".", "..") for part in path.parts):
            raise QualificationError(f"{label}: unsafe ZIP member path {name!r}")
        if path.is_absolute() or ":" in path.parts[0]:
            raise QualificationError(f"{label}: absolute or drive-qualified ZIP path {name!r}")
        if name in entries:
            raise QualificationError(f"{label}: duplicate ZIP member {name!r}")
        mode = (info.external_attr >> 16) & 0o170000
        if mode == stat.S_IFLNK:
            raise QualificationError(f"{label}: symlink ZIP member is not permitted: {name!r}")
        if info.is_dir():
            continue
        entries[name] = info
    return entries

def _finite_float(raw: str, label: str) -> float:
    try:
        value = float(raw)
    except (TypeError, ValueError) as error:
        raise QualificationError(f"{label}: invalid numeric value {raw!r}") from error
    if not math.isfinite(value):
        raise QualificationError(f"{label}: numeric value must be finite")
    return value

def _unit_base_exponents(model_description, unit_name: str) -> dict[str, float]:
    unit = model_description.find(f"UnitDefinitions/Unit[@name='{unit_name}']")
    if unit is None:
        raise QualificationError(f"modelDescription.xml: unit {unit_name!r} is not defined")
    base = unit.find("BaseUnit")
    if base is None:
        raise QualificationError(f"modelDescription.xml: unit {unit_name!r} has no BaseUnit")
    keys = ("kg", "m", "s", "A", "K", "mol", "cd", "rad")
    result = {
        key: _finite_float(base.get(key, "0"), f"modelDescription.xml unit {unit_name} {key}")
        for key in keys
    }
    result["factor"] = _finite_float(base.get("factor", "1"), f"modelDescription.xml unit {unit_name} factor")
    result["offset"] = _finite_float(base.get("offset", "0"), f"modelDescription.xml unit {unit_name} offset")
    return result

EXPECTED_W = {
    "kg": 1.0, "m": 2.0, "s": -3.0, "A": 0.0,
    "K": 0.0, "mol": 0.0, "cd": 0.0, "rad": 0.0,
    "factor": 1.0, "offset": 0.0,
}

def _check_w_unit(unit_map: dict[str, float], label: str) -> None:
    if unit_map != EXPECTED_W:
        raise QualificationError(f"{label}: unit W is not kg·m²·s⁻³ with factor=1, offset=0")

def inspect_fmu_bytes(data: bytes, profile: ComponentProfile) -> dict:
    """Verify the FMU ZIP profile and extract hashes/identity from actual bytes."""
    label = profile.name
    try:
        with zipfile.ZipFile(io.BytesIO(data), "r") as archive:
            entries = _safe_archive_entries(archive, f"{label} FMU")
            if "modelDescription.xml" not in entries:
                raise QualificationError(f"{label} FMU: modelDescription.xml is missing")
            md_bytes = archive.read("modelDescription.xml")
            md = _secure_xml(md_bytes, f"{label} modelDescription.xml")
            if etree.QName(md).localname != "fmiModelDescription":
                raise QualificationError(f"{label} FMU: root is not fmiModelDescription")
            if not (md.get("fmiVersion") or "").startswith("3.0"):
                raise QualificationError(f"{label} FMU: expected FMI 3.0, got {md.get('fmiVersion')!r}")

            cosim = md.find("CoSimulation")
            if cosim is None:
                raise QualificationError(f"{label} FMU: CoSimulation interface is missing")
            model_identifier = cosim.get("modelIdentifier")
            if model_identifier != profile.model_identifier:
                raise QualificationError(
                    f"{label} FMU: modelIdentifier {model_identifier!r} does not match "
                    f"{profile.model_identifier!r}"
                )

            binary_path = f"binaries/{PLATFORM}/{model_identifier}.so"
            if binary_path not in entries:
                raise QualificationError(f"{label} FMU: selected platform binary {binary_path!r} is missing")
            binary_bytes = archive.read(binary_path)
            if not binary_bytes:
                raise QualificationError(f"{label} FMU: selected platform binary is empty")

            variable = md.find(
                f"ModelVariables/Float64[@name='{profile.variable_name}']"
            )
            if variable is None:
                raise QualificationError(
                    f"{label} FMU: Float64 variable {profile.variable_name!r} is missing"
                )
            if variable.get("causality") != profile.causality:
                raise QualificationError(
                    f"{label} FMU: {profile.variable_name} causality must be {profile.causality!r}"
                )
            if variable.get("variability") != "continuous":
                raise QualificationError(
                    f"{label} FMU: {profile.variable_name} variability must be 'continuous'"
                )
            if variable.get("unit") != "W":
                raise QualificationError(
                    f"{label} FMU: {profile.variable_name} unit must be 'W', got {variable.get('unit')!r}"
                )
            try:
                value_reference = int(variable.get("valueReference", ""))
            except ValueError as error:
                raise QualificationError(f"{label} FMU: invalid valueReference") from error
            if value_reference != profile.value_reference:
                raise QualificationError(
                    f"{label} FMU: valueReference must be {profile.value_reference}, "
                    f"got {value_reference}"
                )
            _check_w_unit(_unit_base_exponents(md, "W"), f"{label} FMU")

            return {
                "component_name": profile.name,
                "source_uri": profile.source_uri,
                "fmi_version": md.get("fmiVersion"),
                "fmi_standard_release": FMI_STANDARD_RELEASE,
                "interface": "CoSimulation",
                "model_name": md.get("modelName"),
                "model_identifier": model_identifier,
                "platform": PLATFORM,
                "binary_path": binary_path,
                "binary_sha256": sha256(binary_bytes),
                "model_description_sha256": sha256(md_bytes),
                "fmu_package_sha256": sha256(data),
                "variable": {
                    "name": profile.variable_name,
                    "value_reference": value_reference,
                    "fmi_type": "Float64",
                    "causality": profile.causality,
                    "variability": variable.get("variability"),
                    "unit": variable.get("unit"),
                    "ssp_connector_kind": profile.causality,
                    "ssp_connector_type": "Float64",
                },
                "capabilities": {
                    key: cosim.get(key, "false").lower() == "true"
                    for key in (
                        "canHandleVariableCommunicationStepSize",
                        "canGetAndSetFMUState",
                        "canSerializeFMUState",
                        "canInterpolateInputs",
                        "canReturnEarlyAfterIntermediateUpdate",
                    )
                },
            }
    except zipfile.BadZipFile as error:
        raise QualificationError(f"{label} input is not a valid FMU ZIP archive") from error

def make_ssd() -> bytes:
    """Generate the canonical SSP SSD descriptor for the declared two-FMU graph."""
    q = lambda ns, local: f"{{{ns}}}{local}"
    root = etree.Element(
        q(SSD_NS, "SystemStructureDescription"),
        nsmap={"ssd": SSD_NS, "ssc": SSC_NS},
        version=SSD_VERSION,
        name="GeoFusion-Thermal-Federation",
    )
    system = etree.SubElement(root, q(SSD_NS, "System"), name="GeoFusionThermalFederation")
    elements = etree.SubElement(system, q(SSD_NS, "Elements"))

    for profile in PROFILES:
        component = etree.SubElement(
            elements,
            q(SSD_NS, "Component"),
            name=profile.name,
            source=profile.source_uri,
            type="application/x-fmu-sharedlibrary",
            implementation="CoSimulation",
        )
        connectors = etree.SubElement(component, q(SSD_NS, "Connectors"))
        connector = etree.SubElement(
            connectors,
            q(SSD_NS, "Connector"),
            name=profile.variable_name,
            kind=profile.causality,
        )
        etree.SubElement(connector, q(SSC_NS, "Float64"), unit="W")

    connections = etree.SubElement(system, q(SSD_NS, "Connections"))
    etree.SubElement(
        connections,
        q(SSD_NS, "Connection"),
        startElement="GeothermalSource",
        startConnector="thermalPowerOut",
        endElement="HeatExchangerBuffer",
        endConnector="thermalPowerIn",
    )

    units = etree.SubElement(root, q(SSD_NS, "Units"))
    unit = etree.SubElement(units, q(SSC_NS, "Unit"), name="W")
    etree.SubElement(unit, q(SSC_NS, "BaseUnit"), kg="1", m="2", s="-3")

    etree.SubElement(root, q(SSD_NS, "DefaultExperiment"), startTime="0", stopTime="2")
    return etree.tostring(root, xml_declaration=True, encoding="UTF-8", pretty_print=True)

def validate_ssd(ssd_bytes: bytes) -> dict:
    root = _secure_xml(ssd_bytes, "SystemStructure.ssd")
    if etree.QName(root).namespace != SSD_NS or etree.QName(root).localname != "SystemStructureDescription":
        raise QualificationError("SystemStructure.ssd: incorrect SSP root element or namespace")
    if root.get("version") != SSD_VERSION:
        raise QualificationError(f"SystemStructure.ssd: expected SSD version {SSD_VERSION}")

    try:
        schema_doc = etree.parse(str(SCHEMA_PATH))
        schema = etree.XMLSchema(schema_doc)
        schema.assertValid(root)
    except (OSError, etree.XMLSyntaxError, etree.XMLSchemaParseError, etree.DocumentInvalid) as error:
        raise QualificationError(f"SystemStructure.ssd: SSP 2.0.1 XSD validation failed: {error}") from error

    system = root.find("ssd:System", namespaces=NS)
    if system is None:
        raise QualificationError("SystemStructure.ssd: System element is missing")
    elements = system.find("ssd:Elements", namespaces=NS)
    if elements is None:
        raise QualificationError("SystemStructure.ssd: Elements is missing")

    components = {}
    for component in elements.findall("ssd:Component", namespaces=NS):
        name = component.get("name")
        if not name or name in components:
            raise QualificationError(f"SystemStructure.ssd: missing or duplicate component name {name!r}")
        source = component.get("source")
        if not source:
            raise QualificationError(f"SystemStructure.ssd: component {name} has no source URI")
        source_path = PurePosixPath(source)
        if source_path.is_absolute() or "\\" in source or any(part in ("", ".", "..") for part in source_path.parts):
            raise QualificationError(f"SystemStructure.ssd: unsafe component source URI {source!r}")
        if component.get("implementation") != "CoSimulation":
            raise QualificationError(f"SystemStructure.ssd: {name} must select CoSimulation")
        if component.get("type", "application/x-fmu-sharedlibrary") != "application/x-fmu-sharedlibrary":
            raise QualificationError(f"SystemStructure.ssd: {name} must declare an FMU shared-library component")
        connectors_node = component.find("ssd:Connectors", namespaces=NS)
        if connectors_node is None:
            raise QualificationError(f"SystemStructure.ssd: {name} connectors are missing")
        connector_map = {}
        for connector in connectors_node.findall("ssd:Connector", namespaces=NS):
            connector_name = connector.get("name")
            if not connector_name or connector_name in connector_map:
                raise QualificationError(f"SystemStructure.ssd: duplicate connector on {name}")
            float64 = connector.find("ssc:Float64", namespaces=NS)
            if float64 is None:
                raise QualificationError(f"SystemStructure.ssd: connector {name}.{connector_name} is not Float64")
            connector_map[connector_name] = {
                "kind": connector.get("kind"),
                "type": "Float64",
                "unit": float64.get("unit"),
            }
        components[name] = {"source": source, "connectors": connector_map}

    if set(components) != {"GeothermalSource", "HeatExchangerBuffer"}:
        raise QualificationError("SystemStructure.ssd: component set does not match the two-FMU federation")

    expected_connectors = {
        "GeothermalSource": {"thermalPowerOut": {"kind": "output", "type": "Float64", "unit": "W"}},
        "HeatExchangerBuffer": {"thermalPowerIn": {"kind": "input", "type": "Float64", "unit": "W"}},
    }
    for component_name, expected in expected_connectors.items():
        if components[component_name]["connectors"] != expected:
            raise QualificationError(
                f"SystemStructure.ssd: {component_name} connector/type/kind/unit map mismatch"
            )
        if components[component_name]["source"] != next(
            profile.source_uri for profile in PROFILES if profile.name == component_name
        ):
            raise QualificationError(f"SystemStructure.ssd: unexpected source URI for {component_name}")

    connections = system.find("ssd:Connections", namespaces=NS)
    if connections is None:
        raise QualificationError("SystemStructure.ssd: Connections is missing")
    connection_records = [
        (
            connection.get("startElement"),
            connection.get("startConnector"),
            connection.get("endElement"),
            connection.get("endConnector"),
        )
        for connection in connections.findall("ssd:Connection", namespaces=NS)
    ]
    expected_connection = [
        ("GeothermalSource", "thermalPowerOut", "HeatExchangerBuffer", "thermalPowerIn")
    ]
    if connection_records != expected_connection:
        raise QualificationError("SystemStructure.ssd: connection direction or endpoint identity mismatch")

    units = root.find("ssd:Units/ssc:Unit[@name='W']/ssc:BaseUnit", namespaces=NS)
    if units is None:
        raise QualificationError("SystemStructure.ssd: SI unit W is missing")
    unit_map = {
        key: _finite_float(units.get(key, "0"), f"SSP W unit {key}")
        for key in ("kg", "m", "s", "A", "K", "mol", "cd", "rad")
    }
    unit_map["factor"] = _finite_float(units.get("factor", "1"), "SSP W unit factor")
    unit_map["offset"] = _finite_float(units.get("offset", "0"), "SSP W unit offset")
    _check_w_unit(unit_map, "SSP SystemStructure.ssd")
    return {
        "ssd_version": root.get("version"),
        "system_name": system.get("name"),
        "components": components,
        "connections": connection_records,
        "unit_W": unit_map,
    }

def _json_object_no_duplicates(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise QualificationError(f"evidence manifest contains duplicate JSON key {key!r}")
        result[key] = value
    return result

def _json_bytes(value: dict) -> bytes:
    return (json.dumps(value, sort_keys=True, indent=2, separators=(",", ": ")) + "\n").encode("utf-8")

def build_package(geothermal_fmu: Path, heat_exchanger_fmu: Path, output_path: Path) -> dict:
    """Assemble a deterministic SSP package from two actual FMU files."""
    geothermal_fmu = Path(geothermal_fmu)
    heat_exchanger_fmu = Path(heat_exchanger_fmu)
    output_path = Path(output_path)
    if output_path.is_symlink():
        raise QualificationError(f"SSP output path must not be a symlink: {output_path}")
    resolved_output = output_path.resolve()
    if resolved_output in (geothermal_fmu.resolve(), heat_exchanger_fmu.resolve()):
        raise QualificationError("SSP output path must not overwrite either input FMU")
    for path in (geothermal_fmu, heat_exchanger_fmu):
        if not path.is_file() or path.is_symlink():
            raise QualificationError(f"FMU input must be a regular non-symlink file: {path}")

    fmu_bytes = {
        PROFILES[0].source_uri: geothermal_fmu.read_bytes(),
        PROFILES[1].source_uri: heat_exchanger_fmu.read_bytes(),
    }
    inspections = [
        inspect_fmu_bytes(fmu_bytes[profile.source_uri], profile)
        for profile in PROFILES
    ]
    ssd_bytes = make_ssd()
    ssd_semantics = validate_ssd(ssd_bytes)

    for inspection, profile in zip(inspections, PROFILES):
        connector = ssd_semantics["components"][profile.name]["connectors"][profile.variable_name]
        variable = inspection["variable"]
        if connector["kind"] != variable["causality"] or connector["type"] != variable["fmi_type"] \
                or connector["unit"] != variable["unit"]:
            raise QualificationError(f"{profile.name}: SSP connector is incompatible with FMI variable")

    manifest = {
        "schema_version": "0.1.0",
        "ssp_release": SSP_RELEASE,
        "ssd_format_version": SSD_VERSION,
        "fmi_standard_release": FMI_STANDARD_RELEASE,
        "platform": PLATFORM,
        "system_structure_sha256": sha256(ssd_bytes),
        "components": inspections,
        "connections": [
            {
                "start_element": "GeothermalSource",
                "start_connector": "thermalPowerOut",
                "end_element": "HeatExchangerBuffer",
                "end_connector": "thermalPowerIn",
                "carrier": "thermal",
                "unit": "W",
            }
        ],
        "claim_ceiling": (
            "SSP package/system-structure qualification only. No FMI execution, master, "
            "physical thermofluid or geothermal plant qualification."
        ),
    }
    manifest_bytes = _json_bytes(manifest)

    members = {
        "SystemStructure.ssd": ssd_bytes,
        **fmu_bytes,
        "resources/geofusion-ssp-evidence.json": manifest_bytes,
    }
    output_path.parent.mkdir(parents=True, exist_ok=True)
    fd, temporary_name = tempfile.mkstemp(prefix=output_path.name + ".", suffix=".tmp", dir=output_path.parent)
    os.close(fd)
    temporary_path = Path(temporary_name)
    try:
        with zipfile.ZipFile(temporary_path, "w", compression=zipfile.ZIP_STORED, allowZip64=True) as archive:
            for name in sorted(members):
                info = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
                info.compress_type = zipfile.ZIP_STORED
                info.create_system = 3
                info.external_attr = (stat.S_IFREG | 0o644) << 16
                info.flag_bits |= 0x800
                archive.writestr(info, members[name])
        package_bytes = temporary_path.read_bytes()
        verify_package_bytes(package_bytes)
        temporary_path.replace(output_path)
    finally:
        if temporary_path.exists():
            temporary_path.unlink()

    package_digest = sha256(output_path.read_bytes())
    receipt = {
        "ssp_package": output_path.name,
        "ssp_package_sha256": package_digest,
        "ssp_release": SSP_RELEASE,
        "system_structure_sha256": sha256(ssd_bytes),
        "components": inspections,
    }
    sidecar = output_path.with_suffix(output_path.suffix + ".sha256")
    sidecar.write_text(f"{package_digest}  {output_path.name}\n", encoding="utf-8")
    manifest_sidecar = output_path.with_suffix(output_path.suffix + ".manifest.json")
    manifest_sidecar.write_bytes(_json_bytes(receipt))
    return receipt

def verify_package_bytes(package_bytes: bytes) -> dict:
    """Verify SSP package paths, XSD, component identities, units and hashes."""
    try:
        with zipfile.ZipFile(io.BytesIO(package_bytes), "r") as archive:
            entries = _safe_archive_entries(archive, "SSP package")
            required = {
                "SystemStructure.ssd",
                "resources/GeoFusion-GeothermalSource.fmu",
                "resources/GeoFusion-HeatExchangerBuffer.fmu",
                "resources/geofusion-ssp-evidence.json",
            }
            missing = sorted(required - set(entries))
            if missing:
                raise QualificationError(f"SSP package is missing required members: {missing}")
            ssd_bytes = archive.read("SystemStructure.ssd")
            ssd_semantics = validate_ssd(ssd_bytes)
            manifest = json.loads(
                archive.read("resources/geofusion-ssp-evidence.json"),
                object_pairs_hook=_json_object_no_duplicates,
            )

            if manifest.get("schema_version") != "0.1.0" \
                    or manifest.get("ssp_release") != SSP_RELEASE \
                    or manifest.get("ssd_format_version") != SSD_VERSION \
                    or manifest.get("fmi_standard_release") != FMI_STANDARD_RELEASE:
                raise QualificationError("SSP package evidence manifest version mismatch")
            if manifest.get("system_structure_sha256") != sha256(ssd_bytes):
                raise QualificationError("SSP package evidence manifest has a stale SSD digest")
            if manifest.get("platform") != PLATFORM:
                raise QualificationError("SSP package evidence manifest platform mismatch")
            expected_manifest_connections = [
                {
                    "start_element": "GeothermalSource",
                    "start_connector": "thermalPowerOut",
                    "end_element": "HeatExchangerBuffer",
                    "end_connector": "thermalPowerIn",
                    "carrier": "thermal",
                    "unit": "W",
                }
            ]
            if manifest.get("connections") != expected_manifest_connections:
                raise QualificationError("SSP package evidence manifest connection declaration mismatch")

            actual_inspections = []
            for profile in PROFILES:
                uri = profile.source_uri
                if uri not in entries:
                    raise QualificationError(f"SSP package does not contain referenced FMU {uri!r}")
                actual = inspect_fmu_bytes(archive.read(uri), profile)
                actual_inspections.append(actual)
                ssd_component = ssd_semantics["components"][profile.name]
                if ssd_component["source"] != uri:
                    raise QualificationError(f"SSP source URI mismatch for {profile.name}")

            if manifest.get("components") != actual_inspections:
                raise QualificationError("SSP package evidence manifest does not match actual FMU bytes/metadata")
            return {
                "ssp_release": SSP_RELEASE,
                "package_sha256": sha256(package_bytes),
                "system_structure_sha256": sha256(ssd_bytes),
                "components": actual_inspections,
                "connections": ssd_semantics["connections"],
            }
    except zipfile.BadZipFile as error:
        raise QualificationError("input is not a valid SSP ZIP package") from error

def verify_package(path: Path) -> dict:
    path = Path(path)
    if not path.is_file() or path.is_symlink():
        raise QualificationError(f"SSP package must be a regular non-symlink file: {path}")
    return verify_package_bytes(path.read_bytes())

def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    build = sub.add_parser("build", help="assemble an SSP 2.0.1 package from two FMUs")
    build.add_argument("--geothermal-fmu", type=Path, required=True)
    build.add_argument("--heat-exchanger-fmu", type=Path, required=True)
    build.add_argument("--output", type=Path, required=True)
    verify = sub.add_parser("verify", help="verify an existing SSP package")
    verify.add_argument("package", type=Path)
    args = parser.parse_args(argv)

    try:
        if args.command == "build":
            receipt = build_package(args.geothermal_fmu, args.heat_exchanger_fmu, args.output)
        else:
            receipt = verify_package(args.package)
        print(json.dumps(receipt, indent=2, sort_keys=True))
        return 0
    except (QualificationError, OSError, json.JSONDecodeError) as error:
        print(f"GeoFusion SSP qualification failed: {error}", file=sys.stderr)
        return 2

if __name__ == "__main__":
    raise SystemExit(main())
