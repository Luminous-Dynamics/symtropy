# Textile schedule stress fixtures v0.1

This directory defines a machine-readable fixture format and one **synthetic**
example for the bounded schedule stress harness.

- stress_corpus.schema.json: JSON Schema draft 2020-12 contract.
- stress_corpus.example.json: one qualified heritage-repair job with six cases:
  nominal, increased processing time, delayed input readiness, higher water
  intensity, energy curtailment, and a specialist cell unavailable too long to
  meet the hard finish window.

The example declares explicit time, energy and water unit IDs, keeps its garment
profile and service unit, includes evidence-reference placeholders, and retains
non-feasible cases as negative controls. All evidence references beginning with
fixture: are identifiers for this test corpus, not authentication or factory
measurements.

## Expected behavior of this fixture

- Nominal: exact reference finds a feasible schedule.
- Slow repair: still feasible, with higher tardiness relative to the soft due tick.
- Late specialist input: feasible if it remains inside the hard finish window.
- Water-intensive finishing: no feasible schedule; process water exceeds the
  declared budget.
- Energy curtailed to zero: no feasible schedule; process energy alone exceeds
  the declared hard budget.
- Specialist cell unavailable until tick 8: no feasible schedule; the finish
  would exceed the hard finish tick.

The Rust stress module currently constructs equivalent typed cases in tests.
It does **not** yet load this JSON directly. A future loader must be validated
against this schema with an independent validator before the JSON is treated as
an executable corpus. This distinction is intentional: a valid-looking file is
not runtime evidence until parsing and semantic validation are actually wired.

The scenario set is unweighted and synthetic. Its six examples are not a
sample of disruption frequency, do not estimate real-world probability, and
cannot support factory-savings, compliance or circularity claims. Compute and
record a SHA-256 of the exact input bytes used in every benchmark run; do not
silently reformat or normalize a manifest after measurement without changing
the revision and digest.
