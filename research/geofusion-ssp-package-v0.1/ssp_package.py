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
class VariableProfile:
    name: str
    value_reference: int
    causality: str
    unit: str
    port_id: str
    carrier: str
    boundary_id: str
    port_role: str
    quantity_kind: str

@dataclass(frozen=True)
class ComponentProfile:
    name: str
    source_uri: str
    model_identifier: str
    variable_name: str
    value_reference: int
    causality: str
    variables: tuple[VariableProfile, ...]

PROFILES = (
    ComponentProfile(
        name="GeothermalSource",
        source_uri="resources/GeoFusion-GeothermalSource.fmu",
        model_identifier="GeothermalSource",
        variable_name="thermalPowerOut",
        value_reference=11,
        causality="output",
        variables=(
            VariableProfile(
                name="thermalPowerOut", value_reference=11, causality="output", unit="W",
                port_id="geo.thermal_out", carrier="thermal", boundary_id="geo-to-hx",
                port_role="coupling_source", quantity_kind="power_flow",
            ),
        ),
    ),
    ComponentProfile(
        name="HeatExchangerBuffer",
        source_uri="resources/GeoFusion-HeatExchangerBuffer.fmu",
        model_identifier="HeatExchangerBuffer",
        variable_name="thermalPowerIn",
        value_reference=21,
        causality="input",
        variables=(
            VariableProfile(
                name="thermalPowerIn", value_reference=21, causality="input", unit="W",
                port_id="hx.thermal_in", carrier="thermal", boundary_id="geo-to-hx",
                port_role="coupling_sink", quantity_kind="power_flow",
            ),
            VariableProfile(
                name="thermalService", value_reference=31, causality="output", unit="W",
                port_id="hx.thermal_service", carrier="thermal", boundary_id="hx-terminal-services",
                port_role="terminal_sink", quantity_kind="power_flow",
            ),
            VariableProfile(
                name="heatLoss", value_reference=32, causality="output", unit="W",
                port_id="hx.heat_loss", carrier="thermal", boundary_id="hx-terminal-losses",
                port_role="terminal_sink", quantity_kind="power_flow",
            ),
            VariableProfile(
                name="storageCharge", value_reference=33, causality="output", unit="W",
                port_id="hx.storage_charge", carrier="thermal", boundary_id="hx-terminal-storage",
                port_role="terminal_sink", quantity_kind="power_flow",
            ),
            VariableProfile(
                name="storedEnergy", value_reference=34, causality="output", unit="J",
                port_id="hx.stored_energy", carrier="thermal", boundary_id="hx-storage-state",
                port_role="state_observation", quantity_kind="stored_energy_state",
            ),
        ),
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
        canonical_name = path.as_posix() + ("/" if info.is_dir() else "")
        if name != canonical_name:
            raise QualificationError(f"{label}: non-canonical ZIP member path {name!r}")
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

EXPECTED_UNITS = {
    "W": {
        "kg": 1.0, "m": 2.0, "s": -3.0, "A": 0.0,
        "K": 0.0, "mol": 0.0, "cd": 0.0, "rad": 0.0,
        "factor": 1.0, "offset": 0.0,
    },
    "J": {
        "kg": 1.0, "m": 2.0, "s": -2.0, "A": 0.0,
        "K": 0.0, "mol": 0.0, "cd": 0.0, "rad": 0.0,
        "factor": 1.0, "offset": 0.0,
    },
}

def _check_unit(unit_map: dict[str, float], unit_name: str, label: str) -> None:
    expected = EXPECTED_UNITS.get(unit_name)
    if expected is None or unit_map != expected:
        raise QualificationError(f"{label}: unit {unit_name!r} has unexpected SI base dimensions")

def inspect_fmu_bytes(data: bytes, profile: ComponentProfile) -> dict:
    """Verify every declared FMI variable and extract hashes/identity from actual bytes."""
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

            variable_records = []
            seen_value_references = set()
            for variable_profile in profile.variables:
                variable = md.find(
                    f"ModelVariables/Float64[@name='{variable_profile.name}']"
                )
                if variable is None:
                    raise QualificationError(
                        f"{label} FMU: Float64 variable {variable_profile.name!r} is missing"
                    )
                if variable.get("causality") != variable_profile.causality:
                    raise QualificationError(
                        f"{label} FMU: {variable_profile.name} causality must be {variable_profile.causality!r}"
                    )
                if variable.get("variability") != "continuous":
                    raise QualificationError(
                        f"{label} FMU: {variable_profile.name} variability must be 'continuous'"
                    )
                if variable.get("unit") != variable_profile.unit:
                    raise QualificationError(
                        f"{label} FMU: {variable_profile.name} unit must be {variable_profile.unit!r}, "
                        f"got {variable.get('unit')!r}"
                    )
                try:
                    value_reference = int(variable.get("valueReference", ""))
                except ValueError as error:
                    raise QualificationError(
                        f"{label} FMU: {variable_profile.name} has invalid valueReference"
                    ) from error
                if value_reference != variable_profile.value_reference:
                    raise QualificationError(
                        f"{label} FMU: {variable_profile.name} valueReference must be "
                        f"{variable_profile.value_reference}, got {value_reference}"
                    )
                if value_reference in seen_value_references:
                    raise QualificationError(f"{label} FMU: duplicate valueReference {value_reference}")
                seen_value_references.add(value_reference)
                unit_map = _unit_base_exponents(md, variable_profile.unit)
                _check_unit(unit_map, variable_profile.unit, f"{label} FMU")

                variable_records.append({
                    "name": variable_profile.name,
                    "value_reference": value_reference,
                    "fmi_type": "Float64",
                    "causality": variable_profile.causality,
                    "variability": variable.get("variability"),
                    "unit": variable_profile.unit,
                    "unit_base_exponents": unit_map,
                    "ssp_connector_kind": variable_profile.causality,
                    "ssp_connector_type": "Float64",
                    "port_id": variable_profile.port_id,
                    "carrier": variable_profile.carrier,
                    "boundary_id": variable_profile.boundary_id,
                    "port_role": variable_profile.port_role,
                    "quantity_kind": variable_profile.quantity_kind,
                })

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
                "variables": variable_records,
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
    """Generate the SSP descriptor for every typed FMI port in the federation."""
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
        for variable in profile.variables:
            connector = etree.SubElement(
                connectors,
                q(SSD_NS, "Connector"),
                name=variable.name,
                kind=variable.causality,
            )
            etree.SubElement(connector, q(SSC_NS, "Float64"), unit=variable.unit)

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
    for unit_name, unit_values in EXPECTED_UNITS.items():
        unit = etree.SubElement(units, q(SSC_NS, "Unit"), name=unit_name)
        etree.SubElement(
            unit,
            q(SSC_NS, "BaseUnit"),
            kg=str(int(unit_values["kg"])),
            m=str(int(unit_values["m"])),
            s=str(int(unit_values["s"])),
        )

    etree.SubElement(root, q(SSD_NS, "DefaultExperiment"), startTime="0", stopTime="2")
    return etree.tostring(root, xml_declaration=True, encoding="UTF-8", pretty_print=True)

def validate_ssd(ssd_bytes: bytes) -> dict:
    root = _secure_xml(ssd_bytes, "SystemStructure.ssd")
    if etree.QName(root).namespace != SSD_NS or etree.QName(root).localname != "SystemStructureDescription":
        raise QualificationError("SystemStructure.ssd: incorrect SSP root element or namespace")
    if root.get("version") != SSD_VERSION:
        raise QualificationError(f"SystemStructure.ssd: expected SSD version {SSD_VERSION}")

    try:
        schema_parser = etree.XMLParser(resolve_entities=False, no_network=True, load_dtd=False)
        schema_doc = etree.parse(str(SCHEMA_PATH), schema_parser)
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
            typed_nodes = [
                (type_name, connector.find(f"ssc:{type_name}", namespaces=NS))
                for type_name in ("Float64",)
            ]
            present = [(type_name, node) for type_name, node in typed_nodes if node is not None]
            if len(present) != 1:
                raise QualificationError(
                    f"SystemStructure.ssd: connector {name}.{connector_name} must have exactly one Float64 type"
                )
            type_name, type_node = present[0]
            unit_name = type_node.get("unit")
            if unit_name not in EXPECTED_UNITS:
                raise QualificationError(
                    f"SystemStructure.ssd: connector {name}.{connector_name} uses unsupported unit {unit_name!r}"
                )
            connector_map[connector_name] = {
                "kind": connector.get("kind"),
                "type": type_name,
                "unit": unit_name,
            }
        components[name] = {"source": source, "connectors": connector_map}

    if set(components) != {profile.name for profile in PROFILES}:
        raise QualificationError("SystemStructure.ssd: component set does not match the two-FMU federation")

    for profile in PROFILES:
        expected_connectors = {
            variable.name: {
                "kind": variable.causality,
                "type": "Float64",
                "unit": variable.unit,
            }
            for variable in profile.variables
        }
        if components[profile.name]["connectors"] != expected_connectors:
            raise QualificationError(
                f"SystemStructure.ssd: {profile.name} connector/type/kind/unit map mismatch"
            )
        if components[profile.name]["source"] != profile.source_uri:
            raise QualificationError(f"SystemStructure.ssd: unexpected source URI for {profile.name}")

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

    unit_definitions = {}
    for unit_name in EXPECTED_UNITS:
        node = root.find(f"ssd:Units/ssc:Unit[@name='{unit_name}']/ssc:BaseUnit", namespaces=NS)
        if node is None:
            raise QualificationError(f"SystemStructure.ssd: SI unit {unit_name} is missing")
        unit_map = {
            key: _finite_float(node.get(key, "0"), f"SSP {unit_name} unit {key}")
            for key in ("kg", "m", "s", "A", "K", "mol", "cd", "rad")
        }
        unit_map["factor"] = _finite_float(node.get("factor", "1"), f"SSP {unit_name} unit factor")
        unit_map["offset"] = _finite_float(node.get("offset", "0"), f"SSP {unit_name} unit offset")
        _check_unit(unit_map, unit_name, "SSP SystemStructure.ssd")
        unit_definitions[unit_name] = unit_map

    return {
        "ssd_version": root.get("version"),
        "system_name": system.get("name"),
        "components": components,
        "connections": connection_records,
        "unit_definitions": unit_definitions,
        "unit_W": unit_definitions["W"],
        "unit_J": unit_definitions["J"],
    }

def _canonical_graph_from_semantics(semantics: dict) -> dict:
    """Canonical semantic graph; excludes presentation-only layout/metadata."""
    components = {}
    for component_name in sorted(semantics["components"]):
        component = semantics["components"][component_name]
        connectors = {
            connector_name: {
                "kind": values["kind"],
                "type": values["type"],
                "unit": values["unit"],
            }
            for connector_name, values in sorted(component["connectors"].items())
        }
        components[component_name] = {
            "source_uri": component["source"],
            "connectors": connectors,
        }
    connections = [
        {
            "start_element": record[0],
            "start_connector": record[1],
            "end_element": record[2],
            "end_connector": record[3],
        }
        for record in semantics["connections"]
    ]
    connections.sort(key=lambda record: (
        record["start_element"], record["start_connector"],
        record["end_element"], record["end_connector"],
    ))
    return {
        "ssd_format_version": semantics["ssd_version"],
        "system_name": semantics["system_name"],
        "components": components,
        "connections": connections,
        "unit_definitions": semantics["unit_definitions"],
    }

def canonicalize_ssd(ssd_bytes: bytes) -> dict:
    """Return a canonical graph and digest for a validated SSP SSD."""
    graph = _canonical_graph_from_semantics(validate_ssd(ssd_bytes))
    return {
        "graph": graph,
        "graph_sha256": sha256(_json_bytes(graph)),
    }

def _flatten_graph(value, prefix=""):
    if isinstance(value, dict):
        for key in sorted(value):
            child = f"{prefix}.{key}" if prefix else str(key)
            yield from _flatten_graph(value[key], child)
    elif isinstance(value, list):
        for index, item in enumerate(value):
            yield from _flatten_graph(item, f"{prefix}[{index}]")
    else:
        yield prefix, value

def _compare_canonical_graphs(source_graph: dict, candidate_graph: dict) -> dict:
    source_leaves = dict(_flatten_graph(source_graph))
    candidate_leaves = dict(_flatten_graph(candidate_graph))
    paths = sorted(set(source_leaves) | set(candidate_leaves))
    differences = []
    for path in paths:
        before = source_leaves.get(path, "<MISSING>")
        after = candidate_leaves.get(path, "<MISSING>")
        if before != after:
            differences.append({
                "path": path,
                "source": before,
                "candidate": after,
            })
    equivalent = not differences
    return {
        "disposition": "PASS" if equivalent else "DIFFERENT",
        "qualified": equivalent,
        "source_graph_sha256": sha256(_json_bytes(source_graph)),
        "candidate_graph_sha256": sha256(_json_bytes(candidate_graph)),
        "reason": None if equivalent else "canonical SSP system graphs differ",
        "differences": differences,
    }

def compare_ssd_bytes(source_ssd: bytes, candidate_ssd: bytes) -> dict:
    """Compare semantic graphs, ignoring whitespace, attribute order and layout."""
    source = canonicalize_ssd(source_ssd)
    try:
        candidate = canonicalize_ssd(candidate_ssd)
    except QualificationError as error:
        return {
            "disposition": "REJECT",
            "qualified": False,
            "source_graph_sha256": source["graph_sha256"],
            "candidate_graph_sha256": None,
            "reason": str(error),
            "differences": [],
        }
    return _compare_canonical_graphs(source["graph"], candidate["graph"])

def inspect_ssp_for_roundtrip(package_bytes: bytes, label: str) -> dict:
    """Inspect SSP structure and FMU member bytes without trusting its manifest."""
    try:
        with zipfile.ZipFile(io.BytesIO(package_bytes), "r") as archive:
            entries = _safe_archive_entries(archive, label)
            if "SystemStructure.ssd" not in entries:
                raise QualificationError(f"{label}: SystemStructure.ssd is missing")
            ssd_bytes = archive.read("SystemStructure.ssd")
            semantics = validate_ssd(ssd_bytes)
            graph = _canonical_graph_from_semantics(semantics)
            components = {}
            for profile in PROFILES:
                uri = semantics["components"][profile.name]["source"]
                if uri not in entries:
                    raise QualificationError(f"{label}: referenced FMU {uri!r} is missing")
                fmu_bytes = archive.read(uri)
                inspection = inspect_fmu_bytes(fmu_bytes, profile)
                with zipfile.ZipFile(io.BytesIO(fmu_bytes), "r") as fmu_archive:
                    fmu_entries = _safe_archive_entries(
                        fmu_archive, f"{label}:{profile.name} FMU"
                    )
                    member_digests = {
                        name: {
                            "byte_length": fmu_entries[name].file_size,
                            "sha256": sha256(fmu_archive.read(name)),
                        }
                        for name in sorted(fmu_entries)
                    }
                components[profile.name] = {
                    "source_uri": uri,
                    "fmu_package_sha256": sha256(fmu_bytes),
                    "model_description_sha256": inspection["model_description_sha256"],
                    "binary_sha256": inspection["binary_sha256"],
                    "members": member_digests,
                }
            return {
                "ssp_package_sha256": sha256(package_bytes),
                "ssd_sha256": sha256(ssd_bytes),
                "graph": graph,
                "graph_sha256": sha256(_json_bytes(graph)),
                "components": components,
            }
    except zipfile.BadZipFile as error:
        raise QualificationError(f"{label}: invalid SSP ZIP archive") from error

def compare_ssp_package_bytes(source_package: bytes, candidate_package: bytes) -> dict:
    """Compare two SSP packages including graph and uncompressed FMU contents."""
    try:
        source = inspect_ssp_for_roundtrip(source_package, "source SSP")
        candidate = inspect_ssp_for_roundtrip(candidate_package, "candidate SSP")
    except QualificationError as error:
        return {
            "disposition": "REJECT",
            "qualified": False,
            "reason": str(error),
            "differences": [],
        }

    graph_diff = _compare_canonical_graphs(source["graph"], candidate["graph"])
    component_results = {}
    package_content_equal = True
    for component_name in sorted(source["components"]):
        left = source["components"][component_name]
        right = candidate["components"][component_name]
        left_members = left["members"]
        right_members = right["members"]
        added = sorted(set(right_members) - set(left_members))
        removed = sorted(set(left_members) - set(right_members))
        changed = sorted(
            name for name in set(left_members) & set(right_members)
            if left_members[name] != right_members[name]
        )
        member_content_equal = not (added or removed or changed)
        package_content_equal = package_content_equal and member_content_equal
        component_results[component_name] = {
            "fmu_package_sha256_equal": left["fmu_package_sha256"] == right["fmu_package_sha256"],
            "uncompressed_member_content_equal": member_content_equal,
            "model_description_sha256_equal": (
                left["model_description_sha256"] == right["model_description_sha256"]
            ),
            "binary_sha256_equal": left["binary_sha256"] == right["binary_sha256"],
            "added_fmu_members": added,
            "removed_fmu_members": removed,
            "changed_fmu_members": changed,
        }

    qualified = graph_diff["qualified"] and package_content_equal
    differences = []
    if not graph_diff["qualified"]:
        differences.append({
            "kind": "system_structure",
            "items": graph_diff["differences"],
        })
    for component_name, result in component_results.items():
        if not result["uncompressed_member_content_equal"]:
            differences.append({
                "kind": "fmu_content",
                "component": component_name,
                "added": result["added_fmu_members"],
                "removed": result["removed_fmu_members"],
                "changed": result["changed_fmu_members"],
            })
    return {
        "disposition": "PASS" if qualified else "DIFFERENT",
        "qualified": qualified,
        "reason": None if qualified else "SSP graph or referenced FMU content changed",
        "source_ssp_sha256": source["ssp_package_sha256"],
        "candidate_ssp_sha256": candidate["ssp_package_sha256"],
        "source_graph_sha256": source["graph_sha256"],
        "candidate_graph_sha256": candidate["graph_sha256"],
        "system_structure_equivalent": graph_diff["qualified"],
        "fmu_uncompressed_content_preserved": package_content_equal,
        "components": component_results,
        "differences": differences,
    }

def compare_ssp_packages(source_path: Path, candidate_path: Path) -> dict:
    source_path = Path(source_path)
    candidate_path = Path(candidate_path)
    for path in (source_path, candidate_path):
        if not path.is_file() or path.is_symlink():
            raise QualificationError(f"SSP comparison input must be a regular non-symlink file: {path}")
    return compare_ssp_package_bytes(source_path.read_bytes(), candidate_path.read_bytes())

def _json_object_no_duplicates(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise QualificationError(f"evidence manifest contains duplicate JSON key {key!r}")
        result[key] = value
    return result

def _json_bytes(value: dict) -> bytes:
    return (json.dumps(value, sort_keys=True, indent=2, separators=(",", ": ")) + "\n").encode("utf-8")

def _validate_ssd_fmu_bindings(ssd_semantics: dict, inspections: list[dict]) -> None:
    """Require a one-to-one match between every profiled FMU variable and SSP connector."""
    for inspection in inspections:
        component_name = inspection["component_name"]
        connector_map = ssd_semantics["components"][component_name]["connectors"]
        variables = {record["name"]: record for record in inspection["variables"]}
        if set(connector_map) != set(variables):
            raise QualificationError(
                f"{component_name}: SSP connector set does not match profiled FMI variable set"
            )
        for variable_name, variable in variables.items():
            connector = connector_map[variable_name]
            expected = {
                "kind": variable["causality"],
                "type": variable["fmi_type"],
                "unit": variable["unit"],
            }
            if connector != expected:
                raise QualificationError(
                    f"{component_name}.{variable_name}: SSP connector is incompatible with FMI variable"
                )

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

    _validate_ssd_fmu_bindings(ssd_semantics, inspections)

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

            expected_manifest_keys = {
                "schema_version", "ssp_release", "ssd_format_version",
                "fmi_standard_release", "platform", "system_structure_sha256",
                "components", "connections", "claim_ceiling",
            }
            if set(manifest) != expected_manifest_keys:
                raise QualificationError("SSP package evidence manifest key set mismatch")
            if manifest.get("schema_version") != "0.1.0" \
                    or manifest.get("ssp_release") != SSP_RELEASE \
                    or manifest.get("ssd_format_version") != SSD_VERSION \
                    or manifest.get("fmi_standard_release") != FMI_STANDARD_RELEASE:
                raise QualificationError("SSP package evidence manifest version mismatch")
            expected_claim_ceiling = (
                "SSP package/system-structure qualification only. No FMI execution, master, "
                "physical thermofluid or geothermal plant qualification."
            )
            if manifest.get("claim_ceiling") != expected_claim_ceiling:
                raise QualificationError("SSP package evidence manifest claim ceiling mismatch")
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

            _validate_ssd_fmu_bindings(ssd_semantics, actual_inspections)
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
    compare_ssd = sub.add_parser("compare-ssd", help="compare two SSP SystemStructure.ssd files")
    compare_ssd.add_argument("source", type=Path)
    compare_ssd.add_argument("candidate", type=Path)
    compare_ssp = sub.add_parser("compare-ssp", help="compare two SSP packages for round-trip preservation")
    compare_ssp.add_argument("source", type=Path)
    compare_ssp.add_argument("candidate", type=Path)
    args = parser.parse_args(argv)

    try:
        if args.command == "build":
            receipt = build_package(args.geothermal_fmu, args.heat_exchanger_fmu, args.output)
        elif args.command == "verify":
            receipt = verify_package(args.package)
        elif args.command == "compare-ssd":
            for input_path in (args.source, args.candidate):
                if not input_path.is_file() or input_path.is_symlink():
                    raise QualificationError(f"SSD comparison input must be a regular non-symlink file: {input_path}")
            result = compare_ssd_bytes(args.source.read_bytes(), args.candidate.read_bytes())
            print(json.dumps(result, indent=2, sort_keys=True))
            return 0 if result["qualified"] else 3
        else:
            result = compare_ssp_packages(args.source, args.candidate)
            print(json.dumps(result, indent=2, sort_keys=True))
            return 0 if result["qualified"] else 3
        print(json.dumps(receipt, indent=2, sort_keys=True))
        return 0
    except (QualificationError, OSError, json.JSONDecodeError) as error:
        print(f"GeoFusion SSP qualification failed: {error}", file=sys.stderr)
        return 2

if __name__ == "__main__":
    raise SystemExit(main())
