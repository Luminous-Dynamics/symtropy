# AeroWeft Validation Unit Integrity Oracle

Dependency-free Rust structural validator for aviation model-validation denominators and split independence.

It derives independent sample counts from exact unit identity records, validates nested cluster membership, prevents protected dependence clusters crossing frozen splits, and checks reported denominators against the identity manifest. Repeated rows, frames or samples cannot inflate the independent-unit count.

This crate does not decide whether a source-defined cluster is physically independent. That is an owner-reviewed assumption bound by the manifest and policy. It does not compute confidence intervals or bootstrap results; it composes Mycelix #5637/#5644 for paired cluster bootstrap and known-dependent-DGP qualification.
