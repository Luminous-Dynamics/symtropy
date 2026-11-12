# EV battery closed-loop audit v0.1

Dependency-free Rust reference oracle for the evidence chain from one exact battery design revision through manufacturing runs, cell tests, pack membership, mission outcomes and a downstream decision receipt.

This is a read-only structural/evidence audit. It does not simulate electrochemistry, calculate production yield confidence intervals, optimize a recipe, authorize physical testing, control factory equipment, actuate a BMS or release a battery product.

## Run

From the repository root:

    cargo metadata --manifest-path tools/ev-battery-closed-loop-audit/Cargo.toml --locked --no-deps
    cargo fmt --manifest-path tools/ev-battery-closed-loop-audit/Cargo.toml -- --check
    cargo test --manifest-path tools/ev-battery-closed-loop-audit/Cargo.toml --locked
    cargo clippy --manifest-path tools/ev-battery-closed-loop-audit/Cargo.toml --all-targets --locked -- -D warnings

The JSON schema and fixture are stored under schemas/transport and fixtures/ev_battery. The fixture is synthetic and must never be described as a measured cell, pack or manufacturing result.
