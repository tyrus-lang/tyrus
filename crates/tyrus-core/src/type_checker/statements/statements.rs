use super::*;

#[path = "bindings/bindings.rs"]
mod bindings;
#[path = "branching/branching.rs"]
mod branching;
#[path = "flow_exits/flow_exits.rs"]
mod flow_exits;
#[path = "loops/loops.rs"]
mod loops;
impl TypeChecker {
    pub(super) fn check_stmt(&mut self, statement: &Stmt) {
        if self.is_cancelled() {
            return;
        }
        self.span_stack.push(statement.span);
        self.check_stmt_kind(statement);
        self.span_stack.pop();
    }

    pub(super) fn check_stmt_kind(&mut self, statement: &Stmt) {
        match statement.kind() {
            StmtKind::VarDecl { .. }
            | StmtKind::ArrayDestructure { .. }
            | StmtKind::ObjectDestructure { .. }
            | StmtKind::Assign { .. }
            | StmtKind::IndexAssign { .. }
            | StmtKind::FieldAssign { .. } => self.check_binding_stmt(statement),
            StmtKind::Return { .. } | StmtKind::Break { .. } | StmtKind::Continue => {
                self.check_flow_exit_stmt(statement);
            }
            StmtKind::While { .. }
            | StmtKind::DoWhile { .. }
            | StmtKind::For { .. }
            | StmtKind::ForOf { .. }
            | StmtKind::ForIn { .. } => self.check_loop_stmt(statement),
            StmtKind::Expr(expr) => {
                if self.check_panic_statement(expr) {
                } else if let Expr {
                    kind:
                        ExprKind::BinOp {
                            op: op @ (BinOp::And | BinOp::Or),
                            left,
                            right,
                        },
                    ..
                } = expr
                {
                    let left_ty = self.check_expr(left);
                    if !is_optional_value(&left_ty) {
                        self.require_bool(&left_ty, "left logical operand");
                    }
                    let narrow = null_narrowing(left);
                    self.check_expr_narrowed(right, narrow.as_ref(), matches!(op, BinOp::And));
                } else {
                    let value = self.check_expr(expr);
                    if matches!(value, ValueType::Known(ref ty) if promise_payload(ty).is_some()) {
                        self.error_at(
                            "E3005",
                            "unused `Promise<T>` may let asynchronous work outlive the program boundary"
                                .to_string(),
                            expr.span,
                        );
                    }
                    if matches!(value, ValueType::Known(ref ty) if join_handle_payload(ty).is_some())
                    {
                        self.error_at(
                            "E3005",
                            "unused `JoinHandle<T>` may let asynchronous work outlive the program boundary, and its result or rejection will never be observed"
                                .to_string(),
                            expr.span,
                        );
                    }
                }
            }
            StmtKind::Switch { .. }
            | StmtKind::Match { .. }
            | StmtKind::Throw { .. }
            | StmtKind::Defer { .. }
            | StmtKind::TryCatch { .. } => self.check_branching_stmt(statement),
        }
    }

    pub(super) fn check_block(&mut self, statements: &[Stmt]) {
        self.push_scope();
        for statement in statements {
            if self.is_cancelled() {
                break;
            }
            self.check_stmt(statement);
        }
        self.pop_scope();
    }

    pub(super) fn check_panic_statement(&mut self, expr: &Expr) -> bool {
        let ExprKind::FnCall {
            callee,
            type_args,
            args,
        } = expr.kind()
        else {
            return false;
        };
        if callee != "panic" || !type_args.is_empty() || self.has_symbol(callee) {
            return false;
        }
        if args.len() != 1 {
            self.error(
                "E0061",
                format!(
                    "`panic` expects 1 argument, but {} were provided",
                    args.len()
                ),
                Some(callee.as_str()),
            );
        }
        for (index, arg) in args.iter().enumerate() {
            let actual = self.check_expr(arg);
            if index == 0 {
                self.require_assignable(&Type::Str, &actual, callee, "E0308");
            }
        }
        true
    }
}
