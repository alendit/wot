use std::path::Path;

use crate::error::{Error, Result};
use crate::model::{Language, NodeKind, Outline, OutlineNode};
use crate::source_map::SourceMap;

pub fn parse(path: &Path, source: &str, max_depth: usize) -> Result<Outline> {
    serde_json::from_str::<serde_json::Value>(source).map_err(|error| Error::Parse {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;

    let mut scanner = Scanner::new(source);
    let mut value = scanner.parse_value().map_err(|message| Error::Parse {
        path: path.to_path_buf(),
        message,
    })?;
    scanner.skip_whitespace();

    if scanner.position != source.len() {
        return Err(Error::Parse {
            path: path.to_path_buf(),
            message: "trailing content after JSON value".into(),
        });
    }

    let map = SourceMap::new(source);
    mark_precise_same_line_siblings(&mut value.children, &map);

    Ok(Outline {
        path: path.to_path_buf(),
        language: Language::Json,
        nodes: into_outline_nodes(value.children, &map, 1, max_depth),
    })
}

#[derive(Debug, Clone)]
struct JsonValue {
    range: std::ops::Range<usize>,
    summary: String,
    children: Vec<JsonNode>,
}

#[derive(Debug, Clone)]
struct JsonNode {
    label: String,
    kind: NodeKind,
    range: std::ops::Range<usize>,
    precise: bool,
    children: Vec<JsonNode>,
}

struct Scanner<'a> {
    source: &'a str,
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Scanner<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            position: 0,
        }
    }

    fn parse_value(&mut self) -> std::result::Result<JsonValue, String> {
        self.skip_whitespace();
        match self.peek() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => self.parse_string_value(),
            Some(b'-' | b'0'..=b'9') => self.parse_number_value(),
            Some(b't') => self.parse_literal("true"),
            Some(b'f') => self.parse_literal("false"),
            Some(b'n') => self.parse_literal("null"),
            Some(byte) => Err(format!("unexpected byte `{}`", byte as char)),
            None => Err("unexpected end of JSON input".into()),
        }
    }

    fn parse_object(&mut self) -> std::result::Result<JsonValue, String> {
        let start = self.expect_byte(b'{')?;
        let mut children = Vec::new();
        self.skip_whitespace();

        if self.consume_if(b'}') {
            return Ok(JsonValue {
                range: start..self.position,
                summary: "object".into(),
                children,
            });
        }

        loop {
            self.skip_whitespace();
            let key_start = self.position;
            let key = self.parse_string()?;
            self.skip_whitespace();
            self.expect_byte(b':')?;
            let value = self.parse_value()?;
            children.push(JsonNode {
                label: format!("{key}: {}", value.summary),
                kind: NodeKind::JsonProperty,
                range: key_start..value.range.end,
                precise: false,
                children: value.children,
            });
            self.skip_whitespace();

            if self.consume_if(b',') {
                continue;
            }
            self.expect_byte(b'}')?;
            break;
        }

        Ok(JsonValue {
            range: start..self.position,
            summary: "object".into(),
            children,
        })
    }

    fn parse_array(&mut self) -> std::result::Result<JsonValue, String> {
        let start = self.expect_byte(b'[')?;
        let mut children = Vec::new();
        self.skip_whitespace();

        if self.consume_if(b']') {
            return Ok(JsonValue {
                range: start..self.position,
                summary: "array[0]".into(),
                children,
            });
        }

        loop {
            let index = children.len();
            let value = self.parse_value()?;
            children.push(JsonNode {
                label: format!("[{index}]: {}", value.summary),
                kind: NodeKind::JsonArrayElement,
                range: value.range,
                precise: false,
                children: value.children,
            });
            self.skip_whitespace();

            if self.consume_if(b',') {
                continue;
            }
            self.expect_byte(b']')?;
            break;
        }

        Ok(JsonValue {
            range: start..self.position,
            summary: format!("array[{}]", children.len()),
            children,
        })
    }

    fn parse_string_value(&mut self) -> std::result::Result<JsonValue, String> {
        let start = self.position;
        let value = self.parse_string()?;
        Ok(JsonValue {
            range: start..self.position,
            summary: format!("{:?}", truncate(&value, 40)),
            children: Vec::new(),
        })
    }

    fn parse_number_value(&mut self) -> std::result::Result<JsonValue, String> {
        let start = self.position;
        if self.consume_if(b'-') && !matches!(self.peek(), Some(b'0'..=b'9')) {
            return Err("expected digit after minus sign".into());
        }

        self.consume_digits();

        if self.consume_if(b'.') {
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err("expected digit after decimal point".into());
            }
            self.consume_digits();
        }

        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.position += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.position += 1;
            }
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err("expected digit in exponent".into());
            }
            self.consume_digits();
        }

        Ok(JsonValue {
            range: start..self.position,
            summary: self.source[start..self.position].into(),
            children: Vec::new(),
        })
    }

    fn parse_literal(&mut self, literal: &str) -> std::result::Result<JsonValue, String> {
        let start = self.position;
        if self.source[start..].starts_with(literal) {
            self.position += literal.len();
            Ok(JsonValue {
                range: start..self.position,
                summary: literal.into(),
                children: Vec::new(),
            })
        } else {
            Err(format!("expected `{literal}`"))
        }
    }

    fn parse_string(&mut self) -> std::result::Result<String, String> {
        let start = self.expect_byte(b'"')?;
        let mut escaped = false;

        while let Some(byte) = self.peek() {
            self.position += 1;
            if escaped {
                escaped = false;
                continue;
            }
            match byte {
                b'\\' => escaped = true,
                b'"' => {
                    let raw = &self.source[start..self.position];
                    return serde_json::from_str(raw)
                        .map_err(|error| format!("invalid string literal: {error}"));
                }
                _ => {}
            }
        }

        Err("unterminated string literal".into())
    }

    fn consume_digits(&mut self) {
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.position += 1;
        }
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.position += 1;
        }
    }

    fn expect_byte(&mut self, expected: u8) -> std::result::Result<usize, String> {
        let start = self.position;
        if self.consume_if(expected) {
            Ok(start)
        } else {
            Err(format!("expected `{}`", expected as char))
        }
    }

    fn consume_if(&mut self, expected: u8) -> bool {
        if self.peek() == Some(expected) {
            self.position += 1;
            true
        } else {
            false
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.position).copied()
    }
}

fn mark_precise_same_line_siblings(nodes: &mut [JsonNode], map: &SourceMap<'_>) {
    let mut same_line_counts = std::collections::HashMap::<usize, usize>::new();

    for node in nodes.iter() {
        let start = map.position(node.range.start);
        let end = map.position(node.range.end);
        if start.line == end.line {
            *same_line_counts.entry(start.line).or_default() += 1;
        }
    }

    for node in nodes.iter_mut() {
        let start = map.position(node.range.start);
        if same_line_counts.get(&start.line).copied().unwrap_or(0) > 1 {
            node.precise = true;
        }
        mark_precise_same_line_siblings(&mut node.children, map);
    }
}

fn into_outline_nodes(
    nodes: Vec<JsonNode>,
    map: &SourceMap<'_>,
    depth: usize,
    max_depth: usize,
) -> Vec<OutlineNode> {
    if depth > max_depth {
        return Vec::new();
    }

    nodes
        .into_iter()
        .map(|node| {
            let range = if node.precise {
                map.precise_range(node.range)
            } else {
                map.range(node.range)
            };
            OutlineNode {
                label: node.label,
                kind: node.kind,
                range,
                children: into_outline_nodes(node.children, map, depth + 1, max_depth),
            }
        })
        .collect()
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut output = String::new();
    for (index, character) in value.chars().enumerate() {
        if index == max_chars {
            output.push_str("...");
            break;
        }
        output.push(character);
    }
    output
}
