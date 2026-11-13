use super::{hoist_nested_awaits, is_hoisted_await_name};
use crate::ast::{ExprKind, Item, Program, Stmt, StmtKind};
use crate::parser::Parser;

fn parse(source: &str) -> Program {
    Parser::from_source(source)
        .expect("lexes")
        .parse_program()
        .expect("parses")
}

fn body_of(program: &Program, function: &str) -> Vec<Stmt> {
    program
        .items
        .iter()
        .find_map(|item| match item {
            Item::FnDecl { name, body, .. } if name == function => Some(body.clone()),
            _ => None,
        })
        .expect("function exists")
}

fn hoisted(source: &str) -> Vec<Stmt> {
    let mut program = parse(source);
    hoist_nested_awaits(&mut program);
    body_of(&program, "run")
}

fn is_hoisted_declaration(statement: &Stmt) -> bool {
    matches!(
        &statement.kind,
        StmtKind::VarDecl { name, value: Some(value), .. }
            if is_hoisted_await_name(name) && matches!(value.kind, ExprKind::Await(_))
    )
}

fn unchanged(source: &str) {
    let mut program = parse(source);
    let before = program.clone();
    hoist_nested_awaits(&mut program);
    assert_eq!(
        format!("{:?}", program.items),
        format!("{:?}", before.items),
        "{source}"
    );
}

#[test]
fn a_match_subject_await_moves_before_the_match() {
    let body = hoisted(
        "async function run(): Promise<void> { match (await load()) { Ok(v) => { print(v); } Err(e) => { print(e); } } }",
    );
    assert_eq!(body.len(), 2);
    assert!(is_hoisted_declaration(&body[0]));
    let StmtKind::Match { expr, .. } = &body[1].kind else {
        panic!("expected the match to follow: {body:?}");
    };
    assert!(matches!(&expr.kind, ExprKind::Ident(name) if is_hoisted_await_name(name)));
}

#[test]
fn an_await_under_a_question_mark_in_a_return_moves() {
    let body = hoisted(
        "async function run(): Promise<Result<i32, string>> { return Ok((await load())? + 1); }",
    );
    assert_eq!(body.len(), 2);
    assert!(is_hoisted_declaration(&body[0]));
}

#[test]
fn awaits_in_argument_order_move_one_after_another() {
    let body = hoisted(
        "async function run(): Promise<void> { show(\"label\", await first(), await second()); }",
    );
    assert_eq!(body.len(), 3);
    assert!(is_hoisted_declaration(&body[0]));
    assert!(is_hoisted_declaration(&body[1]));
}

#[test]
fn awaits_inside_loop_bodies_move_within_the_loop() {
    let body = hoisted(
        "async function run(): Promise<void> { for (let i = 0; i < 2; i += 1) { show(await load()); } }",
    );
    let StmtKind::For {
        body: loop_body, ..
    } = &body[0].kind
    else {
        panic!("expected a for loop: {body:?}");
    };
    assert_eq!(loop_body.len(), 2);
    assert!(is_hoisted_declaration(&loop_body[0]));
}

#[test]
fn nothing_moves_past_an_effect_or_out_of_a_conditional_branch() {
    for source in [
        "async function run(): Promise<void> { show(tick(), await load()); }",
        "async function run(): Promise<void> { const ok = ready && await load(); }",
        "async function run(): Promise<void> { const v = cached ?? await load(); }",
        "async function run(): Promise<void> { const v = flag ? await load() : 0; }",
        "async function run(): Promise<void> { show(parse(x)?, await load()); }",
    ] {
        unchanged(source);
    }
}

#[test]
fn direct_awaits_and_loop_conditions_are_left_alone() {
    for source in [
        "async function run(): Promise<void> { const v = await load(); await load(); return; }",
        "async function run(): Promise<i32> { return await load(); }",
        "async function run(): Promise<void> { while (await more()) { print(1); } }",
        "function run(): void { show(await load()); }",
    ] {
        unchanged(source);
    }
}

#[test]
fn an_async_closure_body_is_rewritten_and_a_sync_one_is_not() {
    let mut program = parse(
        "const run = async (): Promise<void> => { show(await load()); };\nconst plain = (): void => { show(await load()); };",
    );
    hoist_nested_awaits(&mut program);
    let bodies = [
        body_of(&program, "run").len(),
        body_of(&program, "plain").len(),
    ];
    assert_eq!(bodies, [2, 1]);
}
