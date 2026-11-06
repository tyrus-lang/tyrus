use crate::ast::{
    Expr, ExprKind, Item, MatchArm, MatchPattern, Program, Stmt, StmtKind, TemplatePart,
    place_root_ident,
};
use crate::collections::{HashMap, HashSet};
use crate::span::Span;
use std::collections::{BTreeMap, BTreeSet};

pub const INVALID_TARGET: &str = "comptime-invalid-target";
pub const NON_CONSTANT: &str = "comptime-non-constant";
pub const FORBIDDEN_EFFECT: &str = "comptime-forbidden-effect";
pub const CYCLE: &str = "comptime-cycle";
pub const STEP_LIMIT: &str = "comptime-step-limit";
pub const DEPTH_LIMIT: &str = "comptime-depth-limit";
pub const VALUE_LIMIT: &str = "comptime-value-limit";
pub const UNSUPPORTED_VALUE: &str = "comptime-unsupported-value";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComptimeEntityKind {
    Function,
    ArrowBinding,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComptimeCallEdge {
    pub caller: String,
    pub callee: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComptimeCapture {
    pub name: String,
    pub use_span: Span,
    pub declaration_span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComptimeEntityFacts {
    pub name: String,
    pub kind: ComptimeEntityKind,
    pub declaration_span: Span,
    pub is_comptime: bool,
    pub calls: Vec<ComptimeCallEdge>,
    pub captures: Vec<ComptimeCapture>,
    pub strongly_connected_component: usize,
    pub eligible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComptimeConstBindingFacts {
    pub name: String,
    pub declaration_span: Span,
    pub dependencies: Vec<String>,
    pub eligible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComptimeRelatedInformation {
    pub span: Span,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComptimeDiagnostic {
    pub code: &'static str,
    pub message: String,
    pub span: Span,
    pub related: Vec<ComptimeRelatedInformation>,
    pub help: Vec<String>,
    pub remove_wrapper: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ComptimeFacts {
    pub entities: Vec<ComptimeEntityFacts>,
    pub const_bindings: Vec<ComptimeConstBindingFacts>,
    pub call_edges: Vec<ComptimeCallEdge>,
    pub diagnostics: Vec<ComptimeDiagnostic>,
}

#[derive(Debug, Clone)]
struct Violation {
    code: &'static str,
    message: String,
    span: Span,
}

#[derive(Debug, Clone)]
struct EntityWork<'a> {
    name: &'a str,
    kind: ComptimeEntityKind,
    declaration_span: Span,
    is_comptime: bool,
    params: BTreeSet<String>,
    body: &'a [Stmt],
    calls: Vec<ComptimeCallEdge>,
    captures: Vec<ComptimeCapture>,
    violations: Vec<Violation>,
}

#[derive(Debug, Clone)]
struct TopBinding<'a> {
    mutable: bool,
    declaration_span: Span,
    value: Option<&'a Expr>,
}

#[derive(Debug, Clone, Copy)]
struct LocalBinding {
    constant: bool,
}

pub fn analyze(program: &Program, source: &str) -> ComptimeFacts {
    Analyzer::new(program, source).run()
}

struct Analyzer<'a> {
    program: &'a Program,
    source: &'a str,
    top_bindings: BTreeMap<String, TopBinding<'a>>,
    enum_names: HashSet<String>,
    entity_indices: BTreeMap<String, usize>,
    comptime_entities: HashSet<String>,
    entities: Vec<EntityWork<'a>>,
}

impl<'a> Analyzer<'a> {
    fn new(program: &'a Program, source: &'a str) -> Self {
        let mut top_bindings = BTreeMap::new();
        let mut enum_names = HashSet::default();
        let mut entities = Vec::new();
        for item in &program.items {
            match item {
                Item::FnDecl {
                    name,
                    is_comptime,
                    params,
                    body,
                    span,
                    ..
                } => {
                    let kind = if source.get(span.start..span.end).is_some_and(|text| {
                        text.trim_start().starts_with("comptime const")
                            || text.trim_start().starts_with("const ")
                    }) {
                        ComptimeEntityKind::ArrowBinding
                    } else {
                        ComptimeEntityKind::Function
                    };
                    let params = params
                        .iter()
                        .flat_map(|param| param.pattern.binding_names())
                        .map(str::to_string)
                        .collect();
                    entities.push(EntityWork {
                        name,
                        kind,
                        declaration_span: *span,
                        is_comptime: *is_comptime,
                        params,
                        body,
                        calls: Vec::new(),
                        captures: Vec::new(),
                        violations: Vec::new(),
                    });
                }
                Item::Stmt(stmt) => {
                    if let StmtKind::VarDecl {
                        mutable,
                        name,
                        value,
                        ..
                    } = stmt.kind()
                    {
                        top_bindings.insert(
                            name.clone(),
                            TopBinding {
                                mutable: *mutable,
                                declaration_span: stmt.span,
                                value: value.as_ref(),
                            },
                        );
                    }
                }
                Item::EnumDecl { name, .. } => {
                    enum_names.insert(name.clone());
                }
                _ => {}
            }
        }
        let entity_indices = entities
            .iter()
            .enumerate()
            .map(|(index, entity)| (entity.name.to_string(), index))
            .collect();
        let comptime_entities = entities
            .iter()
            .filter(|entity| entity.is_comptime)
            .map(|entity| entity.name.to_string())
            .collect();
        Self {
            program,
            source,
            top_bindings,
            enum_names,
            entity_indices,
            comptime_entities,
            entities,
        }
    }

    fn run(mut self) -> ComptimeFacts {
        for index in 0..self.entities.len() {
            self.collect_entity(index);
        }
        let components = strongly_connected_components(&self.entities, &self.entity_indices);
        let (eligible, const_eligibility) = self.eligibility_and_constants(&components);
        let mut diagnostics = self.entity_diagnostics(&components, &eligible, &const_eligibility);
        self.collect_expression_diagnostics(&eligible, &const_eligibility, &mut diagnostics);
        deduplicate_diagnostics(&mut diagnostics);

        let call_edges = self
            .entities
            .iter()
            .flat_map(|entity| entity.calls.iter().cloned())
            .collect();
        let entities = self
            .entities
            .iter()
            .enumerate()
            .map(|(index, entity)| ComptimeEntityFacts {
                name: entity.name.to_string(),
                kind: entity.kind,
                declaration_span: entity.declaration_span,
                is_comptime: entity.is_comptime,
                calls: entity.calls.clone(),
                captures: entity.captures.clone(),
                strongly_connected_component: components[index],
                eligible: eligible[index],
            })
            .collect();
        let const_bindings = self
            .top_bindings
            .iter()
            .filter(|(_, binding)| !binding.mutable)
            .map(|(name, binding)| {
                let mut dependencies = BTreeSet::new();
                if let Some(value) = binding.value {
                    collect_identifiers(value, &mut dependencies);
                }
                ComptimeConstBindingFacts {
                    name: name.clone(),
                    declaration_span: binding.declaration_span,
                    dependencies: dependencies.into_iter().collect(),
                    eligible: const_eligibility.get(name).copied().unwrap_or(false),
                }
            })
            .collect();
        ComptimeFacts {
            entities,
            const_bindings,
            call_edges,
            diagnostics,
        }
    }

    fn collect_entity(&mut self, index: usize) {
        let entity_name = self.entities[index].name;
        let declaration_span = self.entities[index].declaration_span;
        let mut locals = self.entities[index].params.clone();
        collect_local_names(self.entities[index].body, &mut locals);
        let body = self.entities[index].body;
        if self.entities[index].is_comptime {
            let declaration = self
                .program
                .items
                .iter()
                .find(|item| matches!(item, Item::FnDecl { name, .. } if name == entity_name));
            if matches!(declaration, Some(Item::FnDecl { is_async: true, .. })) {
                let async_span = self.keyword_span(declaration_span, "async");
                self.entities[index].violations.push(Violation {
                    code: FORBIDDEN_EFFECT,
                    message: format!("comptime entity `{entity_name}` cannot be async"),
                    span: async_span,
                });
            }
            if let Some(Item::FnDecl { throws, .. }) = declaration
                && !throws.is_empty()
            {
                let throws_span = self.keyword_span(declaration_span, "throws");
                self.entities[index].violations.push(Violation {
                    code: FORBIDDEN_EFFECT,
                    message: format!(
                        "comptime entity `{entity_name}` cannot declare thrown errors"
                    ),
                    span: throws_span,
                });
            }
        }
        let mut scanner = EntityScanner {
            entity_name: self.entities[index].name,
            locals: &locals,
            top_bindings: &self.top_bindings,
            enum_names: &self.enum_names,
            entity_indices: &self.entity_indices,
            comptime_entities: &self.comptime_entities,
            calls: Vec::new(),
            captures: Vec::new(),
            violations: Vec::new(),
        };
        if let Some(Item::FnDecl { params, .. }) = self
            .program
            .items
            .iter()
            .find(|item| matches!(item, Item::FnDecl { name, .. } if name == entity_name))
        {
            for param in params {
                if !param.decorators.is_empty() {
                    scanner.forbidden(
                        param.span,
                        format!("comptime entity `{entity_name}` cannot use parameter decorators"),
                    );
                }
                if let Some(default) = &param.default_value {
                    scanner.expr(default);
                }
            }
        }
        scanner.statements(body);
        self.entities[index].calls = scanner.calls;
        self.entities[index].captures = scanner.captures;
        self.entities[index].violations.extend(scanner.violations);
    }

    fn base_entity_eligibility(&self, components: &[usize]) -> Vec<bool> {
        let component_count = components
            .iter()
            .copied()
            .max()
            .map_or(0, |value| value + 1);
        let mut component_eligible = vec![true; component_count];
        for (index, entity) in self.entities.iter().enumerate() {
            if !entity.is_comptime || !entity.violations.is_empty() {
                component_eligible[components[index]] = false;
            }
        }
        #[allow(clippy::needless_range_loop)]
        for component in 0..component_count {
            let members = components
                .iter()
                .enumerate()
                .filter_map(|(index, value)| (*value == component).then_some(index))
                .collect::<Vec<_>>();
            let recursive = members.len() > 1
                || members.first().is_some_and(|member| {
                    self.entities[*member]
                        .calls
                        .iter()
                        .any(|edge| edge.callee == self.entities[*member].name)
                });
            if recursive
                && members
                    .iter()
                    .any(|member| self.entities[*member].kind == ComptimeEntityKind::ArrowBinding)
            {
                component_eligible[component] = false;
            }
        }
        self.propagate_component_eligibility(components, &mut component_eligible);
        components
            .iter()
            .map(|component| component_eligible[*component])
            .collect()
    }

    fn propagate_component_eligibility(
        &self,
        components: &[usize],
        component_eligible: &mut [bool],
    ) {
        let mut changed = true;
        while changed {
            changed = false;
            for (index, entity) in self.entities.iter().enumerate() {
                let component = components[index];
                if !component_eligible[component] {
                    continue;
                }
                if entity.calls.iter().any(|edge| {
                    self.entity_indices
                        .get(&edge.callee)
                        .is_none_or(|callee| !component_eligible[components[*callee]])
                }) {
                    component_eligible[component] = false;
                    changed = true;
                }
            }
        }
    }

    fn eligibility_and_constants(
        &self,
        components: &[usize],
    ) -> (Vec<bool>, BTreeMap<String, bool>) {
        let base = self.base_entity_eligibility(components);
        let mut eligible = base.clone();
        let mut constants = self.const_eligibility(&eligible);
        for _ in 0..=self.entities.len() + self.top_bindings.len() {
            let mut component_eligible = vec![
                true;
                components
                    .iter()
                    .copied()
                    .max()
                    .map_or(0, |value| value + 1)
            ];
            for (index, is_eligible) in base.iter().enumerate() {
                if !is_eligible
                    || self.entities[index]
                        .captures
                        .iter()
                        .any(|capture| !constants.get(&capture.name).copied().unwrap_or(false))
                {
                    component_eligible[components[index]] = false;
                }
            }
            self.propagate_component_eligibility(components, &mut component_eligible);
            let next_eligible = components
                .iter()
                .map(|component| component_eligible[*component])
                .collect::<Vec<_>>();
            let next_constants = self.const_eligibility(&next_eligible);
            if next_eligible == eligible && next_constants == constants {
                break;
            }
            eligible = next_eligible;
            constants = next_constants;
        }
        (eligible, constants)
    }

    fn entity_diagnostics(
        &self,
        components: &[usize],
        eligible: &[bool],
        const_eligible: &BTreeMap<String, bool>,
    ) -> Vec<ComptimeDiagnostic> {
        let mut diagnostics = Vec::new();
        for (index, entity) in self
            .entities
            .iter()
            .enumerate()
            .filter(|(_, entity)| entity.is_comptime)
        {
            for violation in &entity.violations {
                let mut related = vec![ComptimeRelatedInformation {
                    span: entity.declaration_span,
                    message: format!("comptime entity `{}` is declared here", entity.name),
                }];
                if let Some(capture) = entity
                    .captures
                    .iter()
                    .find(|capture| capture.use_span == violation.span)
                {
                    related.push(ComptimeRelatedInformation {
                        span: capture.declaration_span,
                        message: format!("captured binding `{}` is declared here", capture.name),
                    });
                }
                diagnostics.push(ComptimeDiagnostic {
                    code: violation.code,
                    message: violation.message.clone(),
                    span: violation.span,
                    related,
                    help: violation_help(violation.code, entity.name),
                    remove_wrapper: false,
                });
            }
            let members = components
                .iter()
                .enumerate()
                .filter_map(|(member, component)| {
                    (*component == components[index]).then_some(member)
                })
                .collect::<Vec<_>>();
            let recursive =
                members.len() > 1 || entity.calls.iter().any(|edge| edge.callee == entity.name);
            if recursive
                && members
                    .iter()
                    .any(|member| self.entities[*member].kind == ComptimeEntityKind::ArrowBinding)
            {
                let edge = entity.calls.iter().find(|edge| {
                    members
                        .iter()
                        .any(|member| self.entities[*member].name == edge.callee)
                });
                diagnostics.push(ComptimeDiagnostic {
                    code: CYCLE,
                    message: format!("comptime arrow binding `{}` cannot participate in a recursive cycle", entity.name),
                    span: edge.map_or(entity.declaration_span, |edge| edge.span),
                    related: members
                        .iter()
                        .map(|member| ComptimeRelatedInformation {
                            span: self.entities[*member].declaration_span,
                            message: format!("cycle member `{}` is declared here", self.entities[*member].name),
                        })
                        .collect(),
                    help: vec![format!("declare `{}` with `comptime function` to make recursive initialization explicit", entity.name)],
                    remove_wrapper: false,
                });
            }
            if !eligible[index] && entity.violations.is_empty() && !recursive {
                if let Some((violation, path)) =
                    self.transitive_violation(index, &mut HashSet::default())
                {
                    let mut related = vec![ComptimeRelatedInformation {
                        span: entity.declaration_span,
                        message: format!(
                            "comptime entity `{}` reaches this operation",
                            entity.name
                        ),
                    }];
                    related.extend(path.into_iter().map(|edge| ComptimeRelatedInformation {
                        span: edge.span,
                        message: format!("`{}` calls `{}` here", edge.caller, edge.callee),
                    }));
                    diagnostics.push(ComptimeDiagnostic {
                        code: violation.code,
                        message: format!(
                            "comptime entity `{}` transitively reaches an ineligible operation: {}",
                            entity.name, violation.message
                        ),
                        span: violation.span,
                        related,
                        help: violation_help(violation.code, entity.name),
                        remove_wrapper: false,
                    });
                } else if let Some(capture) = entity
                    .captures
                    .iter()
                    .find(|capture| !const_eligible.get(&capture.name).copied().unwrap_or(false))
                {
                    let cycle = self.const_cycle(&capture.name);
                    let code = if cycle.is_some() { CYCLE } else { NON_CONSTANT };
                    let mut related = vec![ComptimeRelatedInformation {
                        span: capture.declaration_span,
                        message: format!("captured binding `{}` is declared here", capture.name),
                    }];
                    if let Some(cycle) = &cycle {
                        related.extend(cycle.iter().filter_map(|name| {
                            self.top_bindings
                                .get(name)
                                .map(|binding| ComptimeRelatedInformation {
                                    span: binding.declaration_span,
                                    message: format!("constant cycle includes `{name}`"),
                                })
                        }));
                    }
                    diagnostics.push(ComptimeDiagnostic {
                        code,
                        message: cycle.map_or_else(
                            || format!("comptime entity `{}` captures non-constant binding `{}`", entity.name, capture.name),
                            |_| format!("comptime entity `{}` captures constant `{}` from an initialization cycle", entity.name, capture.name),
                        ),
                        span: capture.use_span,
                        related,
                        help: if code == CYCLE { vec![format!("break the constant initialization cycle that reaches `{}`", capture.name)] } else { violation_help(NON_CONSTANT, entity.name) },
                        remove_wrapper: false,
                    });
                }
            }
        }
        diagnostics
    }

    fn transitive_violation(
        &self,
        index: usize,
        visiting: &mut HashSet<usize>,
    ) -> Option<(Violation, Vec<ComptimeCallEdge>)> {
        if !visiting.insert(index) {
            return None;
        }
        if let Some(violation) = self.entities[index].violations.first() {
            visiting.remove(&index);
            return Some((violation.clone(), Vec::new()));
        }
        for edge in &self.entities[index].calls {
            let Some(callee) = self.entity_indices.get(&edge.callee).copied() else {
                continue;
            };
            if let Some((violation, mut path)) = self.transitive_violation(callee, visiting) {
                path.insert(0, edge.clone());
                visiting.remove(&index);
                return Some((violation, path));
            }
        }
        visiting.remove(&index);
        None
    }

    fn const_eligibility(&self, entity_eligible: &[bool]) -> BTreeMap<String, bool> {
        let mut result = self
            .top_bindings
            .keys()
            .map(|name| (name.clone(), false))
            .collect::<BTreeMap<_, _>>();
        for _ in 0..=result.len() {
            let previous = result.clone();
            for (name, binding) in &self.top_bindings {
                let eligible = !binding.mutable
                    && binding.value.is_some_and(|value| {
                        self.expression_is_constant(
                            value,
                            &HashMap::default(),
                            entity_eligible,
                            &previous,
                        )
                    });
                result.insert(name.clone(), eligible);
            }
            if result == previous {
                break;
            }
        }
        result
    }

    fn expression_is_constant(
        &self,
        expr: &Expr,
        locals: &HashMap<String, LocalBinding>,
        entity_eligible: &[bool],
        const_eligible: &BTreeMap<String, bool>,
    ) -> bool {
        ConstantExpression {
            enum_names: &self.enum_names,
            entity_indices: &self.entity_indices,
            entity_eligible,
            const_eligible,
            locals,
        }
        .check(expr)
    }

    fn collect_expression_diagnostics(
        &self,
        entity_eligible: &[bool],
        const_eligible: &BTreeMap<String, bool>,
        diagnostics: &mut Vec<ComptimeDiagnostic>,
    ) {
        for item in &self.program.items {
            match item {
                Item::FnDecl {
                    name,
                    is_comptime,
                    params,
                    body,
                    ..
                } => {
                    let mut locals = HashMap::default();
                    for param in params {
                        for name in param.pattern.binding_names() {
                            locals.insert(
                                name.to_string(),
                                LocalBinding {
                                    constant: *is_comptime,
                                },
                            );
                        }
                    }
                    self.validate_statements(
                        body,
                        &mut locals,
                        entity_eligible,
                        const_eligible,
                        diagnostics,
                        Some(name),
                    );
                }
                Item::Stmt(stmt) => {
                    self.validate_statement(
                        stmt,
                        &mut HashMap::default(),
                        entity_eligible,
                        const_eligible,
                        diagnostics,
                        None,
                    );
                }
                Item::EnumDecl { members, .. } => {
                    for member in members {
                        if let Some(value) = &member.value {
                            self.validate_expr(
                                value,
                                &HashMap::default(),
                                entity_eligible,
                                const_eligible,
                                diagnostics,
                                None,
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn validate_statements(
        &self,
        statements: &[Stmt],
        locals: &mut HashMap<String, LocalBinding>,
        entity_eligible: &[bool],
        const_eligible: &BTreeMap<String, bool>,
        diagnostics: &mut Vec<ComptimeDiagnostic>,
        enclosing: Option<&str>,
    ) {
        for statement in statements {
            self.validate_statement(
                statement,
                locals,
                entity_eligible,
                const_eligible,
                diagnostics,
                enclosing,
            );
        }
    }

    #[allow(clippy::too_many_lines)]
    fn validate_statement(
        &self,
        statement: &Stmt,
        locals: &mut HashMap<String, LocalBinding>,
        entity_eligible: &[bool],
        const_eligible: &BTreeMap<String, bool>,
        diagnostics: &mut Vec<ComptimeDiagnostic>,
        enclosing: Option<&str>,
    ) {
        match statement.kind() {
            StmtKind::VarDecl {
                mutable,
                name,
                value,
                ..
            } => {
                if let Some(value) = value {
                    self.validate_expr(
                        value,
                        locals,
                        entity_eligible,
                        const_eligible,
                        diagnostics,
                        enclosing,
                    );
                }
                let constant = !mutable
                    && value.as_ref().is_some_and(|value| {
                        self.expression_is_constant(value, locals, entity_eligible, const_eligible)
                    });
                locals.insert(name.clone(), LocalBinding { constant });
            }
            StmtKind::ArrayDestructure {
                mutable,
                bindings,
                value,
            }
            | StmtKind::ObjectDestructure {
                mutable,
                bindings,
                value,
            } => {
                self.validate_expr(
                    value,
                    locals,
                    entity_eligible,
                    const_eligible,
                    diagnostics,
                    enclosing,
                );
                let constant = !mutable
                    && self.expression_is_constant(value, locals, entity_eligible, const_eligible);
                for binding in bindings {
                    locals.insert(binding.name.clone(), LocalBinding { constant });
                }
            }
            StmtKind::Assign { name, value, .. } => {
                self.validate_expr(
                    value,
                    locals,
                    entity_eligible,
                    const_eligible,
                    diagnostics,
                    enclosing,
                );
                if let Some(binding) = locals.get_mut(name) {
                    binding.constant = false;
                }
            }
            StmtKind::IndexAssign { index, value, .. } => {
                self.validate_expr(
                    index,
                    locals,
                    entity_eligible,
                    const_eligible,
                    diagnostics,
                    enclosing,
                );
                self.validate_expr(
                    value,
                    locals,
                    entity_eligible,
                    const_eligible,
                    diagnostics,
                    enclosing,
                );
            }
            StmtKind::FieldAssign { object, value, .. } => {
                self.validate_expr(
                    object,
                    locals,
                    entity_eligible,
                    const_eligible,
                    diagnostics,
                    enclosing,
                );
                self.validate_expr(
                    value,
                    locals,
                    entity_eligible,
                    const_eligible,
                    diagnostics,
                    enclosing,
                );
            }
            StmtKind::Return(value) => {
                if let Some(value) = value {
                    self.validate_expr(
                        value,
                        locals,
                        entity_eligible,
                        const_eligible,
                        diagnostics,
                        enclosing,
                    );
                }
            }
            StmtKind::Throw(value) | StmtKind::Expr(value) => self.validate_expr(
                value,
                locals,
                entity_eligible,
                const_eligible,
                diagnostics,
                enclosing,
            ),
            StmtKind::TryCatch {
                try_body,
                catch_binding,
                catch_body,
                finally_body,
                ..
            } => {
                let mut branch = locals.clone();
                self.validate_statements(
                    try_body,
                    &mut branch,
                    entity_eligible,
                    const_eligible,
                    diagnostics,
                    enclosing,
                );
                let mut handler = locals.clone();
                handler.insert(catch_binding.clone(), LocalBinding { constant: false });
                self.validate_statements(
                    catch_body,
                    &mut handler,
                    entity_eligible,
                    const_eligible,
                    diagnostics,
                    enclosing,
                );
                if let Some(finally) = finally_body {
                    self.validate_statements(
                        finally,
                        &mut locals.clone(),
                        entity_eligible,
                        const_eligible,
                        diagnostics,
                        enclosing,
                    );
                }
            }
            StmtKind::Defer { body, .. } => self.validate_statements(
                body,
                &mut locals.clone(),
                entity_eligible,
                const_eligible,
                diagnostics,
                enclosing,
            ),
            StmtKind::While { cond, body } => {
                self.validate_expr(
                    cond,
                    locals,
                    entity_eligible,
                    const_eligible,
                    diagnostics,
                    enclosing,
                );
                self.validate_statements(
                    body,
                    &mut locals.clone(),
                    entity_eligible,
                    const_eligible,
                    diagnostics,
                    enclosing,
                );
            }
            StmtKind::DoWhile { body, cond } => {
                self.validate_statements(
                    body,
                    &mut locals.clone(),
                    entity_eligible,
                    const_eligible,
                    diagnostics,
                    enclosing,
                );
                self.validate_expr(
                    cond,
                    locals,
                    entity_eligible,
                    const_eligible,
                    diagnostics,
                    enclosing,
                );
            }
            StmtKind::For {
                init,
                cond,
                update,
                body,
            } => {
                let mut scope = locals.clone();
                self.validate_statement(
                    init,
                    &mut scope,
                    entity_eligible,
                    const_eligible,
                    diagnostics,
                    enclosing,
                );
                self.validate_expr(
                    cond,
                    &scope,
                    entity_eligible,
                    const_eligible,
                    diagnostics,
                    enclosing,
                );
                self.validate_statement(
                    update,
                    &mut scope,
                    entity_eligible,
                    const_eligible,
                    diagnostics,
                    enclosing,
                );
                self.validate_statements(
                    body,
                    &mut scope,
                    entity_eligible,
                    const_eligible,
                    diagnostics,
                    enclosing,
                );
            }
            StmtKind::ForOf {
                name,
                iterable,
                body,
                ..
            } => {
                self.validate_expr(
                    iterable,
                    locals,
                    entity_eligible,
                    const_eligible,
                    diagnostics,
                    enclosing,
                );
                let mut scope = locals.clone();
                scope.insert(name.clone(), LocalBinding { constant: false });
                self.validate_statements(
                    body,
                    &mut scope,
                    entity_eligible,
                    const_eligible,
                    diagnostics,
                    enclosing,
                );
            }
            StmtKind::ForIn { name, object, body } => {
                self.validate_expr(
                    object,
                    locals,
                    entity_eligible,
                    const_eligible,
                    diagnostics,
                    enclosing,
                );
                let mut scope = locals.clone();
                scope.insert(name.clone(), LocalBinding { constant: false });
                self.validate_statements(
                    body,
                    &mut scope,
                    entity_eligible,
                    const_eligible,
                    diagnostics,
                    enclosing,
                );
            }
            StmtKind::Switch {
                expr,
                cases,
                default,
                ..
            } => {
                self.validate_expr(
                    expr,
                    locals,
                    entity_eligible,
                    const_eligible,
                    diagnostics,
                    enclosing,
                );
                for case in cases {
                    self.validate_expr(
                        &case.value,
                        locals,
                        entity_eligible,
                        const_eligible,
                        diagnostics,
                        enclosing,
                    );
                    self.validate_statements(
                        &case.body,
                        &mut locals.clone(),
                        entity_eligible,
                        const_eligible,
                        diagnostics,
                        enclosing,
                    );
                }
                if let Some(default) = default {
                    self.validate_statements(
                        default,
                        &mut locals.clone(),
                        entity_eligible,
                        const_eligible,
                        diagnostics,
                        enclosing,
                    );
                }
            }
            StmtKind::Match {
                expr,
                arms,
                default,
            } => {
                self.validate_expr(
                    expr,
                    locals,
                    entity_eligible,
                    const_eligible,
                    diagnostics,
                    enclosing,
                );
                for arm in arms {
                    self.validate_match_arm(
                        arm,
                        locals,
                        entity_eligible,
                        const_eligible,
                        diagnostics,
                        enclosing,
                    );
                }
                if let Some(default) = default {
                    self.validate_statements(
                        default,
                        &mut locals.clone(),
                        entity_eligible,
                        const_eligible,
                        diagnostics,
                        enclosing,
                    );
                }
            }
            StmtKind::Break(value) => {
                if let Some(value) = value {
                    self.validate_expr(
                        value,
                        &locals.clone(),
                        entity_eligible,
                        const_eligible,
                        diagnostics,
                        enclosing,
                    );
                }
            }
            StmtKind::Continue => {}
        }
    }

    fn validate_match_arm(
        &self,
        arm: &MatchArm,
        locals: &HashMap<String, LocalBinding>,
        entity_eligible: &[bool],
        const_eligible: &BTreeMap<String, bool>,
        diagnostics: &mut Vec<ComptimeDiagnostic>,
        enclosing: Option<&str>,
    ) {
        let mut scope = locals.clone();
        collect_pattern_bindings(&arm.pattern, &mut scope);
        if let Some(guard) = &arm.guard {
            self.validate_expr(
                guard,
                &scope,
                entity_eligible,
                const_eligible,
                diagnostics,
                enclosing,
            );
        }
        self.validate_statements(
            &arm.body,
            &mut scope,
            entity_eligible,
            const_eligible,
            diagnostics,
            enclosing,
        );
    }

    fn validate_expr(
        &self,
        expr: &Expr,
        locals: &HashMap<String, LocalBinding>,
        entity_eligible: &[bool],
        const_eligible: &BTreeMap<String, bool>,
        diagnostics: &mut Vec<ComptimeDiagnostic>,
        enclosing: Option<&str>,
    ) {
        if let ExprKind::Comptime(inner) = expr.kind() {
            let mut validator = ExpressionValidator {
                analyzer: self,
                locals,
                entity_eligible,
                const_eligible,
                wrapper_span: expr.span,
                enclosing,
                diagnostics,
            };
            validator.check(inner);
            return;
        }
        visit_expr_children(expr, &mut |child| {
            self.validate_expr(
                child,
                locals,
                entity_eligible,
                const_eligible,
                diagnostics,
                enclosing,
            );
        });
        if let ExprKind::ArrowFn { body, .. } = expr.kind() {
            self.validate_statements(
                body,
                &mut locals.clone(),
                entity_eligible,
                const_eligible,
                diagnostics,
                enclosing,
            );
        }
    }

    fn keyword_span(&self, container: Span, keyword: &str) -> Span {
        self.source
            .get(container.start..container.end)
            .and_then(|text| text.find(keyword))
            .map_or(container, |offset| {
                Span::new(
                    container.start + offset,
                    container.start + offset + keyword.len(),
                )
            })
    }

    fn const_cycle(&self, start: &str) -> Option<Vec<String>> {
        fn visit(
            analyzer: &Analyzer<'_>,
            name: &str,
            visiting: &mut Vec<String>,
            visited: &mut HashSet<String>,
        ) -> Option<Vec<String>> {
            if let Some(index) = visiting.iter().position(|current| current == name) {
                return Some(visiting[index..].to_vec());
            }
            if !visited.insert(name.to_string()) {
                return None;
            }
            let binding = analyzer.top_bindings.get(name)?;
            let mut dependencies = BTreeSet::new();
            if let Some(value) = binding.value {
                collect_identifiers(value, &mut dependencies);
            }
            visiting.push(name.to_string());
            for dependency in dependencies {
                if analyzer.top_bindings.contains_key(&dependency)
                    && let Some(cycle) = visit(analyzer, &dependency, visiting, visited)
                {
                    return Some(cycle);
                }
            }
            visiting.pop();
            None
        }
        visit(self, start, &mut Vec::new(), &mut HashSet::default())
    }
}

struct EntityScanner<'a> {
    entity_name: &'a str,
    locals: &'a BTreeSet<String>,
    top_bindings: &'a BTreeMap<String, TopBinding<'a>>,
    enum_names: &'a HashSet<String>,
    entity_indices: &'a BTreeMap<String, usize>,
    comptime_entities: &'a HashSet<String>,
    calls: Vec<ComptimeCallEdge>,
    captures: Vec<ComptimeCapture>,
    violations: Vec<Violation>,
}

impl EntityScanner<'_> {
    fn statements(&mut self, statements: &[Stmt]) {
        for statement in statements {
            self.statement(statement);
        }
    }

    fn statement(&mut self, statement: &Stmt) {
        match statement.kind() {
            StmtKind::VarDecl { value, .. } => {
                if let Some(value) = value {
                    self.expr(value);
                }
            }
            StmtKind::ArrayDestructure { value, .. }
            | StmtKind::ObjectDestructure { value, .. } => self.expr(value),
            StmtKind::Assign { name, value, .. } => {
                self.expr(value);
                if !self.locals.contains(name) {
                    self.forbidden(
                        statement.span,
                        format!(
                            "comptime entity `{}` cannot write external binding `{name}`",
                            self.entity_name
                        ),
                    );
                }
            }
            StmtKind::IndexAssign {
                name, index, value, ..
            } => {
                self.expr(index);
                self.expr(value);
                if !self.locals.contains(name) {
                    self.forbidden(
                        statement.span,
                        format!(
                            "comptime entity `{}` cannot mutate external indexed value `{name}`",
                            self.entity_name
                        ),
                    );
                }
            }
            StmtKind::FieldAssign { object, value, .. } => {
                self.expr(object);
                self.expr(value);
                let local_root =
                    place_root_ident(object).is_some_and(|name| self.locals.contains(name));
                if !local_root {
                    self.forbidden(
                        statement.span,
                        format!(
                            "comptime entity `{}` cannot mutate an external object field",
                            self.entity_name
                        ),
                    );
                }
            }
            StmtKind::Return(value) => {
                if let Some(value) = value {
                    self.expr(value);
                }
            }
            StmtKind::Throw(value) => {
                self.expr(value);
                self.forbidden(
                    statement.span,
                    format!("comptime entity `{}` cannot throw", self.entity_name),
                );
            }
            StmtKind::TryCatch {
                try_body,
                catch_body,
                finally_body,
                ..
            } => {
                self.forbidden(
                    statement.span,
                    format!(
                        "comptime entity `{}` cannot catch exceptions",
                        self.entity_name
                    ),
                );
                self.statements(try_body);
                self.statements(catch_body);
                if let Some(body) = finally_body {
                    self.statements(body);
                }
            }
            StmtKind::While { cond, body } | StmtKind::DoWhile { cond, body } => {
                self.expr(cond);
                self.statements(body);
            }
            StmtKind::For {
                init,
                cond,
                update,
                body,
            } => {
                self.statement(init);
                self.expr(cond);
                self.statement(update);
                self.statements(body);
            }
            StmtKind::ForOf { iterable, body, .. } => {
                self.expr(iterable);
                self.statements(body);
            }
            StmtKind::ForIn { object, body, .. } => {
                self.expr(object);
                self.statements(body);
            }
            StmtKind::Switch {
                expr,
                cases,
                default,
                ..
            } => {
                self.expr(expr);
                for case in cases {
                    self.expr(&case.value);
                    self.statements(&case.body);
                }
                if let Some(default) = default {
                    self.statements(default);
                }
            }
            StmtKind::Match {
                expr,
                arms,
                default,
            } => {
                self.expr(expr);
                for arm in arms {
                    scan_pattern(&arm.pattern, self);
                    if let Some(guard) = &arm.guard {
                        self.expr(guard);
                    }
                    self.statements(&arm.body);
                }
                if let Some(default) = default {
                    self.statements(default);
                }
            }
            StmtKind::Expr(expr) => self.expr(expr),
            StmtKind::Defer { body, .. } => self.statements(body),
            StmtKind::Break(value) => {
                if let Some(value) = value {
                    self.expr(value);
                }
            }
            StmtKind::Continue => {}
        }
    }

    fn expr(&mut self, expr: &Expr) {
        match expr.kind() {
            ExprKind::Ident(name) => self.read(name, expr.span),
            ExprKind::FnCall { callee, args, .. } => {
                self.calls.push(ComptimeCallEdge {
                    caller: self.entity_name.to_string(),
                    callee: callee.clone(),
                    span: expr.span,
                });
                if !self.comptime_entities.contains(callee) {
                    self.violations.push(Violation {
                        code: INVALID_TARGET,
                        message: format!("call to `{callee}` is not a local comptime entity"),
                        span: expr.span,
                    });
                }
                for arg in args {
                    self.expr(arg);
                }
            }
            ExprKind::MethodCall {
                object,
                method,
                method_span,
                args,
                ..
            } => {
                self.expr(object);
                for arg in args {
                    self.expr(arg);
                }
                self.forbidden(
                    *method_span,
                    format!("method call `{method}` is not allowed in comptime evaluation"),
                );
            }
            ExprKind::OptionalMethodCall {
                object,
                method,
                args,
                ..
            } => {
                self.expr(object);
                for arg in args {
                    self.expr(arg);
                }
                self.forbidden(
                    expr.span,
                    format!("method call `{method}` is not allowed in comptime evaluation"),
                );
            }
            ExprKind::CallValue { callee, args, .. } => {
                self.expr(callee);
                for arg in args {
                    self.expr(arg);
                }
                self.violations.push(Violation {
                    code: INVALID_TARGET,
                    message: "indirect function calls are not eligible for comptime evaluation"
                        .to_string(),
                    span: expr.span,
                });
            }
            ExprKind::Await(inner) => {
                self.expr(inner);
                self.forbidden(
                    expr.span,
                    format!("comptime entity `{}` cannot await", self.entity_name),
                );
            }
            ExprKind::Try(inner) => self.expr(inner),
            ExprKind::Borrow { .. } => self.violations.push(Violation {
                code: UNSUPPORTED_VALUE,
                message: "references are not materializable comptime values".to_string(),
                span: expr.span,
            }),
            ExprKind::ArrowFn { body, .. } => {
                self.statements(body);
                self.violations.push(Violation {
                    code: UNSUPPORTED_VALUE,
                    message: "closures cannot be created by a comptime entity".to_string(),
                    span: expr.span,
                });
            }
            ExprKind::If {
                cond,
                then_block,
                else_block,
            } => {
                self.expr(cond);
                self.statements(then_block);
                if let Some(block) = else_block {
                    self.statements(block);
                }
            }
            ExprKind::PrefixIncr(inner)
            | ExprKind::PrefixDecr(inner)
            | ExprKind::PostfixIncr(inner)
            | ExprKind::PostfixDecr(inner) => {
                self.expr(inner);
                if !matches!(inner.kind(), ExprKind::Ident(name) if self.locals.contains(name)) {
                    self.forbidden(
                        expr.span,
                        format!(
                            "comptime entity `{}` cannot perform observable mutation",
                            self.entity_name
                        ),
                    );
                }
            }
            _ => visit_expr_children(expr, &mut |child| self.expr(child)),
        }
    }

    fn read(&mut self, name: &str, span: Span) {
        if self.locals.contains(name) || self.enum_names.contains(name) {
            return;
        }
        if self.entity_indices.contains_key(name) {
            self.violations.push(Violation {
                code: UNSUPPORTED_VALUE,
                message: format!(
                    "function `{name}` cannot be used as a comptime value; call it directly"
                ),
                span,
            });
            return;
        }
        if let Some(binding) = self.top_bindings.get(name) {
            if !self.captures.iter().any(|capture| capture.name == name) {
                self.captures.push(ComptimeCapture {
                    name: name.to_string(),
                    use_span: span,
                    declaration_span: binding.declaration_span,
                });
            }
            if binding.mutable {
                self.violations.push(Violation {
                    code: NON_CONSTANT,
                    message: format!(
                        "comptime entity `{}` cannot read mutable binding `{name}`",
                        self.entity_name
                    ),
                    span,
                });
            }
            return;
        }
        self.violations.push(Violation {
            code: NON_CONSTANT,
            message: format!(
                "comptime entity `{}` cannot read runtime value `{name}`",
                self.entity_name
            ),
            span,
        });
    }

    fn forbidden(&mut self, span: Span, message: String) {
        self.violations.push(Violation {
            code: FORBIDDEN_EFFECT,
            message,
            span,
        });
    }
}

struct ConstantExpression<'a> {
    enum_names: &'a HashSet<String>,
    entity_indices: &'a BTreeMap<String, usize>,
    entity_eligible: &'a [bool],
    const_eligible: &'a BTreeMap<String, bool>,
    locals: &'a HashMap<String, LocalBinding>,
}

impl ConstantExpression<'_> {
    fn check(&self, expr: &Expr) -> bool {
        match expr.kind() {
            ExprKind::StrLit(_)
            | ExprKind::IntLit(_)
            | ExprKind::UIntLit(_)
            | ExprKind::FloatLit(_)
            | ExprKind::DecimalLit(_)
            | ExprKind::BoolLit(_) => true,
            ExprKind::Ident(name) => {
                self.locals
                    .get(name)
                    .is_some_and(|binding| binding.constant)
                    || self.const_eligible.get(name).copied().unwrap_or(false)
                    || self.enum_names.contains(name)
            }
            ExprKind::FnCall { callee, args, .. } => {
                (callee == "isNaN"
                    || self
                        .entity_indices
                        .get(callee)
                        .is_some_and(|index| self.entity_eligible[*index]))
                    && args.iter().all(|arg| self.check(arg))
            }
            ExprKind::ObjectLit(fields) => fields.iter().all(|(_, value)| self.check(value)),
            ExprKind::ArrayLit(values) => values.iter().all(|value| self.check(value)),
            ExprKind::ArrayRepeat { value, count } => self.check(value) && self.check(count),
            ExprKind::TemplateLit(parts) => parts.iter().all(|part| {
                matches!(part, TemplatePart::Text(_))
                    || matches!(part, TemplatePart::Expr(value, _) if self.check(value))
            }),
            ExprKind::BinOp { left, right, .. }
            | ExprKind::In {
                key: left,
                object: right,
            }
            | ExprKind::Range {
                start: left,
                end: right,
                ..
            } => self.check(left) && self.check(right),
            ExprKind::ChainedComparison { terms, .. } => terms.iter().all(|term| self.check(term)),
            ExprKind::Ternary {
                cond,
                then_expr,
                else_expr,
            } => self.check(cond) && self.check(then_expr) && self.check(else_expr),
            ExprKind::FieldAccess { object, .. }
            | ExprKind::OptionalFieldAccess { object, .. }
            | ExprKind::IndexAccess { object, .. } => {
                let index_ok = !matches!(expr.kind(), ExprKind::IndexAccess { index, .. } if !self.check(index));
                index_ok && self.check(object)
            }
            ExprKind::Not(inner)
            | ExprKind::Neg(inner)
            | ExprKind::BitNot(inner)
            | ExprKind::Unwrap(inner)
            | ExprKind::Typeof(inner)
            | ExprKind::Try(inner)
            | ExprKind::Spread(inner)
            | ExprKind::ConstAssert(inner)
            | ExprKind::Comptime(inner) => self.check(inner),
            ExprKind::TypeAssert { value, .. } => self.check(value),
            ExprKind::ObjectUpdate { base, fields } => {
                self.check(base) && fields.iter().all(|(_, value)| self.check(value))
            }
            _ => false,
        }
    }
}

struct ExpressionValidator<'a, 'b> {
    analyzer: &'a Analyzer<'b>,
    locals: &'a HashMap<String, LocalBinding>,
    entity_eligible: &'a [bool],
    const_eligible: &'a BTreeMap<String, bool>,
    wrapper_span: Span,
    enclosing: Option<&'a str>,
    diagnostics: &'a mut Vec<ComptimeDiagnostic>,
}

impl ExpressionValidator<'_, '_> {
    fn check(&mut self, expr: &Expr) {
        match expr.kind() {
            ExprKind::Ident(name) => {
                let constant = self
                    .locals
                    .get(name)
                    .is_some_and(|binding| binding.constant)
                    || self.const_eligible.get(name).copied().unwrap_or(false)
                    || self.analyzer.enum_names.contains(name);
                if !constant {
                    if let Some(cycle) = self.analyzer.const_cycle(name) {
                        self.push(
                            CYCLE,
                            format!("compile-time constant `{name}` participates in an initialization cycle"),
                            expr.span,
                            Some(format!("break the constant initialization cycle that reaches `{name}`")),
                        );
                        if let Some(diagnostic) = self.diagnostics.last_mut() {
                            diagnostic.remove_wrapper = false;
                            diagnostic.related.extend(cycle.iter().filter_map(|member| {
                                self.analyzer.top_bindings.get(member).map(|binding| {
                                    ComptimeRelatedInformation {
                                        span: binding.declaration_span,
                                        message: format!("constant cycle includes `{member}`"),
                                    }
                                })
                            }));
                        }
                        return;
                    }
                    self.push(NON_CONSTANT, format!("`{name}` is not a compile-time constant"), expr.span, Some(format!("use a constant value instead of `{name}`, or evaluate this expression at runtime")));
                }
            }
            ExprKind::FnCall { callee, args, .. } => {
                let eligible = callee == "isNaN"
                    || self
                        .analyzer
                        .entity_indices
                        .get(callee)
                        .is_some_and(|index| self.entity_eligible[*index]);
                if !eligible {
                    self.push(
                        INVALID_TARGET,
                        format!("`{callee}` is not an eligible comptime function"),
                        expr.span,
                        Some(format!(
                            "remove `comptime(...)` to call `{callee}` at runtime"
                        )),
                    );
                    if let Some(index) = self.analyzer.entity_indices.get(callee) {
                        self.add_target_related(*index);
                    }
                }
                for arg in args {
                    self.check(arg);
                }
            }
            ExprKind::MethodCall {
                object,
                method,
                method_span,
                args,
                ..
            } => {
                self.check(object);
                for arg in args {
                    self.check(arg);
                }
                self.push(FORBIDDEN_EFFECT, format!("method call `{method}` is not allowed in comptime evaluation"), *method_span, Some(format!("remove the method call `{method}` from the compile-time expression or evaluate the expression at runtime")));
            }
            ExprKind::OptionalMethodCall {
                object,
                method,
                args,
                ..
            } => {
                self.check(object);
                for arg in args {
                    self.check(arg);
                }
                self.push(FORBIDDEN_EFFECT, format!("method call `{method}` is not allowed in comptime evaluation"), expr.span, Some(format!("remove the method call `{method}` from the compile-time expression or evaluate the expression at runtime")));
            }
            ExprKind::CallValue { callee, args, .. } => {
                self.check(callee);
                for arg in args {
                    self.check(arg);
                }
                self.push(
                    INVALID_TARGET,
                    "indirect function calls are not eligible for comptime evaluation".to_string(),
                    expr.span,
                    Some(
                        "remove `comptime(...)` to perform the indirect call at runtime"
                            .to_string(),
                    ),
                );
            }
            ExprKind::Await(inner) => {
                self.check(inner);
                self.push(FORBIDDEN_EFFECT, "`await` is not allowed in comptime evaluation".to_string(), self.analyzer.keyword_span(expr.span, "await"), Some("remove `await` from the compile-time expression or evaluate the expression at runtime".to_string()));
            }
            ExprKind::Try(inner) => self.check(inner),
            ExprKind::Borrow { .. } => self.push(
                UNSUPPORTED_VALUE,
                "references are not materializable comptime values".to_string(),
                expr.span,
                Some("produce an owned materializable value instead of a reference".to_string()),
            ),
            ExprKind::RegexLit { .. }
            | ExprKind::NewSet { .. }
            | ExprKind::NewMap { .. }
            | ExprKind::ArrowFn { .. } => {
                visit_expr_children(expr, &mut |child| self.check(child));
                self.push(UNSUPPORTED_VALUE, "this value kind is not supported by comptime evaluation".to_string(), expr.span, Some("use a scalar, string, array, tuple, record, or enum value supported by comptime evaluation".to_string()));
            }
            ExprKind::PrefixIncr(_)
            | ExprKind::PrefixDecr(_)
            | ExprKind::PostfixIncr(_)
            | ExprKind::PostfixDecr(_) => {
                visit_expr_children(expr, &mut |child| self.check(child));
                self.push(FORBIDDEN_EFFECT, "mutation is not allowed in comptime expressions".to_string(), expr.span, Some("replace the mutation with an immutable expression or evaluate it at runtime".to_string()));
            }
            _ => visit_expr_children(expr, &mut |child| self.check(child)),
        }
    }

    fn push(&mut self, code: &'static str, message: String, span: Span, help: Option<String>) {
        let mut related = vec![ComptimeRelatedInformation {
            span: self.wrapper_span,
            message: "compile-time evaluation is required by this wrapper".to_string(),
        }];
        if let Some(name) = self.enclosing {
            related.push(ComptimeRelatedInformation {
                span: self.wrapper_span,
                message: format!("wrapper appears inside `{name}`"),
            });
        }
        self.diagnostics.push(ComptimeDiagnostic {
            code,
            message,
            span,
            related,
            help: help.into_iter().collect(),
            remove_wrapper: matches!(code, INVALID_TARGET | NON_CONSTANT),
        });
    }

    fn add_target_related(&mut self, index: usize) {
        let Some(diagnostic) = self.diagnostics.last_mut() else {
            return;
        };
        let target = &self.analyzer.entities[index];
        diagnostic.related.push(ComptimeRelatedInformation {
            span: target.declaration_span,
            message: format!("`{}` is declared here", target.name),
        });
        for violation in &target.violations {
            diagnostic.related.push(ComptimeRelatedInformation {
                span: violation.span,
                message: violation.message.clone(),
            });
        }
    }
}

fn strongly_connected_components(
    entities: &[EntityWork<'_>],
    indices: &BTreeMap<String, usize>,
) -> Vec<usize> {
    struct Tarjan<'a> {
        entities: &'a [EntityWork<'a>],
        indices: &'a BTreeMap<String, usize>,
        next: usize,
        stack: Vec<usize>,
        on_stack: Vec<bool>,
        index: Vec<Option<usize>>,
        low: Vec<usize>,
        components: Vec<usize>,
        component_count: usize,
    }
    impl Tarjan<'_> {
        fn visit(&mut self, node: usize) {
            let index = self.next;
            self.next += 1;
            self.index[node] = Some(index);
            self.low[node] = index;
            self.stack.push(node);
            self.on_stack[node] = true;
            for edge in &self.entities[node].calls {
                let Some(&next) = self.indices.get(&edge.callee) else {
                    continue;
                };
                if self.index[next].is_none() {
                    self.visit(next);
                    self.low[node] = self.low[node].min(self.low[next]);
                } else if self.on_stack[next] {
                    self.low[node] = self.low[node].min(self.index[next].unwrap_or(usize::MAX));
                }
            }
            if self.low[node] == self.index[node].unwrap_or(usize::MAX) {
                loop {
                    let member = self.stack.pop().expect("Tarjan stack must contain root");
                    self.on_stack[member] = false;
                    self.components[member] = self.component_count;
                    if member == node {
                        break;
                    }
                }
                self.component_count += 1;
            }
        }
    }
    let len = entities.len();
    let mut tarjan = Tarjan {
        entities,
        indices,
        next: 0,
        stack: Vec::new(),
        on_stack: vec![false; len],
        index: vec![None; len],
        low: vec![0; len],
        components: vec![0; len],
        component_count: 0,
    };
    for node in 0..len {
        if tarjan.index[node].is_none() {
            tarjan.visit(node);
        }
    }
    tarjan.components
}

fn violation_help(code: &str, entity: &str) -> Vec<String> {
    match code {
        INVALID_TARGET => vec![format!(
            "call only another eligible `comptime` entity from `{entity}`"
        )],
        NON_CONSTANT => vec![format!(
            "pass the value as a parameter to `{entity}` or replace it with an immutable compile-time constant"
        )],
        FORBIDDEN_EFFECT => vec![format!(
            "remove the effect from `{entity}` or remove its `comptime` qualifier"
        )],
        UNSUPPORTED_VALUE => vec![format!(
            "return only values supported by the comptime value model from `{entity}`"
        )],
        _ => Vec::new(),
    }
}

fn deduplicate_diagnostics(diagnostics: &mut Vec<ComptimeDiagnostic>) {
    diagnostics.sort_by_key(|diagnostic| {
        (
            diagnostic.span.start,
            diagnostic.span.end,
            diagnostic.code,
            diagnostic.message.clone(),
        )
    });
    diagnostics.dedup_by(|left, right| {
        left.code == right.code && left.span == right.span && left.message == right.message
    });
}

fn collect_local_names(statements: &[Stmt], names: &mut BTreeSet<String>) {
    for statement in statements {
        match statement.kind() {
            StmtKind::VarDecl { name, .. } => {
                names.insert(name.clone());
            }
            StmtKind::ArrayDestructure { bindings, .. }
            | StmtKind::ObjectDestructure { bindings, .. } => {
                names.extend(bindings.iter().map(|binding| binding.name.clone()));
            }
            StmtKind::TryCatch {
                try_body,
                catch_binding,
                catch_body,
                finally_body,
                ..
            } => {
                names.insert(catch_binding.clone());
                collect_local_names(try_body, names);
                collect_local_names(catch_body, names);
                if let Some(body) = finally_body {
                    collect_local_names(body, names);
                }
            }
            StmtKind::While { body, .. }
            | StmtKind::DoWhile { body, .. }
            | StmtKind::ForOf { body, .. }
            | StmtKind::ForIn { body, .. } => collect_local_names(body, names),
            StmtKind::For {
                init, update, body, ..
            } => {
                collect_local_names(std::slice::from_ref(init), names);
                collect_local_names(std::slice::from_ref(update), names);
                collect_local_names(body, names);
            }
            StmtKind::Switch { cases, default, .. } => {
                for case in cases {
                    collect_local_names(&case.body, names);
                }
                if let Some(body) = default {
                    collect_local_names(body, names);
                }
            }
            StmtKind::Match { arms, default, .. } => {
                for arm in arms {
                    collect_pattern_names(&arm.pattern, names);
                    collect_local_names(&arm.body, names);
                }
                if let Some(body) = default {
                    collect_local_names(body, names);
                }
            }
            _ => {}
        }
    }
}

fn collect_pattern_names(pattern: &MatchPattern, names: &mut BTreeSet<String>) {
    match pattern {
        MatchPattern::Binding(name) => {
            names.insert(name.clone());
        }
        MatchPattern::ResultVariant {
            binding: Some(name),
            ..
        } => {
            names.insert(name.clone());
        }
        MatchPattern::Object(fields) => {
            names.extend(fields.iter().filter_map(|field| field.binding.clone()));
        }
        MatchPattern::Array { elements, .. } => {
            for element in elements {
                collect_pattern_names(&element.pattern, names);
            }
        }
        MatchPattern::Or(patterns) => {
            for pattern in patterns {
                collect_pattern_names(pattern, names);
            }
        }
        MatchPattern::EnumVariant {
            binding: Some(binding),
            ..
        } => collect_pattern_names(binding, names),
        _ => {}
    }
}

fn collect_pattern_bindings(pattern: &MatchPattern, locals: &mut HashMap<String, LocalBinding>) {
    let mut names = BTreeSet::new();
    collect_pattern_names(pattern, &mut names);
    for name in names {
        locals.insert(name, LocalBinding { constant: false });
    }
}

fn scan_pattern(pattern: &MatchPattern, scanner: &mut EntityScanner<'_>) {
    match pattern {
        MatchPattern::Lit(expr) => scanner.expr(expr),
        MatchPattern::Object(fields) => {
            for field in fields {
                if let Some(literal) = &field.literal {
                    scanner.expr(literal);
                }
            }
        }
        MatchPattern::Array { elements, .. } => {
            for element in elements {
                scan_pattern(&element.pattern, scanner);
            }
        }
        MatchPattern::Or(patterns) => {
            for pattern in patterns {
                scan_pattern(pattern, scanner);
            }
        }
        MatchPattern::Range { start, end, .. } => {
            if let Some(start) = start {
                scanner.expr(start);
            }
            if let Some(end) = end {
                scanner.expr(end);
            }
        }
        MatchPattern::Wildcard | MatchPattern::Binding(_) | MatchPattern::ResultVariant { .. } => {}
        MatchPattern::Tuple(patterns) => {
            for pattern in patterns {
                scan_pattern(pattern, scanner);
            }
        }
        MatchPattern::EnumVariant { binding, .. } => {
            if let Some(binding) = binding {
                scan_pattern(binding, scanner);
            }
        }
    }
}

fn collect_identifiers(expr: &Expr, names: &mut BTreeSet<String>) {
    if let ExprKind::Ident(name) = expr.kind() {
        names.insert(name.clone());
    }
    visit_expr_children(expr, &mut |child| collect_identifiers(child, names));
}

fn visit_expr_children(expr: &Expr, visitor: &mut impl FnMut(&Expr)) {
    match expr.kind() {
        ExprKind::NewSet { initial, .. } | ExprKind::NewMap { initial, .. } => {
            if let Some(initial) = initial {
                visitor(initial);
            }
        }
        ExprKind::TemplateLit(parts) => {
            for part in parts {
                if let TemplatePart::Expr(value, _) = part {
                    visitor(value);
                }
            }
        }
        ExprKind::ObjectLit(fields) => {
            for (_, value) in fields {
                visitor(value);
            }
        }
        ExprKind::ObjectUpdate { base, fields } => {
            visitor(base);
            for (_, value) in fields {
                visitor(value);
            }
        }
        ExprKind::ArrayLit(values) | ExprKind::ChainedComparison { terms: values, .. } => {
            for value in values {
                visitor(value);
            }
        }
        ExprKind::ArrayRepeat { value, count } => {
            visitor(value);
            visitor(count);
        }
        ExprKind::FieldAccess { object, .. } | ExprKind::OptionalFieldAccess { object, .. } => {
            visitor(object);
        }
        ExprKind::IndexAccess { object, index }
        | ExprKind::BinOp {
            left: object,
            right: index,
            ..
        }
        | ExprKind::Pipeline {
            left: object,
            right: index,
        }
        | ExprKind::In {
            key: object,
            object: index,
        }
        | ExprKind::Range {
            start: object,
            end: index,
            ..
        } => {
            visitor(object);
            visitor(index);
        }
        ExprKind::Not(value)
        | ExprKind::Neg(value)
        | ExprKind::BitNot(value)
        | ExprKind::PrefixIncr(value)
        | ExprKind::PrefixDecr(value)
        | ExprKind::PostfixIncr(value)
        | ExprKind::PostfixDecr(value)
        | ExprKind::Unwrap(value)
        | ExprKind::Typeof(value)
        | ExprKind::Await(value)
        | ExprKind::Try(value)
        | ExprKind::Spread(value)
        | ExprKind::ConstAssert(value)
        | ExprKind::Comptime(value) => visitor(value),
        ExprKind::Ternary {
            cond,
            then_expr,
            else_expr,
        } => {
            visitor(cond);
            visitor(then_expr);
            visitor(else_expr);
        }
        ExprKind::TypeAssert { value, .. } => visitor(value),
        ExprKind::Is { expr, .. } => visitor(expr),
        ExprKind::OptionalMethodCall { object, args, .. }
        | ExprKind::MethodCall { object, args, .. } => {
            visitor(object);
            for arg in args {
                visitor(arg);
            }
        }
        ExprKind::FnCall { args, .. } => {
            for arg in args {
                visitor(arg);
            }
        }
        ExprKind::CallValue { callee, args, .. } => {
            visitor(callee);
            for arg in args {
                visitor(arg);
            }
        }
        ExprKind::If {
            cond,
            then_block,
            else_block,
        } => {
            visitor(cond);
            for statement in then_block {
                visit_stmt_exprs(statement, visitor);
            }
            if let Some(block) = else_block {
                for statement in block {
                    visit_stmt_exprs(statement, visitor);
                }
            }
        }
        ExprKind::Loop { body } => {
            for statement in body {
                visit_stmt_exprs(statement, visitor);
            }
        }
        ExprKind::ArrowFn { body, .. } => {
            for statement in body {
                visit_stmt_exprs(statement, visitor);
            }
        }
        ExprKind::StrLit(_)
        | ExprKind::RegexLit { .. }
        | ExprKind::IntLit(_)
        | ExprKind::UIntLit(_)
        | ExprKind::FloatLit(_)
        | ExprKind::DecimalLit(_)
        | ExprKind::BoolLit(_)
        | ExprKind::Null
        | ExprKind::None
        | ExprKind::Ident(_)
        | ExprKind::Topic
        | ExprKind::Borrow { .. } => {}
    }
}

fn visit_stmt_exprs(statement: &Stmt, visitor: &mut impl FnMut(&Expr)) {
    match statement.kind() {
        StmtKind::VarDecl { value, .. } => {
            if let Some(value) = value {
                visitor(value);
            }
        }
        StmtKind::ArrayDestructure { value, .. }
        | StmtKind::ObjectDestructure { value, .. }
        | StmtKind::Assign { value, .. }
        | StmtKind::Throw(value)
        | StmtKind::Expr(value) => visitor(value),
        StmtKind::IndexAssign { index, value, .. } => {
            visitor(index);
            visitor(value);
        }
        StmtKind::FieldAssign { object, value, .. } => {
            visitor(object);
            visitor(value);
        }
        StmtKind::Return(value) => {
            if let Some(value) = value {
                visitor(value);
            }
        }
        StmtKind::TryCatch {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            for statement in try_body.iter().chain(catch_body) {
                visit_stmt_exprs(statement, visitor);
            }
            if let Some(body) = finally_body {
                for statement in body {
                    visit_stmt_exprs(statement, visitor);
                }
            }
        }
        StmtKind::While { cond, body } | StmtKind::DoWhile { body, cond } => {
            visitor(cond);
            for statement in body {
                visit_stmt_exprs(statement, visitor);
            }
        }
        StmtKind::For {
            init,
            cond,
            update,
            body,
        } => {
            visit_stmt_exprs(init, visitor);
            visitor(cond);
            visit_stmt_exprs(update, visitor);
            for statement in body {
                visit_stmt_exprs(statement, visitor);
            }
        }
        StmtKind::ForOf { iterable, body, .. } => {
            visitor(iterable);
            for statement in body {
                visit_stmt_exprs(statement, visitor);
            }
        }
        StmtKind::ForIn { object, body, .. } => {
            visitor(object);
            for statement in body {
                visit_stmt_exprs(statement, visitor);
            }
        }
        StmtKind::Switch {
            expr,
            cases,
            default,
            ..
        } => {
            visitor(expr);
            for case in cases {
                visitor(&case.value);
                for statement in &case.body {
                    visit_stmt_exprs(statement, visitor);
                }
            }
            if let Some(body) = default {
                for statement in body {
                    visit_stmt_exprs(statement, visitor);
                }
            }
        }
        StmtKind::Match {
            expr,
            arms,
            default,
        } => {
            visitor(expr);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    visitor(guard);
                }
                for statement in &arm.body {
                    visit_stmt_exprs(statement, visitor);
                }
            }
            if let Some(body) = default {
                for statement in body {
                    visit_stmt_exprs(statement, visitor);
                }
            }
        }
        StmtKind::Break(value) => {
            if let Some(value) = value {
                visitor(value);
            }
        }
        StmtKind::Defer { body, .. } => {
            for statement in body {
                visit_stmt_exprs(statement, visitor);
            }
        }
        StmtKind::Continue => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_spanned;
    use crate::parser::Parser;

    fn facts(source: &str) -> ComptimeFacts {
        let tokens = lex_spanned(source).unwrap();
        let mut parser = Parser::new_with_ranges(
            tokens.iter().map(|(token, _)| token.clone()).collect(),
            tokens.into_iter().map(|(_, range)| range).collect(),
        );
        analyze(&parser.parse_program().unwrap(), source)
    }

    #[test]
    fn records_call_graph_captures_and_recursive_components() {
        let source = "const seed: i32 = 2; comptime function even(value: i32): bool { if (value === 0) { return true; } return odd(value - 1); } comptime function odd(value: i32): bool { if (value === 0) { return false; } return even(value - 1); } comptime const scaled = (value: i32): i32 => value * seed; const answer = comptime(scaled(21));";
        let facts = facts(source);
        assert!(facts.diagnostics.is_empty(), "{:#?}", facts.diagnostics);
        let even = facts
            .entities
            .iter()
            .find(|entity| entity.name == "even")
            .unwrap();
        let odd = facts
            .entities
            .iter()
            .find(|entity| entity.name == "odd")
            .unwrap();
        assert_eq!(
            even.strongly_connected_component,
            odd.strongly_connected_component
        );
        assert!(even.eligible && odd.eligible);
        let scaled = facts
            .entities
            .iter()
            .find(|entity| entity.name == "scaled")
            .unwrap();
        assert_eq!(scaled.kind, ComptimeEntityKind::ArrowBinding);
        assert_eq!(
            scaled
                .captures
                .iter()
                .map(|capture| capture.name.as_str())
                .collect::<Vec<_>>(),
            vec!["seed"]
        );
    }

    #[test]
    fn rejects_transitive_effects_dynamic_values_and_arrow_cycles() {
        let source = "function runtime(): i32 { return 1; } comptime function helper(): i32 { return runtime(); } comptime function entry(): i32 { return helper(); } comptime const infinite = (): i32 => infinite(); let dynamic: i32 = 1; const a = comptime(entry()); const b = comptime(dynamic);";
        let facts = facts(source);
        assert!(
            facts
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == INVALID_TARGET
                    && diagnostic.message.contains("runtime"))
        );
        assert!(
            facts
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == CYCLE)
        );
        assert!(
            facts
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == NON_CONSTANT
                    && diagnostic.message.contains("dynamic")
                    && diagnostic.remove_wrapper)
        );
        assert!(
            !facts
                .entities
                .iter()
                .find(|entity| entity.name == "entry")
                .unwrap()
                .eligible
        );
    }

    #[test]
    fn accepts_array_match_bindings_as_comptime_locals() {
        let source = "comptime function first(values: i32[]): i32 { match (values) { [head, ...tail] => { return head + tail[0]; } _ => { return 0; } } } const answer = comptime(first([1, 2]));";
        let facts = facts(source);
        assert!(facts.diagnostics.is_empty(), "{:#?}", facts.diagnostics);
        assert!(
            facts
                .entities
                .iter()
                .any(|entity| entity.name == "first" && entity.eligible)
        );
    }

    #[test]
    fn detects_constant_initialization_cycles_used_by_comptime() {
        let source = "const first: i32 = second; const second: i32 = first; const value: i32 = comptime(first);";
        let facts = facts(source);
        let diagnostic = facts
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == CYCLE)
            .unwrap_or_else(|| panic!("diagnostics: {:#?}", facts.diagnostics));
        assert!(diagnostic.message.contains("`first`"));
        assert!(
            diagnostic
                .related
                .iter()
                .any(|related| related.message.contains("`first`"))
        );
        assert!(
            diagnostic
                .related
                .iter()
                .any(|related| related.message.contains("`second`"))
        );
        assert!(!diagnostic.remove_wrapper);
    }
}
