#!/usr/bin/env python3
"""Build and verify a deterministic SSP 2.0.1 package for the integrated-flux FMUs."""
from __future__ import annotations

import hashlib
import io
import json
import math
from pathlib import Path, PurePosixPath
import stat
import sys
import tempfile
import zipfile

from lxml import etree

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT.parent / "geofusion-analytic-fmu-pair-v0.1"
XSD = BASE / "third_party" / "SSP-2.0.1" / "schema" / "SystemStructureDescription.xsd"
SSP_RELEASE = "2.0.1"
SSD_VERSION = "2.0"
FMI_VERSION = "3.0"
PLATFORM = "x86_64-linux"
SSD_NS = "http://ssp-standard.org/SSP1/SystemStructureDescription"
SSC_NS = "http://ssp-standard.org/SSP1/SystemStructureCommon"
NS = {"ssd": SSD_NS, "ssc": SSC_NS}
J_UNIT = {
    "kg": 1.0, "m": 2.0, "s": -2.0, "A": 0.0,
    "K": 0.0, "mol": 0.0, "cd": 0.0, "rad": 0.0,
    "factor": 1.0, "offset": 0.0,
}

PROFILES = (
    {
        "name": "IntegratedFluxSource",
        "source_uri": "resources/GeoFusion-IntegratedFluxSource.fmu",
        "model_identifier": "IntegratedFluxSource",
        "variables": (
            {
                "name": "cumulativeThermalEnergy", "value_reference": 11,
                "causality": "output", "unit": "J",
                "port_id": "geo.cumulative_thermal_energy",
                "boundary_id": "geo-source-energy-counter",
                "port_role": "state_observation",
                "quantity_kind": "cumulative_energy_state",
            },
            {
                "name": "intervalThermalEnergy", "value_reference": 12,
                "causality": "output", "unit": "J",
                "port_id": "geo.interval_thermal_energy",
                "boundary_id": "geo-to-hx-interval-energy",
                "port_role": "coupling_source",
                "quantity_kind": "interval_integrated_energy",
            },
        ),
    },
    {
        "name": "IntegratedFluxReceiver",
        "source_uri": "resources/GeoFusion-IntegratedFluxReceiver.fmu",
        "model_identifier": "IntegratedFluxReceiver",
        "variables": (
            {
                "name": "intervalThermalEnergy", "value_reference": 21,
                "causality": "input", "unit": "J",
                "port_id": "hx.interval_thermal_energy",
                "boundary_id": "geo-to-hx-interval-energy",
                "port_role": "coupling_sink",
                "quantity_kind": "interval_integrated_energy",
            },
            {
                "name": "serviceEnergy", "value_reference": 31,
                "causality": "output", "unit": "J",
                "port_id": "hx.service_energy",
                "boundary_id": "hx-terminal-services",
                "port_role": "terminal_sink",
                "quantity_kind": "interval_energy",
            },
            {
                "name": "lossEnergy", "value_reference": 32,
                "causality": "output", "unit": "J",
                "port_id": "hx.loss_energy",
                "boundary_id": "hx-terminal-losses",
                "port_role": "terminal_sink",
                "quantity_kind": "interval_energy",
            },
            {
                "name": "storageDeltaEnergy", "value_reference": 33,
                "causality": "output", "unit": "J",
                "port_id": "hx.storage_delta_energy",
                "boundary_id": "hx-terminal-storage",
                "port_role": "terminal_sink",
                "quantity_kind": "interval_energy",
            },
            {
                "name": "storedEnergy", "value_reference": 34,
                "causality": "output", "unit": "J",
                "port_id": "hx.stored_energy",
                "boundary_id": "hx-storage-state",
                "port_role": "state_observation",
                "quantity_kind": "cumulative_energy_state",
            },
        ),
    },
)


class QualificationError(ValueError):
    """Raised for an invalid SSP package or FMI-to-SSP binding."""


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def secure_xml(data: bytes, label: str):
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


def safe_archive_entries(archive: zipfile.ZipFile, label: str):
    entries = {}
    for info in archive.infolist():
        name = info.filename
        if not name or "\\" in name or name.startswith("/"):
            raise QualificationError(f"{label}: unsafe ZIP member path {name!r}")
        path = PurePosixPath(name)
        canonical = path.as_posix() + ("/" if info.is_dir() else "")
        if name != canonical or any(part in ("", ".", "..") for part in path.parts):
            raise QualificationError(f"{label}: non-canonical/unsafe ZIP member {name!r}")
        if path.is_absolute() or ":" in path.parts[0]:
            raise QualificationError(f"{label}: absolute/drive-qualified ZIP member {name!r}")
        if name in entries:
            raise QualificationError(f"{label}: duplicate ZIP member {name!r}")
        mode = (info.external_attr >> 16) & 0o170000
        if mode == stat.S_IFLNK:
            raise QualificationError(f"{label}: symlink member is not permitted: {name!r}")
        if not info.is_dir():
            entries[name] = info
    return entries


def check_joule_unit(md, label: str) -> None:
    unit = md.find("UnitDefinitions/Unit[@name='J']")
    if unit is None or unit.find("BaseUnit") is None:
        raise QualificationError(f"{label}: joule UnitDefinition missing")
    base = unit.find("BaseUnit")
    values = {
        key: float(base.get(key, "0"))
        for key in ("kg", "m", "s", "A", "K", "mol", "cd", "rad")
    }
    values["factor"] = float(base.get("factor", "1"))
    values["offset"] = float(base.get("offset", "0"))
    if not all(math.isfinite(value) for value in values.values()) or values != J_UNIT:
        raise QualificationError(f"{label}: J must be exactly kg*m^2*s^-2 with factor=1, offset=0")


def inspect_fmu(data: bytes, profile: dict) -> dict:
    label = profile["name"]
    try:
        with zipfile.ZipFile(io.BytesIO(data), "r") as archive:
            entries = safe_archive_entries(archive, f"{label} FMU")
            if "modelDescription.xml" not in entries:
                raise QualificationError(f"{label}: modelDescription.xml missing")
            md_bytes = archive.read("modelDescription.xml")
            md = secure_xml(md_bytes, f"{label} modelDescription.xml")
            if etree.QName(md).localname != "fmiModelDescription" or md.get("fmiVersion") != FMI_VERSION:
                raise QualificationError(f"{label}: expected FMI 3.0 modelDescription")
            cosim = md.find("CoSimulation")
            if cosim is None or cosim.get("modelIdentifier") != profile["model_identifier"]:
                raise QualificationError(f"{label}: CoSimulation/modelIdentifier mismatch")
            binary_path = f"binaries/{PLATFORM}/{profile['model_identifier']}.so"
            if binary_path not in entries or not archive.read(binary_path):
                raise QualificationError(f"{label}: selected platform binary missing or empty")
            check_joule_unit(md, label)

            actual = {}
            for variable in profile["variables"]:
                node = md.find(f"ModelVariables/Float64[@name='{variable['name']}']")
                if node is None:
                    raise QualificationError(f"{label}: variable {variable['name']} missing")
                try:
                    vr = int(node.get("valueReference", ""))
                except ValueError as error:
                    raise QualificationError(f"{label}: invalid value reference for {variable['name']}") from error
                record = {
                    "value_reference": vr,
                    "causality": node.get("causality"),
                    "variability": node.get("variability"),
                    "unit": node.get("unit"),
                }
                expected = {
                    "value_reference": variable["value_reference"],
                    "causality": variable["causality"],
                    "variability": "continuous",
                    "unit": "J",
                }
                if record != expected:
                    raise QualificationError(f"{label}.{variable['name']}: FMI variable metadata mismatch")
                actual[variable["name"]] = record

            return {
                "component_name": profile["name"],
                "source_uri": profile["source_uri"],
                "model_identifier": profile["model_identifier"],
                "fmi_version": md.get("fmiVersion"),
                "binary_path": binary_path,
                "binary_sha256": sha256(archive.read(binary_path)),
                "model_description_sha256": sha256(md_bytes),
                "fmu_package_sha256": sha256(data),
                "variables": [
                    {
                        **variable,
                        **actual[variable["name"]],
                    }
                    for variable in profile["variables"]
                ],
                "capabilities": {
                    "canHandleVariableCommunicationStepSize":
                        cosim.get("canHandleVariableCommunicationStepSize", "false").lower() == "true",
                    "canGetAndSetFMUState":
                        cosim.get("canGetAndSetFMUState", "false").lower() == "true",
                    "canSerializeFMUState":
                        cosim.get("canSerializeFMUState", "false").lower() == "true",
                },
            }
    except zipfile.BadZipFile as error:
        raise QualificationError(f"{label}: invalid FMU ZIP") from error


def make_ssd() -> bytes:
    q = lambda ns, local: f"{{{ns}}}{local}"
    root = etree.Element(
        q(SSD_NS, "SystemStructureDescription"),
        nsmap={"ssd": SSD_NS, "ssc": SSC_NS},
        version=SSD_VERSION,
        name="GeoFusion-Integrated-Flux-Federation",
    )
    system = etree.SubElement(root, q(SSD_NS, "System"), name="GeoFusionIntegratedFluxSystem")
    elements = etree.SubElement(system, q(SSD_NS, "Elements"))

    for profile in PROFILES:
        component = etree.SubElement(
            elements,
            q(SSD_NS, "Component"),
            name=profile["name"],
            source=profile["source_uri"],
            type="application/x-fmu-sharedlibrary",
            implementation="CoSimulation",
        )
        connectors = etree.SubElement(component, q(SSD_NS, "Connectors"))
        for variable in profile["variables"]:
            connector = etree.SubElement(
                connectors,
                q(SSD_NS, "Connector"),
                name=variable["name"],
                kind=variable["causality"],
            )
            etree.SubElement(connector, q(SSC_NS, "Float64"), unit="J")

    connections = etree.SubElement(system, q(SSD_NS, "Connections"))
    etree.SubElement(
        connections,
        q(SSD_NS, "Connection"),
        startElement="IntegratedFluxSource",
        startConnector="intervalThermalEnergy",
        endElement="IntegratedFluxReceiver",
        endConnector="intervalThermalEnergy",
    )

    units = etree.SubElement(root, q(SSD_NS, "Units"))
    unit = etree.SubElement(units, q(SSC_NS, "Unit"), name="J")
    etree.SubElement(unit, q(SSC_NS, "BaseUnit"), kg="1", m="2", s="-2")
    etree.SubElement(root, q(SSD_NS, "DefaultExperiment"), startTime="0", stopTime="120")
    return etree.tostring(root, xml_declaration=True, encoding="UTF-8", pretty_print=True)


def validate_ssd(ssd_bytes: bytes) -> dict:
    root = secure_xml(ssd_bytes, "SystemStructure.ssd")
    if etree.QName(root).namespace != SSD_NS or etree.QName(root).localname != "SystemStructureDescription":
        raise QualificationError("incorrect SSP root or namespace")
    if root.get("version") != SSD_VERSION:
        raise QualificationError(f"expected SSD version {SSD_VERSION}")
    try:
        parser = etree.XMLParser(resolve_entities=False, no_network=True, load_dtd=False)
        schema_doc = etree.parse(str(XSD), parser)
        schema = etree.XMLSchema(schema_doc)
        schema.assertValid(root)
    except (OSError, etree.XMLSyntaxError, etree.XMLSchemaParseError, etree.DocumentInvalid) as error:
        raise QualificationError(f"SSP 2.0.1 XSD validation failed: {error}") from error

    system = root.find("ssd:System", namespaces=NS)
    if system is None:
        raise QualificationError("System missing")
    elements = system.find("ssd:Elements", namespaces=NS)
    if elements is None:
        raise QualificationError("Elements missing")
    components = {}
    for component in elements.findall("ssd:Component", namespaces=NS):
        name = component.get("name")
        if not name or name in components:
            raise QualificationError(f"missing/duplicate component name {name!r}")
        profile = next((item for item in PROFILES if item["name"] == name), None)
        if profile is None or component.get("source") != profile["source_uri"]:
            raise QualificationError(f"unknown component or source URI mismatch: {name}")
        if component.get("implementation") != "CoSimulation" or component.get("type") != "application/x-fmu-sharedlibrary":
            raise QualificationError(f"{name}: incorrect component implementation/type")
        connectors_node = component.find("ssd:Connectors", namespaces=NS)
        if connectors_node is None:
            raise QualificationError(f"{name}: connectors missing")
        connector_map = {}
        for connector in connectors_node.findall("ssd:Connector", namespaces=NS):
            connector_name = connector.get("name")
            if not connector_name or connector_name in connector_map:
                raise QualificationError(f"{name}: duplicate/missing connector")
            float64 = connector.find("ssc:Float64", namespaces=NS)
            if float64 is None or float64.get("unit") != "J":
                raise QualificationError(f"{name}.{connector_name}: connector must be Float64/J")
            connector_map[connector_name] = {"kind": connector.get("kind"), "type": "Float64", "unit": "J"}
        expected = {
            variable["name"]: {"kind": variable["causality"], "type": "Float64", "unit": "J"}
            for variable in profile["variables"]
        }
        if connector_map != expected:
            raise QualificationError(f"{name}: connector set/type/kind/unit mismatch")
        components[name] = {"source": component.get("source"), "connectors": connector_map}

    if set(components) != {profile["name"] for profile in PROFILES}:
        raise QualificationError("component set mismatch")

    connection_node = system.find("ssd:Connections", namespaces=NS)
    if connection_node is None:
        raise QualificationError("Connections missing")
    connections = [
        (
            node.get("startElement"), node.get("startConnector"),
            node.get("endElement"), node.get("endConnector"),
        )
        for node in connection_node.findall("ssd:Connection", namespaces=NS)
    ]
    expected_connections = [
        ("IntegratedFluxSource", "intervalThermalEnergy",
         "IntegratedFluxReceiver", "intervalThermalEnergy")
    ]
    if connections != expected_connections:
        raise QualificationError("interval-energy connection direction/identity mismatch")

    base = root.find("ssd:Units/ssc:Unit[@name='J']/ssc:BaseUnit", namespaces=NS)
    if base is None:
        raise QualificationError("J unit definition missing")
    values = {
        "kg": float(base.get("kg", "0")), "m": float(base.get("m", "0")),
        "s": float(base.get("s", "0")), "A": float(base.get("A", "0")),
        "K": float(base.get("K", "0")), "mol": float(base.get("mol", "0")),
        "cd": float(base.get("cd", "0")), "rad": float(base.get("rad", "0")),
        "factor": float(base.get("factor", "1")), "offset": float(base.get("offset", "0")),
    }
    if values != J_UNIT:
        raise QualificationError("SSP J unit does not equal kg*m^2*s^-2")
    return {
        "ssd_version": root.get("version"),
        "system_name": system.get("name"),
        "components": components,
        "connections": connections,
        "unit_J": values,
    }


def validate_bindings(semantics: dict, inspections: list[dict]) -> None:
    for inspection in inspections:
        connector_map = semantics["components"][inspection["component_name"]]["connectors"]
        variables = {item["name"]: item for item in inspection["variables"]}
        if set(connector_map) != set(variables):
            raise QualificationError(f"{inspection['component_name']}: SSP/FMI port set mismatch")
        for name, variable in variables.items():
            expected = {
                "kind": variable["causality"],
                "type": "Float64",
                "unit": variable["unit"],
            }
            if connector_map[name] != expected:
                raise QualificationError(f"{inspection['component_name']}.{name}: SSP/FMI binding mismatch")


def json_bytes(value: dict) -> bytes:
    return (json.dumps(value, sort_keys=True, indent=2) + "\n").encode("utf-8")


def build_package(source_fmu: Path, receiver_fmu: Path, output_path: Path) -> dict:
    source_fmu, receiver_fmu, output_path = Path(source_fmu), Path(receiver_fmu), Path(output_path)
    for path in (source_fmu, receiver_fmu):
        if not path.is_file() or path.is_symlink():
            raise QualificationError(f"FMU input must be regular non-symlink file: {path}")
    if output_path.is_symlink() or output_path.resolve() in (source_fmu.resolve(), receiver_fmu.resolve()):
        raise QualificationError("SSP output must not be symlink or overwrite an FMU")
    fmu_bytes = {
        PROFILES[0]["source_uri"]: source_fmu.read_bytes(),
        PROFILES[1]["source_uri"]: receiver_fmu.read_bytes(),
    }
    inspections = [
        inspect_fmu(fmu_bytes[profile["source_uri"]], profile)
        for profile in PROFILES
    ]
    ssd_bytes = make_ssd()
    semantics = validate_ssd(ssd_bytes)
    validate_bindings(semantics, inspections)

    manifest = {
        "schema_version": "0.1.0",
        "ssp_release": SSP_RELEASE,
        "ssd_format_version": SSD_VERSION,
        "fmi_version": FMI_VERSION,
        "platform": PLATFORM,
        "system_structure_sha256": sha256(ssd_bytes),
        "components": inspections,
        "connection": {
            "start_element": "IntegratedFluxSource",
            "start_connector": "intervalThermalEnergy",
            "end_element": "IntegratedFluxReceiver",
            "end_connector": "intervalThermalEnergy",
            "carrier": "thermal",
            "quantity_kind": "interval_integrated_energy",
            "unit": "J",
            "boundary_id": "geo-to-hx-interval-energy",
            "sampling_semantics": "integral_over_half_open_interval",
        },
        "claim_ceiling": (
            "SSP structure and FMI port binding only; no cross-tool round-trip, "
            "FMI execution, producer truth, physical validation or master superiority."
        ),
    }
    manifest_bytes = json_bytes(manifest)
    members = {
        "SystemStructure.ssd": ssd_bytes,
        **fmu_bytes,
        "resources/geofusion-integrated-flux-evidence.json": manifest_bytes,
    }
    output_path.parent.mkdir(parents=True, exist_ok=True)
    fd, temp_name = tempfile.mkstemp(prefix=output_path.name + ".", suffix=".tmp", dir=output_path.parent)
    import os
    os.close(fd)
    temporary_path = Path(temp_name)
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

    return {
        "ssp_package": output_path.name,
        "ssp_package_sha256": sha256(output_path.read_bytes()),
        "ssd_sha256": sha256(ssd_bytes),
        "components": inspections,
    }


def verify_package_bytes(data: bytes) -> dict:
    try:
        with zipfile.ZipFile(io.BytesIO(data), "r") as archive:
            entries = safe_archive_entries(archive, "integrated-flux SSP")
            if "SystemStructure.ssd" not in entries:
                raise QualificationError("SystemStructure.ssd missing")
            ssd_bytes = archive.read("SystemStructure.ssd")
            semantics = validate_ssd(ssd_bytes)
            inspections = []
            for profile in PROFILES:
                uri = profile["source_uri"]
                if uri not in entries:
                    raise QualificationError(f"referenced FMU missing: {uri}")
                inspections.append(inspect_fmu(archive.read(uri), profile))
            validate_bindings(semantics, inspections)
            if "resources/geofusion-integrated-flux-evidence.json" not in entries:
                raise QualificationError("integrated-flux evidence manifest missing")
            manifest_bytes = archive.read("resources/geofusion-integrated-flux-evidence.json")
            manifest = json.loads(manifest_bytes)
            if manifest.get("system_structure_sha256") != sha256(ssd_bytes):
                raise QualificationError("manifest SSD digest mismatch")
            if manifest.get("components") != inspections:
                raise QualificationError("manifest component identity mismatch")
            return {
                "package_sha256": sha256(data),
                "ssd_sha256": sha256(ssd_bytes),
                "components": inspections,
                "manifest": manifest,
            }
    except zipfile.BadZipFile as error:
        raise QualificationError("invalid SSP ZIP package") from error


def main() -> int:
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument("source_fmu", type=Path)
    parser.add_argument("receiver_fmu", type=Path)
    parser.add_argument("output_ssp", type=Path)
    args = parser.parse_args()
    receipt = build_package(args.source_fmu, args.receiver_fmu, args.output_ssp)
    print("Integrated-flux SSP 2.0.1 package: PASS")
    print(f"ssp_sha256={receipt['ssp_package_sha256']}")
    print(f"ssd_sha256={receipt['ssd_sha256']}")
    print(f"components={len(receipt['components'])}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"Integrated-flux SSP qualification: FAIL: {error}", file=sys.stderr)
        raise
