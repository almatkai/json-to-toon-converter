//! Transcription of the default encoder in @toon-format/toon 1.3.0.
//! Input is JSON text, not serde_json::Value: JS Number and property order matter.
use std::collections::HashMap;

pub const MAX_INPUT: usize = 1_048_576;
pub const MAX_OUTPUT: usize = 4_194_304;
pub const MAX_DEPTH: usize = 64;
pub const MAX_NODES: usize = 50_000;
pub const MAX_WORK: usize = 8_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    InputLimit,
    OutputLimit,
    DepthLimit,
    NodeLimit,
    WorkLimit,
    InvalidJson,
    UnpairedSurrogate,
}
impl Error {
    pub fn message(self) -> &'static str {
        match self {
            Self::InputLimit => "JSON input exceeds the 1 MiB limit.",
            Self::OutputLimit => "TOON output exceeds the 4 MiB limit.",
            Self::DepthLimit => "JSON nesting exceeds the depth limit of 64.",
            Self::NodeLimit => "JSON exceeds the 50,000 node limit.",
            Self::WorkLimit => "Conversion exceeds the work budget.",
            Self::InvalidJson => "Invalid JSON syntax. Check commas, quotes, and JSON literals.",
            Self::UnpairedSurrogate => {
                "Unpaired UTF-16 surrogate cannot be represented by the native UTF-8 boundary."
            }
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}
impl std::error::Error for Error {}

pub fn js_whitespace(c: char) -> bool {
    matches!(c, '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}
pub fn trim_empty(text: &str) -> bool {
    text.chars().all(js_whitespace)
}

#[derive(Debug)]
enum Node {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Node>),
    Object(Object),
}
#[derive(Debug)]
struct Object {
    entries: Vec<(String, Node)>,
    positions: HashMap<String, usize>,
    // normalizeValue assigns to {}["__proto__"], invoking the legacy setter.
    // This property disappears from own keys, but can affect tabular lookup.
    prototype: Option<Box<Node>>,
}
enum Property<'a> {
    Node(&'a Node),
    Length(usize),
}
impl Property<'_> {
    fn primitive(&self) -> bool {
        match self {
            Self::Node(n) => n.primitive(),
            Self::Length(_) => true,
        }
    }
    fn encode(&self) -> String {
        match self {
            Self::Node(n) => primitive(n),
            Self::Length(n) => n.to_string(),
        }
    }
}
impl Object {
    fn get(&self, key: &str) -> Option<Property<'_>> {
        self.positions
            .get(key)
            .map(|i| Property::Node(&self.entries[*i].1))
            .or_else(|| match self.prototype.as_deref() {
                Some(Node::Object(obj)) => obj.get(key),
                Some(Node::Array(items)) if key == "length" => Some(Property::Length(items.len())),
                Some(Node::Array(items)) => index_key(key)
                    .and_then(|i| items.get(i as usize))
                    .map(Property::Node),
                _ => None,
            })
    }
}
fn index_key(key: &str) -> Option<u32> {
    let value = key.parse::<u32>().ok()?;
    (value != u32::MAX && value.to_string() == key).then_some(value)
}
impl Node {
    fn primitive(&self) -> bool {
        !matches!(self, Self::Array(_) | Self::Object(_))
    }
}

struct Parser<'a> {
    text: &'a str,
    pos: usize,
    nodes: usize,
}
impl Parser<'_> {
    fn ws(&mut self) {
        while self
            .peek()
            .is_some_and(|c| matches!(c, b' ' | b'\n' | b'\r' | b'\t'))
        {
            self.pos += 1;
        }
    }
    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.pos).copied()
    }
    fn take(&mut self, byte: u8) -> bool {
        if self.peek() == Some(byte) {
            self.pos += 1;
            true
        } else {
            false
        }
    }
    fn string(&mut self) -> Result<String, Error> {
        let start = self.pos;
        if !self.take(b'"') {
            return Err(Error::InvalidJson);
        }
        loop {
            match self.peek() {
                None => return Err(Error::InvalidJson),
                Some(b'"') => {
                    self.pos += 1;
                    break;
                }
                Some(b'\\') => {
                    self.pos += 1;
                    if self.peek().is_none() {
                        return Err(Error::InvalidJson);
                    }
                    self.pos += 1;
                }
                Some(_) => self.pos += 1,
            }
        }
        let raw = &self.text[start..self.pos];
        serde_json::from_str(raw).map_err(|_e| {
            if has_unpaired_surrogate(raw) {
                Error::UnpairedSurrogate
            } else {
                Error::InvalidJson
            }
        })
    }
    fn number(&mut self) -> Result<Node, Error> {
        let start = self.pos;
        self.take(b'-');
        if !self.take(b'0') {
            if !self.peek().is_some_and(|b| (b'1'..=b'9').contains(&b)) {
                return Err(Error::InvalidJson);
            }
            while self.peek().is_some_and(|b| b.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        if self.take(b'.') {
            let digits = self.pos;
            while self.peek().is_some_and(|b| b.is_ascii_digit()) {
                self.pos += 1;
            }
            if digits == self.pos {
                return Err(Error::InvalidJson);
            }
        }
        if self.take(b'e') || self.take(b'E') {
            if !self.take(b'+') {
                self.take(b'-');
            }
            let digits = self.pos;
            while self.peek().is_some_and(|b| b.is_ascii_digit()) {
                self.pos += 1;
            }
            if digits == self.pos {
                return Err(Error::InvalidJson);
            }
        }
        let value = self.text[start..self.pos]
            .parse::<f64>()
            .map_err(|_| Error::InvalidJson)?;
        Ok(if value.is_finite() {
            Node::Number(value)
        } else {
            Node::Null
        })
    }
    fn value(&mut self, depth: usize) -> Result<Node, Error> {
        if depth > MAX_DEPTH {
            return Err(Error::DepthLimit);
        }
        self.nodes += 1;
        if self.nodes > MAX_NODES {
            return Err(Error::NodeLimit);
        }
        self.ws();
        let node = match self.peek() {
            Some(b'"') => Node::String(self.string()?),
            Some(b'n' | b't' | b'f') => {
                let (word, value) = match self.peek() {
                    Some(b'n') => ("null", Node::Null),
                    Some(b't') => ("true", Node::Bool(true)),
                    _ => ("false", Node::Bool(false)),
                };
                if !self.text[self.pos..].starts_with(word) {
                    return Err(Error::InvalidJson);
                }
                self.pos += word.len();
                value
            }
            Some(b'[') => {
                self.pos += 1;
                self.ws();
                let mut items = Vec::new();
                if !self.take(b']') {
                    loop {
                        items.push(self.value(depth + 1)?);
                        self.ws();
                        if self.take(b']') {
                            break;
                        }
                        if !self.take(b',') {
                            return Err(Error::InvalidJson);
                        }
                    }
                }
                Node::Array(items)
            }
            Some(b'{') => {
                self.pos += 1;
                self.ws();
                let mut entries = Vec::<(String, Node)>::new();
                let mut positions = HashMap::<String, usize>::new();
                if !self.take(b'}') {
                    loop {
                        self.ws();
                        let key = self.string()?;
                        self.ws();
                        if !self.take(b':') {
                            return Err(Error::InvalidJson);
                        }
                        let value = self.value(depth + 1)?;
                        if let Some(i) = positions.get(&key) {
                            entries[*i].1 = value;
                        } else {
                            positions.insert(key.clone(), entries.len());
                            entries.push((key, value));
                        }
                        self.ws();
                        if self.take(b'}') {
                            break;
                        }
                        if !self.take(b',') {
                            return Err(Error::InvalidJson);
                        }
                    }
                }
                entries.sort_by_key(|(key, _)| index_key(key).map_or((1, 0), |v| (0, v)));
                let mut prototype = None;
                if let Some(i) = entries.iter().position(|(key, _)| key == "__proto__") {
                    let (_, value) = entries.remove(i);
                    if matches!(value, Node::Object(_) | Node::Array(_) | Node::Null) {
                        prototype = Some(Box::new(value));
                    }
                }
                positions.clear();
                for (i, (key, _)) in entries.iter().enumerate() {
                    positions.insert(key.clone(), i);
                }
                Node::Object(Object {
                    entries,
                    positions,
                    prototype,
                })
            }
            Some(b'-' | b'0'..=b'9') => self.number()?,
            _ => return Err(Error::InvalidJson),
        };
        Ok(node)
    }
}

fn has_unpaired_surrogate(raw: &str) -> bool {
    let b = raw.as_bytes();
    let unit = |i: usize| -> Option<u16> {
        if b.get(i..i + 2)? != b"\\u" {
            return None;
        }
        u16::from_str_radix(std::str::from_utf8(b.get(i + 2..i + 6)?).ok()?, 16).ok()
    };
    let mut i = 1;
    while i < b.len() {
        if b[i] != b'\\' {
            i += 1;
            continue;
        }
        if let Some(u) = unit(i) {
            if (0xd800..=0xdbff).contains(&u) {
                if !unit(i + 6).is_some_and(|u| (0xdc00..=0xdfff).contains(&u)) {
                    return true;
                }
                i += 12;
                continue;
            }
            if (0xdc00..=0xdfff).contains(&u) {
                return true;
            }
            i += 6;
        } else {
            i += 2;
        }
    }
    false
}

fn escape(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
fn numeric_like(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut i = usize::from(bytes.first() == Some(&b'-'));
    let start = i;
    while bytes.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
    }
    if i == start {
        return false;
    }
    if bytes.get(i) == Some(&b'.') {
        i += 1;
        let start = i;
        while bytes.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        if start == i {
            return false;
        }
    }
    if matches!(bytes.get(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(bytes.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        let start = i;
        while bytes.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        if start == i {
            return false;
        }
    }
    i == bytes.len()
}
fn key(text: &str) -> String {
    let mut bytes = text.bytes();
    let valid = bytes
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'.');
    if valid {
        text.to_owned()
    } else {
        format!("\"{}\"", escape(text))
    }
}
fn primitive(value: &Node) -> String {
    match value {
        Node::Null => "null".into(),
        Node::Bool(b) => b.to_string(),
        Node::Number(n) => {
            if *n == 0.0 {
                "0".into()
            } else {
                ryu_js::Buffer::new().format(*n).to_owned()
            }
        }
        Node::String(s) => {
            let safe = !s.is_empty()
                && s.trim_matches(js_whitespace) == s
                && !matches!(s.as_str(), "null" | "true" | "false")
                && !numeric_like(s)
                && !s.contains([':', '"', '\\', '[', ']', '{', '}', '\n', '\r', '\t', ','])
                && !s.starts_with('-');
            if safe {
                s.clone()
            } else {
                format!("\"{}\"", escape(s))
            }
        }
        _ => unreachable!("only primitive callers"),
    }
}
struct Encoder {
    out: String,
    work: usize,
    lines: usize,
}
impl Encoder {
    fn charge(&mut self, amount: usize) -> Result<(), Error> {
        self.work = self.work.checked_add(amount).ok_or(Error::WorkLimit)?;
        if self.work > MAX_WORK {
            Err(Error::WorkLimit)
        } else {
            Ok(())
        }
    }
    fn line(&mut self, depth: usize, content: &str) -> Result<(), Error> {
        self.charge(content.len() + depth * 2 + 1)?;
        let size = self.out.len() + depth * 2 + content.len() + usize::from(self.lines != 0);
        if size > MAX_OUTPUT {
            return Err(Error::OutputLimit);
        }
        if self.lines != 0 {
            self.out.push('\n');
        }
        self.lines += 1;
        for _ in 0..depth * 2 {
            self.out.push(' ');
        }
        self.out.push_str(content);
        Ok(())
    }
    fn header(
        &mut self,
        prefix: Option<&str>,
        n: usize,
        fields: Option<&[String]>,
    ) -> Result<String, Error> {
        let mut out = prefix
            .filter(|s| !s.is_empty())
            .map(key)
            .unwrap_or_default();
        out.push_str(&format!("[{n}]"));
        if let Some(fields) = fields {
            out.push('{');
            out.push_str(&fields.iter().map(|s| key(s)).collect::<Vec<_>>().join(","));
            out.push('}');
        }
        out.push(':');
        self.charge(out.len())?;
        Ok(out)
    }
    fn inline(&mut self, prefix: Option<&str>, items: &[Node]) -> Result<String, Error> {
        let mut out = self.header(prefix, items.len(), None)?;
        if !items.is_empty() {
            out.push(' ');
            out.push_str(&items.iter().map(primitive).collect::<Vec<_>>().join(","));
        }
        self.charge(out.len())?;
        Ok(out)
    }
    fn table(&mut self, items: &[Node]) -> Result<Option<Vec<String>>, Error> {
        let Some(Node::Object(first)) = items.first() else {
            return Ok(None);
        };
        if first.entries.is_empty() {
            return Ok(None);
        }
        let keys: Vec<String> = first.entries.iter().map(|(k, _)| k.clone()).collect();
        for item in items {
            let Node::Object(obj) = item else {
                return Ok(None);
            };
            if obj.entries.len() != keys.len() {
                return Ok(None);
            }
            for k in &keys {
                self.charge(k.len() + MAX_DEPTH)?;
                if !obj.get(k).is_some_and(|p| p.primitive()) {
                    return Ok(None);
                }
            }
        }
        Ok(Some(keys))
    }
    fn rows(&mut self, items: &[Node], fields: &[String], depth: usize) -> Result<(), Error> {
        for item in items {
            let Node::Object(obj) = item else {
                unreachable!()
            };
            let row = fields
                .iter()
                .map(|k| obj.get(k).expect("validated table property").encode())
                .collect::<Vec<_>>()
                .join(",");
            self.line(depth, &row)?;
        }
        Ok(())
    }
    fn pair(&mut self, k: &str, value: &Node, depth: usize) -> Result<(), Error> {
        self.charge(k.len() + 1)?;
        match value {
            Node::Array(items) => self.array(Some(k), items, depth),
            Node::Object(obj) => {
                self.line(depth, &format!("{}:", key(k)))?;
                self.object(obj, depth + 1)
            }
            _ => self.line(depth, &format!("{}: {}", key(k), primitive(value))),
        }
    }
    fn object(&mut self, obj: &Object, depth: usize) -> Result<(), Error> {
        for (k, v) in &obj.entries {
            self.pair(k, v, depth)?;
        }
        Ok(())
    }
    fn array(&mut self, prefix: Option<&str>, items: &[Node], depth: usize) -> Result<(), Error> {
        self.charge(items.len())?;
        if items.iter().all(Node::primitive) {
            let line = self.inline(prefix, items)?;
            return self.line(depth, &line);
        }
        if let Some(fields) = self.table(items)? {
            let header = self.header(prefix, items.len(), Some(&fields))?;
            self.line(depth, &header)?;
            return self.rows(items, &fields, depth + 1);
        }
        let header = self.header(prefix, items.len(), None)?;
        self.line(depth, &header)?;
        for item in items {
            self.item(item, depth + 1)?;
        }
        Ok(())
    }
    fn item(&mut self, value: &Node, depth: usize) -> Result<(), Error> {
        self.charge(1)?;
        match value {
            Node::Array(items) => {
                if items.iter().all(Node::primitive) {
                    let line = self.inline(None, items)?;
                    self.line(depth, &format!("- {line}"))
                } else {
                    let header = self.header(None, items.len(), None)?;
                    self.line(depth, &format!("- {header}"))?;
                    for item in items {
                        self.item(item, depth + 1)?;
                    }
                    Ok(())
                }
            }
            Node::Object(obj) => {
                let Some((k, v)) = obj.entries.first() else {
                    return self.line(depth, "-");
                };
                match v {
                    Node::Array(items) if items.iter().all(Node::primitive) => {
                        let line = self.inline(Some(k), items)?;
                        self.line(depth, &format!("- {line}"))?;
                    }
                    Node::Array(items) => {
                        if let Some(fields) = self.table(items)? {
                            let header = self.header(Some(k), items.len(), Some(&fields))?;
                            self.line(depth, &format!("- {header}"))?;
                            self.rows(items, &fields, depth + 1)?;
                        } else {
                            self.line(depth, &format!("- {}[{}]:", key(k), items.len()))?;
                            for item in items {
                                self.item(item, depth + 1)?;
                            }
                        }
                    }
                    Node::Object(nested) => {
                        self.line(depth, &format!("- {}:", key(k)))?;
                        self.object(nested, depth + 2)?;
                    }
                    _ => self.line(depth, &format!("- {}: {}", key(k), primitive(v)))?,
                }
                for (k, v) in obj.entries.iter().skip(1) {
                    self.pair(k, v, depth + 1)?;
                }
                Ok(())
            }
            _ => self.line(depth, &format!("- {}", primitive(value))),
        }
    }
}
pub fn convert(text: &str) -> Result<String, Error> {
    if text.len() > MAX_INPUT {
        return Err(Error::InputLimit);
    }
    let mut parser = Parser {
        text,
        pos: 0,
        nodes: 0,
    };
    let value = parser.value(0)?;
    parser.ws();
    if parser.pos != text.len() {
        return Err(Error::InvalidJson);
    }
    let mut encoder = Encoder {
        out: String::new(),
        work: text.len(),
        lines: 0,
    };
    match &value {
        Node::Array(items) => encoder.array(None, items, 0)?,
        Node::Object(obj) => encoder.object(obj, 0)?,
        _ => encoder.line(0, &primitive(&value))?,
    }
    Ok(encoder.out)
}

#[cfg(test)]
mod budget_tests {
    use super::*;
    #[test]
    fn explicit_encoder_budgets_fail_without_returning_partial_output() {
        let mut e = Encoder {
            out: String::new(),
            work: MAX_WORK,
            lines: 0,
        };
        assert_eq!(e.line(0, "x"), Err(Error::WorkLimit));
        let mut e = Encoder {
            out: "x".repeat(MAX_OUTPUT),
            work: 0,
            lines: 1,
        };
        assert_eq!(e.line(0, "x"), Err(Error::OutputLimit));
    }
}
