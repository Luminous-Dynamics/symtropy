# Synthetic recovery procedure

1. Verify `bagit.txt` and all payload/tag manifests using an RFC 8493-compatible BagIt validator.
2. Read `data/payload/hello.txt` as opaque bytes.
3. Compare its SHA-256 with the known-answer record in `capsule/known_answer.json`.
4. Confirm the capsule identifies two separate decoder reference records.
5. Record that no physical reader, decoder independence or production qualification is established by this fixture.

This is a packaging fixture only; it has no instrument or physical-media dependency.
