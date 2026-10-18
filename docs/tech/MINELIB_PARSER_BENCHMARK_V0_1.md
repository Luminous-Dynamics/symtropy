# MINE-REFERENCE-PARSER-001 — parser and tiny UPIT oracle v0.1

## Scope

A standalone Rust conformance profile for three linked MineLib v1 input forms:

1. Block-model descriptor rows: block ID, x/y/z integer coordinates, then typed attributes declared by a separate manifest.
2. Block-precedence rows: block ID, predecessor count, and predecessor IDs.
3. A restricted optimization-model descriptor for UPIT: NAME, TYPE, NBLOCKS, DISCOUNT RATE, and OBJECTIVE FUNCTION rows.

The [official MineLib v1 format specification](https://minelib.org/v1/minelib_format.pdf) describes the block model, precedence descriptor, and optimization-model descriptor as separate files. It specifies percent-sign comment lines, space/tab/colon separators, zero-based block IDs, predecessor lists, and sections for model types, objective rows, period/resource constraints, and general constraints.

This v0.1 implementation deliberately parses only a strict subset of the complete standard. It rejects CPIT/PCPSP and unknown model sections; it does not parse resource coefficients, resource limits, destinations, or general side constraints. It does not pretend to be a general MineLib loader.

## Paradigm shift

Treat parsing and model assembly as claims that require independent evidence. A syntactically valid file is not enough: a block attribute can be mistaken for an objective coefficient, IDs can drift across files, and precedence can be reversed. The contract therefore binds raw-file identity, declared block attribute schema, graph integrity, model-objective row identity, exact decimal semantics, and exhaustive small-instance feasibility.

A **tiny UPIT exhaustive oracle** enumerates every block subset (with an explicit maximum of 20 blocks), applies the closure/precedence constraint, and maximizes exact scaled integer objective values. A separately implemented verifier checks the winning subset and recomputes its objective. Beyond the limit, the tool returns an indeterminate-limit error rather than claiming an optimum. This is a correctness oracle, not a production optimizer.

The fixture's block_value_usd sum is an audit sum only, not NPV. The UPIT objective is read from the separate model descriptor; no discounting is applied to the supplied coefficients by this toy oracle. In this authored fixture, those descriptor coefficients are intentionally required to match the similarly named block attribute, so cross-file mismatches are caught.

## Frozen synthetic fixture

All inputs are authored for this benchmark; they are not downloaded MineLib instances or site data:
- five synthetic block rows, IDs 0–4, with integer coordinates, declared tonnage, grade and signed fixed-decimal block values;
- five precedence rows and three directed predecessor edges;
- a UPIT descriptor with one objective coefficient per block;
- an explicit manifest defining the attribute columns and fixed decimal scales.

Known answers:
- 5 blocks, IDs [0,1,2,3,4];
- 5 precedence rows, 3 precedence edges;
- 32 possible subsets, of which 12 satisfy precedence;
- exact block-attribute audit sum: 25,000,050 cents;
- model objective coefficient sum: 25,000,050 cents;
- unique optimum selected IDs: [0,1,3,4];
- optimum objective: 25,500,050 cents (USD 255,000.50);
- dependent block 2 has negative objective contribution and is correctly excluded.

The receipt binds the exact raw manifest, block, precedence and model bytes by SHA-256, plus the exact source commit.

## Acceptance gates

- 100% known-answer match to row counts, block IDs, precedence edges, audit sum, model objective rows and exhaustive UPIT output.
- 0 duplicate/gapped block IDs, duplicate/missing precedence rows, dangling/self/duplicate predecessors, or precedence cycles admitted.
- 0 duplicate/missing objective rows, unknown model sections, wrong problem type, objective schema mismatches, or unsupported numeric precision admitted.
- 100% independent selected-subset feasibility and objective recomputation agreement.
- 0 floating-point drift for declared fixed-decimal fields.
- The exact oracle must refuse inputs above the size limit with an explicit indeterminate disposition.
- 100% manifest/file SHA coverage and exact-commit receipt binding.
- Replays over identical bytes/config/commit are deterministic.

## Research / data rights

The official MineLib site states the library is licensed under CC BY-SA 3.0 Unported: https://minelib.org/v1/. Follow attribution and ShareAlike requirements, and check individual instance records for any additional notices before redistributing data. This PR includes no third-party dataset.

Lotsu, Bimpong & Boakye (2026) publish an open-source scheduling study using a synthetic 602-block / 12-period case: https://doi.org/10.3389/frai.2026.1759758. That is a candidate external method comparator only; neither its NPV nor its artifacts are part of this synthetic fixture. The separate artifact record's rights are tracked distinctly in [MINE-REFERENCE-001 #2015](https://github.com/Luminous-Dynamics/symtropy/issues/2015).

## Next ladder

1. v0.1: strict block / precedence / UPIT descriptor profile and exhaustive oracle for tiny inputs.
2. v0.2: CPIT/PCPSP descriptor sections, destinations, per-period resource coefficients and bounds, strict unit/period semantics.
3. v0.3: source-pinned external MineLib instance validation after rights review; no raw data copied without checked terms.
4. v0.4: independently compare against a published solver on exact same model formulation, discounting and constraints.
5. v1: external baseline tournament that reports feasible objective, valid bound, gap, runtime, resource use and reproducibility receipt.

## Boundaries

A successful parse and tiny oracle do not establish that a real mine model is correct, a large schedule is optimal, reserves are certified, or operations are safe/profitable. This work composes with [MINE-SYSTEM-001 #1933](https://github.com/Luminous-Dynamics/symtropy/issues/1933) and [MINE-REFERENCE-001 #2015](https://github.com/Luminous-Dynamics/symtropy/issues/2015).
