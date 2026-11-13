use super::*;

impl Printer {
    pub(super) fn open_block(&mut self) {
        match self.config.brace_style {
            crate::config::BraceStyle::Off => {
                if self
                    .source_block_brace_newlines
                    .pop_front()
                    .unwrap_or(false)
                {
                    self.newline();
                } else {
                    self.push_space();
                }
            }
            crate::config::BraceStyle::SameLine => self.push_space(),
            crate::config::BraceStyle::NextLine => self.newline(),
        }
        self.push("{");
    }

    pub(super) fn print_field_block(&mut self, fields: &[Field]) {
        self.push("{");
        if fields.is_empty() {
            self.push(" }");
            return;
        }
        self.newline_indent();
        for (i, field) in fields.iter().enumerate() {
            if i > 0 {
                self.push(";");
                self.newline();
                if self.config.blank_lines_around_decorations
                    && (!field.decorators.is_empty() || !fields[i - 1].decorators.is_empty())
                {
                    self.newline();
                }
            }
            for decorator in &field.decorators {
                self.print_decorator(decorator);
                self.newline();
            }
            if field.readonly {
                self.push("readonly ");
            }
            self.push(&field.name);
            if field.optional {
                self.push("?");
            }
            self.push(": ");
            self.print_type(&field.ty);
        }
        self.push(";");
        self.newline_dedent();
        self.push("}");
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn print_stmt(&mut self, stmt: &Stmt, top_level: bool) {
        match stmt.kind() {
            StmtKind::VarDecl {
                mutable,
                name,
                ty,
                value,
                onfail,
            } => {
                let mut annotation = ty;
                if *mutable {
                    self.push("let ");
                } else if let (true, Type::Readonly(inner)) = (self.readonly_keyword_allowed, ty) {
                    self.push("readonly const ");
                    annotation = inner.as_ref();
                } else {
                    self.push("const ");
                }
                self.push(name);
                self.print_type_annotation(annotation);
                if let Some(value) = value {
                    self.push(" =");
                    if onfail.is_none() && self.print_char_literal_after_space(ty, value) {
                        if top_level {
                            self.push(";");
                        }
                        return;
                    }
                    if !Self::is_block_expr(value)
                        && self.will_exceed_width(1 + expr_inline_len(value) + 1)
                    {
                        self.indent += 1;
                        self.newline();
                        self.print_expr(value);
                        self.indent -= 1;
                    } else {
                        self.push(" ");
                        self.print_expr(value);
                    }
                }
                if let Some(onfail) = onfail {
                    self.push(" onfail ");
                    match &onfail.handler {
                        OnFailHandler::Expr(handler) => self.print_expr(handler),
                        OnFailHandler::Block(body) => {
                            self.open_block();
                            if !body.is_empty() {
                                self.print_body(body);
                                self.newline();
                            }
                            self.push("}");
                        }
                    }
                }
                if top_level {
                    self.push(";");
                }
            }
            StmtKind::Assign { name, op, value } => {
                self.push(name);
                let op_str = Self::assign_op_str(op);
                self.push(op_str.trim_end());
                if !Self::is_block_expr(value)
                    && self.will_exceed_width(1 + expr_inline_len(value) + 1)
                {
                    self.indent += 1;
                    self.newline();
                    self.print_expr(value);
                    self.indent -= 1;
                } else {
                    self.push(" ");
                    self.print_expr(value);
                }
                if top_level {
                    self.push(";");
                }
            }
            StmtKind::IndexAssign { name, index, value } => {
                self.push(name);
                self.push("[");
                self.print_expr(index);
                self.push("] = ");
                self.print_expr(value);
                if top_level {
                    self.push(";");
                }
            }
            StmtKind::FieldAssign {
                object,
                field,
                op,
                value,
            } => {
                self.print_expr(object);
                self.push(".");
                self.push(field);
                let op_str = Self::assign_op_str(op);
                self.push(op_str.trim_end());
                self.push(" ");
                self.print_expr(value);
                if top_level {
                    self.push(";");
                }
            }
            StmtKind::Return(expr) => {
                self.push("return");
                if let Some(e) = expr {
                    let extra = 1 + expr_inline_len(e) + 1;
                    let is_call = matches!(
                        e.kind(),
                        ExprKind::FnCall { .. }
                            | ExprKind::MethodCall { .. }
                            | ExprKind::OptionalMethodCall { .. }
                            | ExprKind::CallValue { .. }
                    );
                    if !is_call && self.will_exceed_width(extra) {
                        self.indent += 1;
                        self.newline();
                        self.print_expr(e);
                        self.indent -= 1;
                    } else {
                        self.push(" ");
                        self.print_expr(e);
                    }
                }
                self.push(";");
            }
            StmtKind::Break(value) => {
                self.push("break");
                if let Some(value) = value {
                    self.push(" ");
                    self.print_expr(value);
                }
                self.push(";");
            }
            StmtKind::Continue => self.push("continue;"),
            StmtKind::While { cond, body } => {
                self.push("while (");
                self.print_expr(cond);
                self.push(")");
                self.open_block();
                self.print_body(body);
                self.newline();
                self.push("}");
            }
            StmtKind::DoWhile { body, cond } => {
                self.push("do");
                self.open_block();
                self.print_body(body);
                self.newline();
                self.push("}");
                self.brace_keyword_sep();
                self.push("while (");
                self.print_expr(cond);
                self.push(");");
            }
            StmtKind::For {
                init,
                cond,
                update,
                body,
            } => {
                self.push("for (");
                self.readonly_keyword_allowed = false;
                self.print_stmt(init, false);
                self.readonly_keyword_allowed = true;
                self.push("; ");
                self.print_expr(cond);
                self.push("; ");
                self.print_stmt(update, false);
                self.push(")");
                self.open_block();
                self.print_body(body);
                self.newline();
                self.push("}");
            }
            StmtKind::ForOf {
                mutable,
                name,
                iterable,
                body,
            } => {
                self.push("for (");
                if *mutable {
                    self.push("let ");
                } else {
                    self.push("const ");
                }
                self.push(name);
                self.push(" of ");
                self.print_expr(iterable);
                self.push(")");
                self.open_block();
                self.print_body(body);
                self.newline();
                self.push("}");
            }
            StmtKind::ForIn { name, object, body } => {
                self.push("for (const ");
                self.push(name);
                self.push(" in ");
                self.print_expr(object);
                self.push(")");
                self.open_block();
                self.print_body(body);
                self.newline();
                self.push("}");
            }
            StmtKind::ArrayDestructure {
                mutable,
                bindings,
                value,
            } => {
                if *mutable {
                    self.push("let ");
                } else {
                    self.push("const ");
                }
                self.push("[");
                for (i, binding) in bindings.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.push(&binding.name);
                }
                self.push("] = ");
                self.print_expr(value);
                if top_level {
                    self.push(";");
                }
            }
            StmtKind::ObjectDestructure {
                mutable,
                bindings,
                value,
            } => {
                if *mutable {
                    self.push("let ");
                } else {
                    self.push("const ");
                }
                self.push("{");
                for (i, binding) in bindings.iter().enumerate() {
                    if i > 0 {
                        self.push(", ");
                    }
                    self.push(&binding.name);
                }
                self.push("} = ");
                self.print_expr(value);
                if top_level {
                    self.push(";");
                }
            }
            StmtKind::Switch {
                expr,
                cases,
                default,
                default_index,
            } => {
                self.push("switch (");
                self.print_expr(expr);
                self.push(")");
                self.open_block();
                self.indent += 1;
                for (index, case) in cases.iter().enumerate() {
                    if default_index == &Some(index) {
                        self.newline();
                        self.push("default:");
                        self.print_body(
                            default
                                .as_deref()
                                .expect("switch default index must have a body"),
                        );
                    }
                    self.newline();
                    self.push("case ");
                    self.print_expr(&case.value);
                    self.push(":");
                    self.print_body(&case.body);
                }
                if default_index == &Some(cases.len()) {
                    self.newline();
                    self.push("default:");
                    self.print_body(
                        default
                            .as_deref()
                            .expect("switch default index must have a body"),
                    );
                }
                self.indent -= 1;
                self.newline();
                self.push("}");
            }
            StmtKind::Match {
                expr,
                arms,
                default: _,
            } => {
                self.push("match (");
                self.print_expr(expr);
                self.push(")");
                self.open_block();
                self.indent += 1;
                for arm in arms {
                    self.newline();
                    self.print_match_pattern(&arm.pattern);
                    if let Some(guard) = &arm.guard {
                        self.push(" if (");
                        self.print_expr(guard);
                        self.push(")");
                    }
                    self.push(" =>");
                    if arm.compact {
                        let [
                            Stmt {
                                kind: StmtKind::Return(Some(expression)),
                                ..
                            },
                        ] = arm.body.as_slice()
                        else {
                            unreachable!("compact match arm must contain its implicit return");
                        };
                        self.push(" ");
                        self.print_expr(expression);
                        self.push(";");
                    } else {
                        self.open_block();
                        self.print_body(&arm.body);
                        self.newline();
                        self.push("}");
                    }
                }
                self.indent -= 1;
                self.newline();
                self.push("}");
            }
            StmtKind::Expr(expr) => {
                let is_if = matches!(expr.kind(), ExprKind::If { .. });
                self.print_expr(expr);
                if top_level && !is_if {
                    self.push(";");
                }
            }
            StmtKind::Throw(expr) => {
                self.push("throw ");
                self.print_expr(expr);
                self.push(";");
            }
            StmtKind::TryCatch {
                try_body,
                catch_binding,
                catch_type,
                catch_type_span,
                catch_body,
                finally_body,
            } => {
                self.push("try");
                self.open_block();
                self.print_body(try_body);
                self.newline();
                self.push("}");
                self.brace_keyword_sep();
                self.push("catch");
                self.push_space();
                self.push("(");
                self.push(catch_binding);
                if catch_type_span.start != catch_type_span.end {
                    self.push(": ");
                    self.push(&format!("{catch_type}"));
                }
                self.push(")");
                self.open_block();
                self.print_body(catch_body);
                self.newline();
                self.push("}");
                if let Some(stmts) = finally_body {
                    self.brace_keyword_sep();
                    self.push("finally");
                    self.open_block();
                    self.print_body(stmts);
                    self.newline();
                    self.push("}");
                }
            }
            StmtKind::Defer { kind, body, .. } => {
                self.push(match kind {
                    DeferKind::Always => "defer",
                    DeferKind::OnError => "errdefer",
                });
                self.open_block();
                self.print_body(body);
                self.newline();
                self.push("}");
            }
        }
    }

    pub(super) fn is_block_stmt(&mut self, stmt: &Stmt) -> bool {
        match stmt.kind() {
            StmtKind::While { .. }
            | StmtKind::DoWhile { .. }
            | StmtKind::For { .. }
            | StmtKind::ForOf { .. }
            | StmtKind::ForIn { .. }
            | StmtKind::Switch { .. }
            | StmtKind::Match { .. }
            | StmtKind::TryCatch { .. }
            | StmtKind::Defer { .. } => true,
            StmtKind::Expr(Expr {
                kind: ExprKind::If { .. },
                ..
            }) => true,
            StmtKind::VarDecl {
                mutable,
                name,
                ty,
                value: Some(value),
                ..
            } => {
                let kw = if *mutable {
                    "let ".len()
                } else {
                    "const ".len()
                };
                let ann = if matches!(ty, Type::Infer) {
                    0
                } else {
                    2 + type_inline_len(ty)
                };
                self.value_renders_multiline(value, kw + name.len() + ann + 3)
            }
            StmtKind::Assign { name, op, value } => {
                self.value_renders_multiline(value, name.len() + Self::assign_op_str(op).len())
            }
            _ => false,
        }
    }

    pub(super) fn is_block_expr(expr: &Expr) -> bool {
        matches!(
            expr,
            Expr {
                kind: ExprKind::ObjectLit(_),
                ..
            } | Expr {
                kind: ExprKind::ObjectUpdate { .. },
                ..
            } | Expr {
                kind: ExprKind::ArrayLit(_),
                ..
            } | Expr {
                kind: ExprKind::ArrayRepeat { .. },
                ..
            }
        )
    }

    pub(super) fn is_block_item(&mut self, item: &Item) -> bool {
        match item {
            Item::FnDecl { .. } | Item::InterfaceDecl { .. } | Item::EnumDecl { .. } => true,
            Item::Stmt(stmt) => self.is_block_stmt(stmt),
            _ => false,
        }
    }

    pub(super) fn print_match_pattern(&mut self, pattern: &MatchPattern) {
        match pattern {
            MatchPattern::Lit(expr) => self.print_expr(expr),
            MatchPattern::Wildcard => self.push("_"),
            MatchPattern::Binding(name) => self.push(name),
            MatchPattern::ResultVariant { success, binding } => {
                self.push(if *success { "Ok(" } else { "Err(" });
                if let Some(binding) = binding {
                    self.push(binding);
                }
                self.push(")");
            }
            MatchPattern::Object(fields) => {
                self.push("({ ");
                let mut first = true;
                for field in fields {
                    if !first {
                        self.push(", ");
                    }
                    first = false;
                    self.push(&field.key);
                    if let Some(lit) = &field.literal {
                        self.push(": ");
                        self.print_expr(lit);
                    } else if let Some(binding) = &field.binding
                        && binding != &field.key
                    {
                        self.push(": ");
                        self.push(binding);
                    }
                }
                self.push(" })");
            }
            MatchPattern::Array { elements, .. } => {
                self.push("[");
                let mut first = true;
                for elem in elements {
                    if !first {
                        self.push(", ");
                    }
                    first = false;
                    if elem.is_rest() {
                        self.push("...");
                    }
                    self.print_match_pattern(&elem.pattern);
                }
                self.push("]");
            }
            MatchPattern::Or(patterns) => {
                let mut first = true;
                for p in patterns {
                    if !first {
                        self.push(" | ");
                    }
                    first = false;
                    self.print_match_pattern(p);
                }
            }
            MatchPattern::EnumVariant {
                enum_name,
                variant,
                binding,
            } => {
                self.push(enum_name);
                self.push(".");
                self.push(variant);
                if let Some(binding) = binding {
                    match binding.as_ref() {
                        MatchPattern::Tuple(elements) => {
                            self.push("(");
                            for (index, element) in elements.iter().enumerate() {
                                if index > 0 {
                                    self.push(", ");
                                }
                                self.print_match_pattern(element);
                            }
                            self.push(")");
                        }
                        MatchPattern::Object(fields) => {
                            self.push(" { ");
                            for (index, field) in fields.iter().enumerate() {
                                if index > 0 {
                                    self.push(", ");
                                }
                                self.push(&field.key);
                                if let Some(literal) = &field.literal {
                                    self.push(": ");
                                    self.print_expr(literal);
                                } else if let Some(name) = &field.binding
                                    && name != &field.key
                                {
                                    self.push(": ");
                                    self.push(name);
                                }
                            }
                            self.push(" }");
                        }
                        _ => {
                            self.push("(");
                            self.print_match_pattern(binding);
                            self.push(")");
                        }
                    }
                }
            }
            MatchPattern::Tuple(elements) => {
                self.push("(");
                let mut first = true;
                for element in elements {
                    if !first {
                        self.push(", ");
                    }
                    first = false;
                    self.print_match_pattern(element);
                }
                self.push(")");
            }
            MatchPattern::Range {
                start,
                end,
                inclusive,
            } => {
                if let Some(start) = start {
                    self.print_expr(start);
                }
                self.push(if *inclusive { "..=" } else { ".." });
                if let Some(end) = end {
                    self.print_expr(end);
                }
            }
        }
    }
}
