#!/usr/bin/env python3
"""Adversarial SSP 2.0.1 tests for the integrated-flux FMI pair."""
from __future__ import annotations

import json
from pathlib import Path
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

    print("Integrated-flux SSP 2.0.1 package audit: PASS")
    print(f"package_sha256={receipt['package_sha256']}")
    print("negative_controls=wrong-unit,wrong-direction,wrong-transfer,DTD: REJECTED")
    print("determinism=identical-input-bytes: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
