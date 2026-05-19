use wot::model::{Position, SourceRange};
use wot::source_map::SourceMap;

#[test]
fn maps_ascii_offsets_to_one_based_line_and_column_positions() {
    let source = "alpha\nbeta\ngamma\n";
    let map = SourceMap::new(source);

    assert_eq!(map.position(0), Position { line: 1, column: 1 });
    assert_eq!(map.position(6), Position { line: 2, column: 1 });
    assert_eq!(map.position(10), Position { line: 2, column: 5 });
    assert_eq!(map.position(11), Position { line: 3, column: 1 });
}

#[test]
fn maps_unicode_offsets_to_character_columns() {
    let source = "aé🙂\nnext";
    let map = SourceMap::new(source);

    assert_eq!(map.position(0), Position { line: 1, column: 1 });
    assert_eq!(map.position("a".len()), Position { line: 1, column: 2 });
    assert_eq!(map.position("aé".len()), Position { line: 1, column: 3 });
    assert_eq!(
        map.position("aé🙂\n".len()),
        Position { line: 2, column: 1 }
    );
}

#[test]
fn creates_inclusive_line_ranges_from_byte_ranges() {
    let source = "root\n  child\nend\n";
    let map = SourceMap::new(source);

    assert_eq!(map.range(0..source.len()), SourceRange::lines(1, 3));
    assert_eq!(map.range(7..12), SourceRange::lines(2, 2));
}

#[test]
fn keeps_columns_when_a_single_line_range_needs_precision() {
    let source = r#"{"a":1,"b":2}"#;
    let map = SourceMap::new(source);

    assert_eq!(map.precise_range(1..6), SourceRange::spans(1, 2, 1, 7));
}
