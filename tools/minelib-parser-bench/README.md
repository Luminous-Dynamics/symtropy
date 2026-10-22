# MineLib parser and tiny UPIT conformance benchmark

This standalone Rust crate validates an intentionally small MineLib v1 profile using authored synthetic fixtures. It parses three linked inputs:
- block-model rows: block ID, x/y/z coordinates, explicitly declared attributes;
- precedence rows: block ID and a declared predecessor list;
- UPIT optimization-model descriptor: name, type, block count, discount-rate field, and per-block objective rows.

It rejects unrecognized descriptor fields/sections rather than pretending to support general MineLib. Only UPIT is supported in the tiny-model path; CPIT and PCPSP descriptors are intentionally rejected. An exhaustive closure oracle runs only for at most 20 contiguous zero-based blocks, after which the correct disposition is an indeterminate size limit—not an optimality claim.

The manifest must declare the meaning and type of each optional block attribute. This avoids guessing whether a field is tonnage, grade, impurity, or some other value. The fixture also compares its explicit block-value attribute to the corresponding synthetic UPIT objective coefficients to catch cross-file identity/schema mistakes.

Run:

```sh
cargo fmt --manifest-path tools/minelib-parser-bench/Cargo.toml -- --check
cargo test --manifest-path tools/minelib-parser-bench/Cargo.toml
cargo run --quiet --manifest-path tools/minelib-parser-bench/Cargo.toml -- \
  docs/tech/minelib-parser-fixtures-v0.1.synthetic.json \
  --source-commit "$(git rev-parse HEAD)"
```

The runner emits a JSON receipt binding the manifest, block file, precedence file, model descriptor, and exact source commit by SHA-256. Fixed decimals are converted to scaled integers without binary floating point.

The sum of block attribute values is called an audit sum; it is not the mathematical model objective or NPV. The UPIT objective comes from the separately parsed model descriptor. The five-block exhaustive oracle is a correctness reference only, not a production solver.

No third-party MineLib instance is included. All block/precedence/model fixtures are authored synthetic inputs. The format URL is a citation to the official MineLib specification.
