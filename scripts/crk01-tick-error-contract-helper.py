#!/usr/bin/env python3
from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one target, found {count}")
    return text.replace(old, new)


event_path = Path("crates/domains/symtropy-game-state/src/event_v2.rs")
event_text = event_path.read_text()

event_text = replace_once(
    event_text,
    "\n".join([
        "            return Err(EventV2Error::NonMonotonicTick {",
        "                event_id: previous.event_id.clone(),",
        "                previous: previous.simulation_tick,",
        "                actual: simulation_tick,",
        "            });",
    ]),
    "\n".join([
        "            return Err(EventV2Error::AppendNonMonotonicTick {",
        "                previous_event_id: previous.event_id.clone(),",
        "                previous: previous.simulation_tick,",
        "                actual: simulation_tick,",
        "            });",
    ]),
    "append error construction",
)

event_text = replace_once(
    event_text,
    "\n".join([
        "    NonMonotonicTick {",
        "        event_id: StableId,",
        "        previous: u64,",
        "        actual: u64,",
        "    },",
    ]),
    "\n".join([
        "    AppendNonMonotonicTick {",
        "        previous_event_id: StableId,",
        "        previous: u64,",
        "        actual: u64,",
        "    },",
        "    NonMonotonicTick {",
        "        event_id: StableId,",
        "        previous: u64,",
        "        actual: u64,",
        "    },",
    ]),
    "error enum",
)

event_text = replace_once(
    event_text,
    "\n".join([
        "            Self::NonMonotonicTick {",
        "                event_id,",
        "                previous,",
        "                actual,",
        "            } => write!(",
        "                formatter,",
        '                "canonical event {event_id} moved backward from tick {previous} to {actual}"',
        "            ),",
    ]),
    "\n".join([
        "            Self::AppendNonMonotonicTick {",
        "                previous_event_id,",
        "                previous,",
        "                actual,",
        "            } => write!(",
        "                formatter,",
        '                "cannot append canonical event after {previous_event_id}: tick moved backward from {previous} to {actual}"',
        "            ),",
        "            Self::NonMonotonicTick {",
        "                event_id,",
        "                previous,",
        "                actual,",
        "            } => write!(",
        "                formatter,",
        '                "canonical event {event_id} moved backward from tick {previous} to {actual}"',
        "            ),",
    ]),
    "error display",
)

event_path.write_text(event_text)


test_path = Path("crates/domains/symtropy-game-state/tests/canonical_event_v2.rs")
test_text = test_path.read_text()
old_test = """#[test]
fn append_rejects_non_monotonic_tick_without_mutating_chain() {
    let namespace = StableIdNamespace::parse("fold.event").expect("valid namespace");
    let mut chain = EventChainV2::new(namespace, 91);
    chain
        .append(
            10,
            StableEventKind::parse("fold.observed").expect("valid event kind"),
            None,
            None,
            Vec::new(),
            GoldenPayload { value: 5 },
        )
        .expect("first event appends");

    let head_before = chain.head_digest();
    let len_before = chain.events().len();
    let error = chain
        .append(
            9,
            StableEventKind::parse("fold.rewind.applied").expect("valid event kind"),
            None,
            None,
            Vec::new(),
            GoldenPayload { value: 9 },
        )
        .expect_err("backward simulation tick must fail");

    match error {
        EventV2Error::NonMonotonicTick {
            previous, actual, ..
        } => {
            assert_eq!(previous, 10);
            assert_eq!(actual, 9);
        }
        other => panic!("expected NonMonotonicTick, got {other:?}"),
    }
    assert_eq!(chain.events().len(), len_before);
    assert_eq!(chain.head_digest(), head_before);
}
"""
new_test = """#[test]
fn append_rejects_non_monotonic_tick_without_mutating_chain() {
    let namespace = StableIdNamespace::parse("fold.event").expect("valid namespace");
    let mut chain = EventChainV2::new(namespace.clone(), 91);
    let previous_event_id = chain
        .append(
            10,
            StableEventKind::parse("fold.observed").expect("valid event kind"),
            None,
            None,
            Vec::new(),
            GoldenPayload { value: 5 },
        )
        .expect("first event appends");

    let head_before = chain.head_digest();
    let len_before = chain.events().len();
    let error = chain
        .append(
            9,
            StableEventKind::parse("fold.rewind.applied").expect("valid event kind"),
            None,
            None,
            Vec::new(),
            GoldenPayload { value: 9 },
        )
        .expect_err("backward simulation tick must fail");

    assert_eq!(
        error.to_string(),
        format!(
            "cannot append canonical event after {previous_event_id}: tick moved backward from 10 to 9"
        )
    );
    match error {
        EventV2Error::AppendNonMonotonicTick {
            previous_event_id: actual_previous_event_id,
            previous,
            actual,
        } => {
            assert_eq!(actual_previous_event_id, previous_event_id);
            assert_eq!(previous, 10);
            assert_eq!(actual, 9);
        }
        other => panic!("expected AppendNonMonotonicTick, got {other:?}"),
    }
    assert_eq!(chain.events().len(), len_before);
    assert_eq!(chain.head_digest(), head_before);

    let expected_next_id = StableId::derive_v2(&namespace, 91, 1).expect("valid next event id");
    let next_id = chain
        .append(
            11,
            StableEventKind::parse("fold.rewind.applied").expect("valid event kind"),
            None,
            None,
            Vec::new(),
            GoldenPayload { value: 9 },
        )
        .expect("valid append after rejection");
    assert_eq!(next_id, expected_next_id, "rejected append must not reserve an ordinal");
}

#[test]
fn reconstruction_identifies_persisted_non_monotonic_event() {
    let namespace = StableIdNamespace::parse("fold.event").expect("valid namespace");
    let mut chain = EventChainV2::new(namespace.clone(), 91);
    chain
        .append(
            10,
            StableEventKind::parse("fold.observed").expect("valid event kind"),
            None,
            None,
            Vec::new(),
            GoldenPayload { value: 5 },
        )
        .expect("first event appends");
    let offending_event_id = chain
        .append(
            11,
            StableEventKind::parse("fold.rewind.applied").expect("valid event kind"),
            None,
            None,
            Vec::new(),
            GoldenPayload { value: 9 },
        )
        .expect("second event appends");

    let mut events = chain.events().to_vec();
    events[1].simulation_tick = 9;
    let error = EventChainV2::from_events(namespace, 91, events)
        .expect_err("persisted backward tick must fail verification");

    assert_eq!(
        error.to_string(),
        format!("canonical event {offending_event_id} moved backward from tick 10 to 9")
    );
    match error {
        EventV2Error::NonMonotonicTick {
            event_id,
            previous,
            actual,
        } => {
            assert_eq!(event_id, offending_event_id);
            assert_eq!(previous, 10);
            assert_eq!(actual, 9);
        }
        other => panic!("expected NonMonotonicTick, got {other:?}"),
    }
}
"""
test_text = replace_once(test_text, old_test, new_test, "public tick regressions")
test_path.write_text(test_text)


doc_path = Path("crates/domains/symtropy-game-state/CANONICAL_EVENT_V2.md")
doc_text = doc_path.read_text()
marker = """9. recomputed canonical event digest.

### Verified-chain authority boundary"""
replacement = """9. recomputed canonical event digest.

### Monotonic-tick diagnostic identity

Append-time rejection and persisted-chain verification expose different identity facts and therefore use distinct errors:

- `AppendNonMonotonicTick` identifies the **previous committed event** through `previous_event_id`; it does not fabricate or reserve an ID or ordinal for the rejected uncommitted attempt;
- `NonMonotonicTick` identifies the **offending persisted event** through `event_id` during chain reconstruction/verification.

A rejected append leaves the committed chain length and head digest unchanged, and a subsequent valid append receives the same deterministic ordinal/ID it would have received if the rejected attempt had never occurred.

### Verified-chain authority boundary"""
doc_text = replace_once(doc_text, marker, replacement, "verification documentation")
doc_path.write_text(doc_text)
