# MineLib parser conformance prototype

This standalone Rust crate validates a deliberately small subset of the MineLib v1 format using authored synthetic fixtures. It parses the block-model and precedence files only; it does **not** parse the optimization-model descriptor and is not an optimizer.

The manifest declares the meaning and type of each optional block attribute. This is necessary because MineLib allows user-specific block attributes; the parser must not guess a grade, tonnage, or objective column from position alone.

Run:

```sh
cargo fmt --manifest-path tools/minelib-parser-bench/Cargo.toml -- --check
cargo test --manifest-path tools/minelib-parser-bench/Cargo.toml
cargo run --quiet --manifest-path tools/minelib-parser-bench/Cargo.toml -- \
  docs/tech/minelib-parser-fixtures-v0.1.synthetic.json \
  --source-commit "$(git rev-parse HEAD)"
```

The runner emits a JSON receipt binding the manifest, block file, precedence file, and exact source commit. Fixed decimals are converted to scaled integers without binary floating point. The `block_value_usd` total is merely an audit sum over a declared fixture column; it is not the MineLib model's discounted objective and does not validate an optimization result.

No third-party MineLib instance or data file is included. The test files are hand-authored synthetic fixtures, and the source-format URL is cited for structure only.
