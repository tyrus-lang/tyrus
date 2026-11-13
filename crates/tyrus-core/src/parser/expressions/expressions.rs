use super::*;

impl Parser {
    pub(super) fn parse_expr(&mut self) -> Result<Expr, String> {
        self.enter_recursion("expression")?;
        let result = self.parse_ternary();
        self.depth -= 1;
        result
    }

    pub(super) fn parse_unary(&mut self) -> Result<Expr, String> {
        let __start = self.start_span();
        if matches!(self.peek(), Some(Token::PlusPlus)) {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(ExprKind::PrefixIncr(Box::new(expr)).spanned(self.finish_span(__start)));
        }

        if matches!(self.peek(), Some(Token::MinusMinus)) {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(ExprKind::PrefixDecr(Box::new(expr)).spanned(self.finish_span(__start)));
        }

        if matches!(self.peek(), Some(Token::Bang)) {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(ExprKind::Not(Box::new(expr)).spanned(self.finish_span(__start)));
        }

        if matches!(self.peek(), Some(Token::Minus)) {
            if let Some(Token::IntOverflowLit(value)) = self.peek2() {
                if is_i128_min_magnitude(value) {
                    self.advance();
                    self.advance();
                    return Ok(ExprKind::IntLit(i128::MIN).spanned(self.finish_span(__start)));
                }
                self.advance();
                return Err("integer literal does not fit in i128".to_string());
            }
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(ExprKind::Neg(Box::new(expr)).spanned(self.finish_span(__start)));
        }

        if matches!(self.peek(), Some(Token::Plus)) {
            self.advance();
            return self.parse_unary();
        }

        if matches!(self.peek(), Some(Token::Tilde)) {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(ExprKind::BitNot(Box::new(expr)).spanned(self.finish_span(__start)));
        }

        if matches!(self.peek(), Some(Token::Await)) {
            let span = self.token_ranges.get(self.pos).cloned();
            self.advance();
            if !self.peek().is_some_and(Self::can_start_unary_expression) {
                return Err(self.stash_await_missing_expression_error(span));
            }
            self.await_operand = true;
            let expr = self.parse_unary();
            self.await_operand = false;
            let awaited = ExprKind::Await(Box::new(expr?)).spanned(self.finish_span(__start));
            return self.parse_postfix_chain(awaited, __start, false);
        }

        if matches!(self.peek(), Some(Token::Ident(name)) if name == "Set" || name == "Map")
            && matches!(self.peek2(), Some(Token::Lt | Token::LParen))
        {
            return self.parse_collection_constructor();
        }

        if matches!(self.peek(), Some(Token::Ampersand)) {
            self.advance();
            let mutable = if matches!(self.peek(), Some(Token::Ident(s)) if s == "mut") {
                self.advance();
                true
            } else {
                false
            };
            let name = match self.advance() {
                Some(Token::Ident(s)) => s.clone(),
                other => {
                    return Err(format!(
                        "Expected identifier after &, found {}",
                        found(&other)
                    ));
                }
            };
            let place = if matches!(self.peek(), Some(Token::Dot | Token::LBracket)) {
                let base = ExprKind::Ident(name.clone()).spanned(self.finish_span(__start));
                Some(Box::new(self.parse_postfix_from(base)?))
            } else {
                None
            };
            return Ok(ExprKind::Borrow {
                mutable,
                name,
                place,
            }
            .spanned(self.finish_span(__start)));
        }

        self.parse_postfix()
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn parse_primary(&mut self) -> Result<Expr, String> {
        let __start = self.start_span();
        match self.peek().cloned() {
            Some(Token::StringLit(_)) => {
                if let Some(Token::StringLit(s)) = self.advance().cloned() {
                    Ok(ExprKind::StrLit(s).spanned(self.finish_span(__start)))
                } else {
                    unreachable!()
                }
            }
            Some(Token::RegexLit(_)) => {
                if let Some(Token::RegexLit(literal)) = self.advance().cloned() {
                    Ok(ExprKind::RegexLit {
                        pattern: literal.pattern,
                        flags: literal.flags,
                    }
                    .spanned(self.finish_span(__start)))
                } else {
                    unreachable!()
                }
            }
            Some(Token::TemplateLit(_)) => {
                if let Some(Token::TemplateLit(source)) = self.advance().cloned() {
                    self.parse_template_literal(&source)
                } else {
                    unreachable!()
                }
            }
            Some(Token::IntLit(_)) => {
                if let Some(Token::IntLit(n)) = self.advance().cloned() {
                    Ok(ExprKind::IntLit(n).spanned(self.finish_span(__start)))
                } else {
                    unreachable!()
                }
            }
            Some(Token::TypedIntLit(_)) => {
                if let Some(Token::TypedIntLit((value, ty))) = self.advance().cloned() {
                    Ok(ExprKind::TypeAssert {
                        value: Box::new(ExprKind::IntLit(value).spanned(self.previous_span())),
                        ty,
                    }
                    .spanned(self.finish_span(__start)))
                } else {
                    unreachable!()
                }
            }
            Some(Token::TypedUIntLit(_)) => {
                if let Some(Token::TypedUIntLit((value, ty))) = self.advance().cloned() {
                    Ok(ExprKind::TypeAssert {
                        value: Box::new(ExprKind::UIntLit(value).spanned(self.previous_span())),
                        ty,
                    }
                    .spanned(self.finish_span(__start)))
                } else {
                    unreachable!()
                }
            }
            Some(Token::IntOverflowLit(_)) => {
                let Some(Token::IntOverflowLit(raw)) = self.peek().cloned() else {
                    unreachable!();
                };
                match raw.parse::<u128>() {
                    Ok(value) => {
                        self.advance();
                        Ok(ExprKind::UIntLit(value).spanned(self.finish_span(__start)))
                    }
                    Err(_) => Err("integer literal does not fit in u128".to_string()),
                }
            }
            Some(Token::FloatLit(_)) => {
                if let Some(Token::FloatLit(f)) = self.advance().cloned() {
                    Ok(ExprKind::FloatLit(f).spanned(self.finish_span(__start)))
                } else {
                    unreachable!()
                }
            }
            Some(Token::SpecialFloatLit(_)) => {
                if let Some(Token::SpecialFloatLit(f)) = self.advance().cloned() {
                    Ok(ExprKind::FloatLit(f).spanned(self.finish_span(__start)))
                } else {
                    unreachable!()
                }
            }
            Some(Token::DecimalLit(_)) => {
                if let Some(Token::DecimalLit(value)) = self.advance().cloned() {
                    Ok(ExprKind::DecimalLit(value).spanned(self.finish_span(__start)))
                } else {
                    unreachable!()
                }
            }
            Some(Token::TypedFloatLit(_)) => {
                if let Some(Token::TypedFloatLit((value, ty))) = self.advance().cloned() {
                    Ok(ExprKind::TypeAssert {
                        value: Box::new(ExprKind::FloatLit(value).spanned(self.previous_span())),
                        ty,
                    }
                    .spanned(self.finish_span(__start)))
                } else {
                    unreachable!()
                }
            }
            Some(Token::FloatOverflowLit) => Err("float literal does not fit in f64".to_string()),
            Some(Token::True) => {
                self.advance();
                Ok(ExprKind::BoolLit(true).spanned(self.finish_span(__start)))
            }
            Some(Token::False) => {
                self.advance();
                Ok(ExprKind::BoolLit(false).spanned(self.finish_span(__start)))
            }
            Some(Token::Null) => {
                self.advance();
                Ok(ExprKind::Null.spanned(self.finish_span(__start)))
            }
            Some(Token::None) => {
                self.advance();
                Ok(ExprKind::None.spanned(self.finish_span(__start)))
            }
            Some(Token::Percent) => {
                self.advance();
                Ok(ExprKind::Topic.spanned(self.finish_span(__start)))
            }

            Some(Token::Typeof) => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(ExprKind::Typeof(Box::new(expr)).spanned(self.finish_span(__start)))
            }

            Some(Token::Comptime) => {
                self.advance();
                self.expect(&Token::LParen)?;
                let expr = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Ok(ExprKind::Comptime(Box::new(expr)).spanned(self.finish_span(__start)))
            }

            Some(Token::Function | Token::Async) => {
                let is_async = matches!(self.peek(), Some(Token::Async));
                if matches!(self.peek(), Some(Token::Async)) {
                    self.advance();
                }
                if matches!(self.peek(), Some(Token::Function)) {
                    self.advance();
                    if matches!(self.peek(), Some(Token::Ident(_))) {
                        self.advance();
                    }
                    let (type_params, params, ret_type, throws, body) =
                        self.parse_fn_body_parts()?;
                    return Ok(ExprKind::ArrowFn {
                        is_async,
                        type_params,
                        params,
                        ret_type,
                        throws,
                        body,
                    }
                    .spanned(self.finish_span(__start)));
                }
                let (type_params, params, ret_type, throws, body) = self.parse_arrow_fn_parts()?;
                Ok(ExprKind::ArrowFn {
                    is_async,
                    type_params,
                    params,
                    ret_type,
                    throws,
                    body,
                }
                .spanned(self.finish_span(__start)))
            }

            Some(Token::LBrace) => {
                self.advance();
                if matches!(self.peek(), Some(Token::Ellipsis)) {
                    self.advance();
                    let base = self.parse_expr()?;
                    let mut fields = Vec::new();
                    if matches!(self.peek(), Some(Token::Comma)) {
                        self.advance();
                    }
                    while !matches!(self.peek(), Some(Token::RBrace) | None) {
                        let field_start = self.start_span();
                        let is_ident = matches!(self.peek(), Some(Token::Ident(_)));
                        let name = match self.advance() {
                            Some(Token::Ident(name)) => name.clone(),
                            Some(Token::IntLit(value)) => value.to_string(),
                            Some(token) if keyword_as_name(token).is_some() => {
                                keyword_as_name(token).unwrap().to_string()
                            }
                            other => {
                                return Err(format!(
                                    "Expected object field, found {}",
                                    found(&other)
                                ));
                            }
                        };
                        let value = if matches!(self.peek(), Some(Token::Colon)) {
                            self.advance();
                            if matches!(self.peek(), Some(Token::TyVoid)) {
                                self.advance();
                                self.spanned(ExprKind::Ident("void".to_string()), field_start)
                            } else {
                                self.parse_expr()?
                            }
                        } else if is_ident {
                            self.spanned(ExprKind::Ident(name.clone()), field_start)
                        } else {
                            return Err(format!(
                                "error: field `{name}` has no shorthand form; write `{name}: value`"
                            ));
                        };
                        fields.push((name, value));
                        if matches!(self.peek(), Some(Token::Comma)) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    self.expect(&Token::RBrace)?;
                    return Ok(ExprKind::ObjectUpdate {
                        base: Box::new(base),
                        fields,
                    }
                    .spanned(self.finish_span(__start)));
                }
                let mut fields = Vec::new();
                while !matches!(self.peek(), Some(Token::RBrace) | None) {
                    let field_start = self.start_span();
                    let is_ident = matches!(self.peek(), Some(Token::Ident(_)));
                    let name = match self.advance() {
                        Some(Token::Ident(name)) => name.clone(),
                        Some(Token::IntLit(value)) => value.to_string(),
                        Some(token) if keyword_as_name(token).is_some() => {
                            keyword_as_name(token).unwrap().to_string()
                        }
                        other => {
                            return Err(format!("Expected object field, found {}", found(&other)));
                        }
                    };
                    let value = if matches!(self.peek(), Some(Token::Colon)) {
                        self.advance();
                        if matches!(self.peek(), Some(Token::TyVoid)) {
                            self.advance();
                            self.spanned(ExprKind::Ident("void".to_string()), field_start)
                        } else {
                            self.parse_expr()?
                        }
                    } else if is_ident {
                        self.spanned(ExprKind::Ident(name.clone()), field_start)
                    } else {
                        return Err(format!(
                            "error: field `{name}` has no shorthand form; write `{name}: value`"
                        ));
                    };
                    fields.push((name, value));
                    if matches!(self.peek(), Some(Token::Comma)) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                self.expect(&Token::RBrace)?;
                Ok(ExprKind::ObjectLit(fields).spanned(self.finish_span(__start)))
            }

            Some(Token::LBracket) => {
                self.advance();
                if matches!(self.peek(), Some(Token::RBracket)) {
                    self.advance();
                    return Ok(ExprKind::ArrayLit(Vec::new()).spanned(self.finish_span(__start)));
                }
                if matches!(self.peek(), Some(Token::Ellipsis)) {
                    let mut values = Vec::new();
                    while !matches!(self.peek(), Some(Token::RBracket) | None) {
                        if matches!(self.peek(), Some(Token::Comma)) {
                            self.advance();
                        }
                        if matches!(self.peek(), Some(Token::RBracket)) {
                            break;
                        }
                        if matches!(self.peek(), Some(Token::Ellipsis)) {
                            let spread_start = self.start_span();
                            self.advance();
                            let spread_expr = self.parse_expr()?;
                            values.push(
                                self.spanned(ExprKind::Spread(Box::new(spread_expr)), spread_start),
                            );
                        } else {
                            values.push(self.parse_expr()?);
                        }
                    }
                    self.expect(&Token::RBracket)?;
                    return Ok(ExprKind::ArrayLit(values).spanned(self.finish_span(__start)));
                }
                let first_element = self.parse_expr()?;
                if matches!(self.peek(), Some(Token::Semicolon)) {
                    self.advance();
                    let count = self.parse_expr()?;
                    self.expect(&Token::RBracket)?;
                    return Ok(ExprKind::ArrayRepeat {
                        value: Box::new(first_element),
                        count: Box::new(count),
                    }
                    .spanned(self.finish_span(__start)));
                }
                let mut values = vec![first_element];
                while !matches!(self.peek(), Some(Token::RBracket) | None) {
                    if matches!(self.peek(), Some(Token::Comma)) {
                        self.advance();
                    }
                    if matches!(self.peek(), Some(Token::RBracket)) {
                        break;
                    }
                    if matches!(self.peek(), Some(Token::Ellipsis)) {
                        let spread_start = self.start_span();
                        self.advance();
                        let spread_expr = self.parse_expr()?;
                        values.push(
                            self.spanned(ExprKind::Spread(Box::new(spread_expr)), spread_start),
                        );
                    } else {
                        values.push(self.parse_expr()?);
                    }
                }
                self.expect(&Token::RBracket)?;
                Ok(ExprKind::ArrayLit(values).spanned(self.finish_span(__start)))
            }

            Some(Token::LParen) => {
                let mark = self.mark();
                if let Ok((type_params, params, ret_type, throws, body)) =
                    self.parse_arrow_fn_parts()
                {
                    return Ok(ExprKind::ArrowFn {
                        is_async: false,
                        type_params,
                        params,
                        ret_type,
                        throws,
                        body,
                    }
                    .spanned(self.finish_span(__start)));
                }

                self.reset(mark);
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Ok(expr)
            }

            Some(Token::Ident(name)) => {
                let name = name.clone();
                let span = self.token_ranges.get(self.pos).cloned();
                let param_span = self.current_span();
                self.advance();
                if crate::keywords::is_reserved_future(&name) {
                    return Err(self.stash_reserved_error(
                        &name,
                        crate::keywords::ReservedKind::Future,
                        span,
                    ));
                }
                if matches!(self.peek(), Some(Token::Arrow)) {
                    if crate::keywords::is_ambient_builtin(&name) {
                        return Err(self.stash_reserved_error(
                            &name,
                            crate::keywords::ReservedKind::Builtin,
                            span,
                        ));
                    }
                    self.advance();
                    let body = if matches!(self.peek(), Some(Token::LBrace)) {
                        self.parse_block()?
                    } else {
                        let value = self.parse_expr()?;
                        let span = value.span;
                        vec![StmtKind::Return(Some(value)).spanned(span)]
                    };
                    return Ok(ExprKind::ArrowFn {
                        is_async: false,
                        type_params: Vec::new(),
                        params: vec![Param {
                            decorators: Vec::new(),
                            pattern: ParamPattern::Name(name),
                            ty: Type::Infer,
                            mutable: false,
                            optional: false,
                            default_value: None,
                            rest: false,
                            span: param_span,
                        }],
                        ret_type: Type::Infer,
                        throws: Vec::new(),
                        body,
                    }
                    .spanned(self.finish_span(__start)));
                }
                self.parse_expr_from_ident(name)
            }
            Some(
                Token::TyI8
                | Token::TyI16
                | Token::TyI32
                | Token::TyI64
                | Token::TyI128
                | Token::TyIsize
                | Token::TyU8
                | Token::TyU16
                | Token::TyU32
                | Token::TyU64
                | Token::TyU128
                | Token::TyUsize
                | Token::TyF32
                | Token::TyF64
                | Token::TyF128
                | Token::TyDecimal,
            ) => {
                let name = match self.advance() {
                    Some(Token::TyI8) => "i8",
                    Some(Token::TyI16) => "i16",
                    Some(Token::TyI32) => "i32",
                    Some(Token::TyI64) => "i64",
                    Some(Token::TyI128) => "i128",
                    Some(Token::TyIsize) => "isize",
                    Some(Token::TyU8) => "u8",
                    Some(Token::TyU16) => "u16",
                    Some(Token::TyU32) => "u32",
                    Some(Token::TyU64) => "u64",
                    Some(Token::TyU128) => "u128",
                    Some(Token::TyUsize) => "usize",
                    Some(Token::TyF32) => "f32",
                    Some(Token::TyF64) => "f64",
                    Some(Token::TyF128) => "f128",
                    Some(Token::TyDecimal) => "decimal",
                    _ => unreachable!(),
                };
                self.parse_expr_from_ident(name.to_string())
            }

            Some(Token::TyString | Token::TyBool) => {
                let name = match self.advance() {
                    Some(Token::TyString) => "string",
                    Some(Token::TyBool) => "boolean",
                    _ => unreachable!(),
                };
                self.parse_expr_from_ident(name.to_string())
            }

            Some(Token::If) => {
                self.advance();
                self.expect(&Token::LParen)?;
                let cond = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                let then_block = self.parse_block()?;

                let else_block = if matches!(self.peek(), Some(Token::Else)) {
                    self.advance();
                    if matches!(self.peek(), Some(Token::If)) {
                        let value = self.parse_expr()?;
                        let span = value.span;
                        Some(vec![StmtKind::Expr(value).spanned(span)])
                    } else {
                        Some(self.parse_block()?)
                    }
                } else {
                    None
                };

                Ok(ExprKind::If {
                    cond: Box::new(cond),
                    then_block,
                    else_block,
                }
                .spanned(self.finish_span(__start)))
            }

            Some(Token::Loop) => {
                self.advance();
                let body = self.parse_block()?;
                Ok(ExprKind::Loop { body }.spanned(self.finish_span(__start)))
            }

            other => {
                let message = format!("Expected an expression, found {}", found(&other));
                Err(self.fail(message))
            }
        }
    }

    pub(super) fn parse_template_literal(&self, source: &str) -> Result<Expr, String> {
        let __start = self.start_span();
        let mut parts = Vec::new();
        let mut text_start = 0;
        let bytes = source.as_bytes();
        let mut i = 0;

        while i + 1 < bytes.len() {
            if bytes[i] == b'\\' {
                i += 2;
                continue;
            }
            if bytes[i] != b'$' || bytes[i + 1] != b'{' {
                i += 1;
                continue;
            }

            if text_start < i {
                parts.push(TemplatePart::Text(crate::lexer::decode_string_content(
                    &source[text_start..i],
                )));
            }

            let expr_start = i + 2;
            let mut depth = 1;
            i = expr_start;
            while i < bytes.len() && depth > 0 {
                match bytes[i] {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    b'\"' | b'\'' => {
                        let quote = bytes[i];
                        i += 1;
                        while i < bytes.len() && bytes[i] != quote {
                            i += if bytes[i] == b'\\' { 2 } else { 1 };
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            if depth != 0 {
                return Err("Unclosed interpolation in template literal".to_string());
            }

            let (expression_source, format_spec) =
                Self::split_template_format_spec(&source[expr_start..i - 1]);
            let tokens = crate::lexer::lex(expression_source)
                .map_err(|_| "Invalid token in template interpolation".to_string())?;
            let mut parser = Parser::new(tokens);
            let expression = parser.parse_expr()?;
            if parser.peek().is_some() {
                return Err("Unexpected token in template interpolation".to_string());
            }
            parts.push(TemplatePart::Expr(expression, format_spec));
            text_start = i;
        }

        if text_start < source.len() {
            parts.push(TemplatePart::Text(crate::lexer::decode_string_content(
                &source[text_start..],
            )));
        }
        Ok(ExprKind::TemplateLit(parts).spanned(self.finish_span(__start)))
    }

    pub(super) fn parse_expr_from_ident(&mut self, name: String) -> Result<Expr, String> {
        let __start = self.previous_span();
        let mark = self.mark();
        let type_args = if matches!(self.peek(), Some(Token::Lt)) {
            let parsed = (|| -> Result<Vec<Type>, String> {
                self.advance();
                let mut types = Vec::new();
                while !matches!(self.peek(), Some(Token::Gt) | None) {
                    types.push(self.parse_type()?);
                    if matches!(self.peek(), Some(Token::Comma)) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                self.expect(&Token::Gt)?;
                if !matches!(self.peek(), Some(Token::LParen)) {
                    return Err("not a generic call".to_string());
                }
                Ok(types)
            })();
            match parsed {
                Ok(types) => types,
                Err(_) => {
                    self.reset(mark);
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };
        if matches!(self.peek(), Some(Token::LParen)) {
            self.advance();
            let args = self.parse_args()?;
            return Ok(ExprKind::FnCall {
                callee: name,
                type_args,
                args,
            }
            .spanned(self.finish_span(__start)));
        }

        Ok(ExprKind::Ident(name).spanned(self.finish_span(__start)))
    }

    pub(super) fn parse_array_destructure(&mut self, mutable: bool) -> Result<Item, String> {
        let __stmt_start = self.start_span();
        let bindings = self.parse_array_destructure_bindings(mutable)?;
        self.expect(&Token::Eq)?;
        let value = self.parse_expr()?;
        self.expect_semicolon("destructuring declaration")?;
        Ok(Item::Stmt(
            StmtKind::ArrayDestructure {
                mutable,
                bindings,
                value,
            }
            .spanned(self.finish_span(__stmt_start)),
        ))
    }

    pub(super) fn parse_object_destructure(&mut self, mutable: bool) -> Result<Item, String> {
        let __stmt_start = self.start_span();
        let bindings = self.parse_object_destructure_bindings(mutable)?;
        self.expect(&Token::Eq)?;
        let value = self.parse_expr()?;
        self.expect_semicolon("destructuring declaration")?;
        Ok(Item::Stmt(
            StmtKind::ObjectDestructure {
                mutable,
                bindings,
                value,
            }
            .spanned(self.finish_span(__stmt_start)),
        ))
    }

    pub(super) fn parse_array_destructure_stmt(&mut self, mutable: bool) -> Result<Stmt, String> {
        let __stmt_start = self.start_span();
        let bindings = self.parse_array_destructure_bindings(mutable)?;
        self.expect(&Token::Eq)?;
        let value = self.parse_expr()?;
        self.expect_semicolon("destructuring declaration")?;
        Ok(StmtKind::ArrayDestructure {
            mutable,
            bindings,
            value,
        }
        .spanned(self.finish_span(__stmt_start)))
    }

    pub(super) fn parse_object_destructure_stmt(&mut self, mutable: bool) -> Result<Stmt, String> {
        let __stmt_start = self.start_span();
        let bindings = self.parse_object_destructure_bindings(mutable)?;
        self.expect(&Token::Eq)?;
        let value = self.parse_expr()?;
        self.expect_semicolon("destructuring declaration")?;
        Ok(StmtKind::ObjectDestructure {
            mutable,
            bindings,
            value,
        }
        .spanned(self.finish_span(__stmt_start)))
    }

    pub(super) fn parse_array_destructure_bindings(
        &mut self,
        mutable: bool,
    ) -> Result<Vec<DestructureBinding>, String> {
        self.advance();
        let mut bindings = Vec::new();
        while !matches!(self.peek(), Some(Token::RBracket) | None) {
            let (name, discarded) = if matches!(self.peek(), Some(Token::Ident(s)) if s == "_") {
                self.advance();
                (format!("__tyrus_discard_{}", self.pos), true)
            } else {
                (self.expect_binding_ident(BindingKind::Variable)?, false)
            };
            let ty = Type::Infer;
            bindings.push(DestructureBinding {
                name,
                mutable,
                ty,
                discarded,
            });
            if matches!(self.peek(), Some(Token::Comma)) {
                self.advance();
            } else {
                break;
            }
        }
        self.expect(&Token::RBracket)?;
        Ok(bindings)
    }

    pub(super) fn parse_object_destructure_bindings(
        &mut self,
        mutable: bool,
    ) -> Result<Vec<DestructureBinding>, String> {
        self.advance();
        let mut bindings = Vec::new();
        while !matches!(self.peek(), Some(Token::RBrace) | None) {
            let name = self.expect_binding_ident(BindingKind::Variable)?;
            let ty = Type::Infer;
            bindings.push(DestructureBinding {
                name,
                mutable,
                ty,
                discarded: false,
            });
            if matches!(self.peek(), Some(Token::Comma)) {
                self.advance();
            } else {
                break;
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(bindings)
    }

    pub(super) fn parse_arrow_fn_parts(&mut self) -> Result<ParsedFunctionParts, String> {
        let __stmt_start = self.start_span();
        let type_params = self.parse_type_params()?;
        self.type_parameter_scopes
            .push(type_params.iter().map(|param| param.name.clone()).collect());
        self.expect(&Token::LParen)?;
        let params = self.parse_params()?;
        self.expect(&Token::RParen)?;

        let ret_type = if matches!(self.peek(), Some(Token::Colon)) {
            self.advance();
            if matches!(self.peek(), Some(Token::Arrow)) {
                Type::Infer
            } else {
                self.parse_type()?
            }
        } else {
            Type::Infer
        };

        let throws = self.parse_throws_clause()?;

        self.expect(&Token::Arrow)?;

        if !matches!(self.peek(), Some(Token::LBrace)) {
            let expr = self.parse_expr()?;
            self.type_parameter_scopes.pop();
            return Ok((
                type_params,
                params,
                ret_type,
                throws,
                vec![StmtKind::Return(Some(expr)).spanned(self.finish_span(__stmt_start))],
            ));
        }

        let body = self.parse_block()?;
        self.type_parameter_scopes.pop();
        Ok((type_params, params, ret_type, throws, body))
    }

    pub(super) fn parse_literal_expr(&mut self) -> Result<Expr, String> {
        let __start = self.start_span();
        match self.advance() {
            Some(Token::StringLit(s)) => {
                Ok(ExprKind::StrLit(s.clone()).spanned(self.finish_span(__start)))
            }
            Some(Token::RegexLit(literal)) => Ok(ExprKind::RegexLit {
                pattern: literal.pattern.clone(),
                flags: literal.flags.clone(),
            }
            .spanned(self.finish_span(__start))),
            Some(Token::IntLit(n)) => Ok(ExprKind::IntLit(*n).spanned(self.finish_span(__start))),
            Some(Token::IntOverflowLit(raw)) => raw
                .parse::<u128>()
                .map(|value| ExprKind::UIntLit(value).spanned(self.finish_span(__start)))
                .map_err(|_| "integer literal does not fit in u128".to_string()),
            Some(Token::FloatLit(f)) => {
                Ok(ExprKind::FloatLit(f.clone()).spanned(self.finish_span(__start)))
            }
            Some(Token::SpecialFloatLit(f)) => {
                Ok(ExprKind::FloatLit(f.clone()).spanned(self.finish_span(__start)))
            }
            Some(Token::FloatOverflowLit) => Err("float literal does not fit in f64".to_string()),
            Some(Token::True) => Ok(ExprKind::BoolLit(true).spanned(self.finish_span(__start))),
            Some(Token::False) => Ok(ExprKind::BoolLit(false).spanned(self.finish_span(__start))),
            Some(Token::Null) => Ok(ExprKind::Null.spanned(self.finish_span(__start))),
            Some(Token::None) => Ok(ExprKind::None.spanned(self.finish_span(__start))),
            other => Err(format!("Expected literal, found {}", found(&other))),
        }
    }
}
