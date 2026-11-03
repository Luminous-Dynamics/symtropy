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
- connectome outputs (target identities, target cell types, synapse counts, sign and anatomical region);
- reflex outcomes (intervention kind, movement direction, latency, magnitude and behavioural context);
- record-level source digest.

The inventory binds a dataset ID/version and inventory digest. It validates syntax and structure but does not authenticate source artifacts or claim the input data are biologically correct.

## Blind prediction contract

Each prediction binds model, training split, held-out split and prediction digests plus the exact plate ID. The held-out plate must not occur in the training plate list. A future run manifest should bind all data preprocessing, normalization, cell registration, connectome release/materialization, model hyperparameters, random seeds and prediction outputs.

## V1 metrics

For each held-out plate, V1 reports:
- absolute joint-limit angle error when measured tuning is observed;
- reflex direction correctness when a measured intervention outcome is observed;
- motor-target precision, recall and F1 when connectome output targets are observed or explicitly absent within a complete search scope;
- signed pathway accuracy only for target edges whose biological sign is known.

Unknown evidence produces an explicit unscorable reason rather than a negative label. If the observed target set is empty, recall is undefined and returned as None; it is not forced to zero or one.

## Scientific boundaries

The oracle does not infer a missing motor mapping from morphology or nearest-neighbour similarity. It does not treat a connectome edge as proof of a functional synapse. Morphology-derived, graph-derived, experimentally observed and model-predicted claims must remain separate in downstream reports.

The synthetic tests validate the contract and metric arithmetic only. They do not reproduce the CxHP8 paper or establish prediction accuracy on the other 41 hair plates.

## Synthetic tests

The 9 tests cover a valid inventory, unknown-vs-absence semantics, explicitly absent target-set scoring, a known-answer joint limit/reflex/motor-sign fixture, held-out-plate leakage, malformed tuning curves, duplicate identities, invalid morphology/digests and malformed unknown evidence.

## References

- Nature Communications (2026), *Proprioceptive limit detectors contribute to sensorimotor control of the Drosophila leg*: https://doi.org/10.1038/s41467-026-69333-z
- Dryad source data: https://doi.org/10.5061/dryad.fxpnvx153
- PubMed entry: https://pubmed.ncbi.nlm.nih.gov/41680191/

## Run

    cargo fmt --manifest-path research/hair-plate-reflex-oracle-v1/Cargo.toml -- --check
    cargo test --manifest-path research/hair-plate-reflex-oracle-v1/Cargo.toml --locked
    cargo clippy --manifest-path research/hair-plate-reflex-oracle-v1/Cargo.toml --locked --all-targets -- -D warnings

The package is a standalone Cargo workspace under research/. It does not modify Symtropy's production workspace or dependency graph.
