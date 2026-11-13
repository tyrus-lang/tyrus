use super::*;
use crate::lexer;
use crate::parser::Parser;

fn fused_elements(source: &str) -> Vec<String> {
    let tokens = lexer::lex(source).expect("lexes");
    let mut program = Parser::new(tokens).parse_program().expect("parses");
    fuse(&mut program);

    let body = program
        .items
        .iter()
        .find_map(|item| match item {
            Item::FnDecl { body, .. } => Some(body),
            _ => None,
        })
        .expect("a function declaration");

    for stmt in body {
        let StmtKind::VarDecl {
            value: Some(value), ..
        } = stmt.kind()
        else {
            continue;
        };
        if let ExprKind::ArrayLit(values) = value.kind() {
            return values.iter().map(render).collect();
        }
    }
    Vec::new()
}

fn surviving_pushes(source: &str) -> usize {
    let tokens = lexer::lex(source).expect("lexes");
    let mut program = Parser::new(tokens).parse_program().expect("parses");
    fuse(&mut program);

    let body = program
        .items
        .iter()
        .find_map(|item| match item {
            Item::FnDecl { body, .. } => Some(body),
            _ => None,
        })
        .expect("a function declaration");

    body.iter()
        .filter(|stmt| {
            matches!(
                stmt.kind(),
                StmtKind::Expr(Expr {
                    kind: ExprKind::MethodCall { method, .. },
                    ..
                }) if method == "push"
            )
        })
        .count()
}

fn render(expr: &Expr) -> String {
    match expr.kind() {
        ExprKind::IntLit(value) => value.to_string(),
        ExprKind::Ident(name) => name.clone(),
        ExprKind::BinOp { left, right, .. } => format!("{}+{}", render(left), render(right)),
        ExprKind::FnCall { callee, .. } => format!("{callee}()"),
        ExprKind::IndexAccess { object, .. } => format!("{}[]", render(object)),
        other => format!("{other:?}"),
    }
}

#[test]
fn fuses_a_straight_run_of_pushes_into_the_literal() {
    let source = r#"
        const main = (): void => {
            let values: i32[] = [];
            values.push(1);
            values.push(2);
            values.push(3);
            console.log(values[0]);
        };
    "#;
    assert_eq!(fused_elements(source), vec!["1", "2", "3"]);
    assert_eq!(surviving_pushes(source), 0);
}

#[test]
fn fuses_only_the_run_that_touches_the_declaration() {
    let source = r#"
        const main = (): void => {
            let values: i32[] = [];
            values.push(1);
            console.log(values[0]);
            values.push(2);
        };
    "#;
    assert_eq!(fused_elements(source), vec!["1"]);
    assert_eq!(surviving_pushes(source), 1);
}

#[test]
fn refuses_an_element_that_reads_the_array() {
    let source = r#"
        const main = (): void => {
            let values: i32[] = [];
            values.push(1);
            values.push(values[0]);
        };
    "#;
    assert_eq!(fused_elements(source), vec!["1"]);
    assert_eq!(surviving_pushes(source), 1);
}

#[test]
fn refuses_an_element_that_reads_the_array_from_inside_a_closure() {
    let source = r#"
        const main = (): void => {
            let values: i32[] = [];
            values.push(later((): i32 => values[0]));
        };
    "#;
    assert_eq!(fused_elements(source), Vec::<String>::new());
    assert_eq!(surviving_pushes(source), 1);
}

#[test]
fn refuses_a_non_copy_element_type() {
    let source = r#"
        const main = (): void => {
            let values: string[] = [];
            values.push("a");
            values.push("b");
        };
    "#;
    assert_eq!(fused_elements(source), Vec::<String>::new());
    assert_eq!(surviving_pushes(source), 2);
}

#[test]
fn refuses_a_spread_element() {
    let source = r#"
        const main = (): void => {
            let values: i32[] = [];
            values.push(...other);
        };
    "#;
    assert_eq!(fused_elements(source), Vec::<String>::new());
    assert_eq!(surviving_pushes(source), 1);
}

#[test]
fn refuses_a_declaration_that_is_not_empty() {
    let source = r#"
        const main = (): void => {
            let values: i32[] = [9];
            values.push(1);
        };
    "#;
    assert_eq!(fused_elements(source), vec!["9"]);
    assert_eq!(surviving_pushes(source), 1);
}

#[test]
fn refuses_a_push_onto_a_different_array() {
    let source = r#"
        const main = (): void => {
            let values: i32[] = [];
            other.push(1);
        };
    "#;
    assert_eq!(fused_elements(source), Vec::<String>::new());
    assert_eq!(surviving_pushes(source), 1);
}

#[test]
fn fuses_inside_a_nested_block() {
    let source = r#"
        const main = (): void => {
            let round: i32 = 0;
            while (round < 2) {
                let values: i32[] = [];
                values.push(round);
                values.push(round + 1);
                console.log(values[0]);
                round = round + 1;
            }
        };
    "#;
    let tokens = lexer::lex(source).expect("lexes");
    let mut program = Parser::new(tokens).parse_program().expect("parses");
    fuse(&mut program);
    let rendered = format!("{:?}", program.items);
    assert!(
        !rendered.contains("\"push\""),
        "expected the nested block's pushes to be fused"
    );
}
