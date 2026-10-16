# Daywell — Leptos holiday UX prototype

An **isolated Leptos 0.8.22 client-rendered app** for HOLIDAY-UX-001. The separate workspace keeps browser UI dependencies outside Symtropy's simulation workspace.

## Included

- Four selectable trip intents and three paces.
- Three fictional concept destinations with explicitly synthetic prices.
- Editable activities, a derived day-load indicator, and protected open time.
- Explicitly unknown or unverified access evidence and resource boundaries.
- A simulated closure with a no-purchase path, an unconfirmed alternative, and a defer path.
- No account, tracking, geolocation, external API, persistence, booking, payment, or physical-control authority.

## Run locally

Install Rust and the WebAssembly target, then install Trunk:

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
cd tools/holiday-ux-leptos
trunk serve --address 127.0.0.1 --port 8088
```

Open `http://127.0.0.1:8088`.

## Qualification

From the repository root:

```sh
cargo fmt --manifest-path tools/holiday-ux-leptos/Cargo.toml -- --check
cargo test --manifest-path tools/holiday-ux-leptos/Cargo.toml --lib
cargo check --manifest-path tools/holiday-ux-leptos/Cargo.toml --target wasm32-unknown-unknown
```

The standalone GitHub Actions workflow runs these gates for changes in this app. A queued run is not a pass. Before claiming the UI is accessible or improves holidays, perform a manual WCAG 2.2 AA audit and a preregistered usability comparison with a documented baseline and participants with diverse access needs.

## Related work

- [HOLIDAY-UX-001 issue](https://github.com/Luminous-Dynamics/symtropy/issues/1673)
- [Journey and benchmark specification](https://github.com/Luminous-Dynamics/symtropy/blob/research/holiday-experience-ux/docs/research/joy-worlds/holiday-experience-ux-v0.1.md)
- [Machine-readable qualification gates](https://github.com/Luminous-Dynamics/symtropy/blob/research/holiday-experience-ux/docs/research/joy-worlds/holiday-ux-qualification-gates-v0.1.json)
- [Leptos documentation](https://docs.rs/leptos/latest/leptos/)
