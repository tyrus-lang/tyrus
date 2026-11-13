use super::*;

pub(super) fn imported_symbols_from_program(
    tokens: &[tyrus_core::analysis::SpannedToken],
    program: &tyrus_core::ast::Program,
) -> Vec<Symbol> {
    let mut symbols = Vec::new();
    for import in &program.imports {
        for specifier in &import.specifiers {
            let Some(declaration) = tokens
                .iter()
                .rev()
                .find(|token| {
                    token.range.start >= import.span.start
                        && token.range.end <= import.span.end
                        && matches!(
                            &token.token,
                            tyrus_core::lexer::Token::Ident(name) if name == &specifier.local
                        )
                })
                .map(|token| token.range.clone())
            else {
                continue;
            };
            symbols.push(Symbol {
                name: specifier.local.clone(),
                ty: "unknown".to_string(),
                semantic_type: None,
                parameters: Vec::new(),
                is_validator: false,
                kind: SymbolKind::Import,
                declaration,
                scope: 0..usize::MAX,
                exported: false,
                import_source: Some(import.source.clone()),
                imported_name: Some(specifier.imported.clone()),
                documentation: None,
            });
        }
    }
    symbols
}
