// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Read-only NASTRAN bulk-data intake. This module inventories syntax and
// reference topology only. It deliberately does not assemble or solve FEA.

use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::str;

const FIXTURE: &[u8] = include_bytes!("../data/synthetic-equivalent-beam.bdf");

#[derive(Clone, Debug, PartialEq, Eq)]
struct RawLine {
    line_number: usize,
    byte_start: usize,
    byte_end: usize,
    text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Card {
    name: String,
    fields: Vec<String>,
    raw_lines: Vec<RawLine>,
    parse_state: &'static str,
}

#[derive(Clone, Debug)]
struct Deck {
    source_sha256: String,
    source_bytes: usize,
    executive_control: Vec<String>,
    cards: Vec<Card>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ParseError(String);

impl std::fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ParseError {}

type ParseResult<T> = Result<T, ParseError>;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Inventory {
    card_counts: BTreeMap<String, usize>,
    grid_ids: BTreeSet<u32>,
    cbeam_ids: BTreeSet<u32>,
    element_ids: BTreeSet<u32>,
    property_materials: BTreeMap<u32, u32>,
    material_ids: BTreeSet<u32>,
    rbe2_ids: BTreeSet<u32>,
    spc_set_ids: BTreeSet<u32>,
    force_set_ids: BTreeSet<u32>,
    load_set_ids: BTreeSet<u32>,
    opaque_cards: usize,
    orphan_grid_count: usize,
    claim_ceiling: &'static str,
}

fn fail<T>(message: impl Into<String>) -> ParseResult<T> {
    Err(ParseError(message.into()))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing into String cannot fail");
    }
    output
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn fixed_fields(line: &str, line_number: usize) -> ParseResult<Vec<String>> {
    if line.as_bytes().iter().any(|byte| !byte.is_ascii()) {
        return fail(format!("line {line_number}: non-ASCII bytes are unsupported"));
    }
    if line.contains('$') {
        return fail(format!(
            "line {line_number}: inline comments are unsupported; preserve comments as full lines"
        ));
    }
    if line.len() > 80 && line[80..].trim().is_empty() {
        // Trailing blanks beyond column 80 carry no field data.
        return Ok((0..10)
            .map(|index| {
                let start = index * 8;
                line.get(start..start + 8).unwrap_or("").trim().to_owned()
            })
            .collect());
    }
    if line.len() > 80 {
        return fail(format!(
            "line {line_number}: line exceeds the supported 80-column small-field format"
        ));
    }

    let mut padded = line.to_owned();
    while padded.len() < 80 {
        padded.push(' ');
    }
    Ok((0..10)
        .map(|index| padded[index * 8..(index + 1) * 8].trim().to_owned())
        .collect())
}

fn source_lines(bytes: &[u8]) -> ParseResult<Vec<RawLine>> {
    if bytes.is_empty() {
        return fail("source deck is empty");
    }
    if bytes.iter().any(|byte| !byte.is_ascii()) {
        return fail("source deck contains non-ASCII bytes");
    }

    let mut result = Vec::new();
    let mut byte_offset = 0usize;
    for (index, segment) in bytes.split_inclusive(|byte| *byte == b'\n').enumerate() {
        let mut body_len = segment.len();
        if body_len > 0 && segment[body_len - 1] == b'\n' {
            body_len -= 1;
        }
        if body_len > 0 && segment[body_len - 1] == b'\r' {
            body_len -= 1;
        }
        let body = &segment[..body_len];
        let text = str::from_utf8(body)
            .map_err(|_| ParseError(format!("line {}: invalid UTF-8", index + 1)))?;
        result.push(RawLine {
            line_number: index + 1,
            byte_start: byte_offset,
            byte_end: byte_offset + body_len,
            text: text.to_owned(),
        });
        byte_offset += segment.len();
    }
    Ok(result)
}

#[derive(Debug)]
struct PendingCard {
    name: String,
    fields: Vec<String>,
    raw_lines: Vec<RawLine>,
    next_continuation: Option<String>,
}

fn take_card(pending: PendingCard) -> Card {
    let state = if is_supported_card(&pending.name) {
        "parsed"
    } else {
        "opaque"
    };
    Card {
        name: pending.name,
        fields: pending.fields,
        raw_lines: pending.raw_lines,
        parse_state: state,
    }
}

fn is_supported_card(name: &str) -> bool {
    matches!(
        name,
        "GRID"
            | "CBEAM"
            | "MAT1"
            | "PBEAM"
            | "CONM2"
            | "RBE2"
            | "SPC"
            | "SPC1"
            | "FORCE"
            | "LOAD"
            | "PARAM"
    )
}

fn continuation_label(value: &str, line_number: usize) -> ParseResult<Option<String>> {
    if value.trim().is_empty() {
        return Ok(None);
    }
    if !value.starts_with('+') || value.len() < 2 {
        return fail(format!(
            "line {line_number}: only explicit '+' small-field continuation labels are supported"
        ));
    }
    Ok(Some(value.to_owned()))
}

fn parse_deck(bytes: &[u8], expected_sha256: &str) -> ParseResult<Deck> {
    if !valid_sha256(expected_sha256) {
        return fail("expected source digest must be lowercase 64-character SHA-256");
    }
    let actual_sha256 = sha256_hex(bytes);
    if actual_sha256 != expected_sha256 {
        return fail("source SHA-256 does not match the supplied exact-byte digest");
    }

    let lines = source_lines(bytes)?;
    let mut executive_control = Vec::new();
    let mut cards = Vec::new();
    let mut in_bulk = false;
    let mut saw_enddata = false;
    let mut pending: Option<PendingCard> = None;

    for line in lines {
        let trimmed = line.text.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('$') {
            continue;
        }

        if !in_bulk {
            executive_control.push(line.text.clone());
            if trimmed.eq_ignore_ascii_case("BEGIN BULK") {
                in_bulk = true;
            }
            continue;
        }

        if trimmed.eq_ignore_ascii_case("ENDDATA") {
            if pending.is_some() {
                return fail(format!(
                    "line {}: ENDDATA encountered while a continuation is unresolved",
                    line.line_number
                ));
            }
            saw_enddata = true;
            break;
        }

        let fields = fixed_fields(&line.text, line.line_number)?;
        let first = fields[0].trim().to_owned();

        if first.is_empty() {
            if trimmed.is_empty() {
                continue;
            }
            return fail(format!(
                "line {}: blank first field with other content is ambiguous; implicit continuation is rejected",
                line.line_number
            ));
        }

        if first.starts_with('*') {
            return fail(format!(
                "line {}: large-field '*' cards are outside the v0.1 small-field parser",
                line.line_number
            ));
        }

        if first.starts_with('+') {
            let Some(mut current) = pending.take() else {
                return fail(format!(
                    "line {}: orphan continuation line {}",
                    line.line_number, first
                ));
            };
            let expected = current.next_continuation.take().ok_or_else(|| {
                ParseError(format!(
                    "line {}: continuation {} has no matching predecessor label",
                    line.line_number, first
                ))
            })?;
            if first != expected {
                return fail(format!(
                    "line {}: continuation label mismatch; expected {}, received {}",
                    line.line_number, expected, first
                ));
            }
            current.fields.extend(fields[1..9].iter().cloned());
            current.raw_lines.push(line.clone());
            current.next_continuation =
                continuation_label(&fields[9], line.line_number)?;
            if current.next_continuation.is_some() {
                pending = Some(current);
            } else {
                cards.push(take_card(current));
            }
            continue;
        }

        if let Some(current) = pending.take() {
            return fail(format!(
                "line {}: card {} began before continuation {} was supplied",
                line.line_number,
                first,
                current.next_continuation.unwrap_or_default()
            ));
        }

        if first.ends_with('*') {
            return fail(format!(
                "line {}: large-field card {} is unsupported",
                line.line_number, first
            ));
        }

        let continuation = continuation_label(&fields[9], line.line_number)?;
        let new_card = PendingCard {
            name: first.to_ascii_uppercase(),
            fields: fields[1..9].to_vec(),
            raw_lines: vec![line.clone()],
            next_continuation: continuation,
        };
        if new_card.next_continuation.is_some() {
            pending = Some(new_card);
        } else {
            cards.push(take_card(new_card));
        }
    }

    if let Some(current) = pending {
        return fail(format!(
            "end of source: unresolved continuation {} for card {}",
            current.next_continuation.unwrap_or_default(),
            current.name
        ));
    }
    if !in_bulk {
        return fail("BEGIN BULK marker was not found");
    }
    if !saw_enddata {
        return fail("ENDDATA marker was not found");
    }
    if cards.is_empty() {
        return fail("bulk-data section contains no cards");
    }

    Ok(Deck {
        source_sha256: actual_sha256,
        source_bytes: bytes.len(),
        executive_control,
        cards,
    })
}

fn field<'a>(card: &'a Card, index: usize, label: &str) -> ParseResult<&'a str> {
    card.fields
        .get(index)
        .map(String::as_str)
        .ok_or_else(|| ParseError(format!("{}: missing {label} field", card.name)))
}

fn required_text<'a>(card: &'a Card, index: usize, label: &str) -> ParseResult<&'a str> {
    let value = field(card, index, label)?;
    if value.trim().is_empty() {
        return fail(format!("{}: required {label} field is blank", card.name));
    }
    Ok(value)
}

fn parse_u32(value: &str, label: &str) -> ParseResult<u32> {
    value
        .trim()
        .parse::<u32>()
        .map_err(|_| ParseError(format!("{label}: invalid positive integer {value:?}")))
}

fn parse_nastran_float(value: &str, label: &str) -> ParseResult<f64> {
    let value = value.trim();
    if value.is_empty() {
        return fail(format!("{label}: numeric field is blank"));
    }
    let normalized = value.replace('D', "E").replace('d', "E");
    let parsed = if let Ok(number) = normalized.parse::<f64>() {
        number
    } else {
        let split = normalized
            .char_indices()
            .skip(1)
            .filter(|(_, character)| *character == '+' || *character == '-')
            .map(|(index, _)| index)
            .last();
        let Some(index) = split else {
            return fail(format!("{label}: invalid NASTRAN numeric field {value:?}"));
        };
        let (mantissa, exponent) = normalized.split_at(index);
        let exponent = exponent
            .parse::<i32>()
            .map_err(|_| ParseError(format!("{label}: invalid implicit exponent {value:?}")))?;
        let expanded = format!("{mantissa}E{exponent}");
        expanded
            .parse::<f64>()
            .map_err(|_| ParseError(format!("{label}: invalid numeric field {value:?}")))?
    };
    if !parsed.is_finite() {
        return fail(format!("{label}: non-finite numeric value {value:?}"));
    }
    Ok(parsed)
}

fn validate_numeric_if_present(card: &Card, indices: &[usize]) -> ParseResult<()> {
    for index in indices {
        let value = field(card, *index, "numeric")?;
        if !value.trim().is_empty() {
            parse_nastran_float(value, &format!("{} field {}", card.name, index + 1))?;
        }
    }
    Ok(())
}

fn push_unique(set: &mut BTreeSet<u32>, value: u32, label: &str) -> ParseResult<()> {
    if !set.insert(value) {
        return fail(format!("duplicate {label} ID {value}"));
    }
    Ok(())
}

fn parse_spc1(card: &Card, refs: &mut BTreeSet<u32>, set_ids: &mut BTreeSet<u32>) -> ParseResult<()> {
    let sid = parse_u32(required_text(card, 0, "SID")?, "SPC1 SID")?;
    set_ids.insert(sid);
    let _components = required_text(card, 1, "component")?;
    let mut index = 2usize;
    while index < card.fields.len() {
        let token = field(card, index, "grid/THRU")?;
        if token.trim().is_empty() {
            index += 1;
            continue;
        }
        if token.eq_ignore_ascii_case("THRU") {
            return fail("SPC1: THRU token has no preceding grid ID");
        }
        let start = parse_u32(token, "SPC1 grid")?;
        if index + 1 < card.fields.len()
            && field(card, index + 1, "optional THRU")?.eq_ignore_ascii_case("THRU")
        {
            if index + 2 >= card.fields.len() {
                return fail("SPC1: THRU range has no end grid");
            }
            let end = parse_u32(field(card, index + 2, "THRU end")?, "SPC1 THRU end")?;
            if end < start || end - start > 100_000 {
                return fail("SPC1: invalid or excessive THRU range");
            }
            for grid in start..=end {
                refs.insert(grid);
            }
            index += 3;
        } else {
            refs.insert(start);
            index += 1;
        }
    }
    Ok(())
}

fn build_inventory(deck: &Deck) -> ParseResult<Inventory> {
    let mut inventory = Inventory {
        claim_ceiling: "syntax_and_topology_inventory_only",
        ..Inventory::default()
    };

    let mut grid_ids = BTreeSet::new();
    let mut element_ids = BTreeSet::new();
    let mut property_materials = BTreeMap::new();
    let mut material_ids = BTreeSet::new();
    let mut beam_grid_refs = BTreeSet::new();
    let mut rbe_grid_refs = BTreeSet::new();
    let mut spc_grid_refs = BTreeSet::new();
    let mut force_grid_refs = BTreeSet::new();
    let mut force_set_ids = BTreeSet::new();
    let mut load_set_ids = BTreeSet::new();
    let mut load_refs = BTreeSet::new();

    for card in &deck.cards {
        *inventory.card_counts.entry(card.name.clone()).or_insert(0) += 1;
        if card.parse_state == "opaque" {
            inventory.opaque_cards += 1;
            continue;
        }

        match card.name.as_str() {
            "GRID" => {
                let id = parse_u32(required_text(card, 0, "GRID ID")?, "GRID ID")?;
                push_unique(&mut grid_ids, id, "GRID")?;
                let cp = field(card, 1, "GRID CP")?;
                if !cp.is_empty() {
                    parse_u32(cp, "GRID CP")?;
                }
                validate_numeric_if_present(card, &[2, 3, 4])?;
                for index in [2usize, 3, 4] {
                    if field(card, index, "GRID coordinate")?.is_empty() {
                        return fail(format!("GRID {id}: coordinate {} is blank", index - 1));
                    }
                }
            }
            "CBEAM" => {
                let eid = parse_u32(required_text(card, 0, "CBEAM EID")?, "CBEAM EID")?;
                let _pid = parse_u32(required_text(card, 1, "CBEAM PID")?, "CBEAM PID")?;
                let ga = parse_u32(required_text(card, 2, "CBEAM GA")?, "CBEAM GA")?;
                let gb = parse_u32(required_text(card, 3, "CBEAM GB")?, "CBEAM GB")?;
                if ga == gb {
                    return fail(format!("CBEAM {eid}: GA and GB are identical"));
                }
                push_unique(&mut element_ids, eid, "element")?;
                inventory.cbeam_ids.insert(eid);
                beam_grid_refs.insert(ga);
                beam_grid_refs.insert(gb);
                let orientation = [4usize, 5, 6]
                    .map(|index| field(card, index, "CBEAM orientation").unwrap_or(""));
                let present = orientation.iter().filter(|value| !value.is_empty()).count();
                if present != 0 && present != 3 {
                    return fail(format!(
                        "CBEAM {eid}: orientation vector must have zero or three components"
                    ));
                }
                validate_numeric_if_present(card, &[4, 5, 6])?;
            }
            "MAT1" => {
                let mid = parse_u32(required_text(card, 0, "MAT1 MID")?, "MAT1 MID")?;
                push_unique(&mut material_ids, mid, "MAT1")?;
                validate_numeric_if_present(card, &[1, 2, 3, 4, 5, 6, 7])?;
            }
            "PBEAM" => {
                let pid = parse_u32(required_text(card, 0, "PBEAM PID")?, "PBEAM PID")?;
                let mid = parse_u32(required_text(card, 1, "PBEAM MID")?, "PBEAM MID")?;
                if property_materials.insert(pid, mid).is_some() {
                    return fail(format!("duplicate PBEAM property ID {pid}"));
                }
                // v0.1 validates only the core section fields; remaining fields
                // are preserved verbatim until the exact source dialect is admitted.
                validate_numeric_if_present(card, &[2, 3, 4, 5, 6, 7])?;
            }
            "CONM2" => {
                let eid = parse_u32(required_text(card, 0, "CONM2 EID")?, "CONM2 EID")?;
                let grid = parse_u32(required_text(card, 1, "CONM2 GRID")?, "CONM2 GRID")?;
                push_unique(&mut element_ids, eid, "element")?;
                let cid = field(card, 2, "CONM2 CID")?;
                if !cid.is_empty() {
                    parse_u32(cid, "CONM2 CID")?;
                }
                let mass = parse_nastran_float(required_text(card, 3, "CONM2 mass")?, "CONM2 mass")?;
                if mass < 0.0 {
                    return fail(format!("CONM2 {eid}: mass cannot be negative"));
                }
                validate_numeric_if_present(card, &[4, 5, 6, 7, 8, 9, 10, 11, 12])?;
                beam_grid_refs.insert(grid);
            }
            "RBE2" => {
                let eid = parse_u32(required_text(card, 0, "RBE2 EID")?, "RBE2 EID")?;
                let independent = parse_u32(required_text(card, 1, "RBE2 independent GRID")?, "RBE2 independent GRID")?;
                let _components = required_text(card, 2, "RBE2 components")?;
                push_unique(&mut element_ids, eid, "element")?;
                inventory.rbe2_ids.insert(eid);
                rbe_grid_refs.insert(independent);
                let mut dependents = BTreeSet::new();
                for (index, token) in card.fields.iter().enumerate().skip(3) {
                    if token.is_empty() {
                        continue;
                    }
                    let grid = parse_u32(token, &format!("RBE2 dependent GRID field {}", index + 1))?;
                    if !dependents.insert(grid) {
                        return fail(format!("RBE2 {eid}: duplicate dependent GRID {grid}"));
                    }
                    rbe_grid_refs.insert(grid);
                }
                if dependents.is_empty() {
                    return fail(format!("RBE2 {eid}: no dependent GRID IDs"));
                }
            }
            "SPC" => {
                let sid = parse_u32(required_text(card, 0, "SPC SID")?, "SPC SID")?;
                inventory.spc_set_ids.insert(sid);
                if card.fields.len() < 4 || (card.fields.len() - 1) % 3 != 0 {
                    return fail(format!("SPC {sid}: expected GRID/component/value triplets"));
                }
                for index in (1..card.fields.len()).step_by(3) {
                    let grid = parse_u32(field(card, index, "SPC GRID")?, "SPC GRID")?;
                    let components = required_text(card, index + 1, "SPC component")?;
                    if !components.bytes().all(|byte| byte.is_ascii_digit() && byte <= b'6') {
                        return fail(format!("SPC {sid}: invalid component digits {components:?}"));
                    }
                    parse_nastran_float(field(card, index + 2, "SPC value")?, "SPC value")?;
                    spc_grid_refs.insert(grid);
                }
            }
            "SPC1" => {
                parse_spc1(card, &mut spc_grid_refs, &mut inventory.spc_set_ids)?;
            }
            "FORCE" => {
                let sid = parse_u32(required_text(card, 0, "FORCE SID")?, "FORCE SID")?;
                let grid = parse_u32(required_text(card, 1, "FORCE GRID")?, "FORCE GRID")?;
                let cid = field(card, 2, "FORCE CID")?;
                if !cid.is_empty() {
                    parse_u32(cid, "FORCE CID")?;
                }
                for index in [3usize, 4, 5, 6] {
                    parse_nastran_float(field(card, index, "FORCE numeric")?, "FORCE numeric")?;
                }
                force_set_ids.insert(sid);
                force_grid_refs.insert(grid);
            }
            "LOAD" => {
                let sid = parse_u32(required_text(card, 0, "LOAD SID")?, "LOAD SID")?;
                load_set_ids.insert(sid);
                parse_nastran_float(field(card, 1, "LOAD scale")?, "LOAD scale")?;
                if card.fields.len() < 4 || (card.fields.len() - 2) % 2 != 0 {
                    return fail(format!("LOAD {sid}: expected scale pairs after SID"));
                }
                for index in (2..card.fields.len()).step_by(2) {
                    parse_nastran_float(field(card, index, "LOAD scale factor")?, "LOAD factor")?;
                    load_refs.insert(parse_u32(field(card, index + 1, "LOAD set ID")?, "LOAD set ID")?);
                }
            }
            "PARAM" => {
                let _name = required_text(card, 0, "PARAM name")?;
                let _value = required_text(card, 1, "PARAM value")?;
            }
            _ => {
                inventory.opaque_cards += 1;
            }
        }
    }

    for (pid, mid) in &property_materials {
        if !material_ids.contains(mid) {
            return fail(format!("PBEAM {pid}: referenced MAT1 MID {mid} is missing"));
        }
    }

    let all_grid_refs = beam_grid_refs
        .union(&rbe_grid_refs)
        .copied()
        .chain(spc_grid_refs.iter().copied())
        .chain(force_grid_refs.iter().copied())
        .collect::<BTreeSet<_>>();

    for grid in &all_grid_refs {
        if !grid_ids.contains(grid) {
            return fail(format!("referenced GRID {grid} does not exist"));
        }
    }
    for load_ref in &load_refs {
        if !force_set_ids.contains(load_ref) {
            return fail(format!(
                "LOAD references set {load_ref}, but no FORCE entry with that SID was admitted"
            ));
        }
    }

    // A GRID not referenced by an element, rigid link, constraint or force is
    // reported, not automatically rejected: isolated reference/control grids
    // can be intentional in larger BDF models.
    let connected = beam_grid_refs
        .union(&rbe_grid_refs)
        .copied()
        .chain(spc_grid_refs.iter().copied())
        .chain(force_grid_refs.iter().copied())
        .collect::<BTreeSet<_>>();

    inventory.grid_ids = grid_ids.clone();
    inventory.element_ids = element_ids;
    inventory.property_materials = property_materials;
    inventory.material_ids = material_ids;
    inventory.force_set_ids = force_set_ids;
    inventory.load_set_ids = load_set_ids;
    inventory.orphan_grid_count = grid_ids.difference(&connected).count();
    inventory.claim_ceiling = "syntax_and_topology_inventory_only";
    Ok(inventory)
}

fn json_array(values: impl Iterator<Item = u32>) -> String {
    let values = values.map(|value| value.to_string()).collect::<Vec<_>>();
    format!("[{}]", values.join(","))
}

fn inventory_json(deck: &Deck, inventory: &Inventory) -> String {
    let mut counts = String::from("{");
    for (index, (name, count)) in inventory.card_counts.iter().enumerate() {
        if index > 0 {
            counts.push(',');
        }
        write!(&mut counts, "\"{name}\":{count}").expect("writing to String cannot fail");
    }
    counts.push('}');

    format!(
        concat!(
            "{{",
            "\"schema_version\":\"aero-nastran-bdf-intake/0.1.0\",",
            "\"source_sha256\":\"{}\",",
            "\"source_bytes\":{},",
            "\"card_count\":{},",
            "\"card_counts\":{},",
            "\"grid_ids\":{},",
            "\"cbeam_ids\":{},",
            "\"element_ids\":{},",
            "\"material_ids\":{},",
            "\"property_materials\":{},",
            "\"rbe2_ids\":{},",
            "\"spc_set_ids\":{},",
            "\"force_set_ids\":{},",
            "\"load_set_ids\":{},",
            "\"opaque_cards\":{},",
            "\"orphan_grid_count\":{},",
            "\"units_status\":\"UNKNOWN\",",
            "\"material_allowables_status\":\"UNKNOWN\",",
            "\"solver_result\":\"NOT_ASSESSED\",",
            "\"structural_response\":\"NOT_ASSESSED\",",
            "\"claim_ceiling\":\"syntax_and_topology_inventory_only\"",
            "}}"
        ),
        deck.source_sha256,
        deck.source_bytes,
        deck.cards.len(),
        counts,
        json_array(inventory.grid_ids.iter().copied()),
        json_array(inventory.cbeam_ids.iter().copied()),
        json_array(inventory.element_ids.iter().copied()),
        json_array(inventory.material_ids.iter().copied()),
        property_materials_json(&inventory.property_materials),
        json_array(inventory.rbe2_ids.iter().copied()),
        json_array(inventory.spc_set_ids.iter().copied()),
        json_array(inventory.force_set_ids.iter().copied()),
        json_array(inventory.load_set_ids.iter().copied()),
        inventory.opaque_cards,
        inventory.orphan_grid_count,
    )
}

fn property_materials_json(values: &BTreeMap<u32, u32>) -> String {
    let mut output = String::from("{");
    for (index, (property, material)) in values.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        write!(&mut output, "\"{property}\":{material}")
            .expect("writing to String cannot fail");
    }
    output.push('}');
    output
}

fn main() {
    let digest = sha256_hex(FIXTURE);
    let deck = match parse_deck(FIXTURE, &digest) {
        Ok(deck) => deck,
        Err(error) => {
            eprintln!("bdf_parse_error={error}");
            std::process::exit(2);
        }
    };
    let inventory = match build_inventory(&deck) {
        Ok(inventory) => inventory,
        Err(error) => {
            eprintln!("bdf_topology_error={error}");
            std::process::exit(2);
        }
    };
    println!("{}", inventory_json(&deck, &inventory));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed_line(fields: &[&str]) -> String {
        assert!(fields.len() <= 10);
        let mut output = String::new();
        for index in 0..10 {
            let value = fields.get(index).copied().unwrap_or("");
            assert!(value.len() <= 8, "fixture field too long: {value}");
            output.push_str(value);
            output.push_str(&" ".repeat(8 - value.len()));
        }
        output
    }

    fn deck_text(cards: Vec<String>) -> Vec<u8> {
        let mut lines = vec!["SOL 101".to_owned(), "CEND".to_owned(), "SUBCASE 1".to_owned(), "SPC = 1".to_owned(), "LOAD = 2".to_owned(), "BEGIN BULK".to_owned()];
        lines.extend(cards);
        lines.push("ENDDATA".to_owned());
        lines.join("\n").into_bytes()
    }

    fn parse_bytes(bytes: &[u8]) -> ParseResult<(Deck, Inventory)> {
        let digest = sha256_hex(bytes);
        let deck = parse_deck(bytes, &digest)?;
        let inventory = build_inventory(&deck)?;
        Ok((deck, inventory))
    }

    fn minimal_valid_cards() -> Vec<String> {
        vec![
            fixed_line(&["GRID", "1", "0", "0.", "0.", "0."]),
            fixed_line(&["GRID", "2", "0", "1.", "0.", "0."]),
            fixed_line(&["GRID", "3", "0", "2.", "0.", "0."]),
            fixed_line(&["CBEAM", "101", "20", "1", "2", "0.", "1.", "0."]),
            fixed_line(&["CBEAM", "102", "20", "2", "3", "0.", "1.", "0."]),
            fixed_line(&["MAT1", "10", "7.0E10", "2.7E10", ".3"]),
            fixed_line(&["PBEAM", "20", "10", ".001", ".00001", ".00001", "0.", ".00001", "0."]),
            fixed_line(&["CONM2", "201", "2", "0", "5.", "0.", "0.", "0."]),
            fixed_line(&["RBE2", "301", "1", "123", "2", "3"]),
            fixed_line(&["SPC", "1", "1", "123", "0."]),
            fixed_line(&["FORCE", "2", "3", "0", "1000.", "0.", "1.", "0."]),
            fixed_line(&["LOAD", "2", "1.", "1.", "2"]),
            fixed_line(&["PARAM", "WTMASS", ".00259"]),
            fixed_line(&["DUMMY", "1", "OPAQUE"]),
        ]
    }

    #[test]
    fn synthetic_equivalent_beam_fixture_has_stable_topology() {
        let (deck, inventory) = parse_bytes(FIXTURE).expect("fixture must parse");
        assert_eq!(inventory.grid_ids.len(), 3);
        assert_eq!(inventory.cbeam_ids.len(), 2);
        assert_eq!(inventory.element_ids.len(), 4);
        assert_eq!(inventory.material_ids.len(), 1);
        assert_eq!(inventory.property_materials.get(&20), Some(&10));
        assert_eq!(inventory.rbe2_ids.len(), 1);
        assert_eq!(inventory.spc_set_ids, BTreeSet::from([1]));
        assert_eq!(inventory.force_set_ids, BTreeSet::from([2]));
        assert_eq!(inventory.load_set_ids, BTreeSet::from([2]));
        assert_eq!(inventory.opaque_cards, 1);
        assert_eq!(inventory.orphan_grid_count, 0);
        assert_eq!(deck.cards.iter().filter(|card| card.parse_state == "opaque").count(), 1);
    }

    #[test]
    fn duplicate_grid_id_is_rejected() {
        let mut cards = minimal_valid_cards();
        cards.push(fixed_line(&["GRID", "1", "0", "9.", "9.", "9."]));
        let bytes = deck_text(cards);
        assert!(parse_bytes(&bytes).is_err());
    }

    #[test]
    fn missing_cbeam_endpoint_is_rejected() {
        let mut cards = minimal_valid_cards();
        cards[3] = fixed_line(&["CBEAM", "101", "20", "1", "99", "0.", "1.", "0."]);
        let bytes = deck_text(cards);
        assert!(parse_bytes(&bytes).is_err());
    }

    #[test]
    fn missing_pbeam_material_is_rejected() {
        let mut cards = minimal_valid_cards();
        cards[6] = fixed_line(&["PBEAM", "20", "999", ".001", ".00001", ".00001", "0.", ".00001", "0."]);
        let bytes = deck_text(cards);
        assert!(parse_bytes(&bytes).is_err());
    }

    #[test]
    fn rbe2_missing_dependent_grid_is_rejected() {
        let mut cards = minimal_valid_cards();
        cards[8] = fixed_line(&["RBE2", "301", "1", "123", "2", "99"]);
        let bytes = deck_text(cards);
        assert!(parse_bytes(&bytes).is_err());
    }

    #[test]
    fn mismatched_continuation_label_is_rejected() {
        let mut cards = minimal_valid_cards();
        cards[8] = fixed_line(&["RBE2", "301", "1", "123", "2", "", "", "", "", "+A"]);
        cards.insert(9, fixed_line(&["+B", "3"]));
        let bytes = deck_text(cards);
        assert!(parse_bytes(&bytes).is_err());
    }

    #[test]
    fn stale_source_digest_is_rejected() {
        let bytes = deck_text(minimal_valid_cards());
        let wrong = "0".repeat(64);
        assert!(parse_deck(&bytes, &wrong).is_err());
    }

    #[test]
    fn non_finite_coordinate_is_rejected() {
        let mut cards = minimal_valid_cards();
        cards[0] = fixed_line(&["GRID", "1", "0", "NaN", "0.", "0."]);
        let bytes = deck_text(cards);
        assert!(parse_bytes(&bytes).is_err());
    }

    #[test]
    fn explicit_continuation_is_joined_without_losing_raw_provenance() {
        let mut cards = minimal_valid_cards();
        cards[8] = fixed_line(&["RBE2", "301", "1", "123", "2", "", "", "", "", "+A"]);
        cards.insert(9, fixed_line(&["+A", "3"]));
        let bytes = deck_text(cards);
        let digest = sha256_hex(&bytes);
        let deck = parse_deck(&bytes, &digest).expect("continuation must parse");
        let rbe2 = deck.cards.iter().find(|card| card.name == "RBE2").unwrap();
        assert_eq!(rbe2.raw_lines.len(), 2);
        assert_eq!(rbe2.fields[3], "2");
        assert_eq!(rbe2.fields[8], "3");
        assert_eq!(build_inventory(&deck).unwrap().rbe2_ids.len(), 1);
    }

    #[test]
    fn implicit_nastran_exponent_is_parsed_finitely() {
        let value = parse_nastran_float("1.25-3", "test exponent").unwrap();
        assert!((value - 0.00125).abs() < 1.0e-15);
    }

    #[test]
    fn large_field_and_overwide_lines_fail_closed() {
        let bytes = deck_text(vec![fixed_line(&["GRID*", "1", "0", "0.", "0.", "0."])]);
        let digest = sha256_hex(&bytes);
        assert!(parse_deck(&bytes, &digest).is_err());

        let too_wide = format!("{}X", fixed_line(&["GRID", "1"]));
        let bytes = deck_text(vec![too_wide]);
        assert!(parse_bytes(&bytes).is_err());
    }

    #[test]
    fn spc1_thru_range_resolves_every_grid() {
        let card = Card {
            name: "SPC1".to_owned(),
            fields: vec!["1".to_owned(), "123".to_owned(), "1".to_owned(), "THRU".to_owned(), "3".to_owned()],
            raw_lines: Vec::new(),
            parse_state: "parsed",
        };
        let mut refs = BTreeSet::new();
        let mut sets = BTreeSet::new();
        parse_spc1(&card, &mut refs, &mut sets).unwrap();
        assert_eq!(refs, BTreeSet::from([1, 2, 3]));
        assert_eq!(sets, BTreeSet::from([1]));
    }
}
