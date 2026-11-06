use std::collections::BTreeMap;

use tyrus_core::ast::Program;

use crate::{comments::Comment, config::Config, printer::Printer};

pub type AnchoredComments = BTreeMap<usize, Vec<(bool, String)>>;

pub fn anchor_comments(source: &str, comments: &[Comment]) -> AnchoredComments {
    let lines = source.lines().collect::<Vec<_>>();
    super::anchor_comments(&lines, comments)
}

pub fn parse_program(source: &str) -> Option<Program> {
    let mut parser = tyrus_core::parser::Parser::from_source(source).ok()?;
    parser.parse_program().ok()
}

pub fn remove_unused_imports(program: &mut Program) {
    super::remove_unused_imports(program);
}

pub fn sort_imports(program: &mut Program) {
    super::sort_imports(program);
}

pub fn print_program(source: &str, program: &Program, config: &Config) -> String {
    let mut printer = Printer::new(config.clone(), source);
    printer.print_program(program);
    printer.into_string()
}

pub fn build_line_map(source: &str, formatted: &str) -> Vec<Option<usize>> {
    super::build_line_map(source, formatted)
}

pub fn reinsert_comments(
    source: &str,
    formatted: &str,
    comments: &[Comment],
    anchored: &AnchoredComments,
    line_map: &[Option<usize>],
) -> String {
    let source_lines = source.lines().collect::<Vec<_>>();
    if comments.is_empty() {
        super::preserve_blank_lines_only(formatted, &source_lines, line_map)
    } else {
        super::reinsert_comments(formatted, &source_lines, anchored, line_map, true)
    }
}
