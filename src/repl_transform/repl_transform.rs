use std::ops::Range;

use tyrus_core::lexer::{Token, lex_spanned};

pub struct Transform {
    pub source: String,
    pub last_has_step: bool,
}

pub fn transform_units<F>(units: &[String], mut resolve: F) -> Transform
where
    F: FnMut(&str, &str) -> Option<String>,
{
    let mut source = String::new();
    let mut last_has_step = false;
    for (index, unit) in units.iter().enumerate() {
        let mut module = String::new();
        let mut step = String::new();
        transform_unit(unit, &mut module, &mut step, &mut resolve);
        source.push_str(&module);
        if !step.is_empty() {
            source.push_str(&format!(
                "const {} = (): void => {{\n",
                step_name(index + 1)
            ));
            source.push_str(&step);
            source.push_str("};\n");
            last_has_step = true;
        } else {
            last_has_step = false;
        }
    }
    Transform {
        source,
        last_has_step,
    }
}

pub fn step_name(index: usize) -> String {
    format!("__repl_step_{index}")
}

pub fn step_symbol(index: usize) -> String {
    tyrus_core::module_loader::entry_symbol(&step_name(index))
}

fn transform_unit<F>(unit: &str, module: &mut String, step: &mut String, resolve: &mut F)
where
    F: FnMut(&str, &str) -> Option<String>,
{
    let tokens = match lex_spanned(unit) {
        Ok(tokens) => tokens,
        Err(_) => {
            module.push_str(unit);
            return;
        }
    };

    let statements = split_top_level_statements(&tokens);
    for statement in &statements {
        if statement.is_empty() {
            continue;
        }
        let slice_start = statement[0].1.start;
        let slice_end = statement[statement.len() - 1].1.end;
        match statement[0].0 {
            Token::Const | Token::Let => {
                transform_declaration(
                    statement,
                    unit,
                    slice_start,
                    slice_end,
                    module,
                    step,
                    resolve,
                );
            }
            Token::Import
            | Token::Function
            | Token::Async
            | Token::Type
            | Token::Interface
            | Token::Enum
            | Token::Export => {
                module.push_str(&unit[slice_start..slice_end]);
                module.push('\n');
            }
            _ => {
                step.push_str(&unit[slice_start..slice_end]);
                step.push('\n');
            }
        }
    }
}

fn split_top_level_statements(tokens: &[(Token, Range<usize>)]) -> Vec<&[(Token, Range<usize>)]> {
    let mut statements = Vec::new();
    let mut start = 0;
    let mut paren = 0usize;
    let mut bracket = 0usize;
    let mut brace = 0usize;
    for (index, (token, _)) in tokens.iter().enumerate() {
        match token {
            Token::LParen => paren += 1,
            Token::RParen => paren -= 1,
            Token::LBracket => bracket += 1,
            Token::RBracket => bracket -= 1,
            Token::LBrace => brace += 1,
            Token::RBrace => brace -= 1,
            Token::Semicolon if paren == 0 && bracket == 0 && brace == 0 => {
                statements.push(&tokens[start..=index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    if start < tokens.len() {
        statements.push(&tokens[start..]);
    }
    statements
}

fn transform_declaration<F>(
    statement: &[(Token, Range<usize>)],
    unit: &str,
    slice_start: usize,
    slice_end: usize,
    module: &mut String,
    step: &mut String,
    resolve: &mut F,
) where
    F: FnMut(&str, &str) -> Option<String>,
{
    let is_const = matches!(statement[0].0, Token::Const);
    let Some((Token::Ident(name), _)) = statement.get(1) else {
        step.push_str(&unit[slice_start..slice_end]);
        step.push('\n');
        return;
    };
    let Some(assign) = top_level_assign(statement) else {
        module.push_str(&unit[slice_start..slice_end]);
        module.push('\n');
        return;
    };
    let Some(after_assign) = statement.get(assign + 1) else {
        module.push_str(&unit[slice_start..slice_end]);
        module.push('\n');
        return;
    };
    if contains_top_level_arrow(&statement[assign + 1..]) {
        module.push_str(&unit[slice_start..slice_end]);
        module.push('\n');
        return;
    }

    let initializer_start = after_assign.1.start;
    let mut initializer = unit[initializer_start..slice_end].trim();
    if let Some(rest) = initializer.strip_suffix(';') {
        initializer = rest.trim_end();
    }

    let module_type = if initializer.is_empty() {
        None
    } else {
        source_annotation(statement, unit).or_else(|| resolve(name, initializer))
    };

    if let Some(ty) = module_type {
        module.push_str("let ");
        module.push_str(name);
        module.push_str(": ");
        module.push_str(&ty);
        module.push_str(";\n");
    } else if is_const {
        module.push_str("let ");
        module.push_str(&unit[statement[1].1.start..slice_end]);
        module.push('\n');
    } else {
        module.push_str(&unit[slice_start..slice_end]);
        module.push('\n');
    }

    if !initializer.is_empty() {
        step.push_str(name);
        step.push_str(" = ");
        step.push_str(initializer);
        step.push_str(";\n");
    }
}

fn source_annotation(statement: &[(Token, Range<usize>)], unit: &str) -> Option<String> {
    let mut paren = 0usize;
    let mut bracket = 0usize;
    let mut brace = 0usize;
    for (index, (token, range)) in statement.iter().enumerate().skip(2) {
        match token {
            Token::LParen => paren += 1,
            Token::RParen => paren -= 1,
            Token::LBracket => bracket += 1,
            Token::RBracket => bracket -= 1,
            Token::LBrace => brace += 1,
            Token::RBrace => brace -= 1,
            Token::Colon if paren == 0 && bracket == 0 && brace == 0 => {
                let assign = statement
                    .iter()
                    .enumerate()
                    .skip(index + 1)
                    .find(|(_, (token, _))| matches!(token, Token::Eq))
                    .map(|(index, _)| index)?;
                let text = unit[range.end..statement[assign].1.start].trim();
                return if text.is_empty() {
                    None
                } else {
                    Some(text.to_string())
                };
            }
            _ => {}
        }
    }
    None
}

fn top_level_assign(statement: &[(Token, Range<usize>)]) -> Option<usize> {
    let mut paren = 0usize;
    let mut bracket = 0usize;
    let mut brace = 0usize;
    for (index, (token, _)) in statement.iter().enumerate().skip(1) {
        match token {
            Token::LParen => paren += 1,
            Token::RParen => paren -= 1,
            Token::LBracket => bracket += 1,
            Token::RBracket => bracket -= 1,
            Token::LBrace => brace += 1,
            Token::RBrace => brace -= 1,
            Token::Eq if paren == 0 && bracket == 0 && brace == 0 => return Some(index),
            _ => {}
        }
    }
    None
}

fn contains_top_level_arrow(tokens: &[(Token, Range<usize>)]) -> bool {
    let mut paren = 0usize;
    let mut bracket = 0usize;
    let mut brace = 0usize;
    for (token, _) in tokens {
        match token {
            Token::LParen => paren += 1,
            Token::RParen => paren -= 1,
            Token::LBracket => bracket += 1,
            Token::RBracket => bracket -= 1,
            Token::LBrace => brace += 1,
            Token::RBrace => brace -= 1,
            Token::Arrow if paren == 0 && bracket == 0 && brace == 0 => return true,
            _ => {}
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transform(units: &[&str]) -> Transform {
        transform_with(units, |_, _| None)
    }

    fn transform_with(
        units: &[&str],
        resolve: impl FnMut(&str, &str) -> Option<String>,
    ) -> Transform {
        let units: Vec<String> = units.iter().map(std::string::ToString::to_string).collect();
        transform_units(&units, resolve)
    }

    #[test]
    fn keeps_declarations_at_module_scope() {
        let result = transform(&["let counter: i32 = 0;"]);
        assert_eq!(
            result.source,
            "let counter: i32;\nconst __repl_step_1 = (): void => {\ncounter = 0;\n};\n"
        );
    }

    #[test]
    fn rewrites_const_to_let() {
        let result = transform(&["const name: string = \"tyrus\";"]);
        assert_eq!(
            result.source,
            "let name: string;\nconst __repl_step_1 = (): void => {\nname = \"tyrus\";\n};\n"
        );
    }

    #[test]
    fn arrow_function_stays_verbatim() {
        let result = transform(&["const add = (a: i32, b: i32): i32 => a + b;"]);
        assert_eq!(
            result.source,
            "const add = (a: i32, b: i32): i32 => a + b;\n"
        );
        assert!(!result.last_has_step);
    }

    #[test]
    fn statements_go_to_the_step() {
        let result = transform(&["console.log(1);"]);
        assert_eq!(
            result.source,
            "const __repl_step_1 = (): void => {\nconsole.log(1);\n};\n"
        );
        assert!(result.last_has_step);
    }

    #[test]
    fn multiple_units_get_separate_steps() {
        let result = transform(&["let x: i32 = 10;", "console.log(x);", "x = x + 5;"]);
        assert_eq!(
            result.source,
            "let x: i32;\nconst __repl_step_1 = (): void => {\nx = 10;\n};\nconst __repl_step_2 = (): void => {\nconsole.log(x);\n};\nconst __repl_step_3 = (): void => {\nx = x + 5;\n};\n"
        );
    }

    #[test]
    fn untyped_declaration_uses_resolved_type() {
        let mut types = std::collections::HashMap::new();
        types.insert("name".to_string(), "string".to_string());
        types.insert("newName".to_string(), "string".to_string());
        let result = transform_with(
            &["let name = \"Thomas\";", "let newName = name;"],
            move |name, _initializer| types.get(name).cloned(),
        );
        assert_eq!(
            result.source,
            "let name: string;\nconst __repl_step_1 = (): void => {\nname = \"Thomas\";\n};\nlet newName: string;\nconst __repl_step_2 = (): void => {\nnewName = name;\n};\n"
        );
        assert!(!result.source.contains("let newName = name;"));
    }

    #[test]
    fn untyped_declaration_without_resolved_type_falls_back() {
        let result = transform(&["let list = [1, 2, 3];"]);
        assert_eq!(
            result.source,
            "let list = [1, 2, 3];\nconst __repl_step_1 = (): void => {\nlist = [1, 2, 3];\n};\n"
        );
    }

    #[test]
    fn last_has_step_follows_the_last_unit() {
        let result = transform(&["console.log(1);", "const add = (a: i32): i32 => a + 1;"]);
        assert!(!result.last_has_step);
        let result = transform(&[
            "const add = (a: i32): i32 => a + 1;",
            "console.log(add(1));",
        ]);
        assert!(result.last_has_step);
    }

    #[test]
    fn semicolon_inside_strings_and_brackets_do_not_split() {
        let result = transform(&[
            "const greeting: string = \"a;b\";",
            "console.log(greeting);",
            "const list = [1, 2, 3];",
            "console.log(list[1]);",
        ]);
        assert_eq!(
            result.source,
            "let greeting: string;\nconst __repl_step_1 = (): void => {\ngreeting = \"a;b\";\n};\nconst __repl_step_2 = (): void => {\nconsole.log(greeting);\n};\nlet list = [1, 2, 3];\nconst __repl_step_3 = (): void => {\nlist = [1, 2, 3];\n};\nconst __repl_step_4 = (): void => {\nconsole.log(list[1]);\n};\n"
        );
        assert!(result.last_has_step);
    }

    #[test]
    fn function_and_type_declarations_stay_verbatim() {
        let result = transform(&[
            "interface Point { x: i32; y: i32; }",
            "function sum(x: i32, y: i32): i32 { return x + y; }",
        ]);
        assert!(
            result
                .source
                .contains("interface Point { x: i32; y: i32; }")
        );
        assert!(
            result
                .source
                .contains("function sum(x: i32, y: i32): i32 { return x + y; }")
        );
    }

    #[test]
    fn declaration_without_semicolon_is_handled() {
        let result = transform(&["const x: i32 = 5"]);
        assert_eq!(
            result.source,
            "let x: i32;\nconst __repl_step_1 = (): void => {\nx = 5;\n};\n"
        );
    }

    #[test]
    fn declaration_trailing_assign_keeps_source_for_diagnostics() {
        let result = transform(&["let z: i32 = "]);
        assert_eq!(result.source, "let z: i32 =\n");
        assert!(!result.last_has_step);
        let result = transform(&["let z: i32 = ;"]);
        assert_eq!(result.source, "let z: i32 = ;\n");
        assert!(!result.last_has_step);
    }

    #[test]
    fn step_name_uses_one_based_index() {
        assert_eq!(step_name(1), "__repl_step_1");
        assert_eq!(step_name(4), "__repl_step_4");
    }
}
