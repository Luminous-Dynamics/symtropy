# PCC Metrology Known-Truth Fixtures

**Schema:** `symtropy.pcc-metrology-fixtures.v1`  
**Classification:** synthetic known truth  
**Parent issue:** #3752  
**Protocol:** `docs/research/ai-data-centre-energy/PCC_METROLOGY_PROTOCOL.md`

The fixtures cover correction arithmetic, paired covariance, threshold guard-bands, clock alignment, missing intervals, calibration expiry, nonlinearity, shared background load and nested meters.

The paired covariance example is intentionally explicit:
- baseline and candidate standard uncertainty: 2 W each;
- correlation: 0.9, derived from the synthetic shared-chain fixture;
- standard uncertainty of the difference: approximately 0.8944 W;
- with the illustrative k=2 coverage, the lower response bound is approximately 98.211 W;
- the independent-error control gives approximately 2.8284 W standard uncertainty and a lower bound below the 98 W threshold.

This demonstrates why the covariance term matters. It does **not** authorize assuming common-mode cancellation in real data. The actual shared meter chain and operating range must support the covariance model. The k=2 case is an arithmetic control, not a universal coverage rule.

All expected values are synthetic. No calibration certificate, field meter trace, standard conformance or real PCC accuracy claim is included.