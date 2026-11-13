use super::*;

pub(crate) fn push_checker_diagnostic(
    diagnostics: &mut Vec<AnalysisDiagnostic>,
    symbols: &[Symbol],
    references: &[Reference],
    tokens: &[SpannedToken],
    severity: DiagnosticSeverity,
    code: &str,
    message: String,
) {
    let message = normalize_checker_message(message);
    let code = checker_diagnostic_code(code, &message);
    if let Some(name) = checker_undeclared_variable_name(&message)
        && diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "E0425" && diagnostic.message.contains(&format!("`{name}`"))
        })
    {
        return;
    }
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message == message)
    {
        return;
    }
    let field_move = if message.contains("cannot move out of") {
        let mut quotes = message.split('`').skip(1).step_by(2);
        match (quotes.next(), quotes.next()) {
            (Some(owner), Some(field)) => locate_field_access(tokens, owner, field),
            _ => None,
        }
    } else {
        None
    };
    let quoted = message
        .split('`')
        .nth(1)
        .and_then(|name| {
            if message.contains("crosses `await`") {
                return symbols
                    .iter()
                    .find(|symbol| symbol.name == name)
                    .map(|symbol| symbol.declaration.clone())
                    .or_else(|| {
                        references
                            .iter()
                            .find(|reference| reference.name == name)
                            .map(|reference| reference.range.clone())
                    });
            }
            let reference = if message.contains("use of moved value")
                || message.contains("use of dropped value")
                || (message.contains("cannot move") && message.contains("out of"))
            {
                references
                    .iter()
                    .rev()
                    .find(|reference| reference.name == name)
            } else {
                references.iter().find(|reference| reference.name == name)
            };
            reference
                .map(|reference| reference.range.clone())
                .or_else(|| {
                    symbols
                        .iter()
                        .find(|symbol| symbol.name == name)
                        .map(|symbol| symbol.declaration.clone())
                })
        })
        .unwrap_or(0..0);
    let moved_path = if message.contains("use of moved value") {
        message
            .split('`')
            .nth(1)
            .and_then(|path| path.rsplit_once('.'))
            .and_then(|(owner, field)| locate_last_field_access(tokens, owner, field))
    } else {
        None
    };
    let quoted = field_move.or(moved_path).unwrap_or(quoted);
    let mut related = checker_related_information(&message, references);
    if code == "E3082"
        && let Some(await_range) = locate_token_after(Token::Await, tokens, quoted.end)
    {
        related.push(AnalysisRelatedInformation {
            range: await_range,
            message: "borrow is still live at this suspension point".to_string(),
        });
    }
    if message.contains("captured by a pending finalizer")
        && let Some(finalizer) = locate_finalizer_keyword_before(tokens, quoted.end)
    {
        related.push(AnalysisRelatedInformation {
            range: finalizer,
            message: "this finalizer keeps the captured value live until scope exit".to_string(),
        });
    }
    let help = checker_help(&code, &message, &related, symbols);
    diagnostics.push(AnalysisDiagnostic {
        code,
        range: quoted,
        severity,
        message,
        related,
        help,
    });
}

fn checker_undeclared_variable_name(message: &str) -> Option<&str> {
    [
        "use of undeclared variable `",
        "cannot borrow undeclared variable `",
    ]
    .iter()
    .find_map(|prefix| message.strip_prefix(prefix)?.split('`').next())
}

pub(crate) fn normalize_checker_message(message: String) -> String {
    message
        .strip_prefix("error: ")
        .or_else(|| message.strip_prefix("warning: "))
        .unwrap_or(&message)
        .to_string()
}

pub(crate) fn checker_diagnostic_code(fallback: &str, message: &str) -> String {
    if message.contains("captured by a pending finalizer") {
        "E0D07".to_string()
    } else if message.contains("use of moved value") || message.contains("use of dropped value") {
        "E0382".to_string()
    } else if message.contains("cannot move") && message.contains("while it is borrowed") {
        "E0505".to_string()
    } else if message.contains("cannot assign") && message.contains("while it is borrowed") {
        "E0506".to_string()
    } else if message.contains("already mutably borrowed")
        || message.contains("already borrowed")
        || message.contains("while it is mutably borrowed")
    {
        "E0502".to_string()
    } else if message.contains("cannot move") && message.contains("out of") {
        "E3081".to_string()
    } else if message.contains("crosses `await`") {
        "E3082".to_string()
    } else {
        fallback.to_string()
    }
}

pub(crate) fn checker_help(
    code: &str,
    message: &str,
    related: &[AnalysisRelatedInformation],
    symbols: &[Symbol],
) -> Vec<String> {
    match code {
        "E3278" => {
            return vec!["use `digest.sha256` for non-keyed digests or `hmac.sha256` when authenticity is required; retain `digest.md5` only for an explicitly legacy protocol".to_string()];
        }
        "E3100" => {
            return vec![
                "call `reply.status(HttpStatus.X).send(value)` before the handler returns on each success path"
                    .to_string(),
            ];
        }
        "E3102" => {
            return vec!["append `.send(value)` to the `reply.status(...)` call".to_string()];
        }
        "E3101" => {
            return vec![
                "insert `.status(HttpStatus.X)` before `.send(...)`, choosing the status declared for this response in the route"
                    .to_string(),
            ];
        }
        "E3113" => {
            return vec!["write the options inline, as `listen(port, { host: \"127.0.0.1\", requestTimeout: \"30s\" })`; the duration formats are parsed during compilation, which needs the values to be visible at the call".to_string()];
        }
        "E3116" => {
            return vec!["use an address like `10.0.0.1` or a network in CIDR form like `10.0.0.0/8`; the prefix cannot exceed 32 for IPv4 or 128 for IPv6".to_string()];
        }
        "E3110" => {
            return vec![
                "add the type to this module's import from the handler's module; a route resolves a handler against its own signature, so every type that signature names must be visible here, even when this file never writes it"
                    .to_string(),
            ];
        }
        "E3117" => {
            return vec![
                "use only letters, digits and the characters `!#$%&'*+-.^_`|~`; a header name cannot contain a space, a colon or a control character"
                    .to_string(),
            ];
        }
        "E3118" => {
            return vec![
                "remove the call: `connection`, `content-length` and `transfer-encoding` describe the message the runtime sends, and `set-cookie` is written with `reply.setCookie(name, value, options?)`"
                    .to_string(),
            ];
        }
        "E3104" => {
            return vec![
                "keep a single `reply.status(...).send(...)` finalization for each handler path"
                    .to_string(),
            ];
        }
        "E3119" => {
            return vec![
                "configure headers and cookies before `reply.stream()`, then use only the returned `WritableStream<u8[]>`"
                    .to_string(),
            ];
        }
        "E3103" => {
            return vec![
                "set the HTTP status once and chain metadata methods before `status(...)`"
                    .to_string(),
            ];
        }
        _ => {}
    }
    let name = message.split('`').nth(1);
    if code == "E0D07" {
        let Some(name) = name else {
            return Vec::new();
        };
        return vec![format!(
            "keep `{name}` available until the finalizer runs, or register the finalizer after the move"
        )];
    }
    if code == "E0382" && message.contains("use of moved value") {
        let Some(name) = name else {
            return Vec::new();
        };
        let mut help = vec![format!(
            "borrow `{name}` at the move site if it must remain usable later"
        )];
        if let Some(example) = moved_value_fix_example(name, related, symbols) {
            help.push(format!("for example: `{example}`"));
        } else {
            help.push(format!("for example: `&{name}`"));
        }
        return help;
    }
    if code == "E0382" && message.contains("use of dropped value") {
        let Some(name) = name else {
            return Vec::new();
        };
        return vec![format!(
            "remove the later use of `{name}`, or move it before `drop({name})`"
        )];
    }
    if let Some(name) = name {
        if message.contains("declared `const`") || message.contains("immutable") {
            return vec![format!("declare `{name}` with `let` to allow mutation")];
        }
        if message.contains("already borrowed") {
            return vec![format!(
                "end the earlier borrow of `{name}` before borrowing it again"
            )];
        }
    }
    Vec::new()
}

pub(crate) fn common_help(code: &str, subject: Option<&str>) -> Option<String> {
    match code {
        "E3278" => Some("use `digest.sha256` for non-keyed digests or `hmac.sha256` when authenticity is required; retain `digest.md5` only for an explicitly legacy protocol".to_string()),
        "E0308" => Some(
            "change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`"
                .to_string(),
        ),
        "E0384" | "E0500" => Some(match subject {
            Some(name) => format!("declare `{name}` with `let` to allow assignment"),
            None => "declare the binding with `let` to allow assignment".to_string(),
        }),
        "E0717" => Some(
            "use `.Nf` only on a float value and `0Nd` only on an integer value, or remove the format specifier and interpolate the value as-is"
                .to_string(),
        ),
        "E0425" => Some(match subject {
            Some(name)
                if crate::module_loader::namespace_member_display(name).is_some() =>
            {
                let display = crate::module_loader::namespace_member_display(name)
                    .unwrap_or_default();
                let (namespace, member) = display.split_once('.').unwrap_or((&display, ""));
                format!(
                    "export `{member}` from the module imported as `{namespace}`, or check for a typo"
                )
            }
            Some(name) => format!("declare `{name}` before use, or check for a typo"),
            None => "declare the identifier before use, or check for a typo".to_string(),
        }),
        "E0706" => Some(match subject.filter(|name| !name.is_empty()) {
            Some(name) => {
                format!("the narrowest type that holds this value is `{name}`; widen the type or change the value to fit")
            }
            None => "change the value so it fits the expected type".to_string(),
        }),
        "E0708" => Some(
            "annotate the value with `string` to hold text of any length, or keep a single character"
                .to_string(),
        ),
        "E0707" => Some(
            "use a fixed-width type when the limit itself matters, such as `u64.MAX` or `i64.MIN`"
                .to_string(),
        ),
        "E0599" | "E0609" => Some(
            "check the member name and the receiver type; hover the value to see its type"
                .to_string(),
        ),
        "E0605" => Some(
            "build the target value from the source instead of asserting it, or annotate the value with the target type where it is created"
                .to_string(),
        ),
        "E3100" => Some(
            "call `reply.status(HttpStatus.X).send(value)` before the handler returns on each success path"
                .to_string(),
        ),
        "E3102" => {
            Some("append `.send(value)` to the `reply.status(...)` call".to_string())
        }
        "E3101" => Some(
            "insert `.status(HttpStatus.X)` before `.send(...)`, choosing the status declared for this response in the route"
                .to_string(),
        ),
        "E3113" => Some("write the options inline, as `listen(port, { host: \"127.0.0.1\", requestTimeout: \"30s\" })`; the duration formats are parsed during compilation, which needs the values to be visible at the call".to_string()),
        "E3116" => Some("use an address like `10.0.0.1` or a network in CIDR form like `10.0.0.0/8`; the prefix cannot exceed 32 for IPv4 or 128 for IPv6".to_string()),
        "E3110" => Some(
            "add the type to this module's import from the handler's module; a route resolves a handler against its own signature, so every type that signature names must be visible here, even when this file never writes it"
                .to_string(),
        ),
        "E3117" => Some(
            "use only letters, digits and the characters `!#$%&'*+-.^_`|~`; a header name cannot contain a space, a colon or a control character"
                .to_string(),
        ),
        "E3118" => Some(
            "remove the call: `connection`, `content-length` and `transfer-encoding` describe the message the runtime sends, and `set-cookie` is written with `reply.setCookie(name, value, options?)`"
                .to_string(),
        ),
        "E3104" => Some(
            "keep a single `reply.status(...).send(...)` finalization for each handler path"
                .to_string(),
        ),
        "E3119" => Some(
            "configure headers and cookies before `reply.stream()`, then use only the returned `WritableStream<u8[]>`"
                .to_string(),
        ),
        "E3103" => Some(
            "set the HTTP status once and chain metadata methods before `status(...)`".to_string(),
        ),
        "E3003" => subject
            .filter(|ty| !ty.starts_with("Promise<"))
            .map(|ty| format!("change the return type to `Promise<{ty}>`")),
        "E3296" => subject
            .map(|ty| format!("return a value assignable to `{ty}`"))
            .or_else(|| Some("return a value matching the declared promise payload type".to_string())),
        "E3004" => subject
            .map(|ty| format!("return a value assignable to `{ty}` on every path")),
        "E3005" => Some(
            "await the promise, return it from the async function, or store it for an explicit later await"
                .to_string(),
        ),
        "E3001" => {
            Some("remove `await` or change the expression to return `Promise<T>`".to_string())
        }
        "E3008" => Some(
            "return a native `Promise<T>` from an async tyrus function instead of exposing a thenable object"
                .to_string(),
        ),
        _ => None,
    }
}

pub(crate) fn fallback_diagnostic_range(message: &str, tokens: &[SpannedToken]) -> Range<usize> {
    let keyword = if message.contains("return") {
        Some(Token::Return)
    } else if message.contains("condition") {
        tokens
            .iter()
            .find(|token| {
                matches!(
                    token.token,
                    Token::If | Token::While | Token::Loop | Token::For | Token::Do
                )
            })
            .map(|token| token.token.clone())
    } else {
        None
    };
    keyword
        .and_then(|keyword| {
            tokens
                .iter()
                .find(|token| token.token == keyword)
                .map(|token| token.range.clone())
        })
        .unwrap_or(0..0)
}

pub fn render_diagnostic(path: &Path, source: &str, diagnostic: &AnalysisDiagnostic) -> String {
    let mut rendered = render_diagnostic_label(
        Some(path),
        source,
        &diagnostic.range,
        &format!("error[{}]: {}", diagnostic.code, diagnostic.message),
        None,
    );
    append_related_and_help(&mut rendered, Some(path), source, diagnostic);
    rendered
}

pub(crate) fn append_related_and_help(
    rendered: &mut String,
    path: Option<&Path>,
    source: &str,
    diagnostic: &AnalysisDiagnostic,
) {
    for related in &diagnostic.related {
        rendered.push('\n');
        rendered.push_str(&render_diagnostic_label(
            path,
            source,
            &related.range,
            "",
            Some(&related.message),
        ));
    }
    for help in &diagnostic.help {
        rendered.push('\n');
        rendered.push_str(&format!("help: {help}"));
    }
}

pub(crate) fn render_diagnostic_label(
    path: Option<&Path>,
    source: &str,
    range: &Range<usize>,
    header: &str,
    label: Option<&str>,
) -> String {
    let offset = range.start.min(source.len());
    let prefix = &source[..offset];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let line_start = prefix.rfind('\n').map_or(0, |index| index + 1);
    let column = source[line_start..offset].chars().count() + 1;
    let line_text = source[offset..]
        .find('\n')
        .map(|length| &source[line_start..offset + length])
        .unwrap_or(&source[line_start..]);
    let underline_width = range.end.saturating_sub(range.start).max(1);
    let underline = format!(
        "{}{}{}",
        " ".repeat(column.saturating_sub(1)),
        "^".repeat(underline_width),
        label.map_or(String::new(), |message| format!(" {message}"))
    );
    let location = match path {
        Some(path) => format!("at {}:{line}:{column}", path.display()),
        None => format!("line {line}:{column}"),
    };
    let body = format!("{location}\n{line_text}\n{underline}");
    if header.is_empty() {
        body
    } else {
        format!("{header}\n{body}")
    }
}
