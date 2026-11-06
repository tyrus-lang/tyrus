use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Off,
    Info,
    Warn,
    Error,
}

impl Severity {
    pub fn label(self) -> &'static str {
        match self {
            Severity::Off => "off",
            Severity::Info => "info",
            Severity::Warn => "warning",
            Severity::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Correctness,
    Safety,
    Style,
    Performance,
    Complexity,
}

impl Category {
    pub fn label(self) -> &'static str {
        match self {
            Category::Correctness => "correctness",
            Category::Safety => "safety",
            Category::Style => "style",
            Category::Performance => "performance",
            Category::Complexity => "complexity",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RuleMeta {
    pub name: &'static str,
    pub default: Severity,
    pub category: Category,
    pub fixable: bool,
    pub implemented: bool,
    pub description: &'static str,
}

pub static RULES: &[RuleMeta] = &[
    RuleMeta {
        name: "api-no-verb-in-path",
        default: Severity::Warn,
        category: Category::Style,
        fixable: false,
        implemented: true,
        description: "Warns when an HTTP route path uses action verbs instead of resource-oriented paths.",
    },
    RuleMeta {
        name: "api-prefer-resource-nouns",
        default: Severity::Warn,
        category: Category::Style,
        fixable: false,
        implemented: true,
        description: "Warns when HTTP route paths do not look like resource nouns.",
    },
    RuleMeta {
        name: "controller-must-send-response",
        default: Severity::Error,
        category: Category::Correctness,
        fixable: false,
        implemented: false,
        description: "Requires HTTP controllers to complete every success path with `reply.status(...).send(...)`.",
    },
    RuleMeta {
        name: "controller-no-db-access",
        default: Severity::Error,
        category: Category::Safety,
        fixable: false,
        implemented: false,
        description: "Forbids direct database access from HTTP controllers.",
    },
    RuleMeta {
        name: "controller-no-try-catch",
        default: Severity::Warn,
        category: Category::Safety,
        fixable: false,
        implemented: true,
        description: "Warns when HTTP controllers catch expected errors instead of letting services map them.",
    },
    RuleMeta {
        name: "db-explicit-select-columns",
        default: Severity::Info,
        category: Category::Style,
        fixable: true,
        implemented: true,
        description: "Encourages explicit SELECT projections and can expand them from the local result interface.",
    },
    RuleMeta {
        name: "db-no-extra-select-columns",
        default: Severity::Info,
        category: Category::Performance,
        fixable: false,
        implemented: true,
        description: "Warns when a SQL SELECT projects columns that the result interface does not use.",
    },
    RuleMeta {
        name: "exhaustive-match",
        default: Severity::Error,
        category: Category::Correctness,
        fixable: false,
        implemented: false,
        description: "Requires `match` over a closed type (enum, union, literal union, bool) to cover every case. Enforced by the compiler (E0004).",
    },
    RuleMeta {
        name: "export-protect-no-effect",
        default: Severity::Warn,
        category: Category::Style,
        fixable: false,
        implemented: true,
        description: "Warns when `export protect` sits in the source root, where its folder scope already covers the whole project and narrows nothing.",
    },
    RuleMeta {
        name: "http-no-raw-status-code",
        default: Severity::Warn,
        category: Category::Safety,
        fixable: false,
        implemented: true,
        description: "Warns when HTTP responses use raw numeric status codes instead of `HttpStatus` members.",
    },
    RuleMeta {
        name: "no-duplicate-case",
        default: Severity::Error,
        category: Category::Correctness,
        fixable: false,
        implemented: true,
        description: "Detects duplicate cases in `switch` or `match`.",
    },
    RuleMeta {
        name: "no-duplicate-imports",
        default: Severity::Warn,
        category: Category::Style,
        fixable: false,
        implemented: true,
        description: "Detects multiple import statements from the same module.",
    },
    RuleMeta {
        name: "no-empty-block",
        default: Severity::Warn,
        category: Category::Style,
        fixable: false,
        implemented: true,
        description: "Detects empty blocks without an explanatory comment.",
    },
    RuleMeta {
        name: "no-empty-catch",
        default: Severity::Warn,
        category: Category::Safety,
        fixable: false,
        implemented: true,
        description: "Detects an empty `catch` block without an explanatory comment.",
    },
    RuleMeta {
        name: "no-floating-enum",
        default: Severity::Error,
        category: Category::Correctness,
        fixable: false,
        implemented: false,
        description: "Forbids `f64`/`f128` values in an enum. Enforced by the compiler (E0701).",
    },
    RuleMeta {
        name: "no-force-unwrap",
        default: Severity::Error,
        category: Category::Safety,
        fixable: false,
        implemented: false,
        description: "Forbids forced unwrap (`expr!`) on optional values. Enforced by the compiler (E0716).",
    },
    RuleMeta {
        name: "no-heterogeneous-enum",
        default: Severity::Error,
        category: Category::Correctness,
        fixable: false,
        implemented: false,
        description: "Forbids mixing value types within a primitive enum. Enforced by the compiler (E0702).",
    },
    RuleMeta {
        name: "no-implicit-enum-primitive-cast",
        default: Severity::Error,
        category: Category::Safety,
        fixable: false,
        implemented: false,
        description: "Forbids implicit conversion of an enum to its underlying primitive. Enforced by the compiler (E0308).",
    },
    RuleMeta {
        name: "no-implicit-heavy-move",
        default: Severity::Warn,
        category: Category::Performance,
        fixable: false,
        implemented: false,
        description: "Warns when a large or non-Copy value is moved in a potentially surprising way.",
    },
    RuleMeta {
        name: "no-layer-skip",
        default: Severity::Warn,
        category: Category::Safety,
        fixable: false,
        implemented: true,
        description: "Warns when API code skips the route-controller-service-repository layering.",
    },
    RuleMeta {
        name: "no-null-literal",
        default: Severity::Error,
        category: Category::Correctness,
        fixable: false,
        implemented: true,
        description: "Forbids the `null` keyword. Nullable values use `T?`; absence is produced by optional fields, optional parameters, and external boundaries.",
    },
    RuleMeta {
        name: "no-shadow",
        default: Severity::Warn,
        category: Category::Style,
        fixable: false,
        implemented: true,
        description: "Detects a binding that shadows another binding from an enclosing scope.",
    },
    RuleMeta {
        name: "no-string-concat-in-loop",
        default: Severity::Warn,
        category: Category::Performance,
        fixable: false,
        implemented: true,
        description: "Detects repeated string concatenation inside a loop.",
    },
    RuleMeta {
        name: "no-uninitialized-variable",
        default: Severity::Warn,
        category: Category::Correctness,
        fixable: false,
        implemented: true,
        description: "Surfaces deferred `let x: T;` bindings that are used before assignment or never assigned (compiler diagnostics E0381/E0382).",
    },
    RuleMeta {
        name: "no-unnecessary-await",
        default: Severity::Warn,
        category: Category::Style,
        fixable: true,
        implemented: true,
        description: "Detects `return await promise` now that async returns adopt native Promises directly.",
    },
    RuleMeta {
        name: "no-unnecessary-allocation",
        default: Severity::Info,
        category: Category::Performance,
        fixable: false,
        implemented: true,
        description: "Detects clearly unnecessary allocations, such as a full `.slice(0)` copy.",
    },
    RuleMeta {
        name: "no-unnecessary-clone",
        default: Severity::Warn,
        category: Category::Performance,
        fixable: false,
        implemented: false,
        description: "Detects a clone where the value could be borrowed instead.",
    },
    RuleMeta {
        name: "no-upward-imports",
        default: Severity::Error,
        category: Category::Safety,
        fixable: false,
        implemented: true,
        description: "Forbids imports from lower API layers back into higher layers.",
    },
    RuleMeta {
        name: "no-unreachable-code",
        default: Severity::Error,
        category: Category::Correctness,
        fixable: false,
        implemented: true,
        description: "Detects statements that can never execute because they follow `return`, `break`, `continue`, or `throw`.",
    },
    RuleMeta {
        name: "no-unsafe-finally",
        default: Severity::Error,
        category: Category::Safety,
        fixable: false,
        implemented: true,
        description: "Detects `return`, `throw`, `break`, or `continue` inside a `finally` block, which can override the try/catch result.",
    },
    RuleMeta {
        name: "no-useless-null-check",
        default: Severity::Warn,
        category: Category::Correctness,
        fixable: false,
        implemented: false,
        description: "Detects a null check on a value that can never be null.",
    },
    RuleMeta {
        name: "no-unused-functions",
        default: Severity::Warn,
        category: Category::Correctness,
        fixable: false,
        implemented: true,
        description: "Detects non-exported functions that are never called. Exported functions are treated as public API.",
    },
    RuleMeta {
        name: "no-unused-imports",
        default: Severity::Warn,
        category: Category::Correctness,
        fixable: true,
        implemented: true,
        description: "Detects imported names that are never used.",
    },
    RuleMeta {
        name: "no-unused-vars",
        default: Severity::Warn,
        category: Category::Correctness,
        fixable: true,
        implemented: true,
        description: "Detects declared variables that are never used. Prefix a name with `_` to mark it intentionally unused.",
    },
    RuleMeta {
        name: "prefer-borrow",
        default: Severity::Info,
        category: Category::Performance,
        fixable: false,
        implemented: false,
        description: "Suggests passing a reference when a function does not need ownership.",
    },
    RuleMeta {
        name: "prefer-const",
        default: Severity::Warn,
        category: Category::Style,
        fixable: true,
        implemented: true,
        description: "Suggests `const` when a `let` binding is never reassigned.",
    },
    RuleMeta {
        name: "prefer-nullish-coalescing",
        default: Severity::Info,
        category: Category::Style,
        fixable: false,
        implemented: true,
        description: "Suggests `??` instead of a ternary that falls back on null.",
    },
    RuleMeta {
        name: "repository-no-http-error",
        default: Severity::Warn,
        category: Category::Safety,
        fixable: false,
        implemented: true,
        description: "Warns when repositories throw HTTP-layer errors instead of database/domain errors.",
    },
    RuleMeta {
        name: "repository-only-db-access",
        default: Severity::Error,
        category: Category::Safety,
        fixable: false,
        implemented: false,
        description: "Restricts direct database access to repository modules.",
    },
    RuleMeta {
        name: "route-no-business-logic",
        default: Severity::Warn,
        category: Category::Complexity,
        fixable: false,
        implemented: true,
        description: "Warns when route modules contain business logic instead of declarations.",
    },
    RuleMeta {
        name: "service-no-reply",
        default: Severity::Error,
        category: Category::Safety,
        fixable: false,
        implemented: true,
        description: "Forbids services from depending on `HttpReply` or sending HTTP responses directly.",
    },
    RuleMeta {
        name: "use-caught-error",
        default: Severity::Warn,
        category: Category::Safety,
        fixable: true,
        implemented: true,
        description: "Detects a caught error that is never used. Rename it to `_error` to mark it intentionally ignored.",
    },
];

pub fn rule_meta(name: &str) -> Option<&'static RuleMeta> {
    RULES.iter().find(|rule| rule.name == name)
}

#[derive(Debug, Clone)]
pub struct LintConfig {
    severities: BTreeMap<&'static str, Severity>,
}

impl Default for LintConfig {
    fn default() -> Self {
        Self {
            severities: RULES.iter().map(|rule| (rule.name, rule.default)).collect(),
        }
    }
}

impl LintConfig {
    pub fn severity(&self, rule: &str) -> Severity {
        self.severities.get(rule).copied().unwrap_or(Severity::Off)
    }

    pub fn is_enabled(&self, rule: &str) -> bool {
        self.severity(rule) != Severity::Off
    }

    pub fn from_json(source: &str) -> Result<Self, String> {
        let overrides: BTreeMap<String, Severity> = serde_json::from_str(source)
            .map_err(|error| format!("invalid tyrus-lint.json: {error}"))?;
        let mut config = LintConfig::default();
        for (name, severity) in overrides {
            if name == "api-prefer-plural-resources" {
                continue;
            }
            let Some(meta) = rule_meta(&name) else {
                return Err(format!("unknown lint rule `{name}` in tyrus-lint.json"));
            };
            config.severities.insert(meta.name, severity);
        }
        Ok(config)
    }

    pub fn load(dir: &Path) -> Result<Self, String> {
        let path = dir.join("tyrus-lint.json");
        match fs::read_to_string(&path) {
            Ok(source) => Self::from_json(&source),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(LintConfig::default()),
            Err(error) => Err(format!("could not read {}: {error}", path.display())),
        }
    }
}

pub fn default_config_json() -> String {
    let entries: Vec<(String, &'static str)> = RULES
        .iter()
        .filter(|rule| rule.implemented)
        .map(|rule| (rule.name.to_string(), rule.default.label_config()))
        .collect();
    render_config_json(&entries)
}

pub fn merge_config_json(existing: &str) -> Result<String, String> {
    let current: BTreeMap<String, Severity> = serde_json::from_str(existing)
        .map_err(|error| format!("invalid tyrus-lint.json: {error}"))?;
    let mut entries: Vec<(String, &'static str)> = Vec::new();
    let mut emitted: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for rule in RULES.iter().filter(|rule| rule.implemented) {
        let severity = current
            .get(rule.name)
            .copied()
            .unwrap_or(rule.default)
            .label_config();
        entries.push((rule.name.to_string(), severity));
        emitted.insert(rule.name);
    }
    for (name, severity) in &current {
        if !emitted.contains(name.as_str()) {
            entries.push((name.clone(), severity.label_config()));
        }
    }
    Ok(render_config_json(&entries))
}

fn render_config_json(entries: &[(String, &'static str)]) -> String {
    let mut body = String::new();
    for (index, (name, severity)) in entries.iter().enumerate() {
        let comma = if index + 1 < entries.len() { "," } else { "" };
        body.push_str(&format!("    {name:?}: {severity:?}{comma}\n"));
    }
    format!("{{\n{body}}}\n")
}

impl Severity {
    fn label_config(self) -> &'static str {
        match self {
            Severity::Off => "off",
            Severity::Info => "info",
            Severity::Warn => "warn",
            Severity::Error => "error",
        }
    }
}
