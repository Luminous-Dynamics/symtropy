#!/usr/bin/env python3
# Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Bounded, fail-closed materialization for retained evidence envelopes."""

from __future__ import annotations

import hashlib
import stat
import tempfile
import zipfile
from contextlib import contextmanager
from pathlib import Path
from typing import Iterator

EXPECTED_FILES = (
    "EXECUTION_IDENTITY.json",
    "EXECUTION_IDENTITY.sha256",
    "OBSERVATION.json",
    "OBSERVATION.sha256",
)
MAX_MEMBER_BYTES = 1_048_576
MAX_ENVELOPE_BYTES = 4_194_304


class IngestError(ValueError):
    pass


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(64 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _bounded_read(handle, *, label: str, declared_size: int | None = None) -> bytes:
    data = handle.read(MAX_MEMBER_BYTES + 1)
    if len(data) > MAX_MEMBER_BYTES:
        raise IngestError(f"evidence member exceeds {MAX_MEMBER_BYTES} bytes: {label}")
    if handle.read(1):
        raise IngestError(f"evidence member expands beyond {MAX_MEMBER_BYTES} bytes: {label}")
    if declared_size is not None and len(data) != declared_size:
        raise IngestError(
            f"evidence member decoded size mismatch for {label}: "
            f"got {len(data)}, declared {declared_size}"
        )
    return data


def _validate_names(names: list[str], *, kind: str) -> None:
    if len(names) != len(set(names)):
        raise IngestError(f"evidence {kind} contains duplicate member names")
    if sorted(names) != sorted(EXPECTED_FILES):
        raise IngestError(
            f"evidence {kind} must contain exactly the four v1 envelope files; "
            f"found: {sorted(names)}"
        )


def _copy_directory(source: Path, destination: Path) -> None:
    if not source.is_dir() or source.is_symlink():
        raise IngestError(f"evidence path must be a real directory: {source}")
    names = [child.name for child in source.iterdir()]
    _validate_names(names, kind="directory")
    total = 0
    for name in EXPECTED_FILES:
        child = source / name
        if not child.is_file() or child.is_symlink():
            raise IngestError(f"evidence member must be a regular file: {name}")
        with child.open("rb") as handle:
            data = _bounded_read(handle, label=name)
        total += len(data)
        if total > MAX_ENVELOPE_BYTES:
            raise IngestError(f"evidence envelope exceeds {MAX_ENVELOPE_BYTES} bytes")
        (destination / name).write_bytes(data)


def _copy_zip(source: Path, destination: Path) -> str:
    if not source.is_file() or source.is_symlink():
        raise IngestError(f"evidence ZIP must be a real file: {source}")
    if source.stat().st_size > MAX_ENVELOPE_BYTES:
        raise IngestError(f"evidence ZIP exceeds {MAX_ENVELOPE_BYTES} bytes")
    archive_digest = _sha256(source)
    try:
        archive = zipfile.ZipFile(source)
    except (OSError, zipfile.BadZipFile) as error:
        raise IngestError("evidence ZIP is invalid") from error

    with archive:
        infos = archive.infolist()
        _validate_names([info.filename for info in infos], kind="ZIP")
        total = 0
        for info in infos:
            name = info.filename
            if info.is_dir() or "/" in name or "\\" in name:
                raise IngestError(f"ZIP member must be a top-level regular file: {name!r}")
            if info.flag_bits & 0x1:
                raise IngestError(f"encrypted ZIP member is not supported: {name!r}")
            unix_mode = (info.external_attr >> 16) & 0xFFFF
            if unix_mode and stat.S_IFMT(unix_mode) not in (0, stat.S_IFREG):
                raise IngestError(f"ZIP member is not a regular file: {name!r}")
            if info.file_size > MAX_MEMBER_BYTES:
                raise IngestError(f"evidence member exceeds {MAX_MEMBER_BYTES} bytes: {name}")
            total += info.file_size
            if total > MAX_ENVELOPE_BYTES:
                raise IngestError(f"ZIP expands beyond {MAX_ENVELOPE_BYTES} bytes")
            try:
                with archive.open(info, "r") as handle:
                    data = _bounded_read(handle, label=name, declared_size=info.file_size)
            except (OSError, RuntimeError, NotImplementedError, zipfile.BadZipFile) as error:
                raise IngestError(f"failed to decode evidence ZIP member: {name!r}") from error
            (destination / name).write_bytes(data)
    return archive_digest


@contextmanager
def materialize_evidence(path: Path) -> Iterator[tuple[Path, str]]:
    """Yield a bounded temporary four-file directory and optional archive digest."""
    with tempfile.TemporaryDirectory(prefix="symtropy-evidence-envelope-") as temporary:
        destination = Path(temporary)
        if path.is_file() and not path.is_symlink():
            archive_digest = _copy_zip(path, destination)
        else:
            _copy_directory(path, destination)
            archive_digest = ""
        yield destination, archive_digest
