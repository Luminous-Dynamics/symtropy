#!/usr/bin/env python3
"""Strict structural/fixity verifier for the synthetic Physical Recovery Capsule.

This is a small RFC 8493 profile checker for the committed fixture, not a complete
general-purpose BagIt implementation or Draft 2020-12 JSON Schema validator.
It verifies every manifest digest and the capsule's known-answer/source references.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
from typing import Iterable


def digest(path: Path, algorithm: str) -> str:
    hasher = hashlib.new(algorithm)
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def safe_relative_path(raw: str) -> str:
    path = PurePosixPath(raw)
    if (
        not raw
        or path.is_absolute()
        or "\\" in raw
        or any(part in ("", ".", "..") for part in path.parts)
        or raw.startswith("/")
    ):
        raise ValueError(f"unsafe manifest path: {raw!r}")
    return path.as_posix()


def parse_manifest(path: Path) -> dict[str, str]:
    entries: dict[str, str] = {}
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            continue
        pieces = line.split(None, 1)
        if len(pieces) != 2:
            raise ValueError(f"{path.name}:{line_number}: invalid manifest line")
        expected, raw_path = pieces
        relative = safe_relative_path(raw_path)
        if relative in entries:
            raise ValueError(f"{path.name}:{line_number}: duplicate path {relative}")
        entries[relative] = expected.lower()
    return entries


def verify_manifest(root: Path, manifest: Path, algorithm: str, prefix: str | None) -> dict[str, str]:
    entries = parse_manifest(manifest)
    for relative, expected in entries.items():
        if prefix is not None and not relative.startswith(prefix):
            raise ValueError(f"{manifest.name}: path is outside {prefix}: {relative}")
        target = root.joinpath(*PurePosixPath(relative).parts)
        if not target.is_file() or target.is_symlink():
            raise ValueError(f"{manifest.name}: missing or non-regular file: {relative}")
        actual = digest(target, algorithm)
        if actual != expected:
            raise ValueError(f"{manifest.name}: digest mismatch for {relative}")
    return entries


def payload_file_set(root: Path) -> set[str]:
    data = root / "data"
    if not data.is_dir() or data.is_symlink():
        raise ValueError("BagIt payload directory data/ is missing or not a directory")
    result = set()
    for item in data.rglob("*"):
        if item.is_symlink():
            raise ValueError(f"symlinks are not accepted in this profile: {item}")
        if item.is_file():
            result.add("data/" + item.relative_to(data).as_posix())
    return result


def tag_file_set(root: Path) -> set[str]:
    result = set()
    for item in root.rglob("*"):
        if item.is_symlink() or not item.is_file():
            continue
        relative = item.relative_to(root).as_posix()
        if relative.startswith("data/"):
            continue
        if relative.startswith("tagmanifest-"):
            continue
        result.add(relative)
    return result


def verify_bag(root: Path) -> dict:
    bagit_path = root / "bagit.txt"
    if not bagit_path.is_file():
        raise ValueError("bagit.txt is required")
    bagit_lines = bagit_path.read_text(encoding="utf-8").splitlines()
    if bagit_lines != [
        "BagIt-Version: 1.0",
        "Tag-File-Character-Encoding: UTF-8",
    ]:
        raise ValueError("bagit.txt does not declare the supported BagIt 1.0 profile")

    payload_manifests = sorted(root.glob("manifest-*.txt"))
    if not payload_manifests:
        raise ValueError("at least one payload manifest is required")

    payload_entries: dict[str, str] | None = None
    algorithms = []
    for manifest in payload_manifests:
        algorithm = manifest.stem.removeprefix("manifest-")
        hashlib.new(algorithm)
        current = verify_manifest(root, manifest, algorithm, "data/")
        if payload_entries is None:
            payload_entries = current
        elif set(current) != set(payload_entries):
            raise ValueError("payload manifests list different file sets")
        algorithms.append(algorithm)

    if payload_entries is None:
        raise ValueError("no payload files listed")
    actual_payloads = payload_file_set(root)
    if set(payload_entries) != actual_payloads:
        raise ValueError(
            f"payload inventory mismatch: manifest={sorted(payload_entries)}, actual={sorted(actual_payloads)}"
        )

    tag_manifests = sorted(root.glob("tagmanifest-*.txt"))
    if not tag_manifests:
        raise ValueError("this profile requires at least one tag manifest")
    expected_tags = tag_file_set(root)
    for manifest in tag_manifests:
        algorithm = manifest.stem.removeprefix("tagmanifest-")
        hashlib.new(algorithm)
        entries = verify_manifest(root, manifest, algorithm, None)
        if set(entries) != expected_tags:
            raise ValueError(
                f"tag inventory mismatch: manifest={sorted(entries)}, actual={sorted(expected_tags)}"
            )
        for payload_manifest in payload_manifests:
            if payload_manifest.name not in entries:
                raise ValueError(f"{manifest.name} omits {payload_manifest.name}")

    payload_octets = sum(
        (root / relative).stat().st_size for relative in actual_payloads
    )
    bag_info = root / "bag-info.txt"
    if bag_info.exists():
        values = {}
        for line in bag_info.read_text(encoding="utf-8").splitlines():
            if ": " in line:
                key, value = line.split(": ", 1)
                values[key] = value
        oxum = values.get("Payload-Oxum")
        if oxum:
            expected_oxum = f"{payload_octets}.{len(actual_payloads)}"
            if oxum != expected_oxum:
                raise ValueError(f"Payload-Oxum mismatch: {oxum} != {expected_oxum}")

    return {
        "payload_files": len(actual_payloads),
        "payload_bytes": payload_octets,
        "payload_algorithms": algorithms,
        "tag_manifests": len(tag_manifests),
        "tag_files_covered": len(expected_tags),
    }


def verify_capsule(root: Path) -> dict:
    descriptor_path = root / "capsule" / "recovery_capsule.json"
    descriptor = json.loads(descriptor_path.read_text(encoding="utf-8"))
    if descriptor.get("schema_version") != "physical-recovery-capsule-v0.1":
        raise ValueError("unexpected capsule descriptor version")
    if descriptor.get("qualification_status") != "fixture_only":
        raise ValueError("synthetic sample must remain fixture_only")
    if descriptor.get("evidence_class") != "synthetic_fixture":
        raise ValueError("synthetic sample evidence class was promoted")
    if descriptor.get("independence_assessment", {}).get("status") != "unassessed":
        raise ValueError("fixture decoder independence must remain unassessed")
    if descriptor.get("reader_replacement", {}).get("status") != "not_performed":
        raise ValueError("fixture reader replacement must remain not_performed")

    known_answer = json.loads(
        (root / "capsule" / "known_answer.json").read_text(encoding="utf-8")
    )
    payload_path = safe_relative_path(known_answer["payload_path"])
    payload = root.joinpath(*PurePosixPath(payload_path).parts)
    actual_digest = digest(payload, known_answer["expected_digest_algorithm"])
    if actual_digest != known_answer["expected_digest"].lower():
        raise ValueError("known-answer payload digest mismatch")
    if payload.stat().st_size != known_answer["expected_byte_length"]:
        raise ValueError("known-answer payload byte length mismatch")

    decoder_records = descriptor.get("decoder_implementations", [])
    if len(decoder_records) < 2:
        raise ValueError("capsule must declare at least two decoder records")
    for record in decoder_records:
        source_path = safe_relative_path(record["source_ref"])
        source = root.joinpath(*PurePosixPath(source_path).parts)
        actual = digest(source, "sha256")
        if actual != record["source_digest_sha256"].lower():
            raise ValueError(f"decoder source digest mismatch: {source_path}")

    return {
        "capsule_id": descriptor["capsule_id"],
        "qualification_status": descriptor["qualification_status"],
        "evidence_class": descriptor["evidence_class"],
        "known_answer_sha256": actual_digest,
        "decoder_records": len(decoder_records),
        "decoder_independence": "unassessed",
        "reader_replacement": "not_performed",
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "bag",
        nargs="?",
        default=str(Path(__file__).parents[1] / "recovery-capsule-synthetic-v0_1"),
        help="Path to the BagIt bag directory",
    )
    args = parser.parse_args()
    root = Path(args.bag).resolve()
    bag = verify_bag(root)
    capsule = verify_capsule(root)
    print(json.dumps({
        "result": "synthetic_fixture_structure_and_fixity_passed",
        "bag": bag,
        "capsule": capsule,
        "physical_reader_qualification": "not_claimed",
        "production_archive_qualification": "not_claimed",
    }, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
