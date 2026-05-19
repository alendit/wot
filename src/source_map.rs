use std::ops::Range;

use crate::model::{Position, SourceRange};

#[derive(Debug, Clone)]
pub struct SourceMap<'a> {
    source: &'a str,
    line_starts: Vec<usize>,
}

impl<'a> SourceMap<'a> {
    pub fn new(source: &'a str) -> Self {
        let mut line_starts = vec![0];

        for (offset, byte) in source.bytes().enumerate() {
            if byte == b'\n' && offset + 1 < source.len() {
                line_starts.push(offset + 1);
            }
        }

        Self {
            source,
            line_starts,
        }
    }

    pub fn position(&self, offset: usize) -> Position {
        let clamped = offset.min(self.source.len());
        let line_index = match self.line_starts.binary_search(&clamped) {
            Ok(index) => index,
            Err(index) => index.saturating_sub(1),
        };
        let line_start = self.line_starts[line_index];
        let column = self.source[line_start..clamped].chars().count() + 1;

        Position {
            line: line_index + 1,
            column,
        }
    }

    pub fn range(&self, range: Range<usize>) -> SourceRange {
        let start = self.position(range.start);
        let inclusive_end = range.end.saturating_sub(1);
        let end = self.position(inclusive_end);
        SourceRange::lines(start.line, end.line)
    }

    pub fn precise_range(&self, range: Range<usize>) -> SourceRange {
        let start = self.position(range.start);
        let end = self.position(range.end);
        SourceRange::spans(start.line, start.column, end.line, end.column)
    }

    pub fn line_start_offset(&self, line: usize) -> usize {
        self.line_starts
            .get(line.saturating_sub(1))
            .copied()
            .unwrap_or(self.source.len())
    }

    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }
}
