# AeroWeft Operational Assurance Case Oracle

Synthetic-first, dependency-free Rust checker for the structure and evidence applicability of a bounded aviation assurance-case graph.

It checks exact case scope, typed argument edges, required hazard/mitigation coverage, claim support, explicit assumptions and defeaters, evidence disposition, common-cause grouping, and authority-decision records. Its strongest positive result is `REVIEW_READY_ONLY`.

It does not calculate risk, verify signatures, establish the authenticity or truth of flight evidence, accept residual risk, grant clearance, authorize a flight, certify an aircraft, or replace the applicant or competent authority. A completed argument is not operational permission.

It composes Tracks P–V and Mycelix trust/custody/common-cause work; it does not duplicate those owners' algorithms or trust infrastructure. All fixtures are synthetic.
