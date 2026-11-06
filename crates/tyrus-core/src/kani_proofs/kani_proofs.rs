use crate::ctfe::{EvaluationBudget, EvaluationLimits};
use crate::db::CapturedSqlCall;
use crate::span::Span;

#[kani::proof]
fn span_range_round_trip_preserves_bounds() {
    let start: usize = kani::any();
    let end: usize = kani::any();

    let span = Span::new(start, end);

    assert_eq!(Span::from(span.to_range()), span);
}

#[kani::proof]
fn accepted_ctfe_step_never_exceeds_the_budget() {
    let limit: u64 = kani::any();
    let mut budget = EvaluationBudget::new(EvaluationLimits {
        steps: limit,
        call_depth: 1,
        value_elements: 1,
        value_bytes: 1,
    });

    let accepted = budget.consume_step();

    assert!(!accepted || budget.steps() <= limit);
}

#[kani::proof]
fn leaving_a_ctfe_call_restores_zero_depth() {
    let mut budget = EvaluationBudget::new(EvaluationLimits {
        steps: 1,
        call_depth: 1,
        value_elements: 1,
        value_bytes: 1,
    });

    assert!(budget.enter_call());
    budget.leave_call();

    assert_eq!(budget.call_depth(), 0);
}

#[kani::proof]
fn accepted_ctfe_temporary_allocation_never_exceeds_the_budget() {
    let limit: usize = kani::any();
    let requested: usize = kani::any();
    let mut budget = EvaluationBudget::new(EvaluationLimits {
        steps: 1,
        call_depth: 1,
        value_elements: 1,
        value_bytes: limit,
    });

    let accepted = budget.reserve_temporary(requested);

    assert!(!accepted || budget.temporary_bytes() <= limit);
}

#[kani::proof]
fn sql_call_source_ranges_stay_within_the_source_literal() {
    let start: usize = kani::any();
    let end: usize = kani::any();
    let call = CapturedSqlCall::kani_range_fixture();
    let range = call.source_range_for_sql(start..end);
    assert!(range.start >= 20);
    assert!(range.end <= 25);
    assert!(range.start <= range.end);
}
