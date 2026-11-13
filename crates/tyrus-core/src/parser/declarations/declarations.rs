use super::*;

impl Parser {
    pub(crate) fn parse_decorators(&mut self) -> Result<Vec<ValidatorDecorator>, String> {
        let mut decorators = Vec::new();
        while matches!(self.peek(), Some(Token::AtSign)) {
            let decorator_start = self.start_span();
            self.advance();
            let name = match self.advance() {
                Some(Token::Ident(name)) => name.clone(),
                Some(Token::In) => "in".to_string(),
                Some(Token::Default) => "default".to_string(),
                other => {
                    return Err(format!(
                        "Expected validator name after `@`, found {}",
                        found(&other)
                    ));
                }
            };
            let (args, argument_ranges) = if matches!(self.peek(), Some(Token::LParen)) {
                self.advance();
                self.parse_args_with_ranges()?
            } else {
                (Vec::new(), Vec::new())
            };
            decorators.push(ValidatorDecorator {
                display_name: name.clone(),
                name,
                args,
                argument_ranges,
                span: self.finish_span(decorator_start),
            });
        }
        Ok(decorators)
    }

    pub(super) fn parse_import(&mut self) -> Result<ImportDecl, String> {
        let start = self.start_span();
        self.expect(&Token::Import)?;
        let mut specifiers = Vec::new();

        if matches!(self.peek(), Some(Token::Ident(_))) {
            let local_start = self.start_span();
            let local = self.expect_binding_ident(BindingKind::Import)?;
            specifiers.push(ImportSpecifier {
                imported: DEFAULT_EXPORT.to_string(),
                local,
                kind: ImportKind::Default,
                span: self.finish_span(local_start),
            });
            if matches!(self.peek(), Some(Token::Comma)) {
                self.advance();
            } else {
                let source = self.parse_import_source()?;
                return Ok(ImportDecl {
                    source,
                    specifiers,
                    span: self.finish_span(start),
                });
            }
        }

        match self.peek() {
            Some(Token::Star) => {
                let namespace_start = self.start_span();
                self.advance();
                self.expect(&Token::As)?;
                let local = self.expect_binding_ident(BindingKind::Import)?;
                specifiers.push(ImportSpecifier {
                    imported: String::new(),
                    local,
                    kind: ImportKind::Namespace,
                    span: self.finish_span(namespace_start),
                });
            }
            Some(Token::LBrace) => {
                self.advance();
                while !matches!(self.peek(), Some(Token::RBrace) | None) {
                    let specifier_start = self.start_span();
                    let imported_span = self.token_ranges.get(self.pos).cloned();
                    let is_type_only = matches!(self.peek(), Some(Token::Type));
                    if is_type_only {
                        self.advance();
                    }
                    let imported = match self.advance() {
                        Some(Token::Ident(name)) => name.clone(),
                        Some(Token::Default) => DEFAULT_EXPORT.to_string(),
                        other => {
                            return Err(format!("Expected imported name, found {}", found(&other)));
                        }
                    };
                    let local = if matches!(self.peek(), Some(Token::As)) {
                        self.advance();
                        self.expect_binding_ident(BindingKind::Import)?
                    } else if imported == DEFAULT_EXPORT {
                        return Err(
                            "`default` must be aliased: write `import { default as name } from ...` or `import name from ...`"
                                .to_string(),
                        );
                    } else {
                        self.reject_reserved_name(&imported, imported_span)?;
                        imported.clone()
                    };
                    specifiers.push(ImportSpecifier {
                        imported,
                        local,
                        kind: if is_type_only {
                            ImportKind::Type
                        } else {
                            ImportKind::Named
                        },
                        span: self.finish_span(specifier_start),
                    });
                    if matches!(self.peek(), Some(Token::Comma)) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                self.expect(&Token::RBrace)?;
            }
            other => {
                return Err(format!(
                    "Expected `{{`, an identifier or `*` after `import`, found {}",
                    found(&other)
                ));
            }
        }

        let source = self.parse_import_source()?;
        Ok(ImportDecl {
            source,
            specifiers,
            span: self.finish_span(start),
        })
    }

    pub(super) fn parse_import_source(&mut self) -> Result<String, String> {
        self.expect(&Token::From)?;
        let source = match self.advance() {
            Some(Token::StringLit(source)) => source.clone(),
            other => return Err(format!("Expected module string, found {}", found(&other))),
        };
        self.expect_semicolon("import")?;
        Ok(source)
    }

    pub(super) fn parse_export_clause(
        &mut self,
        visibility: Visibility,
        protect_span: Option<Span>,
    ) -> Result<Option<Vec<ExportDecl>>, String> {
        let start = self.start_span();
        match self.peek() {
            Some(Token::Star) => {
                if let Some(span) = protect_span {
                    return Err(self.stash_protect_re_export_error(span));
                }
                self.advance();
                self.expect(&Token::From)?;
                let source = match self.advance() {
                    Some(Token::StringLit(source)) => source.clone(),
                    other => {
                        return Err(format!("Expected module string, found {}", found(&other)));
                    }
                };
                self.expect_semicolon("export")?;
                Ok(Some(vec![ExportDecl::All {
                    source,
                    span: self.finish_span(start),
                }]))
            }
            Some(Token::LBrace) => {
                self.advance();
                let mut entries = Vec::new();
                while !matches!(self.peek(), Some(Token::RBrace) | None) {
                    let entry_start = self.start_span();
                    let local = match self.advance() {
                        Some(Token::Ident(name)) => name.clone(),
                        Some(Token::Default) => DEFAULT_EXPORT.to_string(),
                        other => {
                            return Err(format!("Expected exported name, found {}", found(&other)));
                        }
                    };
                    let exported = if matches!(self.peek(), Some(Token::As)) {
                        self.advance();
                        match self.advance() {
                            Some(Token::Ident(name)) => name.clone(),
                            Some(Token::Default) => DEFAULT_EXPORT.to_string(),
                            other => {
                                return Err(format!(
                                    "Expected export alias, found {}",
                                    found(&other)
                                ));
                            }
                        }
                    } else {
                        local.clone()
                    };
                    entries.push((local, exported, self.finish_span(entry_start)));
                    if matches!(self.peek(), Some(Token::Comma)) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                self.expect(&Token::RBrace)?;
                let source = if matches!(self.peek(), Some(Token::From)) {
                    self.advance();
                    match self.advance() {
                        Some(Token::StringLit(source)) => Some(source.clone()),
                        other => {
                            return Err(format!("Expected module string, found {}", found(&other)));
                        }
                    }
                } else {
                    None
                };
                if source.is_some()
                    && let Some(span) = protect_span
                {
                    return Err(self.stash_protect_re_export_error(span));
                }
                self.expect_semicolon("export")?;
                let declarations = entries
                    .into_iter()
                    .map(|(local, exported, span)| match &source {
                        Some(source) => ExportDecl::Re {
                            exported,
                            imported: local,
                            source: source.clone(),
                            span,
                        },
                        None => ExportDecl::Named {
                            exported,
                            local,
                            inline: false,
                            visibility,
                            span,
                        },
                    })
                    .collect();
                Ok(Some(declarations))
            }
            _ => Ok(None),
        }
    }
}
