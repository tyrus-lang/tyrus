use super::*;
use crate::ast::ExprKind;
use crate::ast::StmtKind;
use crate::module_loader::render_parse_error;
use crate::parser::ParseError;
use crate::type_checker::{ImportedProgramSymbols, TypeChecker};
use std::path::Path;

#[path = "module_loader/crypto.rs"]
mod crypto;
#[path = "module_loader/diagnostics.rs"]
mod diagnostics;
#[path = "module_loader/http.rs"]
mod http;
#[path = "module_loader/import_renames.rs"]
mod import_renames;
#[path = "module_loader/math.rs"]
mod math;
#[path = "module_loader/module_exports.rs"]
mod module_exports;
#[path = "module_loader/module_resolution.rs"]
mod module_resolution;
#[path = "module_loader/protected_exports.rs"]
mod protected_exports;

include!("module_loader_db.rs");
