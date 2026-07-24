#!/usr/bin/env python3
"""Verify an ordered mailbox patch directory and its provenance records.

This gate is intentionally independent of Cargo. It catches stale `series`
files, missing or extra patches, non-sequential numbering, broken SHA-256
ledgers, and mismatched tree records before a campaign is advertised as a
single replayable history.
"""

from __future__ import annotations

import argparse
import hashlib
import re
import sys
from pathlib import Path

PATCH_NAME = re.compile(r"^(?P<number>\d{4})-[A-Za-z0-9_.-]+\.patch$")
TREE_LINE = re.compile(
    r"^(?P<name>[a-z_]+)(?:=|\s+)(?P<value>[0-9a-f]{40})$"
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("patch_dir", type=Path)
    parser.add_argument("--first", type=int)
    parser.add_argument("--last", type=int)
    parser.add_argument("--require-tree-record", action="store_true")
    return parser.parse_args()


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def read_series(patch_dir: Path) -> list[str]:
    series = patch_dir / "series"
    if not series.is_file():
        raise ValueError("missing series file")
    entries = []
    for line_no, raw in enumerate(series.read_text(encoding="utf-8").splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if Path(line).name != line:
            raise ValueError(f"series line {line_no} is not a plain filename: {line!r}")
        entries.append(line)
    if not entries:
        raise ValueError("series contains no patches")
    return entries


def verify_mbox(path: Path) -> None:
    with path.open("rb") as handle:
        first = handle.readline()
        prefix = handle.read(16 * 1024)
    if not first.startswith(b"From "):
        raise ValueError(f"{path.name}: not a git mailbox patch")
    if b"\nSubject: [PATCH" not in b"\n" + prefix:
        raise ValueError(f"{path.name}: missing [PATCH] subject")


def verify_checksums(patch_dir: Path) -> int:
    ledger = patch_dir / "SHA256SUMS"
    if not ledger.is_file():
        raise ValueError("missing SHA256SUMS")
    checked = 0
    for line_no, raw in enumerate(ledger.read_text(encoding="utf-8").splitlines(), 1):
        line = raw.strip()
        if not line:
            continue
        parts = line.split(maxsplit=1)
        if len(parts) != 2:
            raise ValueError(f"SHA256SUMS line {line_no} is malformed")
        expected, relative = parts
        relative = relative.lstrip("*")
        target = patch_dir / relative
        if not target.is_file():
            raise ValueError(f"SHA256SUMS references missing file: {relative}")
        actual = sha256(target)
        if actual != expected:
            raise ValueError(
                f"checksum mismatch for {relative}: expected {expected}, got {actual}"
            )
        checked += 1
    return checked


def verify_tree_record(patch_dir: Path, required: bool) -> dict[str, str]:
    record = patch_dir / "TREES.txt"
    if not record.is_file():
        if required:
            raise ValueError("missing TREES.txt")
        return {}
    values: dict[str, str] = {}
    for line_no, raw in enumerate(record.read_text(encoding="utf-8").splitlines(), 1):
        line = raw.strip()
        if not line:
            continue
        match = TREE_LINE.fullmatch(line)
        if match is None:
            raise ValueError(f"TREES.txt line {line_no} is malformed: {line!r}")
        name = match.group("name")
        if name in values:
            raise ValueError(f"TREES.txt repeats {name}")
        values[name] = match.group("value")
    pairs = (
        ("authored_tree", "replayed_tree"),
        ("authored_final_tree", "replayed_final_tree"),
    )
    selected = next(
        ((authored, replayed) for authored, replayed in pairs if authored in values or replayed in values),
        None,
    )
    if required and selected is None:
        raise ValueError(
            "TREES.txt must contain authored/replayed tree identities "
            "using a supported key pair"
        )
    if selected is not None:
        authored, replayed = selected
        if authored not in values or replayed not in values:
            raise ValueError(f"TREES.txt must contain both {authored} and {replayed}")
        if values[authored] != values[replayed]:
            raise ValueError(f"{authored} and {replayed} are not identical")
    return values


def main() -> int:
    args = parse_args()
    patch_dir = args.patch_dir.resolve()
    errors: list[str] = []
    try:
        entries = read_series(patch_dir)
        if len(entries) != len(set(entries)):
            raise ValueError("series contains duplicate filenames")

        numbers: list[int] = []
        for entry in entries:
            match = PATCH_NAME.fullmatch(entry)
            if match is None:
                raise ValueError(f"invalid patch filename in series: {entry}")
            number = int(match.group("number"))
            numbers.append(number)
            path = patch_dir / entry
            if not path.is_file():
                raise ValueError(f"series references missing patch: {entry}")
            verify_mbox(path)

        expected = list(range(numbers[0], numbers[-1] + 1))
        if numbers != expected:
            raise ValueError(
                f"series numbering is not contiguous: got {numbers[0]}..{numbers[-1]} "
                f"across {len(numbers)} entries"
            )
        if args.first is not None and numbers[0] != args.first:
            raise ValueError(f"first patch is {numbers[0]}, expected {args.first}")
        if args.last is not None and numbers[-1] != args.last:
            raise ValueError(f"last patch is {numbers[-1]}, expected {args.last}")

        listed = set(entries)
        extras = sorted(path.name for path in patch_dir.glob("*.patch") if path.name not in listed)
        if extras:
            raise ValueError(f"patch files omitted from series: {', '.join(extras)}")

        checksum_count = verify_checksums(patch_dir)
        trees = verify_tree_record(patch_dir, args.require_tree_record)
    except (OSError, ValueError) as error:
        errors.append(str(error))

    for error in errors:
        print(f"ERROR: {error}", file=sys.stderr)
    if errors:
        return 1

    print(
        f"patch-series gate passed: {len(entries)} patches "
        f"({numbers[0]}-{numbers[-1]}), {checksum_count} checksums, "
        f"tree_record={'present' if trees else 'absent'}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
