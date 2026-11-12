# ev-battery-ultrasonic-ndt-audit

Dependency-free Rust 1.96 reference audit for sample-held-out ultrasonic electrode thickness evaluation.

Checks exact source snapshot/rights receipt references, physical specimen and paired-state grouping, material/process domain identity, feature availability at prediction time, independent thickness-label evidence, train/validation/test group closure, and one prediction per held-out observation.

Reports pooled regression metrics and material-family stratification. It does not read raw ultrasound, train signal models, authenticate source bytes, certify the metrology system, qualify process capability, or authorize production inspection decisions.

The test fixture is synthetic known-truth data; it is not a reproduction of the Warwick Mendeley dataset.