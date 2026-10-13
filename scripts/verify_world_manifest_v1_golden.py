#!/usr/bin/env python3
"""Independent golden-vector oracle for Symtropy WorldManifestV1.

This is intentionally a pinned-fixture verifier, not a production parser or
general-purpose encoder. It uses only Python's standard library to provide an
independent cross-language check of the Rust v1 canonical encoding.
"""
from __future__ import annotations

import hashlib
import struct

DOMAIN = b"symtropy.robotics.world-manifest.v1\0"
EXPECTED_LENGTH = 1221
EXPECTED_SHA256 = "136ddd6d2d42cd9d496c53c1141f5bafc576b6a8c1e5517dea80770171fe8f26"


class Encoder:
    def __init__(self) -> None:
        self.data = bytearray()

    def raw(self, value: bytes) -> None:
        self.data.extend(value)

    def u8(self, value: int) -> None:
        self.raw(struct.pack(">B", value))

    def u16(self, value: int) -> None:
        self.raw(struct.pack(">H", value))

    def u64(self, value: int) -> None:
        self.raw(struct.pack(">Q", value))

    def count(self, value: int) -> None:
        self.u64(value)

    def string(self, value: str) -> None:
        value = value.encode("utf-8")
        self.count(len(value))
        self.raw(value)

    def opt_string(self, value: str | None) -> None:
        self.u8(0 if value is None else 1)
        if value is not None:
            self.string(value)

    def f64(self, value: float) -> None:
        self.raw(struct.pack(">d", value))

    def opt_f64(self, value: float | None) -> None:
        self.u8(0 if value is None else 1)
        if value is not None:
            self.f64(value)

    def sha256_hex(self, value: str) -> None:
        decoded = bytes.fromhex(value)
        if len(decoded) != 32:
            raise ValueError("expected a 32-byte SHA-256 value")
        self.raw(decoded)


def artifact(e: Encoder, artifact_id: str, uri: str, digest: str) -> None:
    e.string(artifact_id)
    e.string(uri)
    e.sha256_hex(digest)


def manifest_bytes(*, gravity_x: float = 0.0, reverse_tables: bool = False) -> bytes:
    e = Encoder()
    e.raw(DOMAIN)
    e.u16(1)  # canonical format v1
    e.u16(1)  # manifest schema v1
    e.string("commitment-smoke-v1")
    e.u64(3)  # world generation
    e.u64(42)  # seed
    e.u64(1_000_000)  # fixed step ns
    for value in (gravity_x, -9.81, 0.0):
        e.f64(value)

    frames = [
        ("world", None, ("root",)),
        ("robot-base", "world", ("kinematic", "robot-kinematics", "base_link")),
        ("camera-optical", "robot-base",
         ("static", (0.1, 0.0, 0.2), (0.0, 0.0, 0.0, 1.0))),
    ]
    if reverse_tables:
        frames.reverse()
    frames.sort(key=lambda row: row[0])
    e.count(len(frames))
    for frame_id, parent, transform in frames:
        e.string(frame_id)
        e.opt_string(parent)
        if transform[0] == "root":
            e.u8(0)
        elif transform[0] == "static":
            e.u8(1)
            for value in (*transform[1], *transform[2]):
                e.f64(value)
        else:
            e.u8(2)
            e.string(transform[1])
            e.string(transform[2])

    e.u8(2)  # world fidelity: ReducedOrder
    assets = [
        ("environment", "urn:symtropy:test:environment", "ab" * 32),
        ("robot-cad", "urn:symtropy:test:robot-cad", "ab" * 32),
    ]
    if reverse_tables:
        assets.reverse()
    assets.sort(key=lambda row: (row[0], row[1], row[2]))
    e.count(len(assets))
    for row in assets:
        artifact(e, *row)

    solvers = [
        ("rigid-reference", 0, "symtropy-reference", "1.0.0",
         "cd" * 32, "ef" * 32, 1, 1),
    ]
    if reverse_tables:
        solvers.reverse()
    solvers.sort(key=lambda row: row[0])
    e.count(len(solvers))
    for sid, domain, impl, version, build, params, output_version, fidelity in solvers:
        e.string(sid)
        e.u8(domain)
        e.string(impl)
        e.string(version)
        e.sha256_hex(build)
        e.sha256_hex(params)
        e.u16(output_version)
        e.u8(fidelity)

    sensors = [
        ("camera-0", 0, "camera-optical", "rigid-reference",
         ("camera-calibration", "urn:symtropy:test:camera-calibration", "ab" * 32),
         30.0, 5_000_000, 100_000, 0.001, "m", 0.01, 0.0, 1.0),
    ]
    if reverse_tables:
        sensors.reverse()
    sensors.sort(key=lambda row: row[0])
    e.count(len(sensors))
    for sid, modality, frame, model, calibration, rate, latency, jitter, noise, unit, dropout, low, high in sensors:
        e.string(sid)
        e.u8(modality)
        e.string(frame)
        e.opt_string(model)
        if calibration is None:
            e.u8(0)
        else:
            e.u8(1)
            artifact(e, *calibration)
        e.f64(rate)
        e.u64(latency)
        e.u64(jitter)
        e.f64(noise)
        e.string(unit)
        e.f64(dropout)
        e.opt_f64(low)
        e.opt_f64(high)

    platforms = [{
        "id": "robot-0", "type": 5,
        "geometry": ("robot-cad", "urn:symtropy:test:robot-cad", "ab" * 32),
        "kinematics": ("robot-kinematics", "urn:symtropy:test:robot-kinematics", "ab" * 32),
        "base": "robot-base", "mass": 25.0, "rate": 200.0, "controller": 1,
        "actuators": [("arm", -1.0, 1.0, "N*m"), ("feed", 0.0, 0.01, "m/s")],
        "sensors": ["camera-0"],
    }]
    if reverse_tables:
        platforms.reverse()
    platforms.sort(key=lambda row: row["id"])
    e.count(len(platforms))
    for p in platforms:
        e.string(p["id"])
        e.u8(p["type"])
        artifact(e, *p["geometry"])
        artifact(e, *p["kinematics"])
        e.string(p["base"])
        e.f64(p["mass"])
        e.f64(p["rate"])
        e.u16(p["controller"])
        actuators = list(p["actuators"])
        if reverse_tables:
            actuators.reverse()
        actuators.sort(key=lambda row: row[0])
        e.count(len(actuators))
        for aid, low, high, unit in actuators:
            e.string(aid)
            e.f64(low)
            e.f64(high)
            e.string(unit)
        sensor_ids = list(p["sensors"])
        if reverse_tables:
            sensor_ids.reverse()
        sensor_ids.sort()
        e.count(len(sensor_ids))
        for sid in sensor_ids:
            e.string(sid)

    return bytes(e.data)


def main() -> None:
    assert hashlib.sha256(b"abc").hexdigest() == (
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    )
    encoded = manifest_bytes()
    digest = hashlib.sha256(encoded).hexdigest()
    assert len(encoded) == EXPECTED_LENGTH, (len(encoded), EXPECTED_LENGTH)
    assert digest == EXPECTED_SHA256, (digest, EXPECTED_SHA256)
    assert hashlib.sha256(manifest_bytes(reverse_tables=True)).hexdigest() == EXPECTED_SHA256
    changed = hashlib.sha256(manifest_bytes(gravity_x=0.25)).hexdigest()
    assert changed != EXPECTED_SHA256
    print(f"PASS: v1 canonical bytes={len(encoded)} sha256={digest}")
    print("PASS: set/table reorder preserves commitment")
    print(f"PASS: gravity mutation changes commitment ({changed})")


if __name__ == "__main__":
    main()
