use super::*;

pub(crate) fn collect_template_references(
    content: &str,
    content_start: usize,
    references: &mut Vec<Reference>,
) {
    let bytes = content.as_bytes();
    let mut cursor = 0;
    while cursor + 1 < bytes.len() {
        if bytes[cursor] != b'$' || bytes[cursor + 1] != b'{' {
            cursor += 1;
            continue;
        }

        let expression_start = cursor + 2;
        let mut depth = 1usize;
        cursor = expression_start;
        while cursor < bytes.len() && depth > 0 {
            match bytes[cursor] {
                b'{' => depth += 1,
                b'}' => depth -= 1,
                b'"' | b'\'' => {
                    let quote = bytes[cursor];
                    cursor += 1;
                    while cursor < bytes.len() && bytes[cursor] != quote {
                        cursor += if bytes[cursor] == b'\\' { 2 } else { 1 };
                    }
                }
                _ => {}
            }
            cursor += 1;
        }
        if depth != 0 {
            return;
        }

        let expression_end = cursor - 1;
        let expression = &content[expression_start..expression_end];
        let Ok(tokens) = crate::lexer::lex_spanned(expression) else {
            continue;
        };
        for (index, (token, range)) in tokens.iter().enumerate() {
            if let Token::TemplateLit(nested) = token {
                collect_template_references(
                    nested,
                    content_start + expression_start + range.start + 1,
                    references,
                );
            }
            let Token::Ident(name) = token else {
                continue;
            };
            if index > 0 && matches!(tokens[index - 1].0, Token::Dot | Token::OptChain) {
                continue;
            }
            references.push(Reference {
                name: name.clone(),
                range: content_start + expression_start + range.start
                    ..content_start + expression_start + range.end,
                definition: None,
            });
        }
    }
}

pub(crate) fn display_symbol_type(symbol: &Symbol, ty: &Type) -> String {
    match ty {
        Type::Function { params, ret } => {
            format!("({}) => {ret}", display_function_params(symbol, params))
        }
        Type::GenericFunction {
            type_params,
            params,
            ret,
        } => {
            let type_params = type_params
                .iter()
                .map(|param| param.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "<{type_params}>({}) => {ret}",
                display_function_params(symbol, params)
            )
        }
        _ => ty.to_string(),
    }
}

pub(crate) fn enrich_reference_type_mismatch(
    message: &str,
    tokens: &[SpannedToken],
    symbols: &[Symbol],
    references: &[Reference],
) -> Option<EnrichedDiagnostic> {
    let mismatch = parse_type_mismatch(message)?;
    if !mismatch.found.starts_with('&') || mismatch.expected.starts_with('&') {
        return None;
    }
    let declaration = symbols
        .iter()
        .find(|symbol| symbol.name == mismatch.subject)?
        .declaration
        .clone();
    let annotation = find_type_annotation_range(tokens, declaration.start, mismatch.expected)?;
    let borrowed = find_borrowed_initializer_range(tokens, references, &declaration)?;
    Some(EnrichedDiagnostic {
        range: annotation,
        message: format!(
            "mismatched types for `{}`: expected `{}`, found `{}`",
            mismatch.subject, mismatch.expected, mismatch.found
        ),
        related: vec![AnalysisRelatedInformation {
            range: borrowed,
            message: format!(
                "this expression creates a borrow of type `{}`",
                mismatch.found
            ),
        }],
        help: vec![format!(
            "change the annotation to `{}` if this binding should store a borrow",
            mismatch.found
        )],
    })
}

pub(crate) fn collect_import_symbols(
    program: &Program,
    tokens: &[SpannedToken],
    symbols: &mut Vec<Symbol>,
    declarations: &mut HashSet<usize>,
    import_tokens: &mut HashSet<usize>,
) {
    let mut token_index = 0;
    for import in &program.imports {
        while token_index < tokens.len() && !matches!(tokens[token_index].token, Token::Import) {
            token_index += 1;
        }
        let start = token_index;
        while token_index < tokens.len() && !matches!(tokens[token_index].token, Token::Semicolon) {
            import_tokens.insert(token_index);
            token_index += 1;
        }
        import_tokens.insert(token_index);
        let end = token_index.min(tokens.len().saturating_sub(1));

        for specifier in &import.specifiers {
            let candidates = (start..=end)
                .filter(|index| {
                    matches!(&tokens[*index].token, Token::Ident(name) if name == &specifier.local)
                })
                .collect::<Vec<_>>();
            let Some(index) = candidates.last().copied() else {
                continue;
            };
            declarations.insert(index);
            symbols.push(Symbol {
                name: specifier.local.clone(),
                ty: "unknown".to_string(),
                semantic_type: None,
                parameters: Vec::new(),
                is_validator: false,
                kind: SymbolKind::Import,
                declaration: tokens[index].range.clone(),
                scope: 0..usize::MAX,
                exported: false,
                import_source: Some(import.source.clone()),
                imported_name: Some(specifier.imported.clone()),
                documentation: None,
            });
        }
        token_index = token_index.saturating_add(1);
    }
}

pub(crate) fn collect_type_declaration_symbols(
    program: &Program,
    tokens: &[SpannedToken],
    source: &str,
    _docs: &[DocComment],
    exported: &HashSet<String>,
    symbols: &mut Vec<Symbol>,
    declarations: &mut HashSet<usize>,
) {
    // Inherited and intersected fields are not written in this file, so the
    // symbol has to carry the flattened shape or the editor completes only the
    // fields the declaration spells out.
    let defs = crate::type_resolve::GenericDefs::collect(program);
    let mut cursor = 0;
    for item in &program.items {
        if let Item::BehaviorInterfaceDecl { name, .. } = item {
            cursor = advance_token(tokens, cursor, |token| matches!(token, Token::Interface));
            cursor = advance_token(
                tokens,
                cursor,
                |token| matches!(token, Token::Ident(value) if value == name),
            );
            let index = cursor.saturating_sub(1);
            if index < tokens.len() && declarations.insert(index) {
                symbols.push(Symbol {
                    name: name.clone(),
                    ty: format!("behavior interface {name}"),
                    semantic_type: Some(Type::Named(name.clone())),
                    parameters: Vec::new(),
                    is_validator: false,
                    kind: SymbolKind::Interface,
                    declaration: tokens[index].range.clone(),
                    scope: 0..source.len(),
                    exported: exported.contains(name),
                    import_source: None,
                    imported_name: None,
                    documentation: None,
                });
            }
            cursor = advance_token(tokens, cursor, |token| matches!(token, Token::RBrace));
            continue;
        }
        match item {
            Item::InterfaceDecl {
                name,
                extends,
                fields,
                from_type_alias,
                ..
            } => {
                cursor = if *from_type_alias {
                    advance_token(tokens, cursor, |token| matches!(token, Token::Type))
                } else {
                    advance_token(tokens, cursor, |token| matches!(token, Token::Interface))
                };
                cursor = advance_token(
                    tokens,
                    cursor,
                    |token| matches!(token, Token::Ident(value) if value == name),
                );
                let index = cursor.saturating_sub(1);
                if index < tokens.len() && declarations.insert(index) {
                    let range = tokens[index].range.clone();
                    let intersection = *from_type_alias && !extends.is_empty();
                    let mut ignored = Vec::new();
                    let merged = defs
                        .interface_fields(name, &mut ignored)
                        .unwrap_or_else(|| fields.clone());
                    let label = if intersection {
                        let operands = extends
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(" & ");
                        format!("type {name} = {operands}")
                    } else {
                        format!("interface {name}")
                    };
                    symbols.push(Symbol {
                        name: name.clone(),
                        ty: label,
                        semantic_type: Some(Type::Object(merged)),
                        parameters: Vec::new(),
                        is_validator: false,
                        kind: if intersection {
                            SymbolKind::TypeAlias
                        } else {
                            SymbolKind::Interface
                        },
                        declaration: range,
                        scope: 0..source.len(),
                        exported: exported.contains(name),
                        import_source: None,
                        imported_name: None,
                        documentation: None,
                    });
                }
                // An intersection alias declares no field block, so looking for the
                // opening brace would run past the declaration and misplace every
                // symbol after it.
                if *from_type_alias && !extends.is_empty() {
                    cursor =
                        advance_token(tokens, cursor, |token| matches!(token, Token::Semicolon));
                    continue;
                }
                cursor = advance_token(tokens, cursor, |token| matches!(token, Token::LBrace));
                for field in fields {
                    if field.readonly {
                        cursor =
                            advance_token(tokens, cursor, |token| matches!(token, Token::Readonly));
                    }
                    cursor = advance_token(
                        tokens,
                        cursor,
                        |token| matches!(token, Token::Ident(value) if value == &field.name),
                    );
                    let field_index = cursor.saturating_sub(1);
                    if field_index < tokens.len() && declarations.insert(field_index) {
                        let field_range = tokens[field_index].range.clone();
                        let type_str = if field.readonly {
                            format!("readonly {}", field.ty)
                        } else {
                            field.ty.to_string()
                        };
                        let field_name = if field.optional {
                            format!("{}?", field.name)
                        } else {
                            field.name.clone()
                        };
                        symbols.push(Symbol {
                            name: field_name,
                            ty: type_str,
                            semantic_type: Some(field.ty.clone()),
                            parameters: Vec::new(),
                            is_validator: false,
                            kind: SymbolKind::Property,
                            declaration: field_range,
                            scope: 0..source.len(),
                            exported: exported.contains(name),
                            import_source: None,
                            imported_name: None,
                            documentation: None,
                        });
                    }
                }
            }
            Item::TypeAlias { name, ty, .. } => {
                cursor = advance_token(tokens, cursor, |token| matches!(token, Token::Type));
                cursor = advance_token(
                    tokens,
                    cursor,
                    |token| matches!(token, Token::Ident(value) if value == name),
                );
                let index = cursor.saturating_sub(1);
                if index < tokens.len() && declarations.insert(index) {
                    let range = tokens[index].range.clone();
                    symbols.push(Symbol {
                        name: name.clone(),
                        ty: format!("type {name} = {ty}"),
                        semantic_type: Some(ty.clone()),
                        parameters: Vec::new(),
                        is_validator: false,
                        kind: SymbolKind::TypeAlias,
                        declaration: range,
                        scope: 0..source.len(),
                        exported: exported.contains(name),
                        import_source: None,
                        imported_name: None,
                        documentation: None,
                    });
                }
            }
            Item::EnumDecl {
                name,
                repr,
                members,
                ..
            } => {
                let (resolved, _) = resolve_enum(repr, members);
                cursor = advance_token(tokens, cursor, |token| matches!(token, Token::Enum));
                cursor = advance_token(
                    tokens,
                    cursor,
                    |token| matches!(token, Token::Ident(value) if value == name),
                );
                let index = cursor.saturating_sub(1);
                let enum_ty = if resolved.nominal {
                    format!("enum {name}")
                } else {
                    format!("enum {name}: {}", resolved.repr)
                };
                if index < tokens.len() && declarations.insert(index) {
                    let range = tokens[index].range.clone();
                    symbols.push(Symbol {
                        name: name.clone(),
                        ty: enum_ty,
                        semantic_type: Some(Type::Named(name.clone())),
                        parameters: Vec::new(),
                        is_validator: false,
                        kind: SymbolKind::Enum,
                        declaration: range,
                        scope: 0..source.len(),
                        exported: exported.contains(name),
                        import_source: None,
                        imported_name: None,
                        documentation: None,
                    });
                }
                cursor = advance_token(tokens, cursor, |token| matches!(token, Token::LBrace));
                for member in members {
                    cursor = advance_token(
                        tokens,
                        cursor,
                        |token| matches!(token, Token::Ident(value) if value == &member.name),
                    );
                    let member_index = cursor.saturating_sub(1);
                    if member_index < tokens.len() && declarations.insert(member_index) {
                        let member_range = tokens[member_index].range.clone();
                        symbols.push(Symbol {
                            name: member.name.clone(),
                            ty: format!("({name}) {}", member.name),
                            semantic_type: Some(Type::Named(name.clone())),
                            parameters: Vec::new(),
                            is_validator: false,
                            kind: SymbolKind::EnumMember,
                            declaration: member_range,
                            scope: 0..source.len(),
                            exported: exported.contains(name),
                            import_source: None,
                            imported_name: None,
                            documentation: None,
                        });
                    }
                }
            }
            Item::BehaviorInterfaceDecl { .. } => {}
            Item::FnDecl {
                name,
                params,
                ret_type,
                ..
            } => {
                let cursor_before = cursor;
                cursor = advance_token(tokens, cursor, |token| matches!(token, Token::Function));
                let is_keyword_decl = tokens
                    .get(cursor)
                    .map(|t| matches!(&t.token, Token::Ident(v) if v == name))
                    .unwrap_or(false);
                if !is_keyword_decl {
                    cursor = cursor_before;
                    continue;
                }
                cursor = advance_token(
                    tokens,
                    cursor,
                    |token| matches!(token, Token::Ident(value) if value == name),
                );
                let index = cursor.saturating_sub(1);
                if index < tokens.len() && declarations.insert(index) {
                    let range = tokens[index].range.clone();
                    let ty = Type::Function {
                        params: params.iter().map(|p| p.ty.clone()).collect(),
                        ret: Box::new(ret_type.clone()),
                    };
                    let display_ty = format!(
                        "({}) => {ret_type}",
                        params
                            .iter()
                            .map(|p| {
                                let ps = match &p.pattern {
                                    ParamPattern::Name(n) => n.clone(),
                                    ParamPattern::Array(names) => {
                                        format!("[{}]", names.join(", "))
                                    }
                                    ParamPattern::Object(names) => {
                                        format!("{{{}}}", names.join(", "))
                                    }
                                };
                                format!("{ps}: {}", p.ty)
                            })
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                    symbols.push(Symbol {
                        name: name.clone(),
                        ty: display_ty,
                        semantic_type: Some(ty),
                        parameters: params.clone(),
                        is_validator: false,
                        kind: SymbolKind::Function,
                        declaration: range,
                        scope: 0..source.len(),
                        exported: exported.contains(name),
                        import_source: None,
                        imported_name: None,
                        documentation: None,
                    });
                }
            }
            _ => {}
        }
    }
}

pub(crate) fn assign_symbol_scopes(
    symbols: &mut [Symbol],
    tokens: &[SpannedToken],
    source_len: usize,
) {
    let mut stack = Vec::new();
    let mut scopes = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        match token.token {
            Token::LBrace => {
                let is_destructure = index
                    .checked_sub(1)
                    .is_some_and(|prev| matches!(tokens[prev].token, Token::Const | Token::Let));
                stack.push((token.range.start, is_destructure));
            }
            Token::RBrace => {
                if let Some((start, is_destructure)) = stack.pop()
                    && !is_destructure
                {
                    scopes.push(start..token.range.end);
                }
            }
            _ => {}
        }
    }
    scopes.sort_by_key(|scope| (scope.start, scope.end));

    for symbol in symbols {
        symbol.scope = match symbol.kind {
            SymbolKind::Import
            | SymbolKind::Function
            | SymbolKind::Interface
            | SymbolKind::TypeAlias
            | SymbolKind::Enum
            | SymbolKind::EnumMember => 0..source_len,
            SymbolKind::Parameter => scopes
                .iter()
                .filter(|scope| scope.start > symbol.declaration.end)
                .min_by_key(|scope| scope.start)
                .cloned()
                .unwrap_or(0..source_len),
            SymbolKind::Variable | SymbolKind::Property => scopes
                .iter()
                .filter(|scope| {
                    scope.start <= symbol.declaration.start && symbol.declaration.end <= scope.end
                })
                .min_by_key(|scope| scope.end - scope.start)
                .cloned()
                .unwrap_or(0..source_len),
        };
    }
}
