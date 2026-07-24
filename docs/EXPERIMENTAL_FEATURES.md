# Experimental Features Guide

To maintain workspace stability, unfinished or experimental features are quarantined behind feature flags.

## Usage

You can enable experimental modules during build or run commands by adding the `--features experimental` flag to your cargo or `just` command.

### Build/Run Targets

- `just build-experimental`: Builds the workspace with all experimental features enabled.
- `just run-experimental`: Runs the main launcher with all experimental features enabled.

## Adding a New Experimental Feature

If you are developing a new module:

1.  **Ledger It**: Create a description in `docs/ledger/<name>.md`.
2.  **Gate It**:
    - Add the feature to your crate's `Cargo.toml`.
    - Use `#[cfg(feature = "experimental-<name>")]` in your Rust code.
3.  **Unify It**: Add `experimental-<name>` as a member of the unified `experimental` feature set in the relevant `Cargo.toml`.
