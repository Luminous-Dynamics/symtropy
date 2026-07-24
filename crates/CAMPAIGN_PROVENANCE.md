# Campaign provenance and promotion rules

Symtropy patch campaigns are only one replayable history when all of the
following are true at the same time:

1. `series` names every mailbox patch exactly once and in contiguous order.
2. Every listed patch exists and every patch file is listed.
3. `SHA256SUMS` verifies the patch and document bytes being distributed.
4. `TREES.txt` records identical authored and replayed Git tree identities.
5. The source snapshot was produced from that replayed tree, not from an older
   campaign with a newer plan copied beside it.
6. Cargo, tests, and runtime evidence are reported separately from patch replay.

A focused append-only campaign may be valid without proving the complete prior
history. In that case its baseline tree and scope must be named explicitly, and
it must not be repackaged as a full-series archive.

Use:

```text
python3 crates/tools/check_patch_series.py PATCH_DIRECTORY \
  --first 106 --last 112 --require-tree-record
```

This gate validates artifact coherence. It does not compile Rust, authenticate
an archive, or prove that the modeled systems are physically or operationally
correct.
