use super::*;
use crate::ast::expr_referenced_identifiers;

impl TypeChecker {
    pub(super) fn check_array_count(&mut self, element: &Type, args: &[Expr]) -> ValueType {
        self.require_method_arity("count", args, 0, 1);
        let Some(arg) = args.first() else {
            return ValueType::Known(Type::Usize);
        };
        let mut free: Vec<String> = expr_referenced_identifiers(arg)
            .into_iter()
            .filter(|name| self.lookup(name).is_none())
            .collect();
        free.sort();
        match free.len() {
            0 => {
                let actual = self.check_expr(arg);
                if !self.is_assignable(element, &actual) {
                    self.report_count_value_mismatch(element, &actual, arg);
                }
                ValueType::Known(Type::Usize)
            }
            1 => {
                let synthetic = implicit_count_condition(&free[0], arg.clone());
                self.check_array_callback(
                    &synthetic,
                    std::slice::from_ref(element),
                    &Type::Bool,
                    "count",
                );
                ValueType::Known(Type::Usize)
            }
            _ => {
                let names = free
                    .iter()
                    .map(|name| format!("`{name}`"))
                    .collect::<Vec<_>>()
                    .join(", ");
                self.error_at(
                    "E1356",
                    format!(
                        "`count` implicit condition supports exactly one free variable, found {}: {names}",
                        free.len()
                    ),
                    arg.span,
                );
                ValueType::Error
            }
        }
    }

    fn report_count_value_mismatch(&mut self, element: &Type, actual: &ValueType, arg: &Expr) {
        let message = format!(
            "`count` expects a value assignable to `{element}`, found `{}`",
            actual.display()
        );
        let shadowed = matches!(actual, ValueType::Known(Type::Bool))
            && !matches!(transparent_type(element), Type::Bool)
            && expr_referenced_identifiers(arg)
                .iter()
                .any(|name| self.lookup(name).is_some());
        if shadowed {
            let name = expr_referenced_identifiers(arg)
                .into_iter()
                .find(|name| self.lookup(name).is_some())
                .expect("shadowed implies at least one resolvable identifier");
            self.error_at_with_help(
                "E0308",
                message,
                arg.span,
                vec![format!(
                    "`{name}` already refers to a variable in this scope, so this is a constant condition, not a per-element one; rename `{name}` (or the outer variable) to use it as an implicit count condition"
                )],
            );
        } else {
            self.error_at("E0308", message, arg.span);
        }
    }
}

fn implicit_count_condition(free_name: &str, body: Expr) -> Expr {
    let param = Param {
        decorators: Vec::new(),
        pattern: ParamPattern::Name(free_name.to_string()),
        ty: Type::Infer,
        mutable: false,
        optional: false,
        default_value: None,
        rest: false,
        span: Span::DUMMY,
    };
    Expr::synthetic(ExprKind::ArrowFn {
        is_async: false,
        type_params: Vec::new(),
        params: vec![param],
        ret_type: Type::Infer,
        throws: Vec::new(),
        body: vec![Stmt::synthetic(StmtKind::Return(Some(body)))],
    })
}
