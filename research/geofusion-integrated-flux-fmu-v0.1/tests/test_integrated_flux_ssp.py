#!/usr/bin/env python3
"""Adversarial SSP 2.0.1 tests for the integrated-flux FMI pair."""
from __future__ import annotations

import io
import json
from pathlib import Path
import zipfile
import sys
import tempfile

from lxml import etree

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "ssp"))
import integrated_flux_ssp as ssp  # noqa: E402

STAGE = ROOT / "build" / "stage"
SOURCE_FMU = STAGE / "GeoFusion-IntegratedFluxSource.fmu"
RECEIVER_FMU = STAGE / "GeoFusion-IntegratedFluxReceiver.fmu"
PACKAGE = STAGE / "GeoFusion-Integrated-Flux-Federation.ssp"


def expect_reject(operation, label: str) -> None:
    try:
        operation()
    except ssp.QualificationError:
        return
    raise AssertionError(f"negative SSP control was accepted: {label}")


def rewrite_package(source: bytes, replacement_name: str | None = None,
                    replacement_bytes: bytes | None = None, extra_member: tuple[str, bytes] | None = None) -> bytes:
    output = io.BytesIO()
    with zipfile.ZipFile(io.BytesIO(source), "r") as original:
        members = {
            info.filename: original.read(info.filename)
            for info in original.infolist()
            if not info.is_dir()
        }
    if replacement_name is not None and replacement_bytes is not None:
        members[replacement_name] = replacement_bytes
    if extra_member is not None:
        members[extra_member[0]] = extra_member[1]
    with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_STORED) as rewritten:
        for name in sorted(members):
            info = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_STORED
            rewritten.writestr(info, members[name])
    return output.getvalue()


def main() -> int:
    for path in (SOURCE_FMU, RECEIVER_FMU, PACKAGE):
        if not path.is_file():
            raise FileNotFoundError(path)

    package_bytes = PACKAGE.read_bytes()
    receipt = ssp.verify_package_bytes(package_bytes)
    if len(receipt["components"]) != 2:
        raise AssertionError("integrated-flux SSP must contain exactly two FMUs")

    with __import__("zipfile").ZipFile(PACKAGE, "r") as archive:
        manifest = json.loads(archive.read("resources/geofusion-integrated-flux-evidence.json"))
        ssd_bytes = archive.read("SystemStructure.ssd")

    source_vars = {
        item["name"]: item for item in manifest["components"][0]["variables"]
    }
    receiver_vars = {
        item["name"]: item for item in manifest["components"][1]["variables"]
    }
    if set(source_vars) != {"cumulativeThermalEnergy", "intervalThermalEnergy"}:
        raise AssertionError("source J-port map incomplete")
    if set(receiver_vars) != {
        "intervalThermalEnergy", "serviceEnergy", "lossEnergy", "storageDeltaEnergy", "storedEnergy"
    }:
        raise AssertionError("receiver J-port map incomplete")
    transfer = source_vars["intervalThermalEnergy"]
    sink = receiver_vars["intervalThermalEnergy"]
    if (transfer["port_role"], transfer["boundary_id"], transfer["unit"]) != (
        "coupling_source", "geo-to-hx-interval-energy", "J"
    ):
        raise AssertionError("source interval-energy physical binding mismatch")
    if (sink["port_role"], sink["boundary_id"], sink["unit"]) != (
        "coupling_sink", "geo-to-hx-interval-energy", "J"
    ):
        raise AssertionError("receiver interval-energy physical binding mismatch")
    if receiver_vars["storedEnergy"]["port_role"] != "state_observation":
        raise AssertionError("storedEnergy must remain a sampled state observation")

    # The assembler and verifier are byte-deterministic for the same FMUs.
    with tempfile.TemporaryDirectory(prefix="geofusion-ssp-determinism-") as temporary:
        first = Path(temporary) / "first.ssp"
        second = Path(temporary) / "second.ssp"
        ssp.build_package(SOURCE_FMU, RECEIVER_FMU, first)
        ssp.build_package(SOURCE_FMU, RECEIVER_FMU, second)
        if first.read_bytes() != second.read_bytes():
            raise AssertionError("identical FMU inputs did not produce identical SSP package bytes")

    root = etree.fromstring(ssd_bytes)
    source_interval = root.xpath(
        ".//ssd:Component[@name='IntegratedFluxSource']/ssd:Connectors/ssd:Connector[@name='intervalThermalEnergy']",
        namespaces=ssp.NS,
    )
    if len(source_interval) != 1:
        raise AssertionError("source interval connector not found")
    source_interval[0].find("ssc:Float64", namespaces=ssp.NS).set("unit", "W")
    expect_reject(
        lambda: ssp.validate_ssd(etree.tostring(root, xml_declaration=True, encoding="UTF-8")),
        "J-to-W connector unit downgrade",
    )

    root = etree.fromstring(ssd_bytes)
    source_interval = root.xpath(
        ".//ssd:Component[@name='IntegratedFluxSource']/ssd:Connectors/ssd:Connector[@name='intervalThermalEnergy']",
        namespaces=ssp.NS,
    )
    source_interval[0].set("kind", "input")
    expect_reject(
        lambda: ssp.validate_ssd(etree.tostring(root, xml_declaration=True, encoding="UTF-8")),
        "wrong source connector direction",
    )

    root = etree.fromstring(ssd_bytes)
    connection = root.find(".//ssd:Connection", namespaces=ssp.NS)
    connection.set("startConnector", "cumulativeThermalEnergy")
    expect_reject(
        lambda: ssp.validate_ssd(etree.tostring(root, xml_declaration=True, encoding="UTF-8")),
        "rewire cumulative counter instead of interval integral",
    )

    doctype = (
        b'<!DOCTYPE x [<!ENTITY ext SYSTEM "file:///etc/passwd">]>'
        + ssd_bytes.split(b"?>", 1)[-1]
    )
    expect_reject(lambda: ssp.validate_ssd(doctype), "DTD/external entity")

    manifest_name = "resources/geofusion-integrated-flux-evidence.json"
    duplicate_key_manifest = b'{"schema_version":"0.1.0","schema_version":"0.1.0"}'
    duplicate_key_package = rewrite_package(package_bytes, manifest_name, duplicate_key_manifest)
    expect_reject(
        lambda: ssp.verify_package_bytes(duplicate_key_package),
        "duplicate manifest JSON key",
    )
    traversal_package = rewrite_package(
        package_bytes,
        extra_member=("../outside.txt", b"must not escape archive root"),
    )
    expect_reject(
        lambda: ssp.verify_package_bytes(traversal_package),
        "outer SSP archive path traversal",
    )

    print("Integrated-flux SSP 2.0.1 package audit: PASS")
    print(f"package_sha256={receipt['package_sha256']}")
    print("negative_controls=wrong-unit,wrong-direction,wrong-transfer,DTD,duplicate-json-key,path-traversal: REJECTED")
    print("determinism=identical-input-bytes: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
