use super::*;

impl TypeChecker {
    pub(super) fn check_loop_stmt(&mut self, statement: &Stmt) {
        match statement.kind() {
            StmtKind::While { cond, body } => {
                self.invalidate_loop_mutations(body);
                let condition = self.check_expr(cond);
                if !is_optional_value(&condition) {
                    self.require_bool(&condition, "while condition");
                }
                self.control_stack.push(ControlContext::Loop);
                self.branch_depth += 1;
                self.check_block(body);
                self.branch_depth -= 1;
                self.control_stack.pop();
                self.check_busy_wait_loop(cond, body, statement.span);
            }
            StmtKind::DoWhile { body, cond } => {
                self.invalidate_loop_mutations(body);
                self.control_stack.push(ControlContext::Loop);
                self.branch_depth += 1;
                self.check_block(body);
                self.branch_depth -= 1;
                self.control_stack.pop();
                let condition = self.check_expr(cond);
                if !is_optional_value(&condition) {
                    self.require_bool(&condition, "do-while condition");
                }
                self.check_busy_wait_loop(cond, body, statement.span);
            }
            StmtKind::For {
                init,
                cond,
                update,
                body,
            } => {
                self.push_scope();
                self.invalidate_loop_mutations(body);
                self.for_index_hint = self.for_index_hint(init, cond);
                self.check_stmt(init);
                self.for_index_hint = None;
                let condition = self.check_expr(cond);
                if !is_optional_value(&condition) {
                    self.require_bool(&condition, "for condition");
                }
                self.check_stmt(update);
                self.control_stack.push(ControlContext::Loop);
                self.branch_depth += 1;
                self.check_block(body);
                self.branch_depth -= 1;
                self.control_stack.pop();
                self.pop_scope();
            }
            StmtKind::ForOf {
                mutable,
                name,
                iterable,
                body,
            } => {
                let iterable_ty = self.check_expr(iterable);
                if let Some(elements) = tuple_positions(&iterable_ty)
                    && matches!(tuple_sequence(elements), TupleSequence::Mixed { .. })
                {
                    self.error(
                        "E0277",
                        format!(
                            "cannot iterate `{}`: its positions have different types",
                            iterable_ty.display()
                        ),
                        expression_subject(iterable),
                    );
                    return;
                }
                let element_ty = match &iterable_ty {
                    ValueType::Known(Type::Array(element)) => Some(element.as_ref().clone()),
                    ValueType::Known(Type::Tuple(elements)) => match tuple_sequence(elements) {
                        TupleSequence::Uniform { element } => Some(element),
                        TupleSequence::Mixed { .. } | TupleSequence::Empty => None,
                    },
                    ValueType::Known(Type::Readonly(inner)) => match inner.as_ref() {
                        Type::Array(element) => Some(deep_readonly(element.as_ref().clone())),
                        Type::Tuple(elements) => match tuple_sequence(elements) {
                            TupleSequence::Uniform { element } => Some(deep_readonly(element)),
                            TupleSequence::Mixed { .. } | TupleSequence::Empty => None,
                        },
                        _ => None,
                    },
                    ValueType::Array(values) => infer_array_element_type(values),
                    ValueType::Known(
                        Type::Set(element)
                        | Type::BinaryHeap(element)
                        | Type::BinomialHeap(element)
                        | Type::UnionFind(element)
                        | Type::Counter(element)
                        | Type::Deque(element),
                    ) => Some(element.as_ref().clone()),
                    ValueType::Known(Type::Map(key, value)) => Some(Type::Tuple(vec![
                        key.as_ref().clone(),
                        value.as_ref().clone(),
                    ])),
                    ValueType::Known(Type::Range(element)) => Some(element.as_ref().clone()),
                    ValueType::Error | ValueType::Unknown => Some(Type::Unknown),
                    _ => None,
                };

                let Some(element_ty) = element_ty else {
                    self.error(
                        "E0308",
                        format!(
                            "for...of requires an array, tuple, set, map or Range, found `{}`",
                            iterable_ty.display()
                        ),
                        expression_subject(iterable),
                    );
                    return;
                };

                self.invalidate_loop_mutations(body);
                self.push_scope();
                self.declare(
                    name,
                    Symbol {
                        ty: ValueType::known(&element_ty),
                        mutable: *mutable,
                        kind: SymbolKind::Variable,
                        used: false,
                        array_len: None,
                    },
                );
                self.control_stack.push(ControlContext::Loop);
                self.branch_depth += 1;
                self.check_block(body);
                self.branch_depth -= 1;
                self.control_stack.pop();
                self.pop_scope();
            }
            StmtKind::ForIn { name, object, body } => {
                let obj_ty = self.check_expr(object);
                let has_fields = match &obj_ty {
                    ValueType::Known(Type::Named(n)) => self.interfaces.contains_key(n.as_str()),
                    ValueType::Known(Type::Object(_)) => true,
                    _ => false,
                };
                if !has_fields {
                    self.error(
                        "E0308",
                        format!(
                            "for...in requires an object type, found `{}`",
                            obj_ty.display()
                        ),
                        expression_subject(object),
                    );
                    return;
                }
                self.invalidate_loop_mutations(body);
                self.push_scope();
                self.declare(
                    name,
                    Symbol {
                        ty: ValueType::Known(Type::Str),
                        mutable: false,
                        kind: SymbolKind::Variable,
                        used: false,
                        array_len: None,
                    },
                );
                self.control_stack.push(ControlContext::Loop);
                self.branch_depth += 1;
                self.check_block(body);
                self.branch_depth -= 1;
                self.control_stack.pop();
                self.pop_scope();
            }
            _ => {}
        }
    }
}
