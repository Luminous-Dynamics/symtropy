//! Standalone corpus oracle. This binary deliberately does not call the
//! lifecycle contract crate's assess function; it independently checks the
//! frozen gate-state truth table and fixture integrity labels.
use std::collections::BTreeSet;

const CORPUS: &str = include_str!("../../fixtures/lifecycle-cases-v1.tsv");

fn main() {
    let mut ids = BTreeSet::new();
    let mut total = 0usize;
    let mut valid = 0usize;
    let mut adversarial = 0usize;

    for (line_number, line) in CORPUS.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("case_id|") {
            continue;
        }

        let fields = line.split('|').collect::<Vec<_>>();
        assert_eq!(fields.len(), 9, "wrong field count at line {}", line_number + 1);
        assert!(
            ids.insert(fields[0]),
            "duplicate fixture id '{}' at line {}",
            fields[0],
            line_number + 1
        );

        let states = &fields[1..7];
        let mutation = fields[7];
        let expected = fields[8];
        let mut has_failure = false;
        let mut has_unknown = false;
        for state in states {
            match *state {
                "P" => {}
                "F" => has_failure = true,
                "U" => has_unknown = true,
                other => panic!("invalid gate state '{}' at line {}", other, line_number + 1),
            }
        }

        if mutation == "none" {
            let actual = if has_failure {
                "BLOCKED"
            } else if has_unknown {
                "INDETERMINATE"
            } else {
                "REVIEW"
            };
            assert_eq!(
                actual,
                expected,
                "truth-table mismatch in fixture '{}'",
                fields[0]
            );
            valid += 1;
        } else {
            assert_eq!(
                expected,
                "REJECT",
                "adversarial fixture '{}' must be expected to reject",
                fields[0]
            );
            assert!(
                matches!(
                    mutation,
                    "pass_without_evidence"
                        | "missing_reason"
                        | "dangling_reference"
                        | "scope_mismatch"
                        | "duplicate_id"
                        | "malformed_digest"
                ),
                "unknown adversarial mutation '{}'",
                mutation
            );
            adversarial += 1;
        }
        total += 1;
    }

    assert_eq!(total, 12, "frozen corpus size changed without review");
    assert_eq!(valid, 6, "valid truth-table fixture count changed");
    assert_eq!(adversarial, 6, "adversarial fixture count changed");
    println!(
        "verified {total} frozen synthetic fixtures ({valid} truth-table, {adversarial} adversarial); no physical qualification implied"
    );
}
