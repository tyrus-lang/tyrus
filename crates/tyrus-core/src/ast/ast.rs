use crate::span::Span;

#[derive(Debug, Clone)]
pub struct FloatLiteral {
    value: f64,
    text: Box<str>,
}

impl PartialEq for FloatLiteral {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl FloatLiteral {
    pub fn new(value: f64, text: impl Into<Box<str>>) -> Self {
        Self {
            value,
            text: text.into(),
        }
    }

    pub fn from_value(value: f64) -> Self {
        Self::new(value, format!("{value}"))
    }

    pub fn value(&self) -> f64 {
        self.value
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecimalLiteral {
    text: Box<str>,
}

impl DecimalLiteral {
    pub fn new(text: impl Into<Box<str>>) -> Self {
        Self { text: text.into() }
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Infer,
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
    F32,
    F64,
    F128,
    Decimal,
    Bool,
    Char,
    Regex,
    Str,
    Void,
    Named(String),
    Generic(String),
    Array(Box<Type>),
    Tuple(Vec<Type>),
    Readonly(Box<Type>),
    Ref(Box<Type>),
    MutRef(Box<Type>),
    Optional(Box<Type>),
    Result {
        success: Box<Type>,
        error: Box<Type>,
    },
    Range(Box<Type>),
    Function {
        params: Vec<Type>,
        ret: Box<Type>,
    },
    GenericFunction {
        type_params: Vec<TypeParam>,
        params: Vec<Type>,
        ret: Box<Type>,
    },
    Throws {
        function: Box<Type>,
        effects: Vec<Type>,
    },
    Object(Vec<Field>),
    Union(Vec<Type>),
    Unknown,
    Never,
    Lit(Box<Expr>),
    TypeQuery(Box<Expr>),
    Apply {
        name: String,
        args: Vec<Type>,
    },
    KeyOf(Box<Type>),
    Conditional {
        check: Box<Type>,
        bound: Box<Type>,
        then: Box<Type>,
        otherwise: Box<Type>,
    },
    InferVar(String),
    Map(Box<Type>, Box<Type>),
    Set(Box<Type>),
    BinaryHeap(Box<Type>),
    BinomialHeap(Box<Type>),
    UnionFind(Box<Type>),
    Counter(Box<Type>),
    Deque(Box<Type>),
    FenwickTree,
    BubbleSort,
    SegmentTree,
    SparseTable,
    SegmentTreeLazy,
    BitSet,
    LinkedList(Box<Type>),
    DoubleLinkedList(Box<Type>),
    AvlTree(Box<Type>),
    RedBlackTree(Box<Type>),
    SplayTree(Box<Type>),
    Treap(Box<Type>),
    SortedSet(Box<Type>),
    SkipList(Box<Type>),
    SortedMap(Box<Type>, Box<Type>),
    Graph(Box<Type>, Box<Type>),
    Trie,
    BloomFilter,
    SuffixTree,
    Lca,
    Hld,
    Centroid,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeParam {
    pub name: String,
    pub constraint: Option<Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatorDecorator {
    pub name: String,
    pub display_name: String,
    pub args: Vec<Expr>,
    pub argument_ranges: Vec<Range<usize>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DbModelMetadata {
    pub table: String,
    pub span: Span,
    pub timestamps: bool,
    pub snake_case: bool,
    pub fields: Vec<DbModelFieldMetadata>,
    pub relations: Vec<DbModelRelationMetadata>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbModelRelationKind {
    BelongsTo,
    HasOne,
    HasMany,
    ManyToMany,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DbModelRelationMetadata {
    pub field: String,
    pub target_model: String,
    pub foreign_key_field: String,
    pub pivot_table: Option<String>,
    pub pivot_source_column: Option<String>,
    pub pivot_target_column: Option<String>,
    pub kind: DbModelRelationKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DbModelFieldMetadata {
    pub field: String,
    pub column: String,
    pub ty: Type,
    pub id: bool,
    pub id_strategy: Option<String>,
    pub unique: bool,
    pub default_value: Option<Expr>,
    pub foreign_key: Option<(String, String)>,
    pub span: Span,
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Infer => write!(f, "<inferred>"),
            Type::I8 => write!(f, "i8"),
            Type::I16 => write!(f, "i16"),
            Type::I32 => write!(f, "i32"),
            Type::I64 => write!(f, "i64"),
            Type::I128 => write!(f, "i128"),
            Type::Isize => write!(f, "isize"),
            Type::U8 => write!(f, "u8"),
            Type::U16 => write!(f, "u16"),
            Type::U32 => write!(f, "u32"),
            Type::U64 => write!(f, "u64"),
            Type::U128 => write!(f, "u128"),
            Type::Usize => write!(f, "usize"),
            Type::F32 => write!(f, "f32"),
            Type::F64 => write!(f, "f64"),
            Type::F128 => write!(f, "f128"),
            Type::Decimal => write!(f, "decimal"),
            Type::Bool => write!(f, "boolean"),
            Type::Char => write!(f, "char"),
            Type::Regex => write!(f, "Regex"),
            Type::Str => write!(f, "string"),
            Type::Void => write!(f, "void"),
            Type::Named(name) => write!(f, "{name}"),
            Type::Generic(name) => write!(f, "{name}"),
            Type::Array(element) => write!(f, "{element}[]"),
            Type::Tuple(elements) => {
                let elements = elements
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "[{elements}]")
            }
            Type::Readonly(inner) => match inner.as_ref() {
                Type::Array(element) => write!(f, "readonly {element}[]"),
                other => write!(f, "readonly {other}"),
            },
            Type::Ref(t) => write!(f, "&{t}"),
            Type::MutRef(t) => write!(f, "&mut {t}"),
            Type::Optional(t) => write!(f, "{t}?"),
            Type::Result { success, error } => write!(f, "Result<{success}, {error}>"),
            Type::Function { params, ret } => {
                let params = params
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "({params}) => {ret}")
            }
            Type::Throws { function, effects } => {
                let effects = effects
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" | ");
                write!(f, "{function} throws {effects}")
            }
            Type::GenericFunction {
                type_params,
                params,
                ret,
            } => {
                let type_params = type_params
                    .iter()
                    .map(|param| param.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                let params = params
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "<{type_params}>({params}) => {ret}")
            }
            Type::Object(fields) => {
                let fields = fields
                    .iter()
                    .map(|f| {
                        if f.optional {
                            format!("{}?: {}", f.name, f.ty)
                        } else {
                            format!("{}: {}", f.name, f.ty)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                write!(f, "{{ {fields} }}")
            }
            Type::Union(variants) => {
                let variants = variants
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" | ");
                write!(f, "{variants}")
            }
            Type::Unknown => write!(f, "unknown"),
            Type::Never => write!(f, "never"),
            Type::Lit(expr) => write!(f, "{}", lit_display(expr)),
            Type::TypeQuery(expr) => write!(f, "typeof {}", expr_display(expr)),
            Type::Apply { name, args } => {
                let args = args
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                let name = if name == "DbPromise" { "Promise" } else { name };
                write!(f, "{name}<{args}>")
            }
            Type::KeyOf(inner) => write!(f, "keyof {inner}"),
            Type::Conditional {
                check,
                bound,
                then,
                otherwise,
            } => write!(f, "{check} extends {bound} ? {then} : {otherwise}"),
            Type::InferVar(name) => write!(f, "infer {name}"),
            Type::Map(key, value) => write!(f, "Record<{key}, {value}>"),
            Type::Set(value) => write!(f, "Set<{value}>"),
            Type::BinaryHeap(value) => write!(f, "BinaryHeap<{value}>"),
            Type::BinomialHeap(value) => write!(f, "BinomialHeap<{value}>"),
            Type::UnionFind(value) => write!(f, "UnionFind<{value}>"),
            Type::Counter(value) => write!(f, "Counter<{value}>"),
            Type::Deque(value) => write!(f, "Deque<{value}>"),
            Type::FenwickTree => write!(f, "FenwickTree"),
            Type::BubbleSort => write!(f, "BubbleSort"),
            Type::SegmentTree => write!(f, "SegmentTree"),
            Type::SparseTable => write!(f, "SparseTable"),
            Type::SegmentTreeLazy => write!(f, "SegmentTreeLazy"),
            Type::BitSet => write!(f, "BitSet"),
            Type::LinkedList(value) => write!(f, "LinkedList<{value}>"),
            Type::DoubleLinkedList(value) => write!(f, "DoubleLinkedList<{value}>"),
            Type::AvlTree(value) => write!(f, "AvlTree<{value}>"),
            Type::RedBlackTree(value) => write!(f, "RedBlackTree<{value}>"),
            Type::SplayTree(value) => write!(f, "SplayTree<{value}>"),
            Type::Treap(value) => write!(f, "Treap<{value}>"),
            Type::SortedSet(value) => write!(f, "SortedSet<{value}>"),
            Type::SkipList(value) => write!(f, "SkipList<{value}>"),
            Type::SortedMap(key, value) => write!(f, "SortedMap<{key}, {value}>"),
            Type::Graph(node, weight) => write!(f, "Graph<{node}, {weight}>"),
            Type::Trie => write!(f, "Trie"),
            Type::BloomFilter => write!(f, "BloomFilter"),
            Type::SuffixTree => write!(f, "SuffixTree"),
            Type::Lca => write!(f, "Lca"),
            Type::Hld => write!(f, "Hld"),
            Type::Centroid => write!(f, "Centroid"),
            Type::Range(element) => write!(f, "Range<{element}>"),
        }
    }
}

pub fn expr_display(expr: &Expr) -> String {
    match expr.kind() {
        ExprKind::Typeof(inner) => format!("typeof {}", expr_display(inner)),
        ExprKind::StrLit(s) => format!("\"{s}\""),
        ExprKind::RegexLit { pattern, flags } => {
            format!("/{}/{}", pattern.replace('/', "\\/"), flags)
        }
        ExprKind::IntLit(n) => n.to_string(),
        ExprKind::UIntLit(n) => n.to_string(),
        ExprKind::FloatLit(n) => n.text().to_string(),
        ExprKind::DecimalLit(n) => format!("{}d", n.text()),
        ExprKind::BoolLit(true) => "true".to_string(),
        ExprKind::BoolLit(false) => "false".to_string(),
        ExprKind::Null => "null".to_string(),
        ExprKind::None => "None".to_string(),
        ExprKind::Ident(name) => name.clone(),
        ExprKind::Topic => "%".to_string(),
        ExprKind::Pipeline { left, right } => {
            format!("{} |> {}", expr_display(left), expr_display(right))
        }
        _ => "<expr>".to_string(),
    }
}

pub fn contains_outer_topic(expr: &Expr) -> bool {
    match expr.kind() {
        ExprKind::Topic => true,
        ExprKind::Pipeline { .. } => false,
        ExprKind::NewSet { initial, .. } | ExprKind::NewMap { initial, .. } => {
            initial.as_deref().is_some_and(contains_outer_topic)
        }
        ExprKind::TemplateLit(parts) => parts.iter().any(|part| match part {
            TemplatePart::Text(_) => false,
            TemplatePart::Expr(expr, _) => contains_outer_topic(expr),
        }),
        ExprKind::ObjectLit(fields) => fields.iter().any(|(_, value)| contains_outer_topic(value)),
        ExprKind::ObjectUpdate { base, fields } => {
            contains_outer_topic(base)
                || fields.iter().any(|(_, value)| contains_outer_topic(value))
        }
        ExprKind::ArrayLit(values) => values.iter().any(contains_outer_topic),
        ExprKind::ArrayRepeat { value, count } => {
            contains_outer_topic(value) || contains_outer_topic(count)
        }
        ExprKind::FieldAccess { object, .. } | ExprKind::OptionalFieldAccess { object, .. } => {
            contains_outer_topic(object)
        }
        ExprKind::IndexAccess { object, index } => {
            contains_outer_topic(object) || contains_outer_topic(index)
        }
        ExprKind::BinOp { left, right, .. }
        | ExprKind::In {
            key: left,
            object: right,
        } => contains_outer_topic(left) || contains_outer_topic(right),
        ExprKind::Is { expr, .. } => contains_outer_topic(expr),
        ExprKind::Range { start, end, .. } => {
            contains_outer_topic(start) || contains_outer_topic(end)
        }
        ExprKind::ChainedComparison { terms, .. } => terms.iter().any(contains_outer_topic),
        ExprKind::Not(inner)
        | ExprKind::Neg(inner)
        | ExprKind::BitNot(inner)
        | ExprKind::PrefixIncr(inner)
        | ExprKind::PrefixDecr(inner)
        | ExprKind::PostfixIncr(inner)
        | ExprKind::PostfixDecr(inner)
        | ExprKind::Unwrap(inner)
        | ExprKind::Typeof(inner)
        | ExprKind::Await(inner)
        | ExprKind::Try(inner)
        | ExprKind::Spread(inner)
        | ExprKind::TypeAssert { value: inner, .. }
        | ExprKind::ConstAssert(inner)
        | ExprKind::Comptime(inner) => contains_outer_topic(inner),
        ExprKind::Ternary {
            cond,
            then_expr,
            else_expr,
        } => {
            contains_outer_topic(cond)
                || contains_outer_topic(then_expr)
                || contains_outer_topic(else_expr)
        }
        ExprKind::OptionalMethodCall { object, args, .. }
        | ExprKind::MethodCall { object, args, .. } => {
            contains_outer_topic(object) || args.iter().any(contains_outer_topic)
        }
        ExprKind::FnCall { args, .. } => args.iter().any(contains_outer_topic),
        ExprKind::CallValue { callee, args, .. } => {
            contains_outer_topic(callee) || args.iter().any(contains_outer_topic)
        }
        ExprKind::ArrowFn { .. } => false,
        ExprKind::If {
            cond,
            then_block,
            else_block,
        } => {
            contains_outer_topic(cond)
                || then_block.iter().any(stmt_contains_outer_topic)
                || else_block
                    .as_ref()
                    .is_some_and(|block| block.iter().any(stmt_contains_outer_topic))
        }
        ExprKind::Loop { body } => body.iter().any(stmt_contains_outer_topic),
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
        | ExprKind::Borrow { .. } => false,
    }
}

pub fn place_path(expr: &Expr) -> Option<String> {
    match expr.kind() {
        ExprKind::Ident(name) => Some(name.clone()),
        ExprKind::FieldAccess { object, field } => Some(format!("{}.{field}", place_path(object)?)),
        _ => None,
    }
}

/// A value read from storage something else owns: a name, a field or an
/// element of one, or the success of `?` applied to one. An async frame that
/// keeps such a value stores a clone of it, never the value itself. Shared by
/// the type checker and code generation so the two accept the same locals.
pub fn is_borrowed_place(expr: &Expr) -> bool {
    match expr.kind() {
        ExprKind::Ident(_) => true,
        ExprKind::FieldAccess { object, .. } | ExprKind::IndexAccess { object, .. } => {
            is_borrowed_place(object)
        }
        ExprKind::Try(inner) => is_borrowed_place(inner),
        _ => false,
    }
}

pub fn place_root_ident(expr: &Expr) -> Option<&str> {
    match expr.kind() {
        ExprKind::Ident(name) => Some(name),
        ExprKind::FieldAccess { object, .. } | ExprKind::IndexAccess { object, .. } => {
            place_root_ident(object)
        }
        _ => None,
    }
}

fn stmt_contains_outer_topic(stmt: &Stmt) -> bool {
    match stmt.kind() {
        StmtKind::VarDecl { value, .. } => value.as_ref().is_some_and(contains_outer_topic),
        StmtKind::ArrayDestructure { value, .. }
        | StmtKind::ObjectDestructure { value, .. }
        | StmtKind::Assign { value, .. }
        | StmtKind::Throw(value)
        | StmtKind::Expr(value) => contains_outer_topic(value),
        StmtKind::IndexAssign { index, value, .. } => {
            contains_outer_topic(index) || contains_outer_topic(value)
        }
        StmtKind::FieldAssign { object, value, .. } => {
            contains_outer_topic(object) || contains_outer_topic(value)
        }
        StmtKind::Return(value) => value.as_ref().is_some_and(contains_outer_topic),
        StmtKind::Break(value) => value.as_ref().is_some_and(contains_outer_topic),
        StmtKind::While { cond, body } | StmtKind::DoWhile { body, cond } => {
            contains_outer_topic(cond) || body.iter().any(stmt_contains_outer_topic)
        }
        StmtKind::For {
            init,
            cond,
            update,
            body,
        } => {
            stmt_contains_outer_topic(init)
                || contains_outer_topic(cond)
                || stmt_contains_outer_topic(update)
                || body.iter().any(stmt_contains_outer_topic)
        }
        StmtKind::ForOf { iterable, body, .. } => {
            contains_outer_topic(iterable) || body.iter().any(stmt_contains_outer_topic)
        }
        StmtKind::ForIn { object, body, .. } => {
            contains_outer_topic(object) || body.iter().any(stmt_contains_outer_topic)
        }
        StmtKind::Switch { expr, cases, .. } => {
            contains_outer_topic(expr)
                || cases.iter().any(|case| {
                    contains_outer_topic(&case.value)
                        || case.body.iter().any(stmt_contains_outer_topic)
                })
        }
        StmtKind::Match { expr, arms, .. } => {
            contains_outer_topic(expr)
                || arms.iter().any(|arm| {
                    arm.guard.as_ref().is_some_and(contains_outer_topic)
                        || arm.body.iter().any(stmt_contains_outer_topic)
                })
        }
        StmtKind::TryCatch {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            try_body.iter().any(stmt_contains_outer_topic)
                || catch_body.iter().any(stmt_contains_outer_topic)
                || finally_body
                    .as_ref()
                    .is_some_and(|block| block.iter().any(stmt_contains_outer_topic))
        }
        StmtKind::Defer { body, .. } => body.iter().any(stmt_contains_outer_topic),
        StmtKind::Continue => false,
    }
}

pub fn desugar_pipeline(left: &Expr, right: &Expr) -> Option<Expr> {
    let inject_first = |args: &[Expr]| {
        let mut injected = Vec::with_capacity(args.len() + 1);
        injected.push(left.clone());
        injected.extend(args.iter().cloned());
        injected
    };
    match right {
        Expr {
            kind: ExprKind::Ident(name),
            ..
        } => Some(Expr::synthetic(ExprKind::FnCall {
            callee: name.clone(),
            type_args: Vec::new(),
            args: vec![left.clone()],
        })),
        Expr {
            kind: ExprKind::FieldAccess { object, field },
            ..
        } => Some(Expr::synthetic(ExprKind::MethodCall {
            object: object.clone(),
            method: field.clone(),
            method_span: Span::DUMMY,
            type_args: Vec::new(),
            args: vec![left.clone()],
        })),
        Expr {
            kind:
                ExprKind::FnCall {
                    callee,
                    type_args,
                    args,
                },
            ..
        } => Some(Expr::synthetic(ExprKind::FnCall {
            callee: callee.clone(),
            type_args: type_args.clone(),
            args: inject_first(args),
        })),
        Expr {
            kind:
                ExprKind::MethodCall {
                    object,
                    method,
                    type_args,
                    args,
                    ..
                },
            ..
        } => Some(Expr::synthetic(ExprKind::MethodCall {
            object: object.clone(),
            method: method.clone(),
            method_span: Span::DUMMY,
            type_args: type_args.clone(),
            args: inject_first(args),
        })),
        Expr {
            kind:
                ExprKind::OptionalMethodCall {
                    object,
                    method,
                    type_args,
                    args,
                },
            ..
        } => Some(Expr::synthetic(ExprKind::OptionalMethodCall {
            object: object.clone(),
            method: method.clone(),
            type_args: type_args.clone(),
            args: inject_first(args),
        })),
        _ => None,
    }
}

pub fn lit_display(expr: &Expr) -> String {
    match expr.kind() {
        ExprKind::StrLit(s) => format!("\"{s}\""),
        ExprKind::IntLit(n) => n.to_string(),
        ExprKind::UIntLit(n) => n.to_string(),
        ExprKind::FloatLit(n) => float_literal_source(n),
        ExprKind::DecimalLit(n) => format!("{}d", n.text()),
        ExprKind::BoolLit(true) => "true".to_string(),
        ExprKind::BoolLit(false) => "false".to_string(),
        ExprKind::Null => "null".to_string(),
        ExprKind::None => "None".to_string(),
        _ => "<literal>".to_string(),
    }
}

impl Type {
    pub fn is_copy(&self) -> bool {
        match self {
            Type::I8
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
            | Type::Char
            | Type::Regex
            | Type::Function { .. }
            | Type::Never => true,
            Type::Union(variants) if variants.iter().all(|v| matches!(v, Type::Lit(_))) => true,
            Type::Tuple(elements) | Type::Union(elements) => elements.iter().all(Type::is_copy),
            Type::Result { success, error } => success.is_copy() && error.is_copy(),
            Type::Apply { name, args } if name == "Result" && args.len() == 2 => {
                args.iter().all(Type::is_copy)
            }
            _ => false,
        }
    }
}

pub fn float_literal_source(literal: &FloatLiteral) -> String {
    let text = literal.text();
    if text.contains(['.', 'e', 'E']) {
        text.to_string()
    } else {
        format!("{text}.0")
    }
}

pub type BindingName = String;

#[derive(Debug, Clone, PartialEq)]
pub enum ParamPattern {
    Name(String),
    Array(Vec<BindingName>),
    Object(Vec<BindingName>),
}

impl ParamPattern {
    pub fn binding_names(&self) -> Vec<&str> {
        match self {
            ParamPattern::Name(n) => vec![n.as_str()],
            ParamPattern::Array(names) | ParamPattern::Object(names) => {
                names.iter().map(std::string::String::as_str).collect()
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub decorators: Vec<ValidatorDecorator>,
    pub pattern: ParamPattern,
    pub ty: Type,
    pub mutable: bool,
    pub optional: bool,
    pub default_value: Option<Box<Expr>>,
    pub rest: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name: String,
    pub ty: Type,
    pub readonly: bool,
    pub optional: bool,
    pub decorators: Vec<ValidatorDecorator>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EnumPayload {
    Unit,
    Tuple(Vec<Type>),
    Struct(Vec<Field>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumMember {
    pub name: String,
    pub value: Option<Expr>,
    pub payload: EnumPayload,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Eq,
    Neq,
    Gt,
    Lt,
    Gte,
    Lte,
    And,
    Or,
    NullCoalesce,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    UShr,
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

impl Expr {
    pub fn new(kind: ExprKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn synthetic(kind: ExprKind) -> Self {
        Self {
            kind,
            span: Span::DUMMY,
        }
    }

    pub fn kind(&self) -> &ExprKind {
        &self.kind
    }
}

impl PartialEq for Expr {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    StrLit(String),
    RegexLit {
        pattern: String,
        flags: String,
    },
    NewSet {
        ty: Option<Type>,
        initial: Option<Box<Expr>>,
    },
    NewMap {
        key: Option<Type>,
        value: Option<Type>,
        initial: Option<Box<Expr>>,
    },
    TemplateLit(Vec<TemplatePart>),
    IntLit(i128),
    UIntLit(u128),
    FloatLit(FloatLiteral),
    DecimalLit(DecimalLiteral),
    BoolLit(bool),
    Null,
    None,
    ObjectLit(Vec<(String, Expr)>),
    ObjectUpdate {
        base: Box<Expr>,
        fields: Vec<(String, Expr)>,
    },
    ArrayLit(Vec<Expr>),
    ArrayRepeat {
        value: Box<Expr>,
        count: Box<Expr>,
    },

    Ident(String),
    Topic,
    FieldAccess {
        object: Box<Expr>,
        field: String,
    },
    IndexAccess {
        object: Box<Expr>,
        index: Box<Expr>,
    },

    BinOp {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },

    ChainedComparison {
        terms: Vec<Expr>,
        ops: Vec<BinOp>,
    },

    Pipeline {
        left: Box<Expr>,
        right: Box<Expr>,
    },

    Not(Box<Expr>),

    Neg(Box<Expr>),

    BitNot(Box<Expr>),

    PrefixIncr(Box<Expr>),
    PrefixDecr(Box<Expr>),

    PostfixIncr(Box<Expr>),
    PostfixDecr(Box<Expr>),

    Ternary {
        cond: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
    },

    Borrow {
        mutable: bool,
        name: String,
        place: Option<Box<Expr>>,
    },

    Unwrap(Box<Expr>),

    Typeof(Box<Expr>),

    Await(Box<Expr>),

    Try(Box<Expr>),

    Spread(Box<Expr>),

    TypeAssert {
        value: Box<Expr>,
        ty: Type,
    },

    ConstAssert(Box<Expr>),

    Comptime(Box<Expr>),

    OptionalMethodCall {
        object: Box<Expr>,
        method: String,
        type_args: Vec<Type>,
        args: Vec<Expr>,
    },

    OptionalFieldAccess {
        object: Box<Expr>,
        field: String,
    },

    MethodCall {
        object: Box<Expr>,
        method: String,
        method_span: Span,
        type_args: Vec<Type>,
        args: Vec<Expr>,
    },

    FnCall {
        callee: String,
        type_args: Vec<Type>,
        args: Vec<Expr>,
    },

    CallValue {
        callee: Box<Expr>,
        type_args: Vec<Type>,
        args: Vec<Expr>,
    },

    ArrowFn {
        is_async: bool,
        type_params: Vec<TypeParam>,
        params: Vec<Param>,
        ret_type: Type,
        throws: Vec<Type>,
        body: Vec<Stmt>,
    },

    If {
        cond: Box<Expr>,
        then_block: Vec<Stmt>,
        else_block: Option<Vec<Stmt>>,
    },

    Loop {
        body: Vec<Stmt>,
    },

    In {
        key: Box<Expr>,
        object: Box<Expr>,
    },

    Is {
        expr: Box<Expr>,
        ty: Type,
    },

    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        inclusive: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum TemplatePart {
    Text(String),
    Expr(Expr, Option<FormatSpec>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatSpec {
    FixedPoint(u32),
    ZeroPadded(u32),
}

impl FormatSpec {
    pub fn as_written(&self) -> String {
        match self {
            FormatSpec::FixedPoint(precision) => format!(".{precision}f"),
            FormatSpec::ZeroPadded(width) => format!("0{width}d"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DestructureBinding {
    pub name: String,
    pub mutable: bool,
    pub ty: Type,
    pub discarded: bool,
}

#[derive(Debug, Clone)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

impl Stmt {
    pub fn new(kind: StmtKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn synthetic(kind: StmtKind) -> Self {
        Self {
            kind,
            span: Span::DUMMY,
        }
    }

    pub fn kind(&self) -> &StmtKind {
        &self.kind
    }
}

impl PartialEq for Stmt {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    VarDecl {
        mutable: bool,
        name: String,
        ty: Type,
        value: Option<Expr>,
        onfail: Option<OnFail>,
    },

    ArrayDestructure {
        mutable: bool,
        bindings: Vec<DestructureBinding>,
        value: Expr,
    },

    ObjectDestructure {
        mutable: bool,
        bindings: Vec<DestructureBinding>,
        value: Expr,
    },

    Assign {
        name: String,
        op: AssignOp,
        value: Expr,
    },

    IndexAssign {
        name: String,
        index: Expr,
        value: Expr,
    },

    FieldAssign {
        object: Expr,
        field: String,
        op: AssignOp,
        value: Expr,
    },

    Return(Option<Expr>),

    Break(Option<Expr>),

    Continue,

    Defer {
        kind: DeferKind,
        keyword_span: Span,
        body: Vec<Stmt>,
    },

    Throw(Expr),

    TryCatch {
        try_body: Vec<Stmt>,
        catch_binding: String,
        catch_type: Type,
        catch_type_span: Span,
        catch_body: Vec<Stmt>,
        finally_body: Option<Vec<Stmt>>,
    },

    While {
        cond: Expr,
        body: Vec<Stmt>,
    },

    DoWhile {
        body: Vec<Stmt>,
        cond: Expr,
    },

    For {
        init: Box<Stmt>,
        cond: Expr,
        update: Box<Stmt>,
        body: Vec<Stmt>,
    },

    ForOf {
        mutable: bool,
        name: String,
        iterable: Expr,
        body: Vec<Stmt>,
    },

    ForIn {
        name: String,
        object: Expr,
        body: Vec<Stmt>,
    },

    Switch {
        expr: Expr,
        cases: Vec<SwitchCase>,
        default: Option<Vec<Stmt>>,
        default_index: Option<usize>,
    },

    Match {
        expr: Expr,
        arms: Vec<MatchArm>,
        default: Option<Vec<Stmt>>,
    },

    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct OnFail {
    pub handler: OnFailHandler,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OnFailHandler {
    Expr(Expr),
    Block(Vec<Stmt>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeferKind {
    Always,
    OnError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeExitReason {
    Normal,
    Return,
    ErrorReturn,
    TryPropagation,
    Break,
    Continue,
}

impl ScopeExitReason {
    pub const fn runs_errdefer(self) -> bool {
        matches!(self, Self::ErrorReturn | Self::TryPropagation)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SwitchCase {
    pub value: Expr,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub pattern_span: Span,
    pub guard: Option<Expr>,
    pub compact: bool,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectMatchField {
    pub key: String,
    pub binding: Option<String>,
    pub literal: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatchPattern {
    Lit(Expr),
    Wildcard,
    Binding(String),
    ResultVariant {
        success: bool,
        binding: Option<String>,
    },
    Tuple(Vec<MatchPattern>),
    Object(Vec<ObjectMatchField>),
    Array {
        elements: Vec<MatchArrayElement>,
        span: Span,
    },
    Or(Vec<MatchPattern>),
    Range {
        start: Option<Expr>,
        end: Option<Expr>,
        inclusive: bool,
    },
    EnumVariant {
        enum_name: String,
        variant: String,
        binding: Option<Box<MatchPattern>>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArrayElement {
    pub pattern: MatchPattern,
    pub kind: MatchArrayElementKind,
    pub span: Span,
}

impl MatchArrayElement {
    pub const fn is_rest(&self) -> bool {
        matches!(self.kind, MatchArrayElementKind::Rest)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchArrayElementKind {
    Pattern,
    Rest,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssignOp {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    ModAssign,
    PowAssign,
    AndAssign,
    OrAssign,
    NullCoalesceAssign,
    BitAndAssign,
    BitOrAssign,
    BitXorAssign,
    ShlAssign,
    ShrAssign,
    UShrAssign,
}

#[derive(Debug, Clone)]
pub struct BehaviorMethod {
    pub name: String,
    pub type_params: Vec<TypeParam>,
    pub params: Vec<Param>,
    pub ret_type: Type,
    pub throws: Vec<Type>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct BehaviorInterfaceInfo {
    pub targets: Vec<Type>,
    pub extends: Vec<Type>,
    pub methods: Vec<BehaviorMethod>,
    pub bindings: Vec<String>,
    pub binding_methods: Vec<BehaviorMethod>,
}

#[derive(Debug, Clone)]
#[allow(
    clippy::large_enum_variant,
    reason = "boxing top-level statements would add a heap allocation to every script statement"
)]
pub enum Item {
    InterfaceDecl {
        name: String,
        type_params: Vec<TypeParam>,
        extends: Vec<Type>,
        fields: Vec<Field>,
        db_model: Option<DbModelMetadata>,
        from_type_alias: bool,
        span: Span,
    },
    BehaviorInterfaceDecl {
        name: String,
        type_params: Vec<TypeParam>,
        targets: Vec<Type>,
        extends: Vec<Type>,
        methods: Vec<BehaviorMethod>,
        bindings: Vec<String>,
        span: Span,
    },
    TypeAlias {
        name: String,
        type_params: Vec<TypeParam>,
        ty: Type,
        merged_fields: Vec<Field>,
        span: Span,
    },
    EnumDecl {
        name: String,
        repr: Type,
        members: Vec<EnumMember>,
        span: Span,
    },
    FnDecl {
        name: String,
        is_async: bool,
        is_comptime: bool,
        is_validator: bool,
        type_params: Vec<TypeParam>,
        params: Vec<Param>,
        ret_type: Type,
        throws: Vec<Type>,
        body: Vec<Stmt>,
        span: Span,
    },

    Stmt(Stmt),
}

impl Item {
    pub fn span(&self) -> Span {
        match self {
            Item::InterfaceDecl { span, .. }
            | Item::BehaviorInterfaceDecl { span, .. }
            | Item::TypeAlias { span, .. }
            | Item::EnumDecl { span, .. }
            | Item::FnDecl { span, .. } => *span,
            Item::Stmt(statement) => statement.span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Program {
    pub imports: Vec<ImportDecl>,
    pub exports: Vec<ExportDecl>,
    pub items: Vec<Item>,
    pub auto_route_exports: Vec<String>,
}

pub const DEFAULT_EXPORT: &str = "default";

#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub source: String,
    pub specifiers: Vec<ImportSpecifier>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ImportSpecifier {
    pub imported: String,
    pub local: String,
    pub kind: ImportKind,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportKind {
    Named,
    Default,
    Namespace,
    Type,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Visibility {
    #[default]
    Public,
    Protect,
}

#[derive(Debug, Clone)]
pub enum ExportDecl {
    Named {
        exported: String,
        local: String,
        inline: bool,
        visibility: Visibility,
        span: Span,
    },
    Re {
        exported: String,
        imported: String,
        source: String,
        span: Span,
    },
    All {
        source: String,
        span: Span,
    },
}

impl ExportDecl {
    pub fn span(&self) -> Span {
        match self {
            ExportDecl::Named { span, .. }
            | ExportDecl::Re { span, .. }
            | ExportDecl::All { span, .. } => *span,
        }
    }

    pub fn exported_name(&self) -> Option<&str> {
        match self {
            ExportDecl::Named { exported, .. } | ExportDecl::Re { exported, .. } => Some(exported),
            ExportDecl::All { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedEnum {
    pub repr: Type,
    pub nominal: bool,
    pub members: Vec<(String, Expr)>,
    pub member_kinds: Vec<EnumPayload>,
    pub is_algebraic: bool,
}

#[derive(Debug, Clone)]
pub struct EnumDiagnostic {
    pub code: &'static str,
    pub message: String,
    pub member: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
enum EnumLit {
    Int(i128),
    Str,
    Bool,
    Float,
    Other,
}

fn classify_enum_value(expr: &Expr) -> EnumLit {
    match expr.kind() {
        ExprKind::IntLit(n) => EnumLit::Int(*n),
        ExprKind::Neg(inner) => match classify_enum_value(inner) {
            EnumLit::Int(n) => EnumLit::Int(-n),
            other => other,
        },
        ExprKind::StrLit(_) | ExprKind::TemplateLit(_) => EnumLit::Str,
        ExprKind::BoolLit(_) => EnumLit::Bool,
        ExprKind::FloatLit(_) => EnumLit::Float,
        _ => EnumLit::Other,
    }
}

fn enum_kind_name(repr: &Type) -> &'static str {
    match repr {
        repr if is_enum_integer_repr(repr) => "integer",
        Type::Str => "string",
        Type::Bool => "boolean",
        _ => "value",
    }
}

fn is_enum_integer_repr(repr: &Type) -> bool {
    matches!(
        repr,
        Type::I8
            | Type::I16
            | Type::I32
            | Type::I64
            | Type::I128
            | Type::U8
            | Type::U16
            | Type::U32
            | Type::U64
            | Type::U128
    )
}

fn enum_repr_bounds(repr: &Type) -> Option<(i128, Option<i128>)> {
    Some(match repr {
        Type::I8 => (i128::from(i8::MIN), Some(i128::from(i8::MAX))),
        Type::I16 => (i128::from(i16::MIN), Some(i128::from(i16::MAX))),
        Type::I32 => (i128::from(i32::MIN), Some(i128::from(i32::MAX))),
        Type::I64 => (i128::from(i64::MIN), Some(i128::from(i64::MAX))),
        Type::I128 => (i128::MIN, Some(i128::MAX)),
        Type::U8 => (0, Some(i128::from(u8::MAX))),
        Type::U16 => (0, Some(i128::from(u16::MAX))),
        Type::U32 => (0, Some(i128::from(u32::MAX))),
        Type::U64 => (0, Some(i128::from(u64::MAX))),
        Type::U128 => (0, None),
        _ => return None,
    })
}

pub fn resolve_enum(repr: &Type, members: &[EnumMember]) -> (ResolvedEnum, Vec<EnumDiagnostic>) {
    let mut diagnostics = Vec::new();

    let is_algebraic = members
        .iter()
        .any(|m| !matches!(m.payload, EnumPayload::Unit));

    let declared = match repr {
        Type::Void => None,
        Type::Str | Type::Bool => Some(repr.clone()),
        repr if is_enum_integer_repr(repr) => Some(repr.clone()),
        other => {
            diagnostics.push(EnumDiagnostic {
                code: "E0701",
                message: format!(
                    "enum representation `{other}` is not allowed; use a fixed-width integer type (`i8` to `i128`, `u8` to `u128`), `string` or `boolean`"
                ),
                member: None,
            });
            None
        }
    };

    if is_algebraic && declared.is_some() {
        diagnostics.push(EnumDiagnostic {
            code: "E0706",
            message: "enum with tuple or struct variants cannot have an explicit representation; remove the repr annotation"
                .to_string(),
            member: None,
        });
    }

    let has_values = members.iter().any(|member| member.value.is_some());

    if declared.is_none() && !has_values && !is_algebraic {
        let resolved = members
            .iter()
            .enumerate()
            .map(|(index, member)| {
                (
                    member.name.clone(),
                    Expr::synthetic(ExprKind::IntLit(index as i128)),
                )
            })
            .collect();
        let member_kinds = members.iter().map(|m| m.payload.clone()).collect();
        return (
            ResolvedEnum {
                repr: Type::I32,
                nominal: true,
                members: resolved,
                member_kinds,
                is_algebraic: false,
            },
            diagnostics,
        );
    }

    if is_algebraic {
        let resolved = members
            .iter()
            .enumerate()
            .map(|(index, member)| {
                (
                    member.name.clone(),
                    Expr::synthetic(ExprKind::IntLit(index as i128)),
                )
            })
            .collect();
        let member_kinds = members.iter().map(|m| m.payload.clone()).collect();
        return (
            ResolvedEnum {
                repr: Type::Void,
                nominal: false,
                members: resolved,
                member_kinds,
                is_algebraic: true,
            },
            diagnostics,
        );
    }

    for member in members.iter().filter(|member| member.value.is_none()) {
        diagnostics.push(EnumDiagnostic {
            code: "E0703",
            message: format!(
                "enum member `{}` must have an explicit value; cannot mix automatic and valued members",
                member.name
            ),
            member: Some(member.name.clone()),
        });
    }

    let inferred = members.iter().find_map(|member| {
        member
            .value
            .as_ref()
            .map(|value| match classify_enum_value(value) {
                EnumLit::Int(_) => Type::I32,
                EnumLit::Str => Type::Str,
                EnumLit::Bool => Type::Bool,
                _ => Type::Void,
            })
    });
    let repr_ty = declared.clone().or(inferred).unwrap_or(Type::I32);

    let mut resolved_members = Vec::with_capacity(members.len());
    let mut member_kinds = Vec::with_capacity(members.len());
    for member in members {
        let Some(value) = &member.value else {
            resolved_members.push((member.name.clone(), Expr::synthetic(ExprKind::IntLit(0))));
            member_kinds.push(member.payload.clone());
            continue;
        };
        let kind = classify_enum_value(value);
        let matches_repr = match (&repr_ty, &kind) {
            (repr, EnumLit::Int(_)) if is_enum_integer_repr(repr) => true,
            (Type::Str, EnumLit::Str) => true,
            (Type::Bool, EnumLit::Bool) => true,
            _ => false,
        };
        if matches!(kind, EnumLit::Float) {
            diagnostics.push(EnumDiagnostic {
                code: "E0701",
                message: format!(
                    "enum member `{}` has a float value; floats are not allowed in enums",
                    member.name
                ),
                member: Some(member.name.clone()),
            });
        } else if !matches_repr {
            diagnostics.push(EnumDiagnostic {
                code: "E0702",
                message: format!(
                    "enum member `{}` does not match the {} representation; enums must be homogeneous",
                    member.name,
                    enum_kind_name(&repr_ty)
                ),
                member: Some(member.name.clone()),
            });
        } else if let (EnumLit::Int(n), Some((min, max))) = (&kind, enum_repr_bounds(&repr_ty)) {
            let n = *n;
            if n < min || max.is_some_and(|max| n > max) {
                let advice = if matches!(repr_ty, Type::I32) && declared.is_none() {
                    "; annotate the enum with a wider representation, such as `: i64` or `: i128`"
                        .to_string()
                } else {
                    match max {
                        Some(max) => format!("; `{repr_ty}` holds values from {min} to {max}"),
                        None => format!("; `{repr_ty}` holds values from {min} up"),
                    }
                };
                diagnostics.push(EnumDiagnostic {
                    code: "E0705",
                    message: format!(
                        "enum member `{}` value {n} does not fit in `{repr_ty}`{advice}",
                        member.name
                    ),
                    member: Some(member.name.clone()),
                });
            }
        }
        resolved_members.push((member.name.clone(), value.clone()));
        member_kinds.push(member.payload.clone());
    }

    (
        ResolvedEnum {
            repr: repr_ty,
            nominal: false,
            members: resolved_members,
            member_kinds,
            is_algebraic: false,
        },
        diagnostics,
    )
}
use std::ops::Range;

impl ExprKind {
    pub fn spanned(self, span: Span) -> Expr {
        Expr::new(self, span)
    }
}

pub fn expr_referenced_identifiers(expr: &Expr) -> crate::collections::HashSet<String> {
    let mut names = crate::collections::HashSet::default();
    collect_expr_identifiers(expr, &mut names);
    names
}

pub fn block_referenced_identifiers(body: &[Stmt]) -> crate::collections::HashSet<String> {
    let mut names = crate::collections::HashSet::default();
    collect_block_identifiers(body, &mut names);
    names
}

fn collect_expr_identifiers(expr: &Expr, names: &mut crate::collections::HashSet<String>) {
    match expr.kind() {
        ExprKind::Ident(name) | ExprKind::Borrow { name, .. } => {
            names.insert(name.clone());
        }
        ExprKind::FnCall { callee, args, .. } => {
            names.insert(callee.clone());
            for arg in args {
                collect_expr_identifiers(arg, names);
            }
        }
        ExprKind::CallValue { callee, args, .. } => {
            collect_expr_identifiers(callee, names);
            for arg in args {
                collect_expr_identifiers(arg, names);
            }
        }
        ExprKind::ObjectLit(fields) => {
            for (_, value) in fields {
                collect_expr_identifiers(value, names);
            }
        }
        ExprKind::ObjectUpdate { base, fields } => {
            collect_expr_identifiers(base, names);
            for (_, value) in fields {
                collect_expr_identifiers(value, names);
            }
        }
        ExprKind::ArrayLit(values) => {
            for value in values {
                collect_expr_identifiers(value, names);
            }
        }
        ExprKind::ArrayRepeat { value, count } => {
            collect_expr_identifiers(value, names);
            collect_expr_identifiers(count, names);
        }
        ExprKind::FieldAccess { object, .. } | ExprKind::OptionalFieldAccess { object, .. } => {
            collect_expr_identifiers(object, names);
        }
        ExprKind::IndexAccess { object, index } => {
            collect_expr_identifiers(object, names);
            collect_expr_identifiers(index, names);
        }
        ExprKind::BinOp { left, right, .. } => {
            collect_expr_identifiers(left, names);
            collect_expr_identifiers(right, names);
        }
        ExprKind::ChainedComparison { terms, .. } => {
            for term in terms {
                collect_expr_identifiers(term, names);
            }
        }
        ExprKind::Pipeline { left, right } => {
            collect_expr_identifiers(left, names);
            collect_expr_identifiers(right, names);
        }
        ExprKind::Not(value)
        | ExprKind::Neg(value)
        | ExprKind::BitNot(value)
        | ExprKind::Typeof(value)
        | ExprKind::Await(value)
        | ExprKind::Try(value)
        | ExprKind::Unwrap(value) => collect_expr_identifiers(value, names),
        ExprKind::TypeAssert { value, .. } => collect_expr_identifiers(value, names),
        ExprKind::ConstAssert(inner) | ExprKind::Comptime(inner) => {
            collect_expr_identifiers(inner, names);
        }
        ExprKind::PrefixIncr(value)
        | ExprKind::PrefixDecr(value)
        | ExprKind::PostfixIncr(value)
        | ExprKind::PostfixDecr(value) => collect_expr_identifiers(value, names),
        ExprKind::Ternary {
            cond,
            then_expr,
            else_expr,
        } => {
            collect_expr_identifiers(cond, names);
            collect_expr_identifiers(then_expr, names);
            collect_expr_identifiers(else_expr, names);
        }
        ExprKind::TemplateLit(parts) => {
            for part in parts {
                if let TemplatePart::Expr(value, _) = part {
                    collect_expr_identifiers(value, names);
                }
            }
        }
        ExprKind::MethodCall { object, args, .. }
        | ExprKind::OptionalMethodCall { object, args, .. } => {
            collect_expr_identifiers(object, names);
            for arg in args {
                collect_expr_identifiers(arg, names);
            }
        }
        ExprKind::NewSet { initial, .. } | ExprKind::NewMap { initial, .. } => {
            if let Some(initial) = initial {
                collect_expr_identifiers(initial, names);
            }
        }
        ExprKind::ArrowFn { .. } => {}
        ExprKind::If {
            cond,
            then_block,
            else_block,
        } => {
            collect_expr_identifiers(cond, names);
            collect_block_identifiers(then_block, names);
            if let Some(block) = else_block {
                collect_block_identifiers(block, names);
            }
        }
        ExprKind::Loop { body } => collect_block_identifiers(body, names),
        ExprKind::In { key, object } => {
            collect_expr_identifiers(key, names);
            collect_expr_identifiers(object, names);
        }
        ExprKind::Is { expr, .. } => collect_expr_identifiers(expr, names),
        ExprKind::Range { start, end, .. } => {
            collect_expr_identifiers(start, names);
            collect_expr_identifiers(end, names);
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
        | ExprKind::Topic => {}
        ExprKind::Spread(inner) => collect_expr_identifiers(inner, names),
    }
}

fn collect_block_identifiers(body: &[Stmt], names: &mut crate::collections::HashSet<String>) {
    for statement in body {
        match statement.kind() {
            StmtKind::VarDecl { value, .. } => {
                if let Some(value) = value {
                    collect_expr_identifiers(value, names);
                }
            }
            StmtKind::Assign { value, .. }
            | StmtKind::Return(Some(value))
            | StmtKind::Expr(value) => collect_expr_identifiers(value, names),
            StmtKind::IndexAssign { name, index, value } => {
                names.insert(name.clone());
                collect_expr_identifiers(index, names);
                collect_expr_identifiers(value, names);
            }
            StmtKind::FieldAssign { object, value, .. } => {
                collect_expr_identifiers(object, names);
                collect_expr_identifiers(value, names);
            }
            StmtKind::While { cond, body } => {
                collect_expr_identifiers(cond, names);
                collect_block_identifiers(body, names);
            }
            StmtKind::DoWhile { body, cond } => {
                collect_block_identifiers(body, names);
                collect_expr_identifiers(cond, names);
            }
            StmtKind::For {
                init,
                cond,
                update,
                body,
            } => {
                collect_block_identifiers(std::slice::from_ref(init), names);
                collect_expr_identifiers(cond, names);
                collect_block_identifiers(std::slice::from_ref(update), names);
                collect_block_identifiers(body, names);
            }
            StmtKind::ForOf { iterable, body, .. } => {
                collect_expr_identifiers(iterable, names);
                collect_block_identifiers(body, names);
            }
            StmtKind::ForIn { object, body, .. } => {
                collect_expr_identifiers(object, names);
                collect_block_identifiers(body, names);
            }
            StmtKind::Return(None) => {}
            StmtKind::Break(value) => {
                if let Some(value) = value {
                    collect_expr_identifiers(value, names);
                }
            }
            StmtKind::Continue => {}
            StmtKind::ArrayDestructure {
                bindings, value, ..
            }
            | StmtKind::ObjectDestructure {
                bindings, value, ..
            } => {
                for binding in bindings {
                    names.insert(binding.name.clone());
                }
                collect_expr_identifiers(value, names);
            }
            StmtKind::Switch {
                expr,
                cases,
                default,
                ..
            } => {
                collect_expr_identifiers(expr, names);
                for case in cases {
                    collect_expr_identifiers(&case.value, names);
                    collect_block_identifiers(&case.body, names);
                }
                if let Some(body) = default {
                    collect_block_identifiers(body, names);
                }
            }
            StmtKind::Match {
                expr,
                arms,
                default,
            } => {
                collect_expr_identifiers(expr, names);
                for arm in arms {
                    collect_block_identifiers(&arm.body, names);
                    if let Some(guard) = &arm.guard {
                        collect_expr_identifiers(guard, names);
                    }
                }
                if let Some(body) = default {
                    collect_block_identifiers(body, names);
                }
            }
            StmtKind::Throw(expr) => {
                collect_expr_identifiers(expr, names);
            }
            StmtKind::TryCatch {
                try_body,
                catch_body,
                finally_body,
                ..
            } => {
                collect_block_identifiers(try_body, names);
                collect_block_identifiers(catch_body, names);
                if let Some(body) = finally_body {
                    collect_block_identifiers(body, names);
                }
            }
            StmtKind::Defer { body, .. } => collect_block_identifiers(body, names),
        }
    }
}

impl StmtKind {
    pub fn spanned(self, span: Span) -> Stmt {
        Stmt::new(self, span)
    }
}
