use super::*;

/// Ranges a diagnostic of each family has already claimed, so the next one of
/// the same family points at a different occurrence instead of repeating it.
#[derive(Default)]
pub(crate) struct UsedDiagnosticSites {
    pub(crate) oob: Vec<Range<usize>>,
    pub(crate) validator: Vec<Range<usize>>,
    pub(crate) process_env: Vec<Range<usize>>,
    pub(crate) style_text: Vec<Range<usize>>,
    pub(crate) figlet: Vec<Range<usize>>,
    pub(crate) async_return: Vec<Range<usize>>,
    pub(crate) awaits: Vec<Range<usize>>,
    pub(crate) breaks: Vec<Range<usize>>,
    pub(crate) continues: Vec<Range<usize>>,
}

/// The document-wide inputs every diagnostic refinement reads.
#[derive(Clone, Copy)]
pub(crate) struct DiagnosticInputs<'a> {
    pub(crate) source: &'a str,
    pub(crate) tokens: &'a [SpannedToken],
    pub(crate) symbols: &'a [Symbol],
    pub(crate) references: &'a [Reference],
    pub(crate) env_schema: Option<&'a crate::env::EnvSchema>,
}

/// Turns one type checker diagnostic into the published one, narrowing the
/// range and adding related labels and help per diagnostic code.
/// The declaration of `name` inside `node`, so a local is found in the
/// statement that declares it, not wherever the same name first appears in
/// the file.
fn declaration_within(
    name: &str,
    node: Option<&std::ops::Range<usize>>,
    symbols: &[Symbol],
) -> Option<std::ops::Range<usize>> {
    symbols
        .iter()
        .find(|symbol| {
            symbol.name == name
                && node.is_none_or(|node| {
                    symbol.declaration.start >= node.start && symbol.declaration.end <= node.end
                })
        })
        .map(|symbol| symbol.declaration.clone())
}

#[allow(clippy::too_many_lines)]
pub(crate) fn push_refined_diagnostic(
    diagnostic: crate::type_checker::CheckDiagnostic,
    diagnostics: &mut Vec<AnalysisDiagnostic>,
    inputs: DiagnosticInputs<'_>,
    sites: &mut UsedDiagnosticSites,
) {
    let DiagnosticInputs {
        source,
        tokens,
        symbols,
        references,
        env_schema,
    } = inputs;
    let node = diagnostic.span.map(Span::to_range);
    let mut range = diagnostic
        .subject
        .as_deref()
        .and_then(|subject| {
            find_subject_range(subject, symbols, references, diagnostic.prefer_declaration)
        })
        .or_else(|| node.clone())
        .unwrap_or_else(|| fallback_diagnostic_range(&diagnostic.message, tokens));
    let mut message = diagnostic.message;
    let mut related = Vec::new();
    let mut help = diagnostic.help.clone();
    if matches!(
        diagnostic.code,
        "E3160" | "E3161" | "E3162" | "E3163" | "E3164"
    ) && let Some(precise) =
        locate_validator(diagnostic.subject.as_deref(), tokens, &mut sites.validator)
    {
        range = precise;
    }
    if diagnostic.code == "E0606"
        && let Some(precise) = locate_index_access(
            diagnostic.subject.as_deref(),
            &message,
            tokens,
            &mut sites.oob,
        )
    {
        range = precise;
    }
    if diagnostic.code == "E0268" || diagnostic.code == "E0269" {
        let (token, sites) = match diagnostic.subject.as_deref() {
            Some("continue") => (Token::Continue, &mut sites.continues),
            _ => (Token::Break, &mut sites.breaks),
        };
        if let Some(precise) = locate_token(token, tokens, sites) {
            range = precise;
        }
        if diagnostic.code == "E0269" {
            help.push(
                "`match` arms do not fall through, so `break` here is not needed; move it outside the `match` to exit the enclosing control structure"
                    .to_string(),
            );
        } else {
            let keyword = diagnostic.subject.as_deref().unwrap_or("break");
            let target = if keyword == "break" {
                "a loop or switch"
            } else {
                "a loop"
            };
            help.push(format!("`{keyword}` can only be used inside {target}"));
        }
    }
    if diagnostic.code == "E0716" {
        if let Some(precise) = locate_force_unwrap(diagnostic.subject.as_deref(), tokens) {
            range = precise;
        }
        if let Some(name) = diagnostic.subject.as_deref() {
            help.push(format!(
                "check `{name}` before use: `if ({name}) {{ ... }}`"
            ));
        } else {
            help.push("check the optional value with `if (value)` before using it".to_string());
        }
    }
    if diagnostic.code == "E0308" {
        if let Some(enriched) =
            enrich_callback_parameter_type_mismatch(&message, diagnostic.subject.as_deref(), tokens)
        {
            range = enriched.range;
            message = enriched.message;
            related = enriched.related;
            help = enriched.help;
        }
        if let Some(enriched) =
            enrich_reference_type_mismatch(&message, tokens, symbols, references)
        {
            range = enriched.range;
            message = enriched.message;
            related = enriched.related;
            help = enriched.help;
        }
    }
    if diagnostic.code == "E0309"
        && let Some(precise) = locate_unsupported_regex_flag(&message, source, tokens)
    {
        range = precise;
    }
    if diagnostic.code == "E0428"
        && let Some(subject) = &diagnostic.subject
        && let Some(enriched) = enrich_duplicate_declaration(subject, &range, symbols)
    {
        related = enriched.related;
    }
    if diagnostic.code == "E0034"
        && let Some(owners) = message.split_once(" through ").map(|(_, owners)| owners)
    {
        for symbol in symbols.iter().filter(|symbol| {
            symbol.kind == SymbolKind::Interface && owners.contains(&format!("`{}`", symbol.name))
        }) {
            related.push(AnalysisRelatedInformation {
                range: symbol.declaration.clone(),
                message: format!(
                    "behavior interface `{}` contributes this method",
                    symbol.name
                ),
            });
        }
    }
    if diagnostic.code == "E0795"
        && let Some(keyword) = locate_typeof_compared_with(tokens, &range)
    {
        related.push(AnalysisRelatedInformation {
            range: keyword,
            message: "the `typeof` result is compared with this category".to_string(),
        });
    }
    if diagnostic.code == "E3003"
        && let Some(precise) = locate_async_return_annotation(tokens, &mut sites.async_return)
    {
        range = precise;
    }
    if diagnostic.code == "E3296"
        && let Some(annotation) = locate_async_return_annotation_before(tokens, range.start)
    {
        related.push(AnalysisRelatedInformation {
            range: annotation,
            message: "async return payload type is declared here".to_string(),
        });
    }
    if diagnostic.code == "E3004"
        && let Some(annotation) = locate_async_return_annotation(tokens, &mut sites.async_return)
    {
        if let Some(fallthrough) = locate_async_fallthrough(tokens, &annotation) {
            related.push(AnalysisRelatedInformation {
                range: fallthrough,
                message: "control can reach the end of this async function".to_string(),
            });
        }
        range = annotation;
    }
    if diagnostic.code == "E3000"
        && let Some(precise) = locate_token(Token::Await, tokens, &mut sites.awaits)
    {
        range = precise;
    }
    if diagnostic.code == "E3001"
        && let Some(await_range) = locate_token(Token::Await, tokens, &mut sites.awaits)
    {
        if range != await_range {
            related.push(AnalysisRelatedInformation {
                range: range.clone(),
                message: "awaited expression is not a `Promise<T>`".to_string(),
            });
        }
        range = await_range;
    }
    if diagnostic.code == "E0D01"
        && let Some(annotation) = locate_return_annotation_before(tokens, range.start)
    {
        related.push(AnalysisRelatedInformation {
            range: annotation,
            message: "the enclosing return type is declared here".to_string(),
        });
    }
    if matches!(
        diagnostic.code,
        "E0D02" | "E0D03" | "E0D04" | "E0D05" | "E0D06" | "E0D08"
    ) {
        if let Some(finalizer) = locate_finalizer_keyword_before(tokens, range.start) {
            related.push(AnalysisRelatedInformation {
                range: finalizer,
                message: "this finalizer is registered here".to_string(),
            });
        }
        match diagnostic.code {
            "E0D02" => help.push(
                "finish the cleanup body without `return`; the enclosing structured exit resumes after cleanup"
                    .to_string(),
            ),
            "E0D03" => help.push(
                "finish the cleanup body without `break`; the enclosing structured exit resumes after cleanup"
                    .to_string(),
            ),
            "E0D04" => help.push(
                "finish the cleanup body without `continue`; the enclosing structured exit resumes after cleanup"
                    .to_string(),
            ),
            "E0D06" => help.push(
                "handle the cleanup failure inside the finalizer; `?` cannot replace the exit already in progress"
                    .to_string(),
            ),
            "E0D08" => help.push(
                "handle the cleanup failure before the finalizer completes".to_string(),
            ),
            _ => {}
        }
    }
    if diagnostic.code == "E3002" {
        if let Some(await_range) =
            locate_token_after(Token::Await, tokens, range.start.saturating_sub(1))
        {
            range = await_range;
        }
        if let Some(annotation) = locate_async_return_annotation_before(tokens, range.start) {
            related.push(AnalysisRelatedInformation {
                range: annotation,
                message: "this async function is lowered as a resumable state machine".to_string(),
            });
        }
        if help.is_empty() {
            help.push(
                "move the suspension to a standalone statement, for example `const value = await promise;`"
                    .to_string(),
            );
        }
    }
    if diagnostic.code == "E3010" {
        if let Some(await_range) =
            locate_token_after(Token::Await, tokens, range.start.saturating_sub(1))
        {
            range = await_range;
        }
        if let Some(guard) = locate_loop_guard_before_await(tokens, range.start) {
            related.push(AnalysisRelatedInformation {
                range: guard,
                message: "this guard is evaluated first and can skip the suspension".to_string(),
            });
        }
        help.push(
            "keep the guard in the condition and bind the awaited value in the body, for example `while (guard) { const value = await next(); if (value >= limit) { break; } }`; swapping the operands is not equivalent, because the `await` would then run even when the guard is false"
                .to_string(),
        );
    }
    if diagnostic.code == "E3085"
        && let (Some(name), Some(node)) = (diagnostic.subject.as_deref(), node.as_ref())
    {
        if let Some(reference) = references.iter().find(|reference| {
            reference.name == name
                && reference.range.start >= node.start
                && reference.range.end <= node.end
        }) {
            range = reference.range.clone();
            if let Some(declaration) = reference.definition.clone().or_else(|| {
                symbols
                    .iter()
                    .find(|symbol| symbol.name == name)
                    .map(|symbol| symbol.declaration.clone())
            }) {
                related.push(AnalysisRelatedInformation {
                    range: declaration,
                    message: format!("captured value `{name}` is declared here"),
                });
            }
        }
        help.push(format!(
                "pass `{name}` as an explicit parameter to the async closure until captured environment frames are supported"
            ));
    }
    if diagnostic.code == "E3086" {
        help.push(
            "use Copy or string array elements, or an object whose stored fields are Copy, string, or string arrays; other owned parameter patterns require deep frame cleanup"
                .to_string(),
        );
    }
    if diagnostic.code == "E3087" {
        help.push(
            "pass a Copy value or a native `Promise<T>` handle until deep async frame cleanup supports this type"
                .to_string(),
        );
    }
    if diagnostic.code == "E0412"
        && let (Some(name), Some(node)) = (diagnostic.subject.as_deref(), node.as_ref())
        && let Some(declaration) = declaration_within(name, Some(node), symbols)
    {
        range = declaration;
    }
    if diagnostic.code == "E3088"
        && let Some(name) = diagnostic.subject.as_deref()
    {
        if let Some(declaration) = declaration_within(name, node.as_ref(), symbols) {
            range = declaration;
        }
        if let Some(await_range) = locate_token_after(Token::Await, tokens, range.end) {
            related.push(AnalysisRelatedInformation {
                range: await_range,
                message: format!("local `{name}` must be stored before this suspension point"),
            });
        }
        if message.contains("async object local") {
            help.push(format!(
                    "initialize `{name}` as a `const` object literal whose required fields are Copy values, string literals, or string-literal arrays, or move its declaration after the suspension"
                ));
        } else if message.contains("[]") {
            help.push(format!(
                    "initialize `{name}` as an array literal with Copy elements or string literals, use a fresh Copy-element array method result or `string.split`, keep mutable Copy-array assignments on fresh owned forms and mutable string-array assignments on string literals or `string.split`, or move its declaration after the suspension"
                ));
        } else if message.contains("`string`") {
            help.push(format!(
                    "initialize and reassign `{name}` only with string literals or string methods that produce owned storage, move its declaration after the suspension, or use a Copy value"
                ));
        } else {
            help.push(format!(
                    "move `{name}` after the suspension or use a value type with ownership-safe async frame cleanup"
                ));
        }
    }
    if diagnostic.code == "E3007" {
        help.push(
            "return a Copy value, `string`, or an array of Copy elements until this payload has a deep-drop implementation"
                .to_string(),
        );
    }
    if diagnostic.code == "E3082"
        && let Some(await_range) = locate_token_after(Token::Await, tokens, range.end)
    {
        related.push(AnalysisRelatedInformation {
            range: await_range,
            message: "borrow is still live at this suspension point".to_string(),
        });
    }
    if matches!(diagnostic.code, "E3270" | "E3272" | "E3271")
        && let Some(precise) = locate_style_text(
            diagnostic.code,
            diagnostic.subject.as_deref(),
            tokens,
            &mut sites.style_text,
        )
    {
        range = precise;
    }
    if matches!(diagnostic.code, "E3277" | "E3274" | "E3276" | "E3275")
        && let Some(precise) = locate_figlet(
            diagnostic.code,
            diagnostic.subject.as_deref(),
            tokens,
            &mut sites.figlet,
        )
    {
        range = precise;
    }
    if matches!(diagnostic.code, "E3141" | "E3140" | "E3145" | "E3144") {
        if let Some(precise) = locate_process_env(
            diagnostic.code,
            diagnostic.subject.as_deref(),
            tokens,
            &mut sites.process_env,
        ) {
            range = precise;
        }
        let env_file = env_schema.map(|schema| schema.file_path.display().to_string());
        match diagnostic.code {
            "E3141" => {
                if let Some(name) = diagnostic.subject.as_deref() {
                    let env_file = env_file.as_deref().unwrap_or(".env");
                    help.push(format!(
                        "declare `{name}` in `{env_file}` or remove this access"
                    ));
                }
            }
            "E3140" => {
                help.push(
                    "add `\"security\": { \"envFile\": \".env\" }` to tyrus.json or run `tyrus --init`"
                        .to_string(),
                );
            }
            _ => {
                help.push("access environment variables with `process.env.<NAME>`".to_string());
            }
        }
    }
    if diagnostic.code == "E0308"
        && let Some((expected, found)) = numeric_mismatch_types(&message)
    {
        help.push(format!(
                "`{found}` does not convert to `{expected}` on its own; write the conversion explicitly with `as {expected}`"
            ));
        let value_range = node.as_ref().and_then(|node| {
            locate_assignment_value_range(diagnostic.subject.as_deref(), node, tokens)
        });
        if let Some(value_range) = value_range {
            related.push(AnalysisRelatedInformation {
                range: value_range,
                message: format!("value of type `{found}` found here"),
            });
        }
    }
    if help.is_empty() && !diagnostic.help.is_empty() {
        help.extend(diagnostic.help.iter().cloned());
    }
    if help.is_empty()
        && let Some(hint) = common_help(diagnostic.code, diagnostic.subject.as_deref())
    {
        help.push(hint);
    }
    if let Some(node) = node {
        let refines_the_node = range.start >= node.start && range.end <= node.end;
        if !refines_the_node {
            range = node;
        }
    }
    diagnostics.push(AnalysisDiagnostic {
        code: diagnostic.code.to_string(),
        range,
        severity: match diagnostic.severity {
            CheckSeverity::Error => DiagnosticSeverity::Error,
            CheckSeverity::Warning => DiagnosticSeverity::Warning,
        },
        message,
        related,
        help,
    });
}
