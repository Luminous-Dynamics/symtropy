# Textile Feed Presentation Contract v0.1

Standalone Rust contract for bulk textile feed, singulation and sensor-ready presentation.

## What it qualifies

- every incoming item is uniquely identified;
- every presentation attempt is linked to a known incoming item and has a contiguous retry chain;
- presentation attempts have valid intervals and source-bound evidence references;
- every incoming item ends in exactly one terminal disposition;
- missing, duplicate and orphan items cannot disappear from the run;
- physical route destinations are recorded only for routed outcomes;
- a routed outcome requires at least one valid single-item presentation attempt;
- derived metrics retain their denominator and separate first-pass, eventual, conditional and end-to-end performance.

## Metrics are not interchangeable

- **First-pass single presentation:** first attempt produced exactly one item in the sensing field / all incoming items.
- **Eventual single presentation:** at least one attempt produced a valid single presentation / all incoming items.
- **End-to-end routed:** terminally routed items / all incoming items.
- **End-to-end correct route:** correct destination items / all incoming items with an independent reference destination. Held, rejected, manual-review and not-recovered items remain in this denominator.
- **Conditional route accuracy:** correct destination items / reference-labelled items actually routed.

A classifier's conditional accuracy cannot be reported as the line's end-to-end performance.

## Synthetic challenge corpus

- `feed-cases.example.json` contains 22 positive and adversarial event-accounting cases.
- `feed-cases.schema.json` defines the versioned corpus shape.
- The Rust corpus test executes every case mutation, compares exact expected finding codes, and refuses to emit metrics for invalid runs.

The corpus is synthetic. It qualifies event semantics and denominator handling only; it is not evidence of physical presentation or line performance.

## Validation boundaries

The verifier checks event accounting and the declared observation/evidence references. It does not authenticate evidence issuers, control feeders/robots, prove that an item was physically observed, validate the image/sensor, determine fibre composition, measure plant throughput, or prove downstream acceptance. Physical trials must still bind run, line, configuration, lot, item, attempt, operator and instrument identities.

## Commands

~~~sh
cargo test --manifest-path tools/textile-feed-presentation-contract/Cargo.toml
cargo fmt --manifest-path tools/textile-feed-presentation-contract/Cargo.toml --all -- --check
cargo clippy --manifest-path tools/textile-feed-presentation-contract/Cargo.toml --all-targets -- -D warnings
~~~

The example is synthetic. No physical line or garment has been tested by this crate.
