use super::*;

impl Parser {
    pub(super) fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        self.enter_recursion("block")?;
        self.expect(&Token::LBrace)?;
        let mut stmts = vec![];

        while !matches!(self.peek(), Some(Token::RBrace) | None) {
            let stmt = self.parse_stmt()?;
            stmts.push(stmt);
        }

        self.expect(&Token::RBrace)?;
        self.depth -= 1;
        Ok(stmts)
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn parse_stmt(&mut self) -> Result<Stmt, String> {
        let __stmt_start = self.start_span();
        let __start = self.start_span();
        match self.peek() {
            Some(Token::Break) => {
                self.advance();
                let value = if matches!(
                    self.peek(),
                    None | Some(Token::Semicolon | Token::RBrace | Token::RParen)
                ) {
                    None
                } else {
                    Some(self.parse_expr()?)
                };
                self.expect_semicolon("break")?;
                Ok(StmtKind::Break(value).spanned(self.finish_span(__stmt_start)))
            }

            Some(Token::Continue) => {
                self.advance();
                self.expect_semicolon("continue")?;
                Ok(StmtKind::Continue.spanned(self.finish_span(__stmt_start)))
            }

            Some(Token::Defer | Token::ErrDefer) => {
                let kind = if matches!(self.peek(), Some(Token::Defer)) {
                    DeferKind::Always
                } else {
                    DeferKind::OnError
                };
                let keyword_span = self.current_span();
                self.advance();
                let body = if matches!(self.peek(), Some(Token::LBrace)) {
                    self.parse_block()?
                } else {
                    let expression_start = self.start_span();
                    let expression = self.parse_expr()?;
                    self.expect_semicolon("deferred statement")?;
                    vec![StmtKind::Expr(expression).spanned(self.finish_span(expression_start))]
                };
                Ok(StmtKind::Defer {
                    kind,
                    keyword_span,
                    body,
                }
                .spanned(self.finish_span(__stmt_start)))
            }

            Some(Token::While) => {
                self.advance();
                self.expect(&Token::LParen)?;
                let cond = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                let body = self.parse_block()?;
                Ok(StmtKind::While { cond, body }.spanned(self.finish_span(__stmt_start)))
            }

            Some(Token::Do) => {
                self.advance();
                let body = self.parse_block()?;
                self.expect(&Token::While)?;
                self.expect(&Token::LParen)?;
                let cond = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                self.skip_semicolon();
                Ok(StmtKind::DoWhile { body, cond }.spanned(self.finish_span(__stmt_start)))
            }

            Some(Token::For) => {
                self.advance();
                self.expect(&Token::LParen)?;
                if matches!(self.peek(), Some(Token::Const | Token::Let)) {
                    let mark = self.mark();
                    let mutable = matches!(self.peek(), Some(Token::Let));
                    self.advance();
                    if matches!(self.peek(), Some(Token::LBracket | Token::LBrace)) {
                        let is_object = matches!(self.peek(), Some(Token::LBrace));
                        let bindings = if is_object {
                            self.parse_object_destructure_bindings(mutable)?
                        } else {
                            self.parse_array_destructure_bindings(mutable)?
                        };
                        if matches!(self.peek(), Some(Token::In)) {
                            return Err("`for...in` does not support destructuring; its keys are strings, use `for...of` to destructure elements, or bind a single name".to_string());
                        }
                        self.expect(&Token::Of)?;
                        let iterable = self.parse_expr()?;
                        self.expect(&Token::RParen)?;
                        let mut body = self.parse_block()?;
                        let item = format!("__tyrus_forof_item_{}", self.pos);
                        let destructure = if is_object {
                            StmtKind::ObjectDestructure {
                                mutable,
                                bindings,
                                value: ExprKind::Ident(item.clone())
                                    .spanned(self.finish_span(__start)),
                            }
                        } else {
                            StmtKind::ArrayDestructure {
                                mutable,
                                bindings,
                                value: ExprKind::Ident(item.clone())
                                    .spanned(self.finish_span(__start)),
                            }
                        };
                        body.insert(0, destructure.spanned(self.finish_span(__stmt_start)));
                        return Ok(StmtKind::ForOf {
                            mutable: false,
                            name: item,
                            iterable,
                            body,
                        }
                        .spanned(self.finish_span(__stmt_start)));
                    }
                    let name_span = self.token_ranges.get(self.pos).cloned();
                    if let Some(Token::Ident(name)) = self.peek().cloned() {
                        self.advance();
                        if matches!(self.peek(), Some(Token::Of)) {
                            self.advance();
                            self.reject_reserved_name(&name, name_span)?;
                            let iterable = self.parse_expr()?;
                            self.expect(&Token::RParen)?;
                            let body = self.parse_block()?;
                            return Ok(StmtKind::ForOf {
                                mutable,
                                name,
                                iterable,
                                body,
                            }
                            .spanned(self.finish_span(__stmt_start)));
                        }
                        if matches!(self.peek(), Some(Token::In)) {
                            self.advance();
                            self.reject_reserved_name(&name, name_span)?;
                            let object = self.parse_expr()?;
                            self.expect(&Token::RParen)?;
                            let body = self.parse_block()?;
                            return Ok(StmtKind::ForIn { name, object, body }
                                .spanned(self.finish_span(__stmt_start)));
                        }
                    }
                    self.reset(mark);
                }
                let init = Box::new(self.parse_stmt()?);
                let cond = self.parse_expr()?;
                self.expect(&Token::Semicolon)?;
                let update = Box::new(self.parse_stmt_without_terminator()?);
                self.expect(&Token::RParen)?;
                let body = self.parse_block()?;

                if !matches!(
                    init.as_ref().kind(),
                    StmtKind::VarDecl { .. } | StmtKind::Assign { .. }
                ) {
                    return Err(
                        "for initializer must be a variable declaration or assignment".to_string(),
                    );
                }
                let is_update = matches!(
                    update.as_ref(),
                    Stmt {
                        kind: StmtKind::Assign { .. },
                        ..
                    } | Stmt {
                        kind: StmtKind::Expr(Expr {
                            kind: ExprKind::PrefixIncr(_),
                            ..
                        }),
                        ..
                    } | Stmt {
                        kind: StmtKind::Expr(Expr {
                            kind: ExprKind::PrefixDecr(_),
                            ..
                        }),
                        ..
                    } | Stmt {
                        kind: StmtKind::Expr(Expr {
                            kind: ExprKind::PostfixIncr(_),
                            ..
                        }),
                        ..
                    } | Stmt {
                        kind: StmtKind::Expr(Expr {
                            kind: ExprKind::PostfixDecr(_),
                            ..
                        }),
                        ..
                    }
                );
                if !is_update {
                    return Err(
                        "for update must be an assignment or increment/decrement".to_string()
                    );
                }

                Ok(StmtKind::For {
                    init,
                    cond,
                    update,
                    body,
                }
                .spanned(self.finish_span(__stmt_start)))
            }

            Some(Token::Switch) => self.parse_switch_stmt(),

            Some(Token::Match) => self.parse_match_stmt(),

            Some(Token::Readonly | Token::Const | Token::Let) => {
                let is_readonly = self.consume_readonly_binding_prefix()?;
                let mutable = matches!(self.peek(), Some(Token::Let));
                self.advance();
                self.reject_trailing_readonly_binding_prefix()?;
                if is_readonly && mutable {
                    return Err(READONLY_LET_MESSAGE.to_string());
                }

                if matches!(self.peek(), Some(Token::LBracket)) {
                    if is_readonly {
                        return Err(READONLY_DESTRUCTURE_MESSAGE.to_string());
                    }
                    return self.parse_array_destructure_stmt(mutable);
                }
                if matches!(self.peek(), Some(Token::LBrace)) {
                    if is_readonly {
                        return Err(READONLY_DESTRUCTURE_MESSAGE.to_string());
                    }
                    return self.parse_object_destructure_stmt(mutable);
                }

                let name = self.expect_binding_ident(BindingKind::Variable)?;
                if is_readonly
                    && matches!(self.peek(), Some(Token::Eq))
                    && (self.arrow_signature_follows_eq()
                        || matches!(self.peek2(), Some(Token::Async | Token::Function)))
                {
                    return Err(READONLY_FUNCTION_MESSAGE.to_string());
                }

                if matches!(self.peek(), Some(Token::Eq)) && self.arrow_signature_follows_eq() {
                    self.advance();
                    let (type_params, params, ret_type, throws, body) =
                        self.parse_arrow_fn_parts()?;
                    self.expect_semicolon("declaration")?;
                    return Ok(StmtKind::VarDecl {
                        mutable,
                        name,
                        ty: Type::Infer,
                        value: Some(
                            ExprKind::ArrowFn {
                                is_async: false,
                                type_params,
                                params,
                                ret_type,
                                throws,
                                body,
                            }
                            .spanned(self.finish_span(__start)),
                        ),
                        onfail: None,
                    }
                    .spanned(self.finish_span(__stmt_start)));
                }

                let ty = self.parse_var_decl_type()?;
                let ty = if is_readonly {
                    readonly_binding_type(ty)?
                } else {
                    ty
                };

                let (value, onfail) = self.parse_var_decl_value(mutable)?;
                self.expect_semicolon("declaration")?;

                Ok(StmtKind::VarDecl {
                    mutable,
                    name,
                    ty,
                    value,
                    onfail,
                }
                .spanned(self.finish_span(__stmt_start)))
            }

            Some(Token::Return) => {
                self.advance();
                if matches!(self.peek(), Some(Token::Semicolon | Token::RBrace) | None) {
                    self.expect_semicolon("return")?;
                    Ok(StmtKind::Return(None).spanned(self.finish_span(__stmt_start)))
                } else if matches!(self.peek(), Some(Token::TyVoid)) {
                    self.advance();
                    self.expect_semicolon("return")?;
                    Ok(StmtKind::Return(None).spanned(self.finish_span(__stmt_start)))
                } else {
                    let expr = self.parse_expr()?;
                    self.expect_semicolon("return")?;
                    Ok(StmtKind::Return(Some(expr)).spanned(self.finish_span(__stmt_start)))
                }
            }

            Some(Token::Throw) => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect_semicolon("throw")?;
                Ok(StmtKind::Throw(expr).spanned(self.finish_span(__stmt_start)))
            }

            Some(Token::Try) => {
                self.advance();
                let try_body = self.parse_block()?;
                self.expect(&Token::Catch)?;
                self.expect(&Token::LParen)?;
                let catch_binding = self.expect_binding_ident(BindingKind::Catch)?;
                let (catch_type, catch_type_span) = if matches!(self.peek(), Some(Token::Colon)) {
                    self.advance();
                    let __catch_type_start = self.start_span();
                    let catch_type = self.parse_type()?;
                    (catch_type, self.finish_span(__catch_type_start))
                } else {
                    let end = self.previous_span().end;
                    (Type::Named("Error".to_string()), Span::new(end, end))
                };
                self.expect(&Token::RParen)?;
                let catch_body = self.parse_block()?;
                let finally_body = if matches!(self.peek(), Some(Token::Finally)) {
                    self.advance();
                    Some(self.parse_block()?)
                } else {
                    None
                };
                Ok(StmtKind::TryCatch {
                    try_body,
                    catch_binding,
                    catch_type,
                    catch_type_span,
                    catch_body,
                    finally_body,
                }
                .spanned(self.finish_span(__stmt_start)))
            }

            Some(Token::Ident(_)) => {
                let stmt_start = self.mark();
                let name = match self.advance() {
                    Some(Token::Ident(s)) => s.clone(),
                    _ => unreachable!(),
                };

                if matches!(self.peek(), Some(Token::LBracket)) {
                    let mark = self.mark();
                    self.advance();
                    let index = self.parse_expr()?;
                    self.expect(&Token::RBracket)?;
                    if matches!(self.peek(), Some(Token::Eq)) {
                        self.advance();
                        let value = self.parse_expr()?;
                        self.expect_semicolon("assignment")?;
                        return Ok(StmtKind::IndexAssign { name, index, value }
                            .spanned(self.finish_span(__stmt_start)));
                    }
                    self.reset(mark);
                }

                let assign_op = self.parse_assign_op();

                if let Some(op) = assign_op {
                    let value = self.parse_expr()?;
                    self.expect_semicolon("assignment")?;
                    return Ok(StmtKind::Assign { name, op, value }
                        .spanned(self.finish_span(__stmt_start)));
                }

                self.reset(stmt_start);
                let expr = self.parse_expr()?;
                if let Some(op) = self.parse_assign_op() {
                    return self.finish_place_assignment(expr, op, __stmt_start);
                }
                self.expect_statement_terminator("statement")?;
                Ok(StmtKind::Expr(expr).spanned(self.finish_span(__stmt_start)))
            }

            _ => {
                let expr = self.parse_expr()?;
                self.expect_statement_terminator("statement")?;
                Ok(StmtKind::Expr(expr).spanned(self.finish_span(__stmt_start)))
            }
        }
    }

    pub(super) fn parse_format_spec(text: &str) -> Option<FormatSpec> {
        let is_digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
        if let Some(digits) = text.strip_prefix('.').and_then(|s| s.strip_suffix('f')) {
            return is_digits(digits)
                .then(|| digits.parse().ok())
                .flatten()
                .map(FormatSpec::FixedPoint);
        }
        if let Some(digits) = text.strip_prefix('0').and_then(|s| s.strip_suffix('d')) {
            return is_digits(digits)
                .then(|| format!("0{digits}").parse().ok())
                .flatten()
                .map(FormatSpec::ZeroPadded);
        }
        None
    }

    pub(super) fn parse_throws_clause(&mut self) -> Result<Vec<Type>, String> {
        if !matches!(self.peek(), Some(Token::Throws)) {
            return Ok(Vec::new());
        }
        self.advance();
        let ty = self.parse_type()?;
        Ok(match ty {
            Type::Union(types) => types,
            ty => vec![ty],
        })
    }

    pub(super) fn parse_switch_clause_body(&mut self) -> Result<Vec<Stmt>, String> {
        if matches!(self.peek(), Some(Token::LBrace)) {
            return self.parse_block();
        }
        let mut body = Vec::new();
        while !matches!(
            self.peek(),
            Some(Token::Case | Token::Default | Token::RBrace) | None
        ) {
            body.push(self.parse_stmt()?);
        }
        Ok(body)
    }

    pub(super) fn parse_switch_stmt(&mut self) -> Result<Stmt, String> {
        let __stmt_start = self.start_span();
        self.advance();
        self.expect(&Token::LParen)?;
        let expr = self.parse_expr()?;
        self.expect(&Token::RParen)?;
        self.expect(&Token::LBrace)?;

        let mut cases = vec![];
        let mut default = None;
        let mut default_index = None;

        while !matches!(self.peek(), Some(Token::RBrace) | None) {
            match self.peek() {
                Some(Token::Case) => {
                    self.advance();
                    let value = self.parse_expr()?;
                    self.expect(&Token::Colon)?;
                    let body = self.parse_switch_clause_body()?;
                    cases.push(SwitchCase { value, body });
                }
                Some(Token::Default) => {
                    if default.is_some() {
                        return Err("Only one default clause is allowed in a switch".to_string());
                    }
                    self.advance();
                    self.expect(&Token::Colon)?;
                    let body = self.parse_switch_clause_body()?;
                    default = Some(body);
                    default_index = Some(cases.len());
                }
                _ => break,
            }
        }

        self.expect(&Token::RBrace)?;
        Ok(StmtKind::Switch {
            expr,
            cases,
            default,
            default_index,
        }
        .spanned(self.finish_span(__stmt_start)))
    }

    pub(super) fn parse_match_stmt(&mut self) -> Result<Stmt, String> {
        let __stmt_start = self.start_span();
        self.advance();
        self.expect(&Token::LParen)?;
        let expr = self.parse_expr()?;
        self.expect(&Token::RParen)?;
        self.expect(&Token::LBrace)?;

        let mut arms = vec![];
        let default = None;

        while !matches!(self.peek(), Some(Token::RBrace) | None) {
            let __pattern_start = self.start_span();
            let pattern = self.parse_match_pattern()?;
            let pattern_span = self.finish_span(__pattern_start);
            let guard = if matches!(self.peek(), Some(Token::If)) {
                self.advance();
                self.expect(&Token::LParen)?;
                let guard_expr = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Some(guard_expr)
            } else {
                None
            };
            self.expect(&Token::Arrow)?;
            let (compact, body) = if matches!(self.peek(), Some(Token::LBrace)) {
                let body = self.parse_block()?;
                self.skip_semicolon();
                (false, body)
            } else {
                let expression = self.parse_expr()?;
                let span = expression.span;
                self.expect(&Token::Semicolon)?;
                (
                    true,
                    vec![Stmt::new(StmtKind::Return(Some(expression)), span)],
                )
            };
            arms.push(MatchArm {
                pattern,
                pattern_span,
                guard,
                compact,
                body,
            });
        }

        self.expect(&Token::RBrace)?;
        Ok(StmtKind::Match {
            expr,
            arms,
            default,
        }
        .spanned(self.finish_span(__stmt_start)))
    }

    pub(super) fn parse_match_pattern(&mut self) -> Result<MatchPattern, String> {
        let parenthesized = matches!(self.peek(), Some(Token::LParen));
        if parenthesized {
            self.advance();
        }
        let result = self.parse_match_pattern_inner();
        if parenthesized {
            self.expect(&Token::RParen)?;
        }
        result
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn parse_match_pattern_inner(&mut self) -> Result<MatchPattern, String> {
        match self.peek() {
            Some(Token::Ident(s)) if s == "_" => {
                self.advance();
                Ok(MatchPattern::Wildcard)
            }
            Some(Token::Ident(s)) if s == "mut" => {
                self.advance();
                let span = self.token_ranges.get(self.pos).cloned();
                match self.advance().cloned() {
                    Some(Token::Ident(name)) => {
                        self.reject_reserved_name(&name, span)?;
                        Ok(MatchPattern::Binding(name))
                    }
                    other => Err(format!(
                        "Expected identifier after mut, found {}",
                        found(&other)
                    )),
                }
            }
            Some(Token::Ident(s)) => {
                let name = s.clone();
                let span = self.token_ranges.get(self.pos).cloned();
                self.advance();
                if matches!(name.as_str(), "Ok" | "Err")
                    && matches!(self.peek(), Some(Token::LParen))
                {
                    self.advance();
                    let binding = if matches!(self.peek(), Some(Token::RParen)) {
                        None
                    } else {
                        match self.advance().cloned() {
                            Some(Token::Ident(binding)) => Some(binding),
                            other => {
                                return Err(format!(
                                    "Expected identifier binding in `{name}(...)` match pattern, found {}",
                                    found(&other)
                                ));
                            }
                        }
                    };
                    self.expect(&Token::RParen)?;
                    return Ok(MatchPattern::ResultVariant {
                        success: name == "Ok",
                        binding,
                    });
                }
                self.reject_reserved_name(&name, span)?;
                if matches!(self.peek(), Some(Token::Dot)) {
                    self.advance();
                    let variant_name = match self.advance() {
                        Some(Token::Ident(v)) => v.clone(),
                        other => {
                            return Err(format!(
                                "Expected variant name after `.` in enum pattern, found {}",
                                found(&other)
                            ));
                        }
                    };
                    let binding = if matches!(self.peek(), Some(Token::LParen)) {
                        self.advance();
                        let mut patterns = Vec::new();
                        while !matches!(self.peek(), Some(Token::RParen) | None) {
                            patterns.push(self.parse_match_pattern()?);
                            if matches!(self.peek(), Some(Token::Comma)) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                        self.expect(&Token::RParen)?;
                        let inner = if patterns.len() == 1 {
                            patterns.into_iter().next().unwrap()
                        } else {
                            MatchPattern::Tuple(patterns)
                        };
                        Some(Box::new(inner))
                    } else if matches!(self.peek(), Some(Token::LBrace)) {
                        self.advance();
                        let mut fields = vec![];
                        while !matches!(self.peek(), Some(Token::RBrace) | None) {
                            let field_name = match self.advance() {
                                Some(Token::Ident(f)) => f.clone(),
                                Some(token) if keyword_as_name(token).is_some() => {
                                    keyword_as_name(token).unwrap().to_string()
                                }
                                other => {
                                    return Err(format!(
                                        "Expected field name in struct pattern, found {}",
                                        found(&other)
                                    ));
                                }
                            };
                            let field_binding =
                                if matches!(self.peek(), Some(Token::Colon)) {
                                    self.advance();
                                    match self.parse_match_pattern()? {
                                        MatchPattern::Binding(name) => name,
                                        _ => return Err(
                                            "expected an identifier binding in enum struct pattern"
                                                .to_string(),
                                        ),
                                    }
                                } else {
                                    field_name.clone()
                                };
                            fields.push(ObjectMatchField {
                                key: field_name,
                                binding: Some(field_binding),
                                literal: None,
                            });
                            if matches!(self.peek(), Some(Token::Comma)) {
                                self.advance();
                            }
                        }
                        self.expect(&Token::RBrace)?;
                        Some(Box::new(MatchPattern::Object(fields)))
                    } else {
                        None
                    };
                    return Ok(MatchPattern::EnumVariant {
                        enum_name: name,
                        variant: variant_name,
                        binding,
                    });
                }
                Ok(MatchPattern::Binding(name))
            }
            Some(Token::LBrace) => {
                self.advance();
                let mut fields = vec![];
                while !matches!(self.peek(), Some(Token::RBrace) | None) {
                    let key = match self.advance() {
                        Some(Token::Ident(s)) => s.clone(),
                        Some(token) if keyword_as_name(token).is_some() => {
                            keyword_as_name(token).unwrap().to_string()
                        }
                        other => {
                            return Err(format!(
                                "Expected identifier in object pattern, found {}",
                                found(&other)
                            ));
                        }
                    };
                    if matches!(self.peek(), Some(Token::Colon)) {
                        self.advance();
                        match self.peek() {
                            Some(
                                Token::StringLit(_)
                                | Token::IntLit(_)
                                | Token::FloatLit(_)
                                | Token::SpecialFloatLit(_)
                                | Token::True
                                | Token::False
                                | Token::Null
                                | Token::None,
                            ) => {
                                let value = self.parse_literal_expr()?;
                                fields.push(ObjectMatchField {
                                    key,
                                    binding: None,
                                    literal: Some(value),
                                });
                            }
                            Some(Token::Ident(name)) => {
                                let name = name.clone();
                                self.advance();
                                fields.push(ObjectMatchField {
                                    key,
                                    binding: Some(name),
                                    literal: None,
                                });
                            }
                            Some(other) => {
                                return Err(format!(
                                    "Expected literal or identifier after `:` in object pattern, found {other:?}"
                                ));
                            }
                            None => {
                                return Err(
                                    "Expected literal or identifier after `:` in object pattern"
                                        .to_string(),
                                );
                            }
                        }
                    } else {
                        fields.push(ObjectMatchField {
                            key: key.clone(),
                            binding: Some(key),
                            literal: None,
                        });
                    }
                    if matches!(self.peek(), Some(Token::Comma)) {
                        self.advance();
                    }
                }
                self.expect(&Token::RBrace)?;
                Ok(MatchPattern::Object(fields))
            }
            Some(Token::LBracket) => {
                let array_start = self.start_span();
                self.advance();
                let mut elements = vec![];
                while !matches!(self.peek(), Some(Token::RBracket) | None) {
                    let element_start = self.start_span();
                    let rest = matches!(self.peek(), Some(Token::Ellipsis));
                    if rest {
                        self.advance();
                    }
                    let pattern = self.parse_match_pattern()?;
                    elements.push(MatchArrayElement {
                        pattern,
                        kind: if rest {
                            MatchArrayElementKind::Rest
                        } else {
                            MatchArrayElementKind::Pattern
                        },
                        span: self.finish_span(element_start),
                    });
                    if matches!(self.peek(), Some(Token::Comma)) {
                        self.advance();
                    }
                }
                self.expect(&Token::RBracket)?;
                let rest_positions = elements
                    .iter()
                    .enumerate()
                    .filter_map(|(index, element)| element.is_rest().then_some(index))
                    .collect::<Vec<_>>();
                if rest_positions.len() > 1 {
                    let duplicate = &elements[rest_positions[1]];
                    return Err(self.stash_coded_error(
                        "E0791",
                        "an array match pattern may contain at most one rest pattern",
                        duplicate.span,
                        "remove the extra rest pattern",
                    ));
                }
                if let Some(rest_index) = rest_positions.first() {
                    if *rest_index + 1 != elements.len() {
                        let rest = &elements[*rest_index];
                        return Err(self.stash_coded_error(
                            "E0792",
                            "the rest pattern in an array match pattern must be the last element",
                            rest.span,
                            "move the rest pattern to the end of the array pattern",
                        ));
                    }
                    if !matches!(
                        &elements[*rest_index].pattern,
                        MatchPattern::Binding(_) | MatchPattern::Wildcard
                    ) {
                        let rest = &elements[*rest_index];
                        return Err(self.stash_coded_error(
                            "E0793",
                            "an array rest pattern must bind an identifier or use `_`",
                            rest.span,
                            "replace the rest pattern with an identifier or `_`",
                        ));
                    }
                }
                Ok(MatchPattern::Array {
                    elements,
                    span: self.finish_span(array_start),
                })
            }
            Some(Token::DotDot | Token::DotDotEq) => {
                let inclusive = matches!(self.peek(), Some(Token::DotDotEq));
                self.advance();
                let end = self.parse_addition()?;
                Ok(MatchPattern::Range {
                    start: None,
                    end: Some(end),
                    inclusive,
                })
            }
            _ => {
                let expr = self.parse_addition()?;
                match self.peek() {
                    Some(Token::DotDot) => {
                        self.advance();
                        let end = if matches!(self.peek(), Some(Token::Arrow | Token::If)) {
                            None
                        } else {
                            Some(self.parse_addition()?)
                        };
                        return Ok(MatchPattern::Range {
                            start: Some(expr),
                            end,
                            inclusive: false,
                        });
                    }
                    Some(Token::DotDotEq) => {
                        self.advance();
                        let end = self.parse_addition()?;
                        return Ok(MatchPattern::Range {
                            start: Some(expr),
                            end: Some(end),
                            inclusive: true,
                        });
                    }
                    _ => {}
                }
                match &expr.kind() {
                    ExprKind::IntLit(_)
                    | ExprKind::FloatLit(_)
                    | ExprKind::StrLit(_)
                    | ExprKind::BoolLit(_)
                    | ExprKind::Null
                    | ExprKind::None => Ok(MatchPattern::Lit(expr)),
                    _ => Err(
                        "Expected pattern (wildcard _, binding, destructure, literal, or range)"
                            .to_string(),
                    ),
                }
            }
        }
    }
}
