"""Adversarial SSP 2.0.1 package tests.

The FMUs created here are temporary NON-EXECUTABLE test doubles. They have
placeholder binary bytes and are never committed or used for simulation.
"""
from __future__ import annotations

import io
import json
from pathlib import Path
import tempfile
import unittest
import zipfile

from lxml import etree

import ssp_package as ssp


def model_description(
    model_name: str,
    model_identifier: str,
    variable_name: str,
    value_reference: int,
    causality: str,
    unit: str = "W",
) -> bytes:
    return f"""<?xml version="1.0" encoding="UTF-8"?>
<fmiModelDescription fmiVersion="3.0" modelName="{model_name}" instantiationToken="test-only-token" variableNamingConvention="flat">
  <UnitDefinitions>
    <Unit name="W"><BaseUnit kg="1" m="2" s="-3"/></Unit>
  </UnitDefinitions>
  <CoSimulation modelIdentifier="{model_identifier}" canHandleVariableCommunicationStepSize="true" canGetAndSetFMUState="false" canSerializeFMUState="false"/>
  <ModelVariables>
    <Float64 name="{variable_name}" valueReference="{value_reference}" causality="{causality}" variability="continuous" unit="{unit}" start="0"/>
  </ModelVariables>
  <ModelStructure>
    <Output><Unknown index="1"/></Output>
  </ModelStructure>
</fmiModelDescription>
""".encode("utf-8")


def fake_fmu(
    model_name: str,
    model_identifier: str,
    variable_name: str,
    value_reference: int,
    causality: str,
    *,
    unit: str = "W",
    include_binary: bool = True,
    extra_members: dict[str, bytes] | None = None,
) -> bytes:
    """Create a non-executable FMU ZIP double for package tests only."""
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w", compression=zipfile.ZIP_STORED) as archive:
        archive.writestr(
            "modelDescription.xml",
            model_description(model_name, model_identifier, variable_name, value_reference, causality, unit),
        )
        if include_binary:
            archive.writestr(
                f"binaries/{ssp.PLATFORM}/{model_identifier}.so",
                b"NON-EXECUTABLE TEST-ONLY FMU BINARY PLACEHOLDER",
            )
        for name, data in (extra_members or {}).items():
            archive.writestr(name, data)
    return buffer.getvalue()


def rewrite_zip(source: bytes, replacements: dict[str, bytes]) -> bytes:
    """Rewrite archive members in memory; source and untouched bytes are retained."""
    buffer = io.BytesIO()
    with zipfile.ZipFile(io.BytesIO(source), "r") as original:
        members = {info.filename: original.read(info.filename) for info in original.infolist() if not info.is_dir()}
    members.update(replacements)
    with zipfile.ZipFile(buffer, "w", compression=zipfile.ZIP_STORED) as rewritten:
        for name in sorted(members):
            info = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_STORED
            rewritten.writestr(info, members[name])
    return buffer.getvalue()


class SSPPackageTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="geofusion-ssp-test-")
        self.root = Path(self.temporary.name)
        self.geo = self.root / "GeoFusion-GeothermalSource.fmu"
        self.hx = self.root / "GeoFusion-HeatExchangerBuffer.fmu"
        self.output = self.root / "GeoFusion-Thermal-Federation.ssp"
        self.geo.write_bytes(
            fake_fmu("TEST_ONLY_GeothermalSource", "GeothermalSource", "thermalPowerOut", 11, "output")
        )
        self.hx.write_bytes(
            fake_fmu("TEST_ONLY_HeatExchangerBuffer", "HeatExchangerBuffer", "thermalPowerIn", 21, "input")
        )

    def tearDown(self):
        self.temporary.cleanup()

    def build(self) -> bytes:
        ssp.build_package(self.geo, self.hx, self.output)
        return self.output.read_bytes()

    def test_official_ssp_schema_validates_canonical_descriptor(self):
        descriptor = ssp.make_ssd()
        semantics = ssp.validate_ssd(descriptor)
        self.assertEqual(semantics["ssd_version"], "2.0")
        self.assertEqual(semantics["system_name"], "GeoFusionThermalFederation")
        self.assertEqual(len(semantics["connections"]), 1)
        self.assertEqual(semantics["unit_W"]["kg"], 1.0)
        self.assertEqual(semantics["unit_W"]["m"], 2.0)
        self.assertEqual(semantics["unit_W"]["s"], -3.0)

    def test_builds_and_verifies_two_fmu_ssp_package(self):
        package = self.build()
        result = ssp.verify_package_bytes(package)
        self.assertEqual(result["ssp_release"], "2.0.1")
        self.assertEqual(len(result["components"]), 2)
        self.assertEqual(result["connections"], [
            ("GeothermalSource", "thermalPowerOut", "HeatExchangerBuffer", "thermalPowerIn")
        ])
        sidecar = self.output.with_suffix(self.output.suffix + ".sha256")
        self.assertTrue(sidecar.is_file())
        self.assertEqual(sidecar.read_text().split()[0], ssp.sha256(package))

    def test_package_is_byte_deterministic_for_identical_inputs(self):
        first = self.build()
        second_output = self.root / "second.ssp"
        ssp.build_package(self.geo, self.hx, second_output)
        second = second_output.read_bytes()
        self.assertEqual(first, second)
        self.assertEqual(ssp.sha256(first), ssp.sha256(second))

    def test_fmu_output_variable_must_have_declared_name_type_causality_and_unit(self):
        bad = self.root / "bad-output.fmu"
        bad.write_bytes(
            fake_fmu("Bad", "GeothermalSource", "wrongName", 11, "input")
        )
        with self.assertRaises(ssp.QualificationError):
            ssp.inspect_fmu_bytes(bad.read_bytes(), ssp.PROFILES[0])

    def test_fmu_watt_dimension_must_be_exact(self):
        bad = self.root / "bad-unit.fmu"
        bad.write_bytes(
            fake_fmu("Bad", "GeothermalSource", "thermalPowerOut", 11, "output", unit="J")
        )
        with self.assertRaises(ssp.QualificationError):
            ssp.inspect_fmu_bytes(bad.read_bytes(), ssp.PROFILES[0])

    def test_missing_selected_platform_binary_is_rejected(self):
        bad = self.root / "missing-binary.fmu"
        bad.write_bytes(
            fake_fmu("Bad", "GeothermalSource", "thermalPowerOut", 11, "output", include_binary=False)
        )
        with self.assertRaisesRegex(ssp.QualificationError, "platform binary"):
            ssp.inspect_fmu_bytes(bad.read_bytes(), ssp.PROFILES[0])

    def test_wrong_fmi_model_identifier_is_rejected(self):
        bad = self.root / "wrong-id.fmu"
        bad.write_bytes(fake_fmu("Bad", "DifferentIdentifier", "thermalPowerOut", 11, "output"))
        with self.assertRaisesRegex(ssp.QualificationError, "modelIdentifier"):
            ssp.inspect_fmu_bytes(bad.read_bytes(), ssp.PROFILES[0])

    def test_zip_path_traversal_in_fmu_is_rejected(self):
        bad = self.root / "path-traversal.fmu"
        bad.write_bytes(
            fake_fmu(
                "Bad", "GeothermalSource", "thermalPowerOut", 11, "output",
                extra_members={"../escape.txt": b"outside"},
            )
        )
        with self.assertRaisesRegex(ssp.QualificationError, "unsafe ZIP member path"):
            ssp.inspect_fmu_bytes(bad.read_bytes(), ssp.PROFILES[0])

    def test_duplicate_zip_member_names_are_rejected(self):
        duplicate = io.BytesIO()
        with zipfile.ZipFile(duplicate, "w", compression=zipfile.ZIP_STORED) as archive:
            archive.writestr("modelDescription.xml", model_description(
                "Bad", "GeothermalSource", "thermalPowerOut", 11, "output"
            ))
            archive.writestr("modelDescription.xml", b"duplicate")
        with self.assertRaisesRegex(ssp.QualificationError, "duplicate ZIP member"):
            ssp.inspect_fmu_bytes(duplicate.getvalue(), ssp.PROFILES[0])

    def test_wrong_connector_direction_is_rejected_even_when_xsd_valid(self):
        package = self.build()
        with zipfile.ZipFile(io.BytesIO(package), "r") as archive:
            descriptor = archive.read("SystemStructure.ssd")
        root = etree.fromstring(descriptor)
        connector = root.find(".//ssd:Connector[@name='thermalPowerOut']", namespaces=ssp.NS)
        connector.set("kind", "input")
        mutated = rewrite_zip(package, {"SystemStructure.ssd": etree.tostring(root, xml_declaration=True, encoding="UTF-8")})
        with self.assertRaisesRegex(ssp.QualificationError, "connector/type/kind/unit map mismatch"):
            ssp.verify_package_bytes(mutated)

    def test_rewired_or_reversed_connection_is_rejected(self):
        package = self.build()
        with zipfile.ZipFile(io.BytesIO(package), "r") as archive:
            descriptor = archive.read("SystemStructure.ssd")
        root = etree.fromstring(descriptor)
        connection = root.find(".//ssd:Connection", namespaces=ssp.NS)
        connection.set("endConnector", "thermalPowerOut")
        mutated = rewrite_zip(package, {"SystemStructure.ssd": etree.tostring(root, xml_declaration=True, encoding="UTF-8")})
        with self.assertRaisesRegex(ssp.QualificationError, "connection direction or endpoint"):
            ssp.verify_package_bytes(mutated)

    def test_path_traversal_source_uri_is_rejected(self):
        package = self.build()
        with zipfile.ZipFile(io.BytesIO(package), "r") as archive:
            descriptor = archive.read("SystemStructure.ssd")
        root = etree.fromstring(descriptor)
        component = root.find(".//ssd:Component[@name='GeothermalSource']", namespaces=ssp.NS)
        component.set("source", "resources/../../outside.fmu")
        mutated = rewrite_zip(package, {"SystemStructure.ssd": etree.tostring(root, xml_declaration=True, encoding="UTF-8")})
        with self.assertRaisesRegex(ssp.QualificationError, "unsafe component source URI"):
            ssp.verify_package_bytes(mutated)

    def test_tampered_fmu_bytes_fail_manifest_cross_check(self):
        package = self.build()
        with zipfile.ZipFile(io.BytesIO(package), "r") as archive:
            geothermal = archive.read("resources/GeoFusion-GeothermalSource.fmu")
        mutated_fmu = geothermal + b"tamper"
        mutated = rewrite_zip(package, {"resources/GeoFusion-GeothermalSource.fmu": mutated_fmu})
        with self.assertRaisesRegex(ssp.QualificationError, "manifest does not match actual FMU"):
            ssp.verify_package_bytes(mutated)

    def test_ssd_doctype_and_external_entity_are_rejected(self):
        evil = b"""<?xml version="1.0"?>
<!DOCTYPE x [<!ENTITY ext SYSTEM "file:///etc/passwd">]>
<ssd:SystemStructureDescription xmlns:ssd="http://ssp-standard.org/SSP1/SystemStructureDescription"
 xmlns:ssc="http://ssp-standard.org/SSP1/SystemStructureCommon" version="2.0" name="&ext;"/>
"""
        with self.assertRaisesRegex(ssp.QualificationError, "DTD declarations"):
            ssp.validate_ssd(evil)

    def test_symlink_fmu_input_is_rejected(self):
        symlink = self.root / "link.fmu"
        symlink.symlink_to(self.geo)
        with self.assertRaisesRegex(ssp.QualificationError, "non-symlink"):
            ssp.build_package(symlink, self.hx, self.output)


    def test_non_finite_fmu_unit_exponent_is_rejected(self):
        original = fake_fmu(
            "Bad", "GeothermalSource", "thermalPowerOut", 11, "output"
        )
        with zipfile.ZipFile(io.BytesIO(original), "r") as archive:
            md = archive.read("modelDescription.xml")
        md = md.replace(b'm="2"', b'm="NaN"')
        bad = rewrite_zip(original, {"modelDescription.xml": md})
        with self.assertRaisesRegex(ssp.QualificationError, "must be finite"):
            ssp.inspect_fmu_bytes(bad, ssp.PROFILES[0])

    def test_component_mime_type_must_remain_fmu(self):
        package = self.build()
        with zipfile.ZipFile(io.BytesIO(package), "r") as archive:
            descriptor = archive.read("SystemStructure.ssd")
        root = etree.fromstring(descriptor)
        component = root.find(".//ssd:Component[@name='GeothermalSource']", namespaces=ssp.NS)
        component.set("type", "application/zip")
        mutated = rewrite_zip(package, {
            "SystemStructure.ssd": etree.tostring(root, xml_declaration=True, encoding="UTF-8")
        })
        with self.assertRaisesRegex(ssp.QualificationError, "FMU shared-library"):
            ssp.verify_package_bytes(mutated)

    def test_embedded_manifest_duplicate_json_keys_are_rejected(self):
        package = self.build()
        with zipfile.ZipFile(io.BytesIO(package), "r") as archive:
            manifest = archive.read("resources/geofusion-ssp-evidence.json")
        manifest = manifest.replace(
            b'"schema_version": "0.1.0"',
            b'"schema_version": "0.1.0", "schema_version": "0.1.0"',
            1,
        )
        mutated = rewrite_zip(package, {
            "resources/geofusion-ssp-evidence.json": manifest
        })
        with self.assertRaisesRegex(ssp.QualificationError, "duplicate JSON key"):
            ssp.verify_package_bytes(mutated)

    def test_embedded_manifest_connection_must_match_ssd(self):
        package = self.build()
        with zipfile.ZipFile(io.BytesIO(package), "r") as archive:
            manifest = json.loads(archive.read("resources/geofusion-ssp-evidence.json"))
        manifest["connections"][0]["unit"] = "J"
        mutated = rewrite_zip(package, {
            "resources/geofusion-ssp-evidence.json": json.dumps(manifest).encode("utf-8")
        })
        with self.assertRaisesRegex(ssp.QualificationError, "connection declaration mismatch"):
            ssp.verify_package_bytes(mutated)

    def test_outer_ssp_archive_path_traversal_is_rejected(self):
        package = self.build()
        mutated = rewrite_zip(package, {"../escape.txt": b"outside"})
        with self.assertRaisesRegex(ssp.QualificationError, "unsafe ZIP member path"):
            ssp.verify_package_bytes(mutated)

    def test_output_cannot_overwrite_an_input_fmu(self):
        with self.assertRaisesRegex(ssp.QualificationError, "must not overwrite"):
            ssp.build_package(self.geo, self.hx, self.geo)



if __name__ == "__main__":
    unittest.main(verbosity=2)
