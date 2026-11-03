# Hair-Plate Reflex Oracle V1

A dependency-free Rust data and evaluation contract for source-grounded prediction of held-out Drosophila hair-plate sensorimotor reflexes.

## Purpose

The CxHP8 study in Nature Communications (2026) combines in-vivo calcium imaging, connectome reconstruction and activation/silencing during behaviour to link a specific hair plate to joint-limit sensing and motor reflexes. It reports 214 hair-plate mechanosensory neurons across 42 plates and predicts functions for other plates from morphology and downstream connectivity.

This crate establishes the source-qualified inventory and blind prediction/evaluation boundary needed to test whether those predictions generalize to plates whose physiological/intervention labels were withheld.

## Evidence semantics

Every evidence-bearing field has exactly one state:
- **Observed:** a value and the source artifact SHA-256 digest are present.
- **ExplicitlyAbsent:** a complete search scope and a search-receipt digest are present. This is a negative observation only within that declared scope.
- **Unknown:** a typed reason and explanatory detail are present. Unknown is not treated as absent and is not scored as a negative label.

An observed empty list is rejected. Use ExplicitlyAbsent with a scope receipt if a complete search found no entries; use Unknown when coverage or reconstruction is incomplete.

## Inventory fields

A plate record binds:
- taxon, leg, side, segment and joint;
- morphology (hair count, contact normal, orientation axis and geometry digest);
- afferent neuron identities;
- sensory tuning (measured joint-limit angle, threshold definition, response curve and units);
- connectome outputs (target identities, target cell types, synapse counts, transmitter identity, synapse sign, sign-evidence basis, sign-evidence artifact digest and anatomical region);
- reflex outcomes (intervention kind, movement direction, latency, magnitude and behavioural context);
- record-level source digest.

The inventory binds a dataset ID/version, inventory digest and typed source-artifact manifest. Each observed field must resolve to a manifest entry with the matching semantic role (morphology, afferent identity, sensory tuning, connectome output or reflex outcome). Each explicit-absence claim must resolve to a SearchReceipt artifact. Manifest entries carry a locator, source version, media type, optional SPDX license, SHA-256, roles and declared custody mode.

Custody mode is metadata, not proof. This crate validates manifest structure and digest references but does not download bytes, hash them, validate digital signatures or authenticate the source provider. A separate intake/materialization process must verify downloaded bytes against claimed digests before assigning a byte-verified custody mode. The typed materialization receipt binds dataset version, file ID, exact byte count, saved metadata-response digest, local SHA-256, independently recomputed provider-declared checksum and materializer-run digest. Both checksum values are retained; a provider MD5 cannot be substituted for the local SHA-256.

## Blind prediction contract

Each prediction binds model, training split, held-out split and prediction digests plus the exact plate ID. The held-out plate must not occur in the training plate list. The inventory-aware validator checks that the held-out plate and every training plate exist in the pinned inventory and rejects cross-taxon training plates. A joint-limit prediction also binds the exact response-threshold definition. A reflex prediction binds intervention, behavioural context and assay-scope identity; an observed outcome from another assay context cannot be used to infer absence. An explicit-absence search receipt is scored as negative only when its scope ID exactly matches the predicted assay scope. A future run manifest should bind all data preprocessing, normalization, cell registration, connectome release/materialization, model hyperparameters, random seeds and prediction outputs.

## V1 metrics

For each held-out plate, V1 reports:
- absolute joint-limit angle error when measured tuning is observed;
- reflex direction correctness when a measured intervention outcome is observed;
- motor-target precision, recall and F1 when connectome output targets are observed or explicitly absent within a complete search scope;
- primary signed-pathway accuracy only for direct physiological or receptor-supported truth labels;
- separate annotation-sign agreement for transmitter-heuristic labels;
- unknown sign labels remain unscored. Transmitter identity is not itself a measured postsynaptic sign.

Unknown evidence produces an explicit unscorable reason rather than a negative label. Target presence is scored independently from sign. Direct/receptor-supported sign accuracy and transmitter-heuristic agreement are separate metrics, so an annotation rule cannot be reported as physiological validation. If the observed target set is empty, recall is undefined and returned as None; it is not forced to zero or one.

## Scientific boundaries

The oracle does not infer a missing motor mapping from morphology or nearest-neighbour similarity. It does not treat a connectome edge as proof of a functional synapse. Morphology-derived, graph-derived, experimentally observed and model-predicted claims must remain separate in downstream reports.

The synthetic tests validate the contract and metric arithmetic only. They do not reproduce the CxHP8 paper or establish prediction accuracy on the other 41 hair plates.


## Current source dossier (not yet materialized by this crate)

The CxHP8 paper's data availability statement points to the Dryad deposit
https://doi.org/10.5061/dryad.fxpnvx153 (published 2 September 2025, versioned
deposit, approximately 11.71 GB, CC0-1.0). It contains calcium-imaging,
optogenetic activation/silencing and treadmill kinematic datasets, including
`R48A07_R20C06_GCaMP7f_tdTomato.parquet`, the `ChrimsonR` / `GtACR1`
trial tables and chronic `kir_treadmill_dataset.csv` files. The associated
analysis notebooks are released at
https://github.com/Prattbuw/Hair_Plate_Paper/releases/tag/v1.0.0.

The connectome data are separate sources: the paper used FANC CAVE
materialization version 840 (timestamp 2024-01-17T08:10:01.179472) and FlyWire
public release version 783. These exact version identifiers belong in a real
source manifest; a generic "current connectome" label is not acceptable.

Important coverage limit: the physiological and intervention datasets are
primarily for the characterized CxHP8 plate. The other hair plates have
morphology/connectivity-based predictions, not 41 equivalent independent
physiology/perturbation datasets. The held-out-plate benchmark must preserve
this difference and cannot call predictions confirmed reflex labels.

## Synthetic tests

The 36 tests cover a valid inventory, unknown-vs-absence semantics, explicitly absent target-set scoring, a known-answer joint limit/reflex/motor-sign fixture, threshold-definition matching, reflex assay-context scoping, exact absent-scope matching and unmeasured-context protection, held-out-plate leakage, malformed tuning curves, duplicate identities, invalid morphology/digests, malformed unknown evidence and inventory-bound training/holdout membership.

## References

- Nature Communications (2026), *Proprioceptive limit detectors contribute to sensorimotor control of the Drosophila leg*: https://doi.org/10.1038/s41467-026-69333-z
- Dryad source data: https://doi.org/10.5061/dryad.fxpnvx153
- PubMed entry: https://pubmed.ncbi.nlm.nih.gov/41680191/

## Run

    cargo fmt --manifest-path research/hair-plate-reflex-oracle-v1/Cargo.toml -- --check
    cargo test --manifest-path research/hair-plate-reflex-oracle-v1/Cargo.toml --locked
    cargo clippy --manifest-path research/hair-plate-reflex-oracle-v1/Cargo.toml --locked --all-targets -- -D warnings

The package is a standalone Cargo workspace under research/. It does not modify Symtropy's production workspace or dependency graph.
