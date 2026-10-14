#!/usr/bin/env python3
"""Independent stdlib oracle for PHYS-EVID-03B1A canonical issuer policy."""

from __future__ import annotations

import copy
import json
import struct
from pathlib import Path

MAGIC = b"symtropy-physics-issuer-policy\0"
VERSION = 1
MAX_ENTRIES = 1024
MAX_AUTHORITIES = 64
MAX_TAGS = 8
STATUS = {"Active": 1, "Disabled": 2, "RevokedCompromised": 3}
CLAIMS = {1, 2}
SESSIONS = {1}
ATTESTATION_SUITES = {1}
CRYPTO_POLICIES = {1}
FIXTURE = Path(__file__).parent / "fixtures" / "issuer_policy_v01_golden.json"


def be32(value: int) -> bytes:
    return struct.pack(">I", value)


def be64(value: int) -> bytes:
    return struct.pack(">Q", value)


def canonical_set(values: list[int], *, maximum: int, allow_zero: bool = True) -> list[int]:
    if not values:
        raise ValueError("empty scope")
    if len(values) > maximum:
        raise ValueError("scope over bound")
    if len(set(values)) != len(values):
        raise ValueError("duplicate scope member")
    if not allow_zero and any(value == 0 for value in values):
        raise ValueError("zero physical authority")
    return sorted(values)


def encode_draft(doc: dict) -> bytes:
    entries = doc["draft_entries"]
    if len(entries) > MAX_ENTRIES:
        raise ValueError("entry count over bound")

    canonical_entries = []
    credential_ids = []
    for raw in entries:
        credential_id = bytes.fromhex(raw["credential_id_hex"])
        if len(credential_id) != 32:
            raise ValueError("credential id length")
        credential_ids.append(credential_id)
        status = STATUS[raw["status"]]
        authorities = canonical_set(
            [int(value, 16) for value in raw["physical_authority_ids_hex"]],
            maximum=MAX_AUTHORITIES,
            allow_zero=False,
        )
        claim_tags = canonical_set(raw["claim_profile_tags"], maximum=MAX_TAGS)
        session_tags = canonical_set(raw["session_profile_tags"], maximum=MAX_TAGS)
        attestation_tags = canonical_set(raw["attestation_suite_tags"], maximum=MAX_TAGS)
        crypto_tags = canonical_set(raw["crypto_policy_tags"], maximum=MAX_TAGS)
        if not set(claim_tags) <= CLAIMS:
            raise ValueError("unknown claim tag")
        if not set(session_tags) <= SESSIONS:
            raise ValueError("unknown session tag")
        if not set(attestation_tags) <= ATTESTATION_SUITES:
            raise ValueError("unknown attestation tag")
        if not set(crypto_tags) <= CRYPTO_POLICIES:
            raise ValueError("unknown crypto-policy tag")
        canonical_entries.append(
            (
                credential_id,
                status,
                authorities,
                claim_tags,
                session_tags,
                attestation_tags,
                crypto_tags,
            )
        )

    if len(set(credential_ids)) != len(credential_ids):
        raise ValueError("duplicate credential")
    canonical_entries.sort(key=lambda entry: entry[0])

    out = bytearray(MAGIC)
    out += be32(VERSION)
    out += be64(doc["policy_revision"])
    out += be32(len(canonical_entries))
    for credential_id, status, authorities, claims, sessions, suites, cryptos in canonical_entries:
        out += credential_id
        out.append(status)
        out += be32(len(authorities))
        for authority in authorities:
            out += authority.to_bytes(16, "big")
        for tags in (claims, sessions, suites, cryptos):
            out += be32(len(tags))
            for tag in tags:
                out += be32(tag)
    return bytes(out)


class Reader:
    def __init__(self, data: bytes):
        self.data = data
        self.offset = 0

    def take(self, size: int) -> bytes:
        end = self.offset + size
        if end > len(self.data):
            raise ValueError("truncated")
        chunk = self.data[self.offset:end]
        self.offset = end
        return chunk

    def u8(self) -> int:
        return self.take(1)[0]

    def u32(self) -> int:
        return int.from_bytes(self.take(4), "big")

    def u64(self) -> int:
        return int.from_bytes(self.take(8), "big")

    def u128(self) -> int:
        return int.from_bytes(self.take(16), "big")


def read_strict_tags(reader: Reader, allowed: set[int]) -> list[int]:
    count = reader.u32()
    if count == 0 or count > MAX_TAGS:
        raise ValueError("tag count")
    values = [reader.u32() for _ in range(count)]
    if any(value not in allowed for value in values):
        raise ValueError("unknown tag")
    if any(a >= b for a, b in zip(values, values[1:])):
        raise ValueError("non-canonical tag order")
    return values


def decode_canonical(data: bytes) -> dict:
    reader = Reader(data)
    if reader.take(len(MAGIC)) != MAGIC:
        raise ValueError("bad magic")
    if reader.u32() != VERSION:
        raise ValueError("bad version")
    revision = reader.u64()
    entry_count = reader.u32()
    if entry_count > MAX_ENTRIES:
        raise ValueError("entry count over bound")

    entries = []
    previous_credential = None
    for _ in range(entry_count):
        credential_id = reader.take(32)
        if previous_credential is not None and credential_id <= previous_credential:
            raise ValueError("non-canonical credential order")
        previous_credential = credential_id
        status = reader.u8()
        if status not in STATUS.values():
            raise ValueError("unknown status")
        authority_count = reader.u32()
        if authority_count == 0 or authority_count > MAX_AUTHORITIES:
            raise ValueError("authority count")
        authorities = [reader.u128() for _ in range(authority_count)]
        if any(value == 0 for value in authorities):
            raise ValueError("zero authority")
        if any(a >= b for a, b in zip(authorities, authorities[1:])):
            raise ValueError("non-canonical authority order")
        claims = read_strict_tags(reader, CLAIMS)
        sessions = read_strict_tags(reader, SESSIONS)
        suites = read_strict_tags(reader, ATTESTATION_SUITES)
        cryptos = read_strict_tags(reader, CRYPTO_POLICIES)
        entries.append((credential_id, status, authorities, claims, sessions, suites, cryptos))

    if reader.offset != len(data):
        raise ValueError("trailing bytes")
    return {"policy_revision": revision, "entries": entries}


def must_fail(fn, *args) -> None:
    try:
        fn(*args)
    except (ValueError, KeyError):
        return
    raise AssertionError("negative control unexpectedly accepted")


def main() -> None:
    fixture = json.loads(FIXTURE.read_text())
    encoded = encode_draft(fixture)
    expected = bytes.fromhex(fixture["canonical_hex"])
    assert len(expected) == fixture["canonical_length"] == 237
    assert encoded == expected
    decoded = decode_canonical(encoded)
    assert decoded["policy_revision"] == 7
    assert decoded["entries"][0][0] == bytes([0x11]) * 32
    assert decoded["entries"][0][2] == [0x10, 0x20]
    assert decoded["entries"][0][3] == [1, 2]

    reordered = copy.deepcopy(fixture)
    reordered["draft_entries"].reverse()
    reordered["draft_entries"][0]["physical_authority_ids_hex"].reverse()
    reordered["draft_entries"][0]["claim_profile_tags"].reverse()
    assert encode_draft(reordered) == encoded

    duplicate_credential = copy.deepcopy(fixture)
    duplicate_credential["draft_entries"][0]["credential_id_hex"] = duplicate_credential["draft_entries"][1]["credential_id_hex"]
    must_fail(encode_draft, duplicate_credential)

    duplicate_authority = copy.deepcopy(fixture)
    duplicate_authority["draft_entries"][1]["physical_authority_ids_hex"] = [
        "00000000000000000000000000000010",
        "00000000000000000000000000000010",
    ]
    must_fail(encode_draft, duplicate_authority)

    zero_authority = copy.deepcopy(fixture)
    zero_authority["draft_entries"][0]["physical_authority_ids_hex"] = ["0" * 32]
    must_fail(encode_draft, zero_authority)

    unknown_tag = copy.deepcopy(fixture)
    unknown_tag["draft_entries"][0]["claim_profile_tags"] = [99]
    must_fail(encode_draft, unknown_tag)

    header_len = len(MAGIC) + 4 + 8 + 4
    unknown_status_wire = bytearray(encoded)
    unknown_status_wire[header_len + 32] = 0xFF
    must_fail(decode_canonical, bytes(unknown_status_wire))

    unsorted_credentials = bytearray(encoded)
    second = encoded.find(bytes([0x22]) * 32)
    assert second > header_len
    first_id = bytes(unsorted_credentials[header_len:header_len + 32])
    second_id = bytes(unsorted_credentials[second:second + 32])
    unsorted_credentials[header_len:header_len + 32] = second_id
    unsorted_credentials[second:second + 32] = first_id
    must_fail(decode_canonical, bytes(unsorted_credentials))

    overbound = MAGIC + be32(VERSION) + be64(0) + be32(MAX_ENTRIES + 1)
    must_fail(decode_canonical, overbound)

    for end in range(len(encoded)):
        must_fail(decode_canonical, encoded[:end])
    must_fail(decode_canonical, encoded + b"\x00")

    print("PHYS-EVID-03B1A independent issuer-policy oracle: PASS")


if __name__ == "__main__":
    main()
