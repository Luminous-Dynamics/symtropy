use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq)]
enum Json {
    Null,
    Bool,
    Number(String),
    String(String),
    Array(Vec<Json>),
    Object(BTreeMap<String, Json>),
}

struct Parser<'a> {
    input: &'a str,
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, bytes: input.as_bytes(), pos: 0 }
    }

    fn error(&self, message: &str) -> String {
        format!("JSON byte {}: {}", self.pos, message)
    }

    fn whitespace(&mut self) {
        while self.pos < self.bytes.len()
            && matches!(self.bytes[self.pos], b' ' | b'\n' | b'\r' | b'\t')
        {
            self.pos += 1;
        }
    }

    fn parse(mut self) -> Result<Json, String> {
        self.whitespace();
        let value = self.value()?;
        self.whitespace();
        if self.pos != self.bytes.len() {
            return Err(self.error("trailing content"));
        }
        Ok(value)
    }

    fn value(&mut self) -> Result<Json, String> {
        self.whitespace();
        match self.bytes.get(self.pos).copied() {
            Some(b'n') => { self.literal(b"null")?; Ok(Json::Null) }
            Some(b't') => { self.literal(b"true")?; Ok(Json::Bool) }
            Some(b'f') => { self.literal(b"false")?; Ok(Json::Bool) }
            Some(b'"') => Ok(Json::String(self.string()?)),
            Some(b'[') => self.array(),
            Some(b'{') => self.object(),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => Err(self.error("unexpected token")),
            None => Err(self.error("unexpected end of input")),
        }
    }

    fn literal(&mut self, literal: &[u8]) -> Result<(), String> {
        if self.bytes.get(self.pos..self.pos + literal.len()) == Some(literal) {
            self.pos += literal.len();
            Ok(())
        } else {
            Err(self.error("invalid literal"))
        }
    }

    fn string(&mut self) -> Result<String, String> {
        if self.bytes.get(self.pos) != Some(&b'"') {
            return Err(self.error("expected string"));
        }
        self.pos += 1;
        let mut output = String::new();
        while self.pos < self.bytes.len() {
            match self.bytes[self.pos] {
                b'"' => {
                    self.pos += 1;
                    return Ok(output);
                }
                b'\\' => {
                    self.pos += 1;
                    let escape = *self.bytes.get(self.pos).ok_or_else(|| self.error("incomplete escape"))?;
                    self.pos += 1;
                    match escape {
                        b'"' => output.push('"'),
                        b'\\' => output.push('\\'),
                        b'/' => output.push('/'),
                        b'b' => output.push('\u{0008}'),
                        b'f' => output.push('\u{000c}'),
                        b'n' => output.push('\n'),
                        b'r' => output.push('\r'),
                        b't' => output.push('\t'),
                        b'u' => {
                            let first = self.hex4()?;
                            let scalar = if (0xD800..=0xDBFF).contains(&first) {
                                if self.bytes.get(self.pos..self.pos + 2) != Some(b"\\u") {
                                    return Err(self.error("high surrogate without low surrogate"));
                                }
                                self.pos += 2;
                                let second = self.hex4()?;
                                if !(0xDC00..=0xDFFF).contains(&second) {
                                    return Err(self.error("invalid low surrogate"));
                                }
                                0x10000 + ((u32::from(first) - 0xD800) << 10) + (u32::from(second) - 0xDC00)
                            } else if (0xDC00..=0xDFFF).contains(&first) {
                                return Err(self.error("unpaired low surrogate"));
                            } else {
                                u32::from(first)
                            };
                            output.push(char::from_u32(scalar).ok_or_else(|| self.error("invalid Unicode scalar"))?);
                        }
                        _ => return Err(self.error("invalid escape")),
                    }
                }
                byte if byte < 0x20 => return Err(self.error("unescaped control character")),
                _ => {
                    let ch = self.input[self.pos..].chars().next().ok_or_else(|| self.error("invalid UTF-8"))?;
                    output.push(ch);
                    self.pos += ch.len_utf8();
                }
            }
        }
        Err(self.error("unterminated string"))
    }

    fn hex4(&mut self) -> Result<u16, String> {
        let end = self.pos.checked_add(4).ok_or_else(|| self.error("Unicode escape overflow"))?;
        let slice = self.bytes.get(self.pos..end).ok_or_else(|| self.error("short Unicode escape"))?;
        let mut value = 0u16;
        for byte in slice {
            let digit = match byte {
                b'0'..=b'9' => u16::from(*byte - b'0'),
                b'a'..=b'f' => u16::from(*byte - b'a' + 10),
                b'A'..=b'F' => u16::from(*byte - b'A' + 10),
                _ => return Err(self.error("invalid hex digit in Unicode escape")),
            };
            value = (value << 4) | digit;
        }
        self.pos = end;
        Ok(value)
    }

    fn array(&mut self) -> Result<Json, String> {
        self.pos += 1;
        self.whitespace();
        let mut values = Vec::new();
        if self.bytes.get(self.pos) == Some(&b']') {
            self.pos += 1;
            return Ok(Json::Array(values));
        }
        loop {
            values.push(self.value()?);
            self.whitespace();
            match self.bytes.get(self.pos) {
                Some(b',') => { self.pos += 1; self.whitespace(); }
                Some(b']') => { self.pos += 1; return Ok(Json::Array(values)); }
                _ => return Err(self.error("expected ',' or ']'")),
            }
        }
    }

    fn object(&mut self) -> Result<Json, String> {
        self.pos += 1;
        self.whitespace();
        let mut values = BTreeMap::new();
        if self.bytes.get(self.pos) == Some(&b'}') {
            self.pos += 1;
            return Ok(Json::Object(values));
        }
        loop {
            self.whitespace();
            if self.bytes.get(self.pos) != Some(&b'"') {
                return Err(self.error("object key must be a string"));
            }
            let key = self.string()?;
            self.whitespace();
            if self.bytes.get(self.pos) != Some(&b':') {
                return Err(self.error("expected ':' after object key"));
            }
            self.pos += 1;
            let value = self.value()?;
            if values.insert(key.clone(), value).is_some() {
                return Err(self.error(&format!("duplicate object key '{}'", key)));
            }
            self.whitespace();
            match self.bytes.get(self.pos) {
                Some(b',') => { self.pos += 1; self.whitespace(); }
                Some(b'}') => { self.pos += 1; return Ok(Json::Object(values)); }
                _ => return Err(self.error("expected ',' or '}'")),
            }
        }
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.pos;
        if self.bytes.get(self.pos) == Some(&b'-') { self.pos += 1; }
        match self.bytes.get(self.pos) {
            Some(b'0') => {
                self.pos += 1;
                if matches!(self.bytes.get(self.pos), Some(b'0'..=b'9')) {
                    return Err(self.error("leading zero in number"));
                }
            }
            Some(b'1'..=b'9') => {
                self.pos += 1;
                while matches!(self.bytes.get(self.pos), Some(b'0'..=b'9')) { self.pos += 1; }
            }
            _ => return Err(self.error("invalid number")),
        }
        if self.bytes.get(self.pos) == Some(&b'.') {
            self.pos += 1;
            let fractional_start = self.pos;
            while matches!(self.bytes.get(self.pos), Some(b'0'..=b'9')) { self.pos += 1; }
            if self.pos == fractional_start { return Err(self.error("fraction requires a digit")); }
        }
        if matches!(self.bytes.get(self.pos), Some(b'e' | b'E')) {
            self.pos += 1;
            if matches!(self.bytes.get(self.pos), Some(b'+' | b'-')) { self.pos += 1; }
            let exponent_start = self.pos;
            while matches!(self.bytes.get(self.pos), Some(b'0'..=b'9')) { self.pos += 1; }
            if self.pos == exponent_start { return Err(self.error("exponent requires a digit")); }
        }
        Ok(Json::Number(self.input[start..self.pos].to_owned()))
    }
}

fn object(value: &Json) -> Option<&BTreeMap<String, Json>> {
    if let Json::Object(map) = value { Some(map) } else { None }
}
fn array(value: &Json) -> Option<&[Json]> {
    if let Json::Array(items) = value { Some(items) } else { None }
}
fn string(value: &Json) -> Option<&str> {
    if let Json::String(text) = value { Some(text) } else { None }
}
fn field<'a>(value: &'a Json, name: &str) -> Option<&'a Json> {
    object(value)?.get(name)
}
fn required<'a>(value: &'a Json, name: &str, path: &str, errors: &mut Vec<String>) -> Option<&'a Json> {
    match field(value, name) {
        Some(v) => Some(v),
        None => {
            errors.push(format!("{}: missing required field '{}'", path, name));
            None
        }
    }
}
fn string_field<'a>(value: &'a Json, name: &str, path: &str, errors: &mut Vec<String>) -> Option<&'a str> {
    match required(value, name, path, errors).and_then(string) {
        Some(s) => Some(s),
        None => {
            if field(value, name).is_some() {
                errors.push(format!("{}.{}: expected string", path, name));
            }
            None
        }
    }
}
fn is_integer(value: &Json) -> Option<i128> {
    match value {
        Json::Number(raw) if !raw.contains('.') && !raw.contains('e') && !raw.contains('E') => raw.parse::<i128>().ok(),
        _ => None,
    }
}
fn is_number(value: &Json) -> Option<f64> {
    match value {
        Json::Number(raw) => raw.parse::<f64>().ok().filter(|n| n.is_finite()),
        _ => None,
    }
}
fn check_allowed(value: &Json, allowed: &[&str], path: &str, errors: &mut Vec<String>) {
    if let Some(map) = object(value) {
        for key in map.keys() {
            if !allowed.contains(&key.as_str()) {
                errors.push(format!("{}.{}: unknown field", path, key));
            }
        }
    }
}
fn expect_object<'a>(value: Option<&'a Json>, path: &str, errors: &mut Vec<String>) -> Option<&'a Json> {
    match value {
        Some(v) if object(v).is_some() => Some(v),
        Some(_) => { errors.push(format!("{}: expected object", path)); None }
        None => None,
    }
}
fn expect_array<'a>(value: Option<&'a Json>, path: &str, errors: &mut Vec<String>) -> Option<&'a [Json]> {
    match value {
        Some(v) => match array(v) {
            Some(items) => Some(items),
            None => { errors.push(format!("{}: expected array", path)); None }
        },
        None => None,
    }
}
fn check_enum(value: &Json, values: &[&str], path: &str, errors: &mut Vec<String>) -> Option<String> {
    match string(value) {
        Some(s) if values.contains(&s) => Some(s.to_owned()),
        Some(s) => { errors.push(format!("{}: unsupported value '{}'", path, s)); None }
        None => { errors.push(format!("{}: expected string enum", path)); None }
    }
}
fn check_string_array(value: &Json, path: &str, errors: &mut Vec<String>) {
    if let Some(items) = array(value) {
        for (i, item) in items.iter().enumerate() {
            if string(item).is_none() {
                errors.push(format!("{}[{}]: expected string", path, i));
            }
        }
    } else {
        errors.push(format!("{}: expected array", path));
    }
}
fn check_hash(value: &Json, path: &str, errors: &mut Vec<String>) {
    match string(value) {
        Some(s) if s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()) => {}
        Some(_) => errors.push(format!("{}: expected 64 hexadecimal characters", path)),
        None => errors.push(format!("{}: expected digest string", path)),
    }
}
fn check_revision(value: &Json, path: &str, errors: &mut Vec<String>) {
    match string(value) {
        Some(s) if (s.len() == 40 || s.len() == 64) && s.bytes().all(|b| b.is_ascii_hexdigit()) => {}
        Some(_) => errors.push(format!("{}: expected 40- or 64-character hexadecimal revision", path)),
        None => errors.push(format!("{}: expected revision string", path)),
    }
}
fn positive_integer_field(value: &Json, key: &str, path: &str, zero_ok: bool, errors: &mut Vec<String>) {
    match field(value, key).and_then(is_integer) {
        Some(n) if (if zero_ok { n >= 0 } else { n > 0 }) => {}
        Some(_) => errors.push(format!("{}.{}: integer is outside permitted range", path, key)),
        None => errors.push(format!("{}.{}: expected integer", path, key)),
    }
}
fn check_string_field_nonempty(value: &Json, key: &str, path: &str, errors: &mut Vec<String>) {
    match string_field(value, key, path, errors) {
        Some(s) if !s.trim().is_empty() => {}
        Some(_) => errors.push(format!("{}.{}: must not be empty", path, key)),
        None => {}
    }
}

pub fn validate_json_syntax(input: &str) -> Result<(), String> {
    Parser::new(input).parse().map(|_| ())
}

pub fn validate_manifest_json(input: &str) -> Vec<String> {
    let root = match Parser::new(input).parse() {
        Ok(v) => v,
        Err(e) => return vec![e],
    };
    let mut errors = Vec::new();
    if object(&root).is_none() {
        return vec!["$: expected object".to_owned()];
    }
    check_allowed(&root, &["schema_version","benchmark_id","title","claim_class","scope","source","generator","protocol","candidate","baselines","oracle","metrics","notes"], "$", &mut errors);
    if string_field(&root, "schema_version", "$", &mut errors) != Some("shipyard-benchmark-manifest/v0") {
        errors.push("$.schema_version: unsupported schema version".to_owned());
    }
    if let Some(id) = string_field(&root, "benchmark_id", "$", &mut errors) {
        let chars: Vec<char> = id.chars().collect();
        if chars.len() < 3 || chars.len() > 128
            || (!chars[0].is_ascii_lowercase() && !chars[0].is_ascii_digit())
            || chars.iter().any(|c| !(c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-')))
        {
            errors.push("$.benchmark_id: invalid identifier".to_owned());
        }
    }
    check_string_field_nonempty(&root, "title", "$", &mut errors);
    if let Some(v) = field(&root, "claim_class") {
        check_enum(v, &["semantic-correctness","algorithmic-quality","operational-performance","industrial-validation"], "$.claim_class", &mut errors);
    } else {
        errors.push("$.claim_class: missing required field".to_owned());
    }

    if let Some(scope) = expect_object(field(&root,"scope"), "$.scope", &mut errors) {
        check_allowed(scope, &["models","supported_constraints","unsupported_constraints"], "$.scope", &mut errors);
        for key in ["models","supported_constraints","unsupported_constraints"] {
            if let Some(v) = field(scope,key) {
                if key == "models" {
                    if let Some(items) = expect_array(Some(v), "$.scope.models", &mut errors) {
                        if items.is_empty() { errors.push("$.scope.models: must not be empty".to_owned()); }
                        let mut seen = BTreeSet::new();
                        for (i,item) in items.iter().enumerate() {
                            if let Some(m) = check_enum(item, &["rectangular-space-time","spatial-transfers","unary-resources","execution-replay","irregular-polygons","multi-crane-physics"], &format!("$.scope.models[{}]",i), &mut errors) {
                                if !seen.insert(m) { errors.push(format!("$.scope.models[{}]: duplicate model",i)); }
                            }
                        }
                    }
                } else { check_string_array(v, &format!("$.scope.{}",key), &mut errors); }
            } else { errors.push(format!("$.scope.{}: missing required field",key)); }
        }
    } else if field(&root,"scope").is_none() {
        errors.push("$.scope: missing required field".to_owned());
    }

    if let Some(source) = expect_object(field(&root,"source"), "$.source", &mut errors) {
        check_allowed(source, &["kind","uri","sha256","case_count","license_or_access_basis","notes"], "$.source", &mut errors);
        if let Some(v) = field(source,"kind") {
            check_enum(v, &["generated","published","permissioned-industrial","hand-authored"], "$.source.kind", &mut errors);
            if string(v) == Some("generated") && field(&root,"generator").is_none() {
                errors.push("$.generator: required when source.kind is generated".to_owned());
            }
        } else { errors.push("$.source.kind: missing required field".to_owned()); }
        check_string_field_nonempty(source,"uri","$.source",&mut errors);
        if let Some(v) = required(source,"sha256","$.source",&mut errors) { check_hash(v,"$.source.sha256",&mut errors); }
        positive_integer_field(source,"case_count","$.source",false,&mut errors);
        if let Some(v) = field(source,"license_or_access_basis") {
            if string(v).map(|s|!s.trim().is_empty()).unwrap_or(false) {} else {
                errors.push("$.source.license_or_access_basis: expected non-empty string".to_owned());
            }
        }
    } else if field(&root,"source").is_none() {
        errors.push("$.source: missing required field".to_owned());
    }

    if let Some(generator) = field(&root,"generator") {
        if let Some(generator) = expect_object(Some(generator),"$.generator",&mut errors) {
            check_allowed(generator,&["name","version","seed","parameters_sha256"],"$.generator",&mut errors);
            check_string_field_nonempty(generator,"name","$.generator",&mut errors);
            check_string_field_nonempty(generator,"version","$.generator",&mut errors);
            positive_integer_field(generator,"seed","$.generator",true,&mut errors);
            if let Some(v) = required(generator,"parameters_sha256","$.generator",&mut errors) { check_hash(v,"$.generator.parameters_sha256",&mut errors); }
            if field(&root,"source").and_then(|s|field(s,"kind")).and_then(string) != Some("generated") {
                errors.push("$.generator: only allowed for source.kind 'generated'".to_owned());
            }
        }
    }

    if let Some(protocol) = expect_object(field(&root,"protocol"),"$.protocol",&mut errors) {
        check_allowed(protocol,&["hardware_class","wall_clock_budget_ms","candidate_evaluation_budget","repetitions","warmup_runs","random_seed","timeout_policy","notes"],"$.protocol",&mut errors);
        check_string_field_nonempty(protocol,"hardware_class","$.protocol",&mut errors);
        for (key,zero_ok) in [("wall_clock_budget_ms",false),("candidate_evaluation_budget",false),("repetitions",false),("warmup_runs",true),("random_seed",true)] {
            positive_integer_field(protocol,key,"$.protocol",zero_ok,&mut errors);
        }
        if let Some(v)=field(protocol,"timeout_policy") {
            check_enum(v,&["count-as-no-solution","censored-runtime","invalidate-run"],"$.protocol.timeout_policy",&mut errors);
        } else { errors.push("$.protocol.timeout_policy: missing required field".to_owned()); }
    } else if field(&root,"protocol").is_none() {
        errors.push("$.protocol: missing required field".to_owned());
    }

    if let Some(v) = expect_object(field(&root,"candidate"),"$.candidate",&mut errors) {
        check_allowed(v,&["name","revision","config_sha256"],"$.candidate",&mut errors);
        check_string_field_nonempty(v,"name","$.candidate",&mut errors);
        if let Some(rev)=required(v,"revision","$.candidate",&mut errors) { check_revision(rev,"$.candidate.revision",&mut errors); }
        if let Some(hash)=required(v,"config_sha256","$.candidate",&mut errors) { check_hash(hash,"$.candidate.config_sha256",&mut errors); }
    } else if field(&root,"candidate").is_none() { errors.push("$.candidate: missing required field".to_owned()); }

    if let Some(baselines) = expect_array(field(&root,"baselines"),"$.baselines",&mut errors) {
        if baselines.is_empty() { errors.push("$.baselines: at least one baseline required".to_owned()); }
        let mut names = BTreeSet::new();
        for (i,b) in baselines.iter().enumerate() {
            let path=format!("$.baselines[{}]",i);
            if let Some(b)=expect_object(Some(b),&path,&mut errors) {
                check_allowed(b,&["name","revision","config_sha256"],&path,&mut errors);
                if let Some(name)=string_field(b,"name",&path,&mut errors) {
                    if name.trim().is_empty() { errors.push(format!("{}.name: must not be empty",path)); }
                    if !names.insert(name.to_owned()) { errors.push(format!("{}.name: duplicate baseline name",path)); }
                }
                if let Some(rev)=required(b,"revision",&path,&mut errors) { check_revision(rev,&format!("{}.revision",path),&mut errors); }
                if let Some(hash)=required(b,"config_sha256",&path,&mut errors) { check_hash(hash,&format!("{}.config_sha256",path),&mut errors); }
            }
        }
    } else if field(&root,"baselines").is_none() { errors.push("$.baselines: missing required field".to_owned()); }

    if let Some(oracle)=expect_object(field(&root,"oracle"),"$.oracle",&mut errors) {
        check_allowed(oracle,&["kind","implementation_id","revision","source_sha256","limitations"],"$.oracle",&mut errors);
        if let Some(v)=field(oracle,"kind") { check_enum(v,&["exact-enumeration","independent-verifier","published-reference","industrial-execution-trace","none"],"$.oracle.kind",&mut errors); }
        else { errors.push("$.oracle.kind: missing required field".to_owned()); }
        check_string_field_nonempty(oracle,"implementation_id","$.oracle",&mut errors);
        if let Some(v)=field(oracle,"source_sha256") { check_hash(v,"$.oracle.source_sha256",&mut errors); }
        if let Some(v)=field(oracle,"limitations") { check_string_array(v,"$.oracle.limitations",&mut errors); }
    } else if field(&root,"oracle").is_none() { errors.push("$.oracle: missing required field".to_owned()); }

    if let Some(metrics)=expect_array(field(&root,"metrics"),"$.metrics",&mut errors) {
        if metrics.is_empty() { errors.push("$.metrics: at least one metric required".to_owned()); }
        let allowed=["false_feasible_count","false_infeasible_count","feasible_cases_found","feasible_case_recall","area_time_utilization","vehicle_travel_time","makespan","tardiness","recovery_horizon","unaffected_plan_churn","reservation_changes","rehandles","objective_regret","runtime_ms","peak_memory_bytes","timeout_count","bound_gap","start_deviation_ticks","completion_deviation_ticks","missing_observation_count"];
        let mut seen=BTreeSet::new();
        for (i,m) in metrics.iter().enumerate() {
            if let Some(name)=check_enum(m,&allowed,&format!("$.metrics[{}]",i),&mut errors) {
                if !seen.insert(name) { errors.push(format!("$.metrics[{}]: duplicate metric",i)); }
            }
        }
    } else if field(&root,"metrics").is_none() { errors.push("$.metrics: missing required field".to_owned()); }
    errors.sort();
    errors
}


fn sha256_hex(input: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
        0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
        0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
        0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
        0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
        0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
        0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
        0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2,
    ];
    let mut data = input.to_vec();
    let bit_len = (input.len() as u64).wrapping_mul(8);
    data.push(0x80);
    while data.len() % 64 != 56 { data.push(0); }
    data.extend_from_slice(&bit_len.to_be_bytes());

    let mut h = [
        0x6a09e667u32,0xbb67ae85,0x3c6ef372,0xa54ff53a,
        0x510e527f,0x9b05688c,0x1f83d9ab,0x5be0cd19,
    ];
    for chunk in data.chunks_exact(64) {
        let mut w = [0u32;64];
        for (i, word) in chunk.chunks_exact(4).take(16).enumerate() {
            w[i] = u32::from_be_bytes([word[0],word[1],word[2],word[3]]);
        }
        for i in 16..64 {
            let x=w[i-15];
            let y=w[i-2];
            let s0=x.rotate_right(7)^x.rotate_right(18)^(x>>3);
            let s1=y.rotate_right(17)^y.rotate_right(19)^(y>>10);
            w[i]=w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1);
        }
        let (mut a,mut b,mut c,mut d,mut e,mut f,mut g,mut z)=(h[0],h[1],h[2],h[3],h[4],h[5],h[6],h[7]);
        for i in 0..64 {
            let s1=e.rotate_right(6)^e.rotate_right(11)^e.rotate_right(25);
            let ch=(e&f)^(!e&g);
            let t1=z.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0=a.rotate_right(2)^a.rotate_right(13)^a.rotate_right(22);
            let maj=(a&b)^(a&c)^(b&c);
            let t2=s0.wrapping_add(maj);
            z=g; g=f; f=e; e=d.wrapping_add(t1);
            d=c; c=b; b=a; a=t1.wrapping_add(t2);
        }
        for (slot, add) in h.iter_mut().zip([a,b,c,d,e,f,g,z]) {
            *slot=slot.wrapping_add(add);
        }
    }
    let mut output=String::with_capacity(64);
    for word in h { output.push_str(&format!("{:08x}",word)); }
    output
}

pub fn audit_report_json(manifest_input: &str, report_input: &str) -> Vec<String> {
    let manifest = match Parser::new(manifest_input).parse() {
        Ok(v) => v,
        Err(e) => return vec![format!("manifest: {}",e)],
    };
    let report = match Parser::new(report_input).parse() {
        Ok(v) => v,
        Err(e) => return vec![format!("report: {}",e)],
    };
    let mut errors=validate_manifest_json(manifest_input);
    if object(&report).is_none() {
        errors.push("$report: expected object".to_owned());
        errors.sort();
        return errors;
    }
    check_allowed(&report,&["schema_version","benchmark_id","manifest_sha256","status","run_metadata","case_results","summary","evidence"],"$report",&mut errors);
    if string_field(&report,"schema_version","$report",&mut errors) != Some("shipyard-benchmark-report/v0") {
        errors.push("$report.schema_version: unsupported schema version".to_owned());
    }
    if let Some(report_id)=string_field(&report,"benchmark_id","$report",&mut errors) {
        if field(&manifest,"benchmark_id").and_then(string) != Some(report_id) {
            errors.push("$report.benchmark_id: does not match manifest".to_owned());
        }
    }
    if let Some(v)=required(&report,"manifest_sha256","$report",&mut errors) {
        check_hash(v,"$report.manifest_sha256",&mut errors);
        let expected=sha256_hex(manifest_input.as_bytes());
        if string(v).map(|s|s.to_ascii_lowercase()) != Some(expected) {
            errors.push("$report.manifest_sha256: does not match exact manifest bytes".to_owned());
        }
    }
    if let Some(v)=field(&report,"status") { check_enum(v,&["complete","incomplete","invalid"],"$report.status",&mut errors); }
    else { errors.push("$report.status: missing required field".to_owned()); }

    if let Some(meta)=expect_object(field(&report,"run_metadata"),"$report.run_metadata",&mut errors) {
        check_allowed(meta,&["run_id","started_at_utc","git_commit_sha","executable_sha256","hardware_class","rustc_version","os","kernel","notes"],"$report.run_metadata",&mut errors);
        for key in ["git_commit_sha","executable_sha256","hardware_class","rustc_version"] {
            if let Some(v)=required(meta,key,"$report.run_metadata",&mut errors) {
                if key=="git_commit_sha" { check_revision(v,"$report.run_metadata.git_commit_sha",&mut errors); }
                else if key=="executable_sha256" { check_hash(v,"$report.run_metadata.executable_sha256",&mut errors); }
                else if !string(v).map(|s|!s.trim().is_empty()).unwrap_or(false) {
                    errors.push(format!("$report.run_metadata.{}: expected non-empty string",key));
                }
            }
        }
        if let (Some(run_revision),Some(candidate_revision)) = (
            field(meta,"git_commit_sha").and_then(string),
            field(&manifest,"candidate").and_then(|v|field(v,"revision")).and_then(string),
        ) {
            if !run_revision.eq_ignore_ascii_case(candidate_revision) {
                errors.push("$report.run_metadata.git_commit_sha: does not match manifest candidate revision".to_owned());
            }
        }
    } else if field(&report,"run_metadata").is_none() { errors.push("$report.run_metadata: missing required field".to_owned()); }

    let mut ids=BTreeSet::new();
    let mut feasible_claims=0usize;
    let mut infeasibility_proofs=0usize;
    let mut no_solution=0usize;
    let mut unknown=0usize;
    let mut timeouts=0usize;
    let mut invalid_outputs=0usize;
    let mut not_run=0usize;
    let mut feasible_reference_cases_found=0usize;
    let mut feasible_reference_cases_missed=0usize;
    let mut false_feasible=0usize;
    let mut false_infeasible=0usize;
    let cases=expect_array(field(&report,"case_results"),"$report.case_results",&mut errors);
    if let Some(cases)=cases {
        for (i,case) in cases.iter().enumerate() {
            let path=format!("$report.case_results[{}]",i);
            if let Some(case)=expect_object(Some(case),&path,&mut errors) {
                check_allowed(case,&["case_id","outcome","runtime_ms","peak_memory_bytes","metrics","verifier","notes"],&path,&mut errors);
                if let Some(id)=string_field(case,"case_id",&path,&mut errors) {
                    if id.trim().is_empty() { errors.push(format!("{}.case_id: must not be empty",path)); }
                    if !ids.insert(id.to_owned()) { errors.push(format!("{}.case_id: duplicate case identifier",path)); }
                }
                if let Some(v)=required(case,"runtime_ms",&path,&mut errors) {
                    if is_number(v).map(|n|n>=0.0).unwrap_or(false) {} else {
                        errors.push(format!("{}.runtime_ms: expected non-negative finite number",path));
                    }
                }
                if let Some(v)=field(case,"peak_memory_bytes") {
                    if is_integer(v).map(|n|n>=0).unwrap_or(false) {} else { errors.push(format!("{}.peak_memory_bytes: expected non-negative integer",path)); }
                }
                if let Some(metrics)=field(case,"metrics") {
                    if let Some(map)=object(metrics) {
                        for (k,v) in map {
                            if !matches!(v,Json::Null) && is_number(v).is_none() {
                                errors.push(format!("{}.metrics.{}: expected numeric or null metric",path,k));
                            }
                        }
                    } else { errors.push(format!("{}.metrics: expected object",path)); }
                }
                let outcome=if let Some(v)=field(case,"outcome") {
                    check_enum(v,&["feasible-claim","infeasibility-proof","no-solution","unknown","timeout","invalid-output","not-run"],&format!("{}.outcome",path),&mut errors)
                } else {
                    errors.push(format!("{}.outcome: missing required field",path)); None
                };
                match outcome.as_deref() {
                    Some("feasible-claim")=>feasible_claims+=1,
                    Some("infeasibility-proof")=>infeasibility_proofs+=1,
                    Some("no-solution")=>no_solution+=1,
                    Some("unknown")=>unknown+=1,
                    Some("timeout")=>timeouts+=1,
                    Some("invalid-output")=>invalid_outputs+=1,
                    Some("not-run")=>not_run+=1,
                    _=>{}
                }

                if let Some(verifier)=expect_object(field(case,"verifier"),&format!("{}.verifier",path),&mut errors) {
                    check_allowed(verifier,&["status","oracle_outcome","reference_outcome","violation_count","oracle_id","evidence_sha256"],&format!("{}.verifier",path),&mut errors);
                    let status=field(verifier,"status").and_then(|v|check_enum(v,&["passed","failed","not-run","not-applicable"],&format!("{}.verifier.status",path),&mut errors));
                    let oracle_outcome=field(verifier,"oracle_outcome").and_then(|v|check_enum(v,&["match","mismatch","unknown","not-applicable"],&format!("{}.verifier.oracle_outcome",path),&mut errors));
                    let reference=field(verifier,"reference_outcome").and_then(|v|check_enum(v,&["feasible","infeasible","unknown","not-applicable"],&format!("{}.verifier.reference_outcome",path),&mut errors));
                    if let Some(v)=required(verifier,"violation_count",&format!("{}.verifier",path),&mut errors) {
                        if is_integer(v).map(|n|n>=0).unwrap_or(false) {} else { errors.push(format!("{}.verifier.violation_count: expected non-negative integer",path)); }
                    }
                    if let Some(v)=field(verifier,"evidence_sha256") { check_hash(v,&format!("{}.verifier.evidence_sha256",path),&mut errors); }
                    if let Some(s)=outcome.as_deref() {
                        let reference_label=reference.as_deref();
                        let oracle_label=oracle_outcome.as_deref();
                        let expected_status=match (s,reference_label,oracle_label) {
                            ("feasible-claim",Some("feasible"),Some("match")) => Some("passed"),
                            ("infeasibility-proof",Some("infeasible"),Some("match")) => Some("passed"),
                            ("feasible-claim",Some("infeasible"),Some("mismatch")) => Some("failed"),
                            ("infeasibility-proof",Some("feasible"),Some("mismatch")) => Some("failed"),
                            ("feasible-claim",Some("unknown"|"not-applicable"),Some("unknown"|"not-applicable")) => Some("passed"),
                            ("no-solution"|"unknown"|"timeout"|"invalid-output"|"not-run",_,_) => None,
                            _ => None,
                        };
                        if let Some(expected)=expected_status {
                            if status.as_deref()!=Some(expected) {
                                errors.push(format!("{}.verifier.status: expected '{}' for the recorded candidate/reference outcome",path,expected));
                            }
                        } else if matches!(s,"no-solution"|"unknown"|"timeout"|"invalid-output"|"not-run") && status.as_deref()==Some("passed") {
                            errors.push(format!("{}.verifier.status: outcome '{}' is inconsistent with passed classification",path,s));
                        }
                        if s=="infeasibility-proof" {
                            let oracle_proves_infeasible = reference_label==Some("infeasible") && oracle_label==Some("match");
                            let explicitly_measured_false_infeasible = reference_label==Some("feasible") && oracle_label==Some("mismatch");
                            if !oracle_proves_infeasible && !explicitly_measured_false_infeasible {
                                errors.push(format!("{}.outcome: infeasibility cannot be inferred from heuristic exhaustion, timeout, or an unknown reference",path));
                            }
                        }
                    }
                    match (outcome.as_deref(),reference.as_deref(),oracle_outcome.as_deref()) {
                        (Some("feasible-claim"),Some("infeasible"),Some("mismatch"))=>false_feasible+=1,
                        (Some("infeasibility-proof"),Some("feasible"),Some("mismatch"))=>false_infeasible+=1,
                        _=>{}
                    }
                    if reference.as_deref()==Some("feasible") {
                        if outcome.as_deref()==Some("feasible-claim") && oracle_outcome.as_deref()==Some("match") {
                            feasible_reference_cases_found+=1;
                        } else {
                            feasible_reference_cases_missed+=1;
                        }
                    }
                    match (outcome.as_deref(),reference.as_deref(),oracle_outcome.as_deref()) {
                        (Some("feasible-claim"),Some("feasible"),Some("match"))=>{},
                        (Some("infeasibility-proof"),Some("infeasible"),Some("match"))=>{},
                        (Some("feasible-claim"),Some("infeasible"),Some("mismatch"))=>{},
                        (Some("infeasibility-proof"),Some("feasible"),Some("mismatch"))=>{},
                        (Some("no-solution"|"unknown"|"timeout"|"invalid-output"|"not-run"),_,Some("unknown"|"not-applicable"))=>{},
                        (Some("feasible-claim"|"infeasibility-proof"),Some("unknown"|"not-applicable"),Some("unknown"|"not-applicable"))=>{},
                        (_,Some("feasible"|"infeasible"),Some("unknown"|"not-applicable"))=>{
                            errors.push(format!("{}.verifier.oracle_outcome: known reference outcome requires match or mismatch for a classified result",path));
                        },
                        (Some("feasible-claim"),Some("feasible"),Some("mismatch"))|
                        (Some("infeasibility-proof"),Some("infeasible"),Some("mismatch"))|
                        (Some("feasible-claim"),Some("infeasible"),Some("match"))|
                        (Some("infeasibility-proof"),Some("feasible"),Some("match"))=>{
                            errors.push(format!("{}.verifier.oracle_outcome: contradicts candidate and reference outcomes",path));
                        },
                        _=>errors.push(format!("{}.verifier: missing or inconsistent oracle classification",path)),
                    }
                } else if field(case,"verifier").is_none() {
                    errors.push(format!("{}.verifier: missing required field",path));
                }
            }
        }
    } else if field(&report,"case_results").is_none() {
        errors.push("$report.case_results: missing required field".to_owned());
    }

    let expected_case_count=field(&manifest,"source").and_then(|s|field(s,"case_count")).and_then(is_integer);
    if field(&report,"status").and_then(string)==Some("complete") {
        match (expected_case_count,cases) {
            (Some(expected),Some(records)) if i128::try_from(records.len()).ok()==Some(expected) => {},
            (Some(expected),Some(records)) => errors.push(format!("$report.status: complete report has {} case records; manifest requires {}",records.len(),expected)),
            _=>errors.push("$report.status: cannot establish expected case count from manifest".to_owned()),
        }
        if not_run>0 { errors.push("$report.status: complete report contains not-run cases".to_owned()); }
    }

    if let Some(summary)=expect_object(field(&report,"summary"),"$report.summary",&mut errors) {
        check_allowed(summary,&["case_count","feasible_claim_count","infeasibility_proof_count","no_solution_count","unknown_count","timeout_count","invalid_output_count","not_run_count","feasible_reference_cases_found_count","feasible_reference_cases_missed_count","false_feasible_count","false_infeasible_count","aggregate_metrics"],"$report.summary",&mut errors);
        if let Some(metrics)=field(summary,"aggregate_metrics") {
            if let Some(metrics)=object(metrics) {
                for (name,value) in metrics {
                    if !matches!(value,Json::Null) && is_number(value).is_none() {
                        errors.push(format!("$report.summary.aggregate_metrics.{}: expected numeric or null metric",name));
                    }
                }
            } else {
                errors.push("$report.summary.aggregate_metrics: expected object".to_owned());
            }
        }
        let expected=[
            ("case_count",cases.map(|v|v.len()).unwrap_or(0)),
            ("feasible_claim_count",feasible_claims),
            ("infeasibility_proof_count",infeasibility_proofs),
            ("no_solution_count",no_solution),
            ("unknown_count",unknown),
            ("timeout_count",timeouts),
            ("invalid_output_count",invalid_outputs),
            ("not_run_count",not_run),
            ("feasible_reference_cases_found_count",feasible_reference_cases_found),
            ("feasible_reference_cases_missed_count",feasible_reference_cases_missed),
            ("false_feasible_count",false_feasible),
            ("false_infeasible_count",false_infeasible),
        ];
        for (key,count) in expected {
            match field(summary,key).and_then(is_integer) {
                Some(n) if usize::try_from(n).ok()==Some(count)=>{},
                Some(n)=>errors.push(format!("$report.summary.{}: {} does not match recomputed count {}",key,n,count)),
                None=>errors.push(format!("$report.summary.{}: expected integer count",key)),
            }
        }
    } else if field(&report,"summary").is_none() { errors.push("$report.summary: missing required field".to_owned()); }

    if let Some(ev)=field(&report,"evidence") {
        if let Some(ev)=expect_object(Some(ev),"$report.evidence",&mut errors) {
            check_allowed(ev,&["artifact_uri","notes"],"$report.evidence",&mut errors);
        }
    }
    errors.sort();
    errors
}

#[cfg(test)]
mod internal_tests {
    #[test]
    fn sha256_known_vectors() {
        assert_eq!(super::sha256_hex(b""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_eq!(super::sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }
}
