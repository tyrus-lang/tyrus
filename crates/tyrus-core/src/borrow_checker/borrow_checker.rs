use crate::ast::*;
use crate::collections::{HashMap, HashSet};
use crate::span::Span;

#[path = "cfg/cfg.rs"]
mod cfg;
#[path = "expressions/expressions.rs"]
mod expressions;
#[path = "field_moves/field_moves.rs"]
mod field_moves;
#[path = "liveness/liveness.rs"]
mod liveness;
#[path = "narrowing/narrowing.rs"]
mod narrowing;
#[path = "references/references.rs"]
mod references;
#[path = "state/state.rs"]
mod state;
#[path = "statements/statements.rs"]
mod statements;

use cfg::*;
pub(crate) use liveness::closure_capture_names;
use liveness::*;
use state::*;

#[cfg(test)]
#[path = "../../tests/unit/borrow_checker.rs"]
mod tests;

pub struct BorrowChecker {
    pub(crate) scopes: Vec<Scope>,
    pub(crate) finalizer_scopes: Vec<Vec<ScheduledFinalizer>>,
    pub(crate) errors: Vec<String>,
    pub(crate) warnings: Vec<String>,
    pub(crate) external_names: HashSet<String>,
    pub(crate) data_interface_names: HashSet<String>,
    pub(crate) enum_namespaces: HashSet<String>,
    pub(crate) external_prefixes: Vec<String>,
    pub(crate) consuming_imports: HashSet<String>,
    pub(crate) replacing_imports: HashSet<String>,
    pub(crate) async_depth: usize,
    pub(crate) in_non_escaping_callback: bool,
    pub(crate) interface_fields: HashMap<String, Vec<Field>>,
    pub(crate) current_liveness: Option<HashMap<crate::span::Span, HashSet<String>>>,
}
impl Default for BorrowChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl BorrowChecker {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::default()],
            finalizer_scopes: vec![Vec::new()],
            errors: vec![],
            warnings: vec![],
            external_names: HashSet::default(),
            data_interface_names: HashSet::default(),
            enum_namespaces: HashSet::default(),
            external_prefixes: Vec::new(),
            consuming_imports: HashSet::default(),
            replacing_imports: HashSet::default(),
            async_depth: 0,
            in_non_escaping_callback: false,
            interface_fields: HashMap::default(),
            current_liveness: None,
        }
    }

    pub fn set_interface_fields(&mut self, fields: HashMap<String, Vec<Field>>) {
        self.interface_fields.extend(fields);
    }

    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    pub fn check_program(&mut self, program: &Program) -> Vec<String> {
        for item in &program.items {
            if let Item::InterfaceDecl { name, fields, .. } = item {
                self.interface_fields.insert(name.clone(), fields.clone());
            }
        }
        self.data_interface_names
            .extend(program.items.iter().filter_map(|item| {
                let Item::InterfaceDecl { name, .. } = item else {
                    return None;
                };
                Some(name.clone())
            }));
        self.consuming_imports.extend(
            program
                .imports
                .iter()
                .filter(|import| import.source == "tyrus:memory")
                .flat_map(|import| import.specifiers.iter())
                .filter(|specifier| matches!(specifier.imported.as_str(), "drop" | "zeroize"))
                .map(|specifier| specifier.local.clone()),
        );
        self.replacing_imports.extend(
            program
                .imports
                .iter()
                .filter(|import| import.source == "tyrus:memory")
                .flat_map(|import| import.specifiers.iter())
                .filter(|specifier| specifier.imported == "replace")
                .map(|specifier| specifier.local.clone()),
        );
        self.external_names.extend(
            program
                .imports
                .iter()
                .flat_map(|import| import.specifiers.iter())
                .map(|specifier| specifier.local.clone()),
        );
        self.external_prefixes.extend(
            program
                .imports
                .iter()
                .flat_map(|import| {
                    import
                        .specifiers
                        .iter()
                        .map(move |specifier| (&import.source, specifier))
                })
                .filter(|(source, specifier)| {
                    crate::module_loader::is_namespace_specifier(source, specifier)
                })
                .map(|(_, specifier)| {
                    crate::module_loader::namespace_member_binding(&specifier.local, "")
                }),
        );
        for item in &program.items {
            if let Item::Stmt(Stmt {
                kind:
                    StmtKind::VarDecl {
                        mutable, name, ty, ..
                    },
                ..
            }) = item
            {
                self.declare(name, *mutable, ty.clone());
            } else if let Item::FnDecl {
                name,
                params,
                ret_type,
                ..
            } = item
            {
                self.declare(
                    name,
                    false,
                    Type::Function {
                        params: params.iter().map(|param| param.ty.clone()).collect(),
                        ret: Box::new(ret_type.clone()),
                    },
                );
            } else if let Item::EnumDecl { name, .. } = item {
                self.declare(name, false, Type::Named(name.clone()));
                self.enum_namespaces.insert(name.clone());
            }
        }
        for item in &program.items {
            self.check_item(item);
        }
        self.errors.clone()
    }

    fn check_item(&mut self, item: &Item) {
        match item {
            Item::InterfaceDecl { .. } => {}
            Item::BehaviorInterfaceDecl { .. } => {}
            Item::TypeAlias { .. } => {}
            Item::EnumDecl { .. } => {}
            Item::FnDecl {
                name: _,
                is_async,
                params,
                ret_type: _,
                body,
                ..
            } => {
                let cfg = Cfg::from_stmts(body);
                let liveness = cfg.compute_liveness();
                let prev_liveness = self.current_liveness.replace(liveness);
                let previous_async_depth = self.async_depth;
                self.async_depth = if *is_async { 1 } else { 0 };
                self.push_scope();
                for p in params {
                    match &p.pattern {
                        ParamPattern::Name(name) => {
                            self.declare_parameter(name, p.mutable, p.ty.clone());
                        }
                        ParamPattern::Array(names) => {
                            let element_ty = if let Type::Array(element) = &p.ty {
                                element.as_ref().clone()
                            } else {
                                p.ty.clone()
                            };
                            for name in names {
                                self.declare_parameter(name, p.mutable, element_ty.clone());
                            }
                        }
                        ParamPattern::Object(names) => {
                            let fields = if let Type::Object(fields) = &p.ty {
                                fields.clone()
                            } else {
                                vec![]
                            };
                            for name in names {
                                let field_ty = fields
                                    .iter()
                                    .find(|f| f.name == *name)
                                    .map(field_borrow_type)
                                    .unwrap_or_else(|| p.ty.clone());
                                self.declare_parameter(name, p.mutable, field_ty);
                            }
                        }
                    }
                }
                self.check_stmt_sequence(body);
                self.pop_scope();
                self.async_depth = previous_async_depth;
                self.current_liveness = prev_liveness;
            }
            Item::Stmt(stmt) => self.check_stmt(stmt),
        }
    }
}
