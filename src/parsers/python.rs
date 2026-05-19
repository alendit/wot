use std::path::Path;

use ruff_python_ast::{ModModule, Stmt, StmtClassDef, StmtFunctionDef};
use ruff_python_parser::parse_module;
use ruff_text_size::{Ranged, TextRange};

use crate::error::{Error, Result};
use crate::model::{Language, NodeKind, Outline, OutlineNode};
use crate::source_map::SourceMap;

pub fn parse(path: &Path, source: &str, max_depth: usize) -> Result<Outline> {
    let module = parse_module(source).map_err(|error| Error::Parse {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    let module = module.into_syntax();
    let map = SourceMap::new(source);

    Ok(Outline {
        path: path.to_path_buf(),
        language: Language::Python,
        nodes: build_suite(&module, &map, 1, max_depth),
    })
}

fn build_suite(
    module: &ModModule,
    map: &SourceMap<'_>,
    depth: usize,
    max_depth: usize,
) -> Vec<OutlineNode> {
    module
        .body
        .iter()
        .filter_map(|statement| build_statement(statement, map, depth, max_depth))
        .collect()
}

fn build_statement(
    statement: &Stmt,
    map: &SourceMap<'_>,
    depth: usize,
    max_depth: usize,
) -> Option<OutlineNode> {
    match statement {
        Stmt::ClassDef(class) => build_class(class, map, depth, max_depth),
        Stmt::FunctionDef(function) => build_function(function, map, depth, max_depth),
        _ => None,
    }
}

fn build_class(
    class: &StmtClassDef,
    map: &SourceMap<'_>,
    depth: usize,
    max_depth: usize,
) -> Option<OutlineNode> {
    let children = class
        .body
        .iter()
        .filter_map(|statement| build_statement(statement, map, depth + 1, max_depth))
        .collect();

    (depth <= max_depth).then(|| OutlineNode {
        label: format!("class {}", class.name.id),
        kind: NodeKind::Class,
        range: map.range(declaration_range(class.range, class.decorator_list.first())),
        children,
    })
}

fn build_function(
    function: &StmtFunctionDef,
    map: &SourceMap<'_>,
    depth: usize,
    max_depth: usize,
) -> Option<OutlineNode> {
    let children = function
        .body
        .iter()
        .filter_map(|statement| build_statement(statement, map, depth + 1, max_depth))
        .collect();
    let prefix = if function.is_async {
        "async def"
    } else {
        "def"
    };

    (depth <= max_depth).then(|| OutlineNode {
        label: format!("{prefix} {}", function.name.id),
        kind: NodeKind::Function,
        range: map.range(declaration_range(
            function.range,
            function.decorator_list.first(),
        )),
        children,
    })
}

fn declaration_range(
    range: TextRange,
    first_decorator: Option<&ruff_python_ast::Decorator>,
) -> std::ops::Range<usize> {
    let start = first_decorator
        .map(Ranged::start)
        .unwrap_or_else(|| range.start())
        .to_usize();
    start..range.end().to_usize()
}
