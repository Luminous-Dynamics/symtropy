# Common Ground — accessible home design prototype

An isolated Leptos 0.8.22 client-rendered application for ACCESS-HOME-001. It is deliberately not a member of the main Symtropy simulation workspace.

## What it does

- Lets a resident choose overlapping functional design dimensions without asking for a diagnosis.
- Keeps a universal task-planning baseline, then adds room/task-specific prompts for selected needs.
- Shows why each prompt exists, what evidence is still missing and who should review it.
- Surfaces design tensions involving sight, hearing, mobility, sensory preferences and automation.
- Lets a resident include or remove prompts and generate a human-readable draft brief.
- Distinguishes a proposed prompt from a verified design claim.

No personal data is persisted. There is no AI diagnosis, building-code engine, structural analysis, verified layout, supplier integration, or life-safety approval.

## Run locally

Install Rust with the WebAssembly target and Trunk. In a shell:

    rustup target add wasm32-unknown-unknown
    cargo install trunk --locked
    cd tools/accessible-home-leptos
    trunk serve --address 127.0.0.1 --port 8089

Open http://127.0.0.1:8089.

## Qualification

From the repository root:

    cargo fmt --manifest-path tools/accessible-home-leptos/Cargo.toml -- --check
    cargo test --manifest-path tools/accessible-home-leptos/Cargo.toml --lib
    cargo check --manifest-path tools/accessible-home-leptos/Cargo.toml --target wasm32-unknown-unknown

The dedicated workflow runs those checks. A queued runner is not a pass. A compile pass does not establish accessibility; perform a manual WCAG 2.2 AA audit and co-design tests with disabled residents using their assistive technologies before calling the UI accessible.

## Scope boundary

This tool helps residents articulate tasks and requirements. It does not replace the existing Symtropy housing evidence and jurisdiction work, an architect, an access consultant, an occupational therapist, a fire engineer, an electrician, a building official or other qualified professionals. All dimensions and regulations must be verified for the actual site, person, equipment and jurisdiction.

## Related work

- ACCESS-HOME-001 issue: https://github.com/Luminous-Dynamics/symtropy/issues/1687
- Existing housing evidence contracts (draft PR): https://github.com/Luminous-Dynamics/symtropy/pull/1596
- ISO 21542:2021: https://www.iso.org/standard/71860.html
- South African National Strategic Framework on Universal Design and Access: https://www.gov.za/sites/default/files/gcis_document/202110/45328gen606.pdf
- RNIB home adaptation guidance: https://www.rnib.org.uk/living-with-sight-loss/independent-living/adapting-your-home/
- RNID smoke alarm guidance: https://rnid.org.uk/information-and-support/technology-and-products/smoke-alarm-systems/
