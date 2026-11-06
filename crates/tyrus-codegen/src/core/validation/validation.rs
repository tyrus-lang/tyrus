use super::*;

pub(super) fn validate_codegen_item(item: &Item) -> Result<(), String> {
    match item {
        Item::InterfaceDecl { fields, .. } => {
            for field in fields {
                validate_codegen_type(&field.ty)?;
            }
        }
        Item::BehaviorInterfaceDecl {
            methods, targets, ..
        } => {
            for target in targets {
                validate_codegen_type(target)?;
            }
            for method in methods {
                for param in &method.params {
                    validate_codegen_type(&param.ty)?;
                }
                validate_codegen_type(&method.ret_type)?;
                validate_codegen_stmts(&method.body)?;
            }
        }
        Item::TypeAlias { ty, .. } => validate_codegen_type(ty)?,
        Item::EnumDecl { .. } => {}
        Item::FnDecl {
            name,
            is_async,
            params,
            ret_type,
            body,
            ..
        } => {
            if *is_async {
                let payload = promise_payload_type(ret_type);
                if !matches!(ret_type, Type::Infer) && payload.is_none() {
                    return Err(
                        "error: async codegen requires a `Promise<T>` return type".to_string()
                    );
                }
                if name == "main"
                    && (!params.is_empty()
                        || payload.is_some_and(|payload| !matches!(payload, Type::Void)))
                {
                    return Err(
                        "error: `async main` must have no parameters and return `Promise<void>`"
                            .to_string(),
                    );
                }
            }
            for param in params {
                validate_codegen_type(&param.ty)?;
                if let Some(default_value) = &param.default_value {
                    validate_codegen_expr(default_value)?;
                }
            }
            if !*is_async {
                validate_codegen_type(ret_type)?;
            }
            if *is_async {
                validate_async_codegen_stmts(body)?;
            } else {
                validate_codegen_stmts(body)?;
            }
        }
        Item::Stmt(stmt) => validate_codegen_stmt(stmt)?,
    }
    Ok(())
}

pub(super) fn validate_codegen_stmts(stmts: &[Stmt]) -> Result<(), String> {
    for stmt in stmts {
        validate_codegen_stmt(stmt)?;
    }
    Ok(())
}

pub(super) fn validate_codegen_stmt(stmt: &Stmt) -> Result<(), String> {
    match stmt.kind() {
        StmtKind::VarDecl { ty, value, .. } => {
            validate_codegen_type(ty)?;
            if let Some(value) = value {
                validate_codegen_expr(value)?;
            }
        }
        StmtKind::Assign { value, .. } => {
            if let ExprKind::Await(inner) = value.kind() {
                validate_codegen_expr(inner)?;
            } else {
                validate_codegen_expr(value)?;
            }
        }
        StmtKind::Throw(value) => validate_codegen_expr(value)?,
        StmtKind::Defer { body, .. } => validate_codegen_stmts(body)?,
        StmtKind::Return(Some(value)) => {
            if let ExprKind::Await(inner) = value.kind() {
                validate_codegen_expr(inner)?;
            } else {
                validate_codegen_expr(value)?;
            }
        }
        StmtKind::IndexAssign { index, value, .. } => {
            validate_codegen_expr(index)?;
            validate_codegen_expr(value)?;
        }
        StmtKind::FieldAssign { object, value, .. } => {
            validate_codegen_expr(object)?;
            validate_codegen_expr(value)?;
        }
        StmtKind::Return(None) | StmtKind::Break(None) | StmtKind::Continue => {}
        StmtKind::Break(Some(value)) => validate_codegen_expr(value)?,
        StmtKind::While { cond, body } => {
            validate_codegen_expr(cond)?;
            validate_codegen_stmts(body)?;
        }
        StmtKind::DoWhile { body, cond } => {
            validate_codegen_stmts(body)?;
            validate_codegen_expr(cond)?;
        }
        StmtKind::For {
            init,
            cond,
            update,
            body,
        } => {
            validate_codegen_stmt(init)?;
            validate_codegen_expr(cond)?;
            validate_codegen_stmt(update)?;
            validate_codegen_stmts(body)?;
        }
        StmtKind::ForOf { iterable, body, .. } => {
            validate_codegen_expr(iterable)?;
            validate_codegen_stmts(body)?;
        }
        StmtKind::ForIn { object, body, .. } => {
            validate_codegen_expr(object)?;
            validate_codegen_stmts(body)?;
        }
        StmtKind::ArrayDestructure {
            bindings, value, ..
        }
        | StmtKind::ObjectDestructure {
            bindings, value, ..
        } => {
            for binding in bindings {
                validate_codegen_type(&binding.ty)?;
            }
            validate_codegen_expr(value)?;
        }
        StmtKind::Expr(expr) => {
            if let ExprKind::Await(inner) = expr.kind() {
                validate_codegen_expr(inner)?;
            } else {
                validate_codegen_expr(expr)?;
            }
        }
        StmtKind::Switch {
            expr,
            cases,
            default,
            ..
        } => {
            validate_codegen_expr(expr)?;
            for case in cases {
                validate_codegen_expr(&case.value)?;
                validate_codegen_stmts(&case.body)?;
            }
            if let Some(default) = default {
                validate_codegen_stmts(default)?;
            }
        }
        StmtKind::Match {
            expr,
            arms,
            default,
        } => {
            validate_codegen_expr(expr)?;
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    validate_codegen_expr(guard)?;
                }
                validate_codegen_stmts(&arm.body)?;
            }
            if let Some(default) = default {
                validate_codegen_stmts(default)?;
            }
        }
        StmtKind::TryCatch {
            try_body,
            catch_type,
            catch_type_span: _,
            catch_body,
            finally_body,
            ..
        } => {
            validate_codegen_stmts(try_body)?;
            validate_codegen_type(catch_type)?;
            validate_codegen_stmts(catch_body)?;
            if let Some(finally_body) = finally_body {
                validate_codegen_stmts(finally_body)?;
            }
        }
    }
    Ok(())
}

pub(super) fn validate_codegen_expr(expr: &Expr) -> Result<(), String> {
    match expr.kind() {
        ExprKind::Await(_) => return Err("error: await codegen is not implemented yet".to_string()),
        ExprKind::Try(inner) => validate_codegen_expr(inner)?,
        ExprKind::ArrowFn {
            is_async,
            params,
            ret_type,
            body,
            ..
        } => {
            for param in params {
                validate_codegen_type(&param.ty)?;
                if let Some(default_value) = &param.default_value {
                    validate_codegen_expr(default_value)?;
                }
            }
            if *is_async {
                validate_async_codegen_stmts(body)?;
            } else {
                validate_codegen_type(ret_type)?;
                validate_codegen_stmts(body)?;
            }
        }
        ExprKind::ObjectLit(fields) => {
            for (_, value) in fields {
                validate_codegen_expr(value)?;
            }
        }
        ExprKind::ArrayLit(values) => {
            for value in values {
                validate_codegen_expr(value)?;
            }
        }
        ExprKind::ArrayRepeat { value, count } => {
            validate_codegen_expr(value)?;
            validate_codegen_expr(count)?;
        }
        ExprKind::ObjectUpdate { base, fields } => {
            validate_codegen_expr(base)?;
            for (_, value) in fields {
                validate_codegen_expr(value)?;
            }
        }
        ExprKind::FieldAccess { object, .. } | ExprKind::OptionalFieldAccess { object, .. } => {
            validate_codegen_expr(object)?;
        }
        ExprKind::IndexAccess { object, index } => {
            validate_codegen_expr(object)?;
            validate_codegen_expr(index)?;
        }
        ExprKind::Unwrap(inner)
        | ExprKind::Not(inner)
        | ExprKind::Neg(inner)
        | ExprKind::BitNot(inner)
        | ExprKind::Typeof(inner)
        | ExprKind::Spread(inner)
        | ExprKind::ConstAssert(inner)
        | ExprKind::Comptime(inner)
        | ExprKind::PrefixIncr(inner)
        | ExprKind::PrefixDecr(inner)
        | ExprKind::PostfixIncr(inner)
        | ExprKind::PostfixDecr(inner) => validate_codegen_expr(inner)?,
        ExprKind::TypeAssert { value, ty } => {
            validate_codegen_expr(value)?;
            validate_codegen_type(ty)?;
        }
        ExprKind::BinOp { left, right, .. } | ExprKind::Pipeline { left, right } => {
            validate_codegen_expr(left)?;
            validate_codegen_expr(right)?;
        }
        ExprKind::ChainedComparison { terms, .. } => {
            for term in terms {
                validate_codegen_expr(term)?;
            }
        }
        ExprKind::TemplateLit(parts) => {
            for part in parts {
                if let TemplatePart::Expr(expr, _) = part {
                    validate_codegen_expr(expr)?;
                }
            }
        }
        ExprKind::CallValue {
            callee,
            args,
            type_args,
        } => {
            validate_codegen_expr(callee)?;
            for ty in type_args {
                validate_codegen_type(ty)?;
            }
            for arg in args {
                validate_codegen_expr(arg)?;
            }
        }
        ExprKind::FnCall {
            args, type_args, ..
        } => {
            for ty in type_args {
                validate_codegen_type(ty)?;
            }
            for arg in args {
                validate_codegen_expr(arg)?;
            }
        }
        ExprKind::NewSet { ty, initial } => {
            if let Some(ty) = ty {
                validate_codegen_type(ty)?;
            }
            if let Some(initial) = initial {
                validate_codegen_expr(initial)?;
            }
        }
        ExprKind::NewMap {
            key,
            value,
            initial,
        } => {
            if let Some(key) = key {
                validate_codegen_type(key)?;
            }
            if let Some(value) = value {
                validate_codegen_type(value)?;
            }
            if let Some(initial) = initial {
                validate_codegen_expr(initial)?;
            }
        }
        ExprKind::MethodCall { object, args, .. }
        | ExprKind::OptionalMethodCall { object, args, .. } => {
            validate_codegen_expr(object)?;
            for arg in args {
                validate_codegen_expr(arg)?;
            }
        }
        ExprKind::Ternary {
            cond,
            then_expr,
            else_expr,
        } => {
            validate_codegen_expr(cond)?;
            validate_codegen_expr(then_expr)?;
            validate_codegen_expr(else_expr)?;
        }
        ExprKind::If {
            cond,
            then_block,
            else_block,
        } => {
            validate_codegen_expr(cond)?;
            validate_codegen_stmts(then_block)?;
            if let Some(else_block) = else_block {
                validate_codegen_stmts(else_block)?;
            }
        }
        ExprKind::Loop { body } => validate_codegen_stmts(body)?,
        ExprKind::In { key, object } => {
            validate_codegen_expr(key)?;
            validate_codegen_expr(object)?;
        }
        ExprKind::Range { start, end, .. } => {
            validate_codegen_expr(start)?;
            validate_codegen_expr(end)?;
        }
        ExprKind::Ident(_)
        | ExprKind::IntLit(_)
        | ExprKind::UIntLit(_)
        | ExprKind::FloatLit(_)
        | ExprKind::DecimalLit(_)
        | ExprKind::StrLit(_)
        | ExprKind::RegexLit { .. }
        | ExprKind::BoolLit(_)
        | ExprKind::Null
        | ExprKind::None
        | ExprKind::Borrow { .. }
        | ExprKind::Topic
        | ExprKind::Is { .. } => {}
    }
    Ok(())
}

pub(super) fn validate_codegen_type(ty: &Type) -> Result<(), String> {
    match ty {
        Type::Throws { function, effects } => {
            validate_codegen_type(function)?;
            for effect in effects {
                validate_codegen_type(effect)?;
            }
            Ok(())
        }
        Type::Apply { name, args }
            if matches!(
                name.as_str(),
                "Promise"
                    | "DbPromise"
                    | "JoinHandle"
                    | "Sender"
                    | "Receiver"
                    | "Mutex"
                    | "MutexGuard"
                    | "RwLock"
                    | "RwReadGuard"
                    | "RwWriteGuard"
            ) =>
        {
            if args.len() == 1 {
                validate_codegen_type(&args[0])
            } else {
                Err(format!(
                    "error: `{name}` received {} type argument(s), but exactly 1 is required",
                    args.len()
                ))
            }
        }
        Type::Array(inner)
        | Type::Optional(inner)
        | Type::Ref(inner)
        | Type::MutRef(inner)
        | Type::Readonly(inner)
        | Type::Range(inner)
        | Type::KeyOf(inner) => validate_codegen_type(inner),
        Type::Result { success, error } => {
            validate_codegen_type(success)?;
            validate_codegen_type(error)
        }
        Type::Tuple(elements) | Type::Union(elements) => {
            for element in elements {
                validate_codegen_type(element)?;
            }
            Ok(())
        }
        Type::Object(fields) => {
            for field in fields {
                validate_codegen_type(&field.ty)?;
            }
            Ok(())
        }
        Type::Function { params, ret } | Type::GenericFunction { params, ret, .. } => {
            for param in params {
                validate_codegen_type(param)?;
            }
            validate_codegen_type(ret)
        }
        Type::Apply { args, .. } => {
            for arg in args {
                validate_codegen_type(arg)?;
            }
            Ok(())
        }
        Type::Conditional {
            check,
            bound,
            then,
            otherwise,
        } => {
            validate_codegen_type(check)?;
            validate_codegen_type(bound)?;
            validate_codegen_type(then)?;
            validate_codegen_type(otherwise)
        }
        Type::Lit(expr) | Type::TypeQuery(expr) => validate_codegen_expr(expr),
        Type::Map(key, value) => {
            validate_codegen_type(key)?;
            validate_codegen_type(value)
        }
        Type::Set(value) => validate_codegen_type(value),
        Type::BinaryHeap(value) => validate_codegen_type(value),
        Type::BinomialHeap(value) => validate_codegen_type(value),
        Type::UnionFind(value) => validate_codegen_type(value),
        Type::Counter(value) => validate_codegen_type(value),
        Type::Deque(value) => validate_codegen_type(value),
        Type::LinkedList(value) => validate_codegen_type(value),
        Type::DoubleLinkedList(value) => validate_codegen_type(value),
        Type::AvlTree(value) => validate_codegen_type(value),
        Type::RedBlackTree(value) => validate_codegen_type(value),
        Type::SplayTree(value) => validate_codegen_type(value),
        Type::Treap(value) => validate_codegen_type(value),
        Type::SortedSet(value) => validate_codegen_type(value),
        Type::SkipList(value) => validate_codegen_type(value),
        Type::SortedMap(key, value) => {
            validate_codegen_type(key)?;
            validate_codegen_type(value)
        }
        Type::Graph(node, weight) => {
            validate_codegen_type(node)?;
            validate_codegen_type(weight)
        }
        Type::FenwickTree
        | Type::BubbleSort
        | Type::SegmentTree
        | Type::SparseTable
        | Type::SegmentTreeLazy
        | Type::BitSet
        | Type::Trie
        | Type::BloomFilter
        | Type::SuffixTree
        | Type::Lca
        | Type::Hld
        | Type::Centroid
        | Type::Infer
        | Type::Char
        | Type::I8
        | Type::I16
        | Type::I32
        | Type::I64
        | Type::I128
        | Type::Isize
        | Type::U8
        | Type::U16
        | Type::U32
        | Type::U64
        | Type::U128
        | Type::Usize
        | Type::F32
        | Type::F64
        | Type::F128
        | Type::Decimal
        | Type::Bool
        | Type::Str
        | Type::Regex
        | Type::Named(_)
        | Type::Generic(_)
        | Type::Unknown
        | Type::Never
        | Type::Void
        | Type::InferVar(_) => Ok(()),
    }
}
