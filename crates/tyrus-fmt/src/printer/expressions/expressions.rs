use super::*;

impl Printer {
    #[allow(clippy::too_many_lines)]
    pub(super) fn print_expr(&mut self, expr: &Expr) {
        match expr.kind() {
            ExprKind::StrLit(s) => self.print_str_lit(expr, s),
            ExprKind::RegexLit { pattern, flags } => {
                self.push("/");
                self.push(&pattern.replace('/', "\\/"));
                self.push("/");
                self.push(flags);
            }
            ExprKind::NewSet { ty, initial } => {
                self.push("Set");
                if let Some(ty) = ty {
                    self.push("<");
                    self.print_type(ty);
                    self.push(">");
                }
                self.push("(");
                if let Some(initial) = initial {
                    self.print_expr(initial);
                }
                self.push(")");
            }
            ExprKind::NewMap {
                key,
                value,
                initial,
            } => {
                self.push("Map");
                if let (Some(key), Some(value)) = (key, value) {
                    self.push("<");
                    self.print_type(key);
                    self.push(", ");
                    self.print_type(value);
                    self.push(">");
                }
                self.push("(");
                if let Some(initial) = initial {
                    self.print_expr(initial);
                }
                self.push(")");
            }
            ExprKind::TemplateLit(parts) => self.print_template_lit(parts),
            ExprKind::IntLit(n) => self.push(&n.to_string()),
            ExprKind::UIntLit(n) => self.push(&n.to_string()),
            ExprKind::FloatLit(n) => {
                let s = format(n);
                self.push(&s);
            }
            ExprKind::DecimalLit(n) => {
                self.push(n.text());
                self.push("d");
            }
            ExprKind::BoolLit(true) => self.push("true"),
            ExprKind::BoolLit(false) => self.push("false"),
            ExprKind::Null => self.push("null"),
            ExprKind::None => self.push("None"),
            ExprKind::Topic => self.push("%"),
            ExprKind::ObjectLit(fields) => {
                self.print_object_lit(fields);
            }
            ExprKind::ObjectUpdate { base, fields } => {
                self.print_object_update(base, fields);
            }
            ExprKind::ArrayLit(elements) => {
                self.print_array_lit(elements);
            }
            ExprKind::ArrayRepeat { value, count } => {
                self.push("[");
                self.print_expr(value);
                self.push("; ");
                self.print_expr(count);
                self.push("]");
            }
            ExprKind::Ident(name) => self.push(name),
            ExprKind::FieldAccess { object, field } => {
                self.print_expr(object);
                self.push(".");
                self.push(field);
            }
            ExprKind::OptionalFieldAccess { object, field } => {
                self.print_expr(object);
                self.push("?.");
                self.push(field);
            }
            ExprKind::IndexAccess { object, index } => {
                self.print_expr(object);
                self.push("[");
                self.print_expr(index);
                self.push("]");
            }
            ExprKind::BinOp { op, left, right } => {
                self.print_binop(op, left, right);
            }
            ExprKind::ChainedComparison { terms, ops } => {
                self.print_chained_comparison(terms, ops);
            }
            ExprKind::Pipeline { left, right } => {
                self.print_pipeline(left, right);
            }
            ExprKind::Not(inner) => {
                if let ExprKind::Is { expr, ty } = inner.kind() {
                    self.print_expr(expr);
                    self.push(" not is ");
                    self.print_type(ty);
                } else {
                    self.push("!");
                    self.print_expr(inner);
                }
            }
            ExprKind::Neg(inner) => {
                self.push("-");
                self.print_expr(inner);
            }
            ExprKind::BitNot(inner) => {
                self.push("~");
                self.print_expr(inner);
            }
            ExprKind::Typeof(inner) => {
                self.push("typeof ");
                self.print_expr(inner);
            }
            ExprKind::Await(inner) => {
                self.push("await ");
                self.print_expr(inner);
            }
            ExprKind::Try(inner) => {
                self.print_expr(inner);
                self.push("?");
            }
            ExprKind::PrefixIncr(inner) => {
                self.push("++");
                self.print_expr(inner);
            }
            ExprKind::PrefixDecr(inner) => {
                self.push("--");
                self.print_expr(inner);
            }
            ExprKind::PostfixIncr(inner) => {
                self.print_expr(inner);
                self.push("++");
            }
            ExprKind::PostfixDecr(inner) => {
                self.print_expr(inner);
                self.push("--");
            }
            ExprKind::Ternary {
                cond,
                then_expr,
                else_expr,
            } => {
                let inline_size = 3
                    + expr_inline_len(cond)
                    + 3
                    + expr_inline_len(then_expr)
                    + 3
                    + expr_inline_len(else_expr);
                if self.will_exceed_width(inline_size) {
                    self.print_expr(cond);
                    self.indent += 1;
                    self.print_ternary_chain(then_expr, else_expr);
                    self.indent -= 1;
                } else {
                    self.print_expr(cond);
                    self.push(" ? ");
                    self.print_expr(then_expr);
                    self.push(" : ");
                    self.print_expr(else_expr);
                }
            }
            ExprKind::Borrow {
                mutable,
                name,
                place,
            } => {
                if *mutable {
                    self.push("&mut ");
                } else {
                    self.push("&");
                }
                match place {
                    Some(place) => self.print_expr(place),
                    None => self.push(name),
                }
            }
            ExprKind::Unwrap(inner) => {
                self.print_expr(inner);
                self.push("!");
            }
            ExprKind::OptionalMethodCall {
                object,
                method,
                type_args,
                args,
            } => {
                self.print_expr(object);
                self.push("?.");
                self.push(method);
                if !type_args.is_empty() {
                    self.push("<");
                    for (index, ty) in type_args.iter().enumerate() {
                        if index > 0 {
                            self.push(", ");
                        }
                        self.print_type(ty);
                    }
                    self.push(">");
                }
                self.push("(");
                self.print_args_or_list(args);
            }
            ExprKind::MethodCall {
                object,
                method,
                type_args,
                args,
                ..
            } => {
                self.print_expr(object);
                self.push(".");
                self.push(method);
                if !type_args.is_empty() {
                    self.push("<");
                    for (index, ty) in type_args.iter().enumerate() {
                        if index > 0 {
                            self.push(", ");
                        }
                        self.print_type(ty);
                    }
                    self.push(">");
                }
                self.push("(");
                self.print_args_or_list(args);
            }
            ExprKind::FnCall {
                callee,
                type_args,
                args,
            } => {
                self.push(callee);
                if !type_args.is_empty() {
                    self.push("<");
                    let mut first = true;
                    for ta in type_args {
                        if !first {
                            self.push(", ");
                        }
                        first = false;
                        self.print_type(ta);
                    }
                    self.push(">");
                }
                self.push("(");
                self.print_args_or_list(args);
            }
            ExprKind::CallValue { callee, args, .. } => {
                self.print_expr(callee);
                self.push("(");
                self.print_args_or_list(args);
            }
            ExprKind::ArrowFn {
                is_async,
                type_params,
                params,
                ret_type,
                throws,
                body,
            } => {
                if *is_async {
                    self.push("async ");
                }
                self.print_type_params(type_params);
                self.push("(");
                self.print_params(params);
                self.push(")");
                self.print_type_annotation(ret_type);
                self.print_throws_clause(throws);
                self.push(" =>");
                self.open_block();
                if !body.is_empty() {
                    self.print_body(body);
                    self.newline();
                }
                self.push("}");
            }
            ExprKind::Spread(inner) => {
                self.push("...");
                self.print_expr(inner);
            }
            ExprKind::TypeAssert { value, ty } => {
                if let (ExprKind::Ident(name), Type::Array(element)) = (value.kind(), ty)
                    && matches!(element.as_ref(), Type::Named(element_name) if element_name == name)
                {
                    self.push(name);
                    self.push("[]");
                    return;
                }
                if let Some(source) = self.typed_numeric_source(expr) {
                    self.push(&source);
                    return;
                }
                let needs_parens = expr_precedence(value).is_some();
                if needs_parens {
                    self.push("(");
                }
                self.print_expr(value);
                if needs_parens {
                    self.push(")");
                }
                self.push(" as ");
                self.print_type(ty);
            }
            ExprKind::ConstAssert(inner) => {
                self.print_expr(inner);
                self.push(" as const");
            }
            ExprKind::Comptime(inner) => {
                self.push("comptime(");
                self.print_expr(inner);
                self.push(")");
            }
            ExprKind::In { key, object } => {
                self.print_expr(key);
                self.push(" in ");
                self.print_expr(object);
            }
            ExprKind::Is { expr, ty } => {
                self.print_expr(expr);
                self.push(" is ");
                self.print_type(ty);
            }
            ExprKind::Range {
                start,
                end,
                inclusive,
            } => {
                self.print_expr(start);
                self.push(if *inclusive { "..=" } else { ".." });
                self.print_expr(end);
            }
            ExprKind::If {
                cond,
                then_block,
                else_block,
            } => {
                self.push("if (");
                self.print_expr(cond);
                self.push(")");
                self.open_block();
                self.print_body(then_block);
                self.newline();
                self.push("}");
                if let Some(else_body) = else_block {
                    if else_body.len() == 1
                        && let Stmt {
                            kind:
                                StmtKind::Expr(
                                    inner_if @ Expr {
                                        kind: ExprKind::If { .. },
                                        ..
                                    },
                                ),
                            ..
                        } = &else_body[0]
                    {
                        self.brace_keyword_sep();
                        self.push("else ");
                        self.print_expr(inner_if);
                        return;
                    }
                    self.brace_keyword_sep();
                    self.push("else");
                    self.open_block();
                    self.print_body(else_body);
                    self.newline();
                    self.push("}");
                }
            }
            ExprKind::Loop { body } => {
                self.push("loop");
                self.open_block();
                self.print_body(body);
                self.newline();
                self.push("}");
            }
        }
    }

    pub(super) fn print_char_literal_after_space(&mut self, ty: &Type, value: &Expr) -> bool {
        if !matches!(ty, Type::Char) || !matches!(value.kind(), ExprKind::StrLit(_)) {
            return false;
        }
        self.push(" ");
        self.print_char_literal(ty, value)
    }

    pub(super) fn print_char_literal(&mut self, ty: &Type, value: &Expr) -> bool {
        if !matches!(ty, Type::Char) {
            return false;
        }
        let ExprKind::StrLit(text) = value.kind() else {
            return false;
        };
        self.push(&encode_string(text, '\''));
        true
    }

    pub(super) fn print_template_lit(&mut self, parts: &[TemplatePart]) {
        self.push("`");
        for part in parts {
            match part {
                TemplatePart::Text(t) => self.push(t),
                TemplatePart::Expr(e, spec) => {
                    self.push("${");
                    self.print_expr(e);
                    if let Some(spec) = spec {
                        self.push(":");
                        self.push(&spec.as_written());
                    }
                    self.push("}");
                }
            }
        }
        self.push("`");
    }

    pub(super) fn print_expr_list(&mut self, args: &[Expr]) {
        let mut first = true;
        for arg in args {
            if !first {
                self.push(", ");
            }
            first = false;
            self.print_expr(arg);
        }
    }

    pub(super) fn print_array_lit(&mut self, elements: &[Expr]) {
        self.push("[");
        if elements.is_empty() {
            self.push("]");
            return;
        }
        if self.should_expand_array(elements) {
            self.newline_indent();
            let mut first = true;
            for elem in elements {
                if !first {
                    self.push(",");
                    self.newline();
                }
                first = false;
                self.print_expr(elem);
            }
            if self.has_trailing_comma() {
                self.push(",");
            }
            self.newline_dedent();
            self.push("]");
        } else {
            let mut first = true;
            for elem in elements {
                if !first {
                    self.push(", ");
                }
                first = false;
                self.print_expr(elem);
            }
            self.push("]");
        }
    }

    pub(super) fn print_object_field(&mut self, key: &str, val: &Expr) {
        if matches!(val.kind(), ExprKind::Ident(name) if name == key) {
            self.push(key);
        } else {
            self.push(key);
            self.push(": ");
            self.print_expr(val);
        }
    }

    pub(super) fn print_object_lit(&mut self, fields: &[(String, Expr)]) {
        self.push("{");
        if fields.is_empty() {
            self.push(" }");
            return;
        }
        if self.should_expand_object(fields) {
            self.newline_indent();
            let mut first = true;
            for (key, val) in fields {
                if !first {
                    self.push(",");
                    self.newline();
                }
                first = false;
                self.print_object_field(key, val);
            }
            if self.has_trailing_comma() {
                self.push(",");
            }
            self.newline_dedent();
            self.push("}");
        } else {
            self.push_space();
            let mut first = true;
            for (key, val) in fields {
                if !first {
                    self.push(", ");
                }
                first = false;
                self.print_object_field(key, val);
            }
            self.push_space();
            self.push("}");
        }
    }

    pub(super) fn print_object_update(&mut self, base: &Expr, fields: &[(String, Expr)]) {
        self.push("{ ...");
        self.print_expr(base);
        for (key, value) in fields {
            self.push(", ");
            self.print_object_field(key, value);
        }
        self.push(" }");
    }

    pub(super) fn should_expand_array(&self, elements: &[Expr]) -> bool {
        let base = match self.config.array_expand {
            crate::config::LiteralExpand::Off => return false,
            crate::config::LiteralExpand::ExpandAll => elements.len() > 1,
            crate::config::LiteralExpand::Simple => {
                elements.len() > 1 && elements.iter().any(|e| self.is_complex_expr(e))
            }
        };
        if base {
            return true;
        }
        if elements.len() <= 1 {
            return false;
        }
        let inner: usize = elements.iter().map(expr_inline_len).sum();
        let commas = 2 * (elements.len() - 1);
        self.will_exceed_width(1 + inner + commas)
    }

    pub(super) fn should_expand_object(&self, fields: &[(String, Expr)]) -> bool {
        let base = match self.config.object_expand {
            crate::config::LiteralExpand::Off => return false,
            crate::config::LiteralExpand::ExpandAll => fields.len() > 1,
            crate::config::LiteralExpand::Simple => {
                fields.len() > 1 && fields.iter().any(|(_, v)| self.is_complex_expr(v))
            }
        };
        if base {
            return true;
        }
        if fields.len() <= 1 {
            return false;
        }
        let space = if self.config.brace_spacing { 2 } else { 0 };
        let inner: usize = fields
            .iter()
            .map(|(k, v)| {
                if matches!(v.kind(), ExprKind::Ident(name) if name == k) {
                    k.len()
                } else {
                    k.len() + 2 + expr_inline_len(v)
                }
            })
            .sum();
        let commas = 2 * (fields.len() - 1);
        self.will_exceed_width(1 + space + inner + commas + space)
    }

    pub(super) fn is_complex_expr(&self, expr: &Expr) -> bool {
        matches!(
            expr,
            Expr {
                kind: ExprKind::FnCall { .. },
                ..
            } | Expr {
                kind: ExprKind::MethodCall { .. },
                ..
            } | Expr {
                kind: ExprKind::ObjectLit(..),
                ..
            } | Expr {
                kind: ExprKind::ArrayLit(..),
                ..
            } | Expr {
                kind: ExprKind::ArrayRepeat { .. },
                ..
            } | Expr {
                kind: ExprKind::TemplateLit(..),
                ..
            } | Expr {
                kind: ExprKind::BinOp { .. },
                ..
            } | Expr {
                kind: ExprKind::ArrowFn { .. },
                ..
            } | Expr {
                kind: ExprKind::If { .. },
                ..
            } | Expr {
                kind: ExprKind::OptionalMethodCall { .. },
                ..
            } | Expr {
                kind: ExprKind::Unwrap(..),
                ..
            } | Expr {
                kind: ExprKind::PrefixIncr(..),
                ..
            } | Expr {
                kind: ExprKind::PrefixDecr(..),
                ..
            } | Expr {
                kind: ExprKind::PostfixIncr(..),
                ..
            } | Expr {
                kind: ExprKind::PostfixDecr(..),
                ..
            } | Expr {
                kind: ExprKind::Not(..),
                ..
            } | Expr {
                kind: ExprKind::Neg(..),
                ..
            } | Expr {
                kind: ExprKind::Typeof(..),
                ..
            } | Expr {
                kind: ExprKind::Await(..),
                ..
            } | Expr {
                kind: ExprKind::Ternary { .. },
                ..
            } | Expr {
                kind: ExprKind::Spread(..),
                ..
            }
        )
    }
}
