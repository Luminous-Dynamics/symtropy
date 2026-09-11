# Tskit differential ancestry oracle V1

Status: PHYLO-04E qualification contract.

This document defines an **external qualification oracle** for Symtropy's modeled-locus ancestry simplification. It is not a production dependency, runtime wire format, or replacement for Symtropy's persistent ancestry authority.

## Pinned oracle

V1 requires exactly:

- Python 3.11+;
- `tskit==1.0.3`;
- `scripts/qualification/phylo_tskit_oracle.py`;
- `scripts/qualification/phylo-tskit-v1.json`.

The script fails closed if a different tskit version is imported.

## Identity boundary

`AncestryCopyId` is persistent genealogical identity.

A tskit node-table row is qualification storage identity only. Simplification is allowed to renumber or delete table rows. V1 therefore uses the old-to-new map returned by `TreeSequence.simplify(..., map_nodes=True)` and translates every surviving row back into the fixture's persistent copy ID before comparison.

No test may compare raw tskit row IDs to Symtropy ancestry IDs.

## Synthetic sequence coordinates

V1 represents each modeled locus as one synthetic non-overlapping unit interval:

- locus 0 -> `[0, 1)`;
- locus 1 -> `[1, 2)`;
- etc.

These coordinates exist solely so tskit can express a different ancestry tree at each modeled locus.

They are **not** physical base-pair positions, genetic-map coordinates, crossover breakpoints, chromosome lengths, or production serialization. They must never be imported back into Symtropy as biological evidence.

## Time transform

Symtropy ancestry generation increases forward in time. Tskit node time increases backward from the present.

For each fixture the oracle uses the monotone transform:

`tskit_time = max_fixture_generation - ancestry_generation`.

Only ancestry ordering is compared; the transformed values are qualification scaffolding.

## Simplification policy

The V1 oracle calls tskit simplification with:

- focal Symtropy copies as tskit samples;
- protected Symtropy copies as additional oracle samples/anchors;
- `keep_unary=False`;
- `keep_input_roots=True`;
- `map_nodes=True`;
- provenance recording disabled for deterministic report comparison.

Treating a protected copy as an oracle sample is a comparison device. It does not change the meaning of `AncestryRetentionSet::protected_copy_ids` in Symtropy.

## Comparison contract

After simplification, V1 translates retained rows back to persistent copy IDs and requires exact agreement with each frozen fixture for:

- retained persistent-copy identity;
- required removed-copy identity;
- nearest retained parent at every modeled locus;
- branch topology;
- focal/protected retention;
- different reachable roots at different modeled loci;
- per-locus bypass of globally retained nodes.

The oracle does not compare serialized tables, row numbers, metadata bytes, or synthetic coordinates.

## Frozen corpus

V1 includes seven cases:

1. pure unary-chain contraction;
2. protected unary anchor retention;
3. two-focal branch/coalescent retention;
4. two modeled loci with different retained roots;
5. a node retained globally because it branches at locus A but bypassed as unary at locus B;
6. unreachable lineage removal;
7. the six-node/six-edge final canonical topology expected by `ANCESTRY_SCHEDULE_INVARIANCE_V1.md`.

A changed ancestry source, focal set, protected set, or expected topology must fail the oracle rather than be silently normalized.

## Running

Create an isolated qualification environment and install the pinned dependency:

`python3 -m venv .venv-phylo-tskit`

`.venv-phylo-tskit/bin/pip install -r scripts/qualification/phylo-tskit-requirements.txt`

Then run:

`.venv-phylo-tskit/bin/python scripts/qualification/phylo_tskit_oracle.py`

An optional machine-readable report can be written with `--json-output <path>`.

Generated environments/reports are evidence artifacts, not source authority, unless separately captured by the repository's evidence process.

## Fail-closed behavior

The oracle rejects:

- the wrong tskit version;
- missing/duplicate fixture IDs;
- unknown focal/protected IDs;
- overlapping focal/protected authority;
- unknown loci or edge endpoints;
- generation-order violations;
- duplicate incoming ancestry at one `(child, locus)`;
- retained-node disagreement;
- expected-removal disagreement;
- any parent-topology disagreement.

## Scientific boundary

Passing this oracle would establish agreement with one independently implemented simplification engine on the frozen small modeled-locus corpus under this declared translation policy.

It would **not** establish:

- tskit wire-format compatibility;
- physical-sequence ancestry;
- exact crossover coordinates;
- mutation-table equivalence;
- organism pedigree equivalence;
- species/ecological validity;
- correctness outside the qualified fixture domain.

Any future semantic disagreement with tskit must be classified as either a Symtropy defect, an oracle-translation defect, or an explicitly documented model difference. It must never be normalized away merely to make the differential test pass.

## Evidence status

Source presence is not execution evidence. A PASS exists only when the exact branch subject executes the pinned oracle successfully and records the exact source/tool environment required by the evidence policy.
