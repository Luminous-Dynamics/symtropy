# AeroWeft Validation Experiment Design Oracle

Synthetic-first Rust selector for aviation model-discrimination and validation experiment plans.

It filters inadmissible experiments before Pareto-ranking the remaining candidates across decision-relevant uncertainty reduction, model discrimination, worst-case robustness, decision relevance, uninformative probability and cost. It deliberately avoids a single weighted score or maximum-entropy objective.

This tool does not authorize tests, control aircraft, run simulators, calculate real expected information gain, validate physical models, authenticate references, or certify operations. Authorization and safety-envelope status are supplied inputs from responsible owners and are not verified here.

It composes AeroWeft Track T/U and Symtropy #4487/#460/#4366/#3749/#4446. It does not create a new simulator or physics qualification corpus.
