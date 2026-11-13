use super::*;

impl Parser {
    pub(super) fn parse_type_alias_body(&mut self) -> Result<Vec<Type>, String> {
        let first = self.parse_type()?;
        if !matches!(self.peek(), Some(Token::Ampersand)) {
            return Ok(vec![first]);
        }
        let mut operands = vec![first];
        while matches!(self.peek(), Some(Token::Ampersand)) {
            if operands.len() >= MAX_INTERSECTION_OPERANDS {
                return Err(format!(
                    "a type intersection accepts at most {MAX_INTERSECTION_OPERANDS} operands"
                ));
            }
            let operator = self.current_span();
            self.advance();
            let operand = self.parse_type()?;
            // `|` binds tighter here, so `A & B | C` parses as `A & (B | C)` and
            // the operand arrives as a union. Reporting that as a plain non-object
            // operand hides the reason, so the mix gets its own diagnostic and
            // says how to spell out the grouping.
            if matches!(operands.last(), Some(Type::Union(_))) || matches!(operand, Type::Union(_))
            {
                return Err(self.stash_intersection_union_error(operator));
            }
            operands.push(operand);
        }
        Ok(operands)
    }

    pub(super) fn parse_annotation_type(&mut self) -> Result<Type, String> {
        let ty = self.parse_type()?;
        self.reject_type_intersection()?;
        Ok(ty)
    }

    pub(super) fn reject_type_intersection(&mut self) -> Result<(), String> {
        if !matches!(self.peek(), Some(Token::Ampersand)) {
            return Ok(());
        }
        let span = self.current_span();
        Err(self.stash_intersection_error(span))
    }

    pub(super) fn parse_type(&mut self) -> Result<Type, String> {
        self.enter_recursion("type")?;
        let result = self.parse_type_flagged(true);
        self.depth -= 1;
        result
    }

    pub(super) fn parse_type_flagged(&mut self, allow_optional: bool) -> Result<Type, String> {
        if matches!(self.peek(), Some(Token::Readonly)) {
            self.advance();
            let inner = self.parse_type()?;
            return Ok(Type::Readonly(Box::new(inner)));
        }

        if matches!(self.peek(), Some(Token::Ident(name)) if name == "keyof") {
            self.advance();
            let inner = self.parse_type()?;
            return Ok(Type::KeyOf(Box::new(inner)));
        }

        if matches!(self.peek(), Some(Token::Ident(name)) if name == "infer") {
            self.advance();
            let name = match self.advance() {
                Some(Token::Ident(variable)) => variable.clone(),
                other => {
                    return Err(format!(
                        "expected an inference variable name after `infer`, found {}",
                        found(&other)
                    ));
                }
            };
            return Ok(Type::InferVar(name));
        }

        if matches!(self.peek(), Some(Token::LParen)) {
            self.advance();
            let mut params = Vec::new();
            let mut parameter_list_only = false;
            while !matches!(self.peek(), Some(Token::RParen) | None) {
                if matches!(self.peek(), Some(Token::Ellipsis)) {
                    self.advance();
                    parameter_list_only = true;
                }
                let named = matches!(self.peek(), Some(Token::Ident(_)))
                    && (matches!(self.peek2(), Some(Token::Colon))
                        || (matches!(self.peek2(), Some(Token::Question))
                            && matches!(self.tokens.get(self.pos + 2), Some(Token::Colon))));
                if named {
                    parameter_list_only = true;
                    self.advance();
                    if matches!(self.peek(), Some(Token::Question)) {
                        self.advance();
                    }
                    self.expect(&Token::Colon)?;
                }
                params.push(self.parse_type()?);
                if matches!(self.peek(), Some(Token::Comma)) {
                    self.advance();
                } else {
                    break;
                }
            }
            if matches!(self.peek(), Some(Token::Ampersand)) {
                let operator = self.current_span();
                return Err(self.stash_intersection_union_error(operator));
            }
            self.expect(&Token::RParen)?;
            if !matches!(self.peek(), Some(Token::Arrow))
                && params.len() == 1
                && !parameter_list_only
            {
                let mut grouped = params.pop().expect("length checked above");
                let mut array_depth = 0;
                while matches!(self.peek(), Some(Token::LBracket)) {
                    array_depth += 1;
                    if array_depth > MAX_RECURSION_DEPTH {
                        return Err("type is nested too deeply".to_string());
                    }
                    self.advance();
                    self.expect(&Token::RBracket)?;
                    grouped = Type::Array(Box::new(grouped));
                }
                if allow_optional && matches!(self.peek(), Some(Token::Question)) {
                    self.advance();
                    grouped = Type::Optional(Box::new(grouped));
                }
                return Ok(grouped);
            }
            self.expect(&Token::Arrow)?;
            let ret = self.parse_type()?;
            let function = Type::Function {
                params,
                ret: Box::new(ret),
            };
            if matches!(self.peek(), Some(Token::Throws)) {
                self.advance();
                let effects = match self.parse_type()? {
                    Type::Union(effects) => effects,
                    effect => vec![effect],
                };
                return Ok(Type::Throws {
                    function: Box::new(function),
                    effects,
                });
            }
            return Ok(function);
        }

        if matches!(self.peek(), Some(Token::Ampersand)) {
            self.advance();
            if matches!(self.peek(), Some(Token::Ident(s)) if s == "mut") {
                self.advance();
                let inner = self.parse_type()?;
                return Ok(Type::MutRef(Box::new(inner)));
            }
            let inner = self.parse_type()?;
            return Ok(Type::Ref(Box::new(inner)));
        }

        let mut base = if matches!(self.peek(), Some(Token::LBrace)) {
            self.parse_inline_object_type()?
        } else if matches!(self.peek(), Some(Token::LBracket)) {
            self.parse_tuple_type()?
        } else if matches!(self.peek(), Some(Token::Ident(name)) if name == "Array") {
            self.advance();
            self.expect(&Token::Lt)?;
            let element = self.parse_type()?;
            self.expect_gt()?;
            Type::Array(Box::new(element))
        } else {
            self.parse_base_type()?
        };

        let mut array_depth = 0;
        while matches!(self.peek(), Some(Token::LBracket)) {
            array_depth += 1;
            if array_depth > MAX_RECURSION_DEPTH {
                return Err("type is nested too deeply".to_string());
            }
            self.advance();
            self.expect(&Token::RBracket)?;
            base = Type::Array(Box::new(base));
        }

        if allow_optional && matches!(self.peek(), Some(Token::Question)) {
            self.advance();
            base = Type::Optional(Box::new(base));
        }

        if matches!(self.peek(), Some(Token::Pipe)) {
            let mut variants = vec![base];
            while matches!(self.peek(), Some(Token::Pipe)) {
                self.advance();
                if !matches!(self.peek(), Some(Token::Pipe) | None)
                    && !matches!(self.peek(), Some(Token::RBrace | Token::Semicolon))
                {
                    let next = self.parse_type()?;
                    if let Type::Union(inner) = next {
                        variants.extend(inner);
                    } else {
                        variants.push(next);
                    }
                }
            }
            base = Type::Union(variants);
        }

        if matches!(self.peek(), Some(Token::Extends)) {
            self.advance();
            let bound = self.parse_type_flagged(false)?;
            self.expect(&Token::Question)?;
            let then = self.parse_type()?;
            self.expect(&Token::Colon)?;
            let otherwise = self.parse_type()?;
            base = Type::Conditional {
                check: Box::new(base),
                bound: Box::new(bound),
                then: Box::new(then),
                otherwise: Box::new(otherwise),
            };
        }

        Ok(base)
    }

    pub(super) fn parse_type_arguments(&mut self) -> Result<Vec<Type>, String> {
        self.expect(&Token::Lt)?;
        let mut args = Vec::new();
        while !matches!(
            self.peek(),
            Some(Token::Gt | Token::Shr | Token::UShr) | None
        ) {
            args.push(self.parse_type()?);
            if matches!(self.peek(), Some(Token::Comma)) {
                self.advance();
            } else {
                break;
            }
        }
        self.expect_gt()?;
        Ok(args)
    }

    pub(super) fn parse_tuple_type(&mut self) -> Result<Type, String> {
        self.expect(&Token::LBracket)?;
        if matches!(self.peek(), Some(Token::RBracket)) {
            self.advance();
            return Ok(Type::Tuple(Vec::new()));
        }
        let first = self.parse_type()?;
        if matches!(self.peek(), Some(Token::Semicolon)) {
            self.advance();
            let count = match self.advance() {
                Some(Token::IntLit(count)) if *count > 0 => usize::try_from(*count).ok(),
                Some(Token::TypedIntLit((count, _))) if *count > 0 => usize::try_from(*count).ok(),
                Some(Token::TypedUIntLit((count, _))) if *count > 0 => usize::try_from(*count).ok(),
                _ => None,
            }
            .filter(|count| (1..=65_536).contains(count))
            .ok_or_else(|| {
                "fixed array length must be an integer from 1 through 65536".to_string()
            })?;
            self.expect(&Token::RBracket)?;
            return Ok(Type::Tuple(vec![first; count]));
        }

        let mut elements = vec![first];
        while matches!(self.peek(), Some(Token::Comma)) {
            self.advance();
            if matches!(self.peek(), Some(Token::RBracket)) {
                break;
            }
            elements.push(self.parse_type()?);
        }
        self.expect(&Token::RBracket)?;
        Ok(Type::Tuple(elements))
    }

    pub(super) fn parse_type_params(&mut self) -> Result<Vec<TypeParam>, String> {
        if !matches!(self.peek(), Some(Token::Lt)) {
            return Ok(Vec::new());
        }
        self.advance();
        let mut params = Vec::new();
        while !matches!(self.peek(), Some(Token::Gt) | None) {
            let name = self.expect_binding_ident(BindingKind::Type)?;
            let constraint = if matches!(self.peek(), Some(Token::Extends)) {
                self.advance();
                Some(self.parse_type()?)
            } else {
                None
            };
            params.push(TypeParam { name, constraint });
            if matches!(self.peek(), Some(Token::Comma)) {
                self.advance();
            } else {
                break;
            }
        }
        self.expect(&Token::Gt)?;
        Ok(params)
    }
}
