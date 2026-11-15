# AFEX site, plot and observation identity gate v0.1

**Issue:** [#5470 — AFEX-SITE-AND-PLOT-IDENTITY-001](https://github.com/Luminous-Dynamics/symtropy/issues/5470)  
**Parent AFEX design:** [#5408](https://github.com/Luminous-Dynamics/symtropy/issues/5408)  
**Support crosswalk:** [AFEX_DATA_SUPPORT_CROSSWALK_V0_1.md](AFEX_DATA_SUPPORT_CROSSWALK_V0_1.md)  
**Dataset register:** [DATASET_CANDIDATE_REGISTER_V0_1.json](DATASET_CANDIDATE_REGISTER_V0_1.json)  
**Date:** 15 November 2026  
**Status:** synthetic identity-policy verifier; no actual source-key equivalence adjudicated

## 1. Why identity needs its own gate

The AFEX parent collection groups 15 child records. That is collection membership, not proof that every child row shares the same exact site, plot, block, subplot, campaign or sample frame.

The official EIDC parent record describes a randomized-block experiment with eight treatments and four replicates. Individual child records carry distinct spatial/temporal metadata and nested sampling units. For example:
- tree, root, LAI and litter trap datasets describe the site approximately 80 km north of Manaus;
- ant and seedling leaf-herbivory records describe BDFFP approximately 100 km north of Manaus;
- catalogue spatial resolution ranges from 50 m to 1,000 m;
- ant subplots are 1×1 m, seedling subplots contain marked individuals/leaves, fine-root samples are depth composites, and litterfall uses repeated traps.

These descriptions may be generalized metadata, not necessarily different field locations. But the difference is unresolved until plot maps, source keys, field protocol or author confirmation establish equivalence.

## 2. Identity is hierarchical

1. Experiment ID and protocol.
2. Site ID and coordinate reference/extent.
3. Block ID.
4. Plot ID and treatment assignment.
5. Subplot/sample-frame type and sample ID.
6. Campaign/date/interval and depth.
7. Source-native observation and measurement operator.

Every child source must retain its source ID and row/sample identity. An ID string is only unique within its declared parent namespace and sample-frame type.

## 3. Join semantics

- **Plot-context join:** the exact same randomized plot may be used as a common grouping key across different source-native sample frames. The measurement rows remain separate, and support/date/depth are preserved.
- **Observation join:** requires matching experiment, site, block, plot, treatment, sample-frame type and sample identity, plus compatible depth/time support.
- **Documented alias:** allowed only with an explicit, cited mapping record and bounded site/block/treatment/scope.
- **Unresolved site:** if site identity is absent and descriptions differ, do not infer equivalence from AFEX membership or approximate distance.
- **Conflict:** explicit site, block, plot, treatment, sample-frame, depth or interval mismatch fails closed.

Fuzzy string matching and catalogue coordinate proximity are discovery aids, not join authority.

## 4. Executable v0.1

The synthetic corpus [cases/afex-source-identity-synthetic-v0.1.json](cases/afex-source-identity-synthetic-v0.1.json) contains seven known-truth cases:
- exact plot-context match with different nested frames;
- unresolved 80 km / 100 km site descriptions;
- same plot string at different sites;
- same nested ID string in different sample-frame types;
- a documented synthetic plot alias;
- same treatment with a block/plot conflict;
- observation-level depth conflict.

The Rust validator emits dispositions and binds the exact fixture bytes. Synthetic alias evidence is explicitly fictional and does not establish a real AFEX plot mapping.

## 5. Required empirical admission sequence

1. Retrieve each child archive and verify SHA-256, licence, file list and citation.
2. Extract raw site, block, plot, subplot, tree, trap, collar, core, campaign and depth identifiers without normalization.
3. Compare source-native values to the AFEX field protocol and any plot map.
4. Obtain author/custodian confirmation for unresolved aliases or site descriptions.
5. Construct a versioned mapping table with effective scope and evidence reference.
6. Run exact identity validator before support intersection or data fusion.
7. Keep unmatched source-native observations rather than deleting them or inventing joins.

## 6. Claim ceiling

The synthetic PASS validates join policy only. No raw child archive is admitted and no actual site/plot alias is declared. The 80 km versus 100 km descriptions remain unresolved; the validator must not convert that uncertainty into a statement that there are definitely two sites.
