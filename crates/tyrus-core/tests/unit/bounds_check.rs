use super::*;
use crate::parser::Parser;

fn checked(source: &str) -> CheckedIndexes {
    let spanned = crate::lexer::lex_spanned(source).expect("lexes");
    let tokens = spanned.iter().map(|(token, _)| token.clone()).collect();
    let ranges = spanned.iter().map(|(_, range)| range.clone()).collect();
    let program = Parser::new_with_ranges(tokens, ranges)
        .parse_program()
        .expect("parses");
    analyze(&program)
}

fn count(source: &str) -> usize {
    checked(source).len()
}

#[test]
fn a_counting_loop_over_a_parameter_proves_its_own_indexing() {
    assert_eq!(
        count(
            r#"
function total(values: i32[]): i32 {
    let sum = 0;
    for (let i: usize = 0; i < values.length; i += 1) {
        sum += values[i];
    }
    return sum;
}
"#,
        ),
        1
    );
}

#[test]
fn every_access_in_the_body_is_proved_not_only_the_first() {
    assert_eq!(
        count(
            r#"
function span(values: i32[]): i32 {
    let low = 0;
    for (let i: usize = 0; i < values.length; i += 1) {
        low += values[i];
        low -= values[i];
    }
    return low;
}
"#,
        ),
        2
    );
}

#[test]
fn writing_an_element_is_proved_the_same_way_reading_one_is() {
    assert_eq!(
        count(
            r#"
function double(values: i32[]): void {
    for (let i: usize = 0; i < values.length; i += 1) {
        values[i] = values[i] * 2;
    }
}
"#,
        ),
        2
    );
}

#[test]
fn the_postfix_increment_form_counts_too() {
    assert_eq!(
        count(
            r#"
function total(values: i32[]): i32 {
    let sum = 0;
    for (let i: usize = 0; i < values.length; i++) {
        sum += values[i];
    }
    return sum;
}
"#,
        ),
        1
    );
}

#[test]
fn a_loop_over_a_mutable_global_array_proves_nothing() {
    assert_eq!(
        count(
            r#"
let values: i32[] = [1, 2, 3];

function grow(): void {
    values.push(4);
}

function total(): i32 {
    let sum = 0;
    for (let i: usize = 0; i < values.length; i += 1) {
        grow();
        sum += values[i];
    }
    return sum;
}
"#,
        ),
        0
    );
}

#[test]
fn a_loop_over_an_immutable_global_array_is_proved() {
    assert_eq!(
        count(
            r#"
const values: i32[] = [1, 2, 3];

function total(): i32 {
    let sum = values[0];
    for (let i: usize = 0; i < values.length; i += 1) {
        sum += values[i];
    }
    return sum;
}
"#,
        ),
        2
    );
}

#[test]
fn growing_the_array_inside_the_loop_proves_nothing() {
    assert_eq!(
        count(
            r#"
function grow(values: i32[]): i32 {
    let sum = 0;
    for (let i: usize = 0; i < values.length; i += 1) {
        values.push(1);
        sum += values[i];
    }
    return sum;
}
"#,
        ),
        0
    );
}

#[test]
fn writing_the_counter_inside_the_loop_proves_nothing() {
    assert_eq!(
        count(
            r#"
function skip(values: i32[]): i32 {
    let sum = 0;
    for (let i: usize = 0; i < values.length; i += 1) {
        i += 3;
        sum += values[i];
    }
    return sum;
}
"#,
        ),
        0
    );
}

#[test]
fn mentioning_the_counter_inside_a_closure_proves_nothing() {
    assert_eq!(
        count(
            r#"
function total(values: i32[]): i32 {
    let sum = 0;
    for (let i: usize = 0; i < values.length; i += 1) {
        const bump = (): void => { i += 1; };
        sum += values[i];
    }
    return sum;
}
"#,
        ),
        0
    );
}

#[test]
fn handing_the_array_to_a_function_proves_nothing() {
    assert_eq!(
        count(
            r#"
function sink(other: i32[]): i32 {
    return other.length as i32;
}

function total(values: i32[]): i32 {
    let sum = 0;
    for (let i: usize = 0; i < values.length; i += 1) {
        sum += sink(values);
        sum += values[i];
    }
    return sum;
}
"#,
        ),
        0
    );
}

#[test]
fn shadowing_the_counter_inside_the_body_proves_nothing() {
    assert_eq!(
        count(
            r#"
function total(values: i32[]): i32 {
    let sum = 0;
    for (let i: usize = 0; i < values.length; i += 1) {
        while (sum < 0) {
            const i: usize = 9;
            sum += values[i] as i32;
        }
        sum += values[i];
    }
    return sum;
}
"#,
        ),
        0
    );
}

#[test]
fn a_different_index_expression_keeps_its_check() {
    assert_eq!(
        count(
            r#"
function total(values: i32[], other: usize): i32 {
    let sum = 0;
    for (let i: usize = 0; i < values.length; i += 1) {
        sum += values[other];
        sum += values[i];
    }
    return sum;
}
"#,
        ),
        1
    );
}

#[test]
fn a_condition_against_something_other_than_the_length_proves_nothing() {
    assert_eq!(
        count(
            r#"
function total(values: i32[]): i32 {
    let sum = 0;
    for (let i: usize = 0; i < 3; i += 1) {
        sum += values[i];
    }
    return sum;
}
"#,
        ),
        0
    );
}

#[test]
fn nested_loops_are_each_proved_separately() {
    assert_eq!(
        count(
            r#"
function total(values: i32[]): i32 {
    let sum = 0;
    for (let i: usize = 0; i < values.length; i += 1) {
        for (let j: usize = 0; j < values.length; j += 1) {
            sum += values[j];
        }
        sum += values[i];
    }
    return sum;
}
"#,
        ),
        2
    );
}

#[test]
fn a_loop_over_a_local_array_proves_its_indexing() {
    assert_eq!(
        count(
            r#"
function total(): i32 {
    const values: i32[] = [1, 2, 3];
    let sum = 0;
    for (let i: usize = 0; i < values.length; i += 1) {
        sum += values[i];
    }
    return sum;
}
"#,
        ),
        1
    );
}

#[test]
fn a_loop_inside_an_arrow_function_is_judged_the_same_way() {
    assert_eq!(
        count(
            r#"
const main = (): void => {
    const values: i32[] = [1, 2, 3];
    let sum = 0;
    for (let i: usize = 0; i < values.length; i += 1) {
        sum += values[i];
    }
    console.log(sum);
};
"#,
        ),
        1
    );
}

#[test]
fn a_constant_index_into_a_readonly_const_table_is_proved() {
    assert_eq!(
        count(
            r#"
readonly const PRIMES: i32[] = [2, 3, 5, 7];

function third(): i32 {
    return PRIMES[2];
}
"#,
        ),
        1
    );
}

#[test]
fn a_plain_const_table_proves_the_same_thing() {
    assert_eq!(
        count(
            r#"
const PRIMES: i32[] = [2, 3, 5, 7];

function third(): i32 {
    return PRIMES[2];
}
"#,
        ),
        1
    );
}

#[test]
fn a_constant_index_past_the_end_proves_nothing() {
    assert_eq!(
        count(
            r#"
const PRIMES: i32[] = [2, 3, 5, 7];

function past(): i32 {
    return PRIMES[4];
}
"#,
        ),
        0
    );
}

#[test]
fn the_last_slot_is_proved_and_the_one_after_it_is_not() {
    let proved = checked(
        r#"
const PRIMES: i32[] = [2, 3, 5, 7];

function edges(): i32 {
    return PRIMES[3] + PRIMES[4];
}
"#,
    );
    assert_eq!(proved.len(), 1);
}

#[test]
fn a_constant_index_into_a_mutable_binding_proves_nothing() {
    assert_eq!(
        count(
            r#"
function pick(): i32 {
    let values: i32[] = [1, 2, 3];
    values.push(4);
    return values[2];
}
"#,
        ),
        0
    );
}

#[test]
fn a_spread_in_the_initializer_proves_nothing() {
    assert_eq!(
        count(
            r#"
function pick(others: i32[]): i32 {
    const values: i32[] = [1, ...others, 3];
    return values[2];
}
"#,
        ),
        0
    );
}

#[test]
fn a_constant_index_into_a_local_const_table_is_proved() {
    assert_eq!(
        count(
            r#"
function pick(): i32 {
    const values: i32[] = [1, 2, 3];
    return values[1];
}
"#,
        ),
        1
    );
}

#[test]
fn handing_a_const_table_to_a_function_proves_nothing() {
    assert_eq!(
        count(
            r#"
function consume(values: i32[]): void {}

function pick(): i32 {
    const values: i32[] = [1, 2, 3];
    consume(values);
    return values[1];
}
"#,
        ),
        0
    );
}

#[test]
fn a_parameter_shadowing_a_global_table_proves_nothing() {
    assert_eq!(
        count(
            r#"
const PRIMES: i32[] = [2, 3, 5, 7];

function third(PRIMES: i32[]): i32 {
    return PRIMES[2];
}
"#,
        ),
        0
    );
}

#[test]
fn a_nested_binding_shadowing_a_const_table_proves_nothing() {
    assert_eq!(
        count(
            r#"
function pick(flag: bool): i32 {
    const values: i32[] = [1, 2, 3];
    if (flag) {
        let values: i32[] = [9];
        return values[2];
    }
    return 0;
}
"#,
        ),
        0
    );
}

#[test]
fn a_for_of_variable_shadowing_a_const_table_proves_nothing() {
    assert_eq!(
        count(
            r#"
function pick(rows: i32[][]): i32 {
    const values: i32[] = [1, 2, 3];
    for (const values of rows) {
        return values[2];
    }
    return 0;
}
"#,
        ),
        0
    );
}

#[test]
fn a_computed_index_into_a_const_table_proves_nothing() {
    assert_eq!(
        count(
            r#"
function pick(slot: usize): i32 {
    const values: i32[] = [1, 2, 3];
    return values[slot];
}
"#,
        ),
        0
    );
}

#[test]
fn a_const_table_read_only_from_a_closure_proves_nothing() {
    assert_eq!(
        count(
            r#"
function pick(): i32 {
    const values: i32[] = [1, 2, 3];
    const read = (): i32 => values[1];
    return read();
}
"#,
        ),
        0
    );
}

#[test]
fn a_constant_index_at_top_level_is_proved() {
    assert_eq!(
        count(
            r#"
const PRIMES: i32[] = [2, 3, 5, 7];
console.log(PRIMES[3]);
"#,
        ),
        1
    );
}

#[test]
fn an_arrow_main_reads_a_global_table_the_same_way() {
    assert_eq!(
        count(
            r#"
const PRIMES: i32[] = [2, 3, 5, 7];

const main = (): void => {
    console.log(PRIMES[3]);
};
"#,
        ),
        1
    );
}

#[test]
fn a_match_pattern_shadowing_a_const_table_proves_nothing() {
    assert_eq!(
        count(
            r#"
function pick(rows: i32[]): i32 {
    const values: i32[] = [1, 2, 3];
    match (rows) {
        values => {
            return values[2];
        }
    }
}
"#,
        ),
        0
    );
}

#[test]
fn a_match_pattern_shadowing_the_loop_array_proves_nothing() {
    assert_eq!(
        count(
            r#"
function total(values: i32[], rows: i32[]): i32 {
    let sum = 0;
    for (let i: usize = 0; i < values.length; i += 1) {
        match (rows) {
            values => {
                sum += values[i];
            }
        }
    }
    return sum;
}
"#,
        ),
        0
    );
}

#[test]
fn an_array_match_rest_binding_shadows_an_outer_table() {
    assert_eq!(
        count(
            r#"
function pick(rows: i32[]): i32 {
    const rest: i32[] = [1, 2, 3];
    match (rows) {
        [first, ...rest] => {
            return rest[2];
        }
        _ => {
            return first;
        }
    }
}
"#,
        ),
        0
    );
}
