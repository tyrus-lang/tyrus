use crate::ast::StmtKind;
use crate::ast::{
    Expr, ExprKind, Field, Item, Param, Program, Stmt, TemplatePart, Type, TypeParam,
};
use crate::collections::{HashMap, HashSet};
use crate::span::Span;

#[derive(Debug, Clone)]
pub struct ResolveDiag {
    pub code: &'static str,
    pub message: String,
    pub subject: Option<String>,
}

struct InterfaceDef {
    type_params: Vec<TypeParam>,
    extends: Vec<Type>,
    fields: Vec<Field>,
    from_intersection: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FieldMergeOrigin {
    Extends,
    Intersection,
}

impl FieldMergeOrigin {
    fn of(def: &InterfaceDef) -> Self {
        if def.from_intersection {
            Self::Intersection
        } else {
            Self::Extends
        }
    }

    const fn conflict_code(self) -> &'static str {
        match self {
            Self::Extends => "E3204",
            Self::Intersection => "E3214",
        }
    }

    const fn conflict_context(self) -> &'static str {
        match self {
            Self::Extends => "across inheritance",
            Self::Intersection => "across the intersection",
        }
    }

    const fn operand_code(self) -> &'static str {
        match self {
            Self::Extends => "E3203",
            Self::Intersection => "E3213",
        }
    }

    const fn cycle_code(self) -> &'static str {
        match self {
            Self::Extends => "E3205",
            Self::Intersection => "E3216",
        }
    }

    const fn cycle_message(self) -> &'static str {
        match self {
            Self::Extends => "interface inheritance forms a cycle",
            Self::Intersection => "type intersection forms a cycle",
        }
    }
}

struct AliasDef {
    type_params: Vec<TypeParam>,
    body: Type,
}

#[derive(Default)]
pub struct GenericDefs {
    interfaces: HashMap<String, InterfaceDef>,
    aliases: HashMap<String, AliasDef>,
    lower_unknown: bool,
}

impl GenericDefs {
    pub fn collect(program: &Program) -> Self {
        let mut defs = GenericDefs::default();
        for item in &program.items {
            match item {
                Item::InterfaceDecl {
                    name,
                    type_params,
                    extends,
                    fields,
                    from_type_alias,
                    ..
                } => {
                    defs.interfaces.insert(
                        name.clone(),
                        InterfaceDef {
                            type_params: type_params.clone(),
                            extends: extends.clone(),
                            fields: fields.clone(),
                            from_intersection: *from_type_alias && !extends.is_empty(),
                        },
                    );
                }
                Item::TypeAlias {
                    name,
                    type_params,
                    ty,
                    ..
                } => {
                    defs.aliases.insert(
                        name.clone(),
                        AliasDef {
                            type_params: type_params.clone(),
                            body: ty.clone(),
                        },
                    );
                }
                _ => {}
            }
        }
        defs
    }

    pub fn is_defined(&self, name: &str) -> bool {
        self.interfaces.contains_key(name) || self.aliases.contains_key(name)
    }

    /// Registers an interface that another module declared. The type checker is
    /// never handed the imported module's items, only its already flattened
    /// field lists, so without this the local `GenericDefs` cannot resolve a
    /// supertype that lives in another file and every field of the child is
    /// dropped along with the inherited ones. The fields arrive flattened, so
    /// the definition carries no `extends` of its own.
    pub fn register_imported_interface(&mut self, name: &str, fields: Vec<Field>) {
        if self.interfaces.contains_key(name) || self.aliases.contains_key(name) {
            return;
        }
        self.interfaces.insert(
            name.to_string(),
            InterfaceDef {
                type_params: Vec::new(),
                extends: Vec::new(),
                fields,
                from_intersection: false,
            },
        );
    }

    pub fn register_db_page(&mut self) {
        self.interfaces.insert(
            "Page".to_string(),
            InterfaceDef {
                type_params: vec![TypeParam {
                    name: "T".to_string(),
                    constraint: None,
                }],
                extends: Vec::new(),
                fields: crate::builtin_interfaces::db_page_fields(),
                from_intersection: false,
            },
        );
    }

    pub fn register_stream_alias(&mut self, alias: &str, canonical: &str) {
        let parameters = match canonical {
            "ReadableStream" | "WritableStream" => vec!["T"],
            "DuplexStream" => vec!["TRead", "TWrite"],
            "TransformStream" => vec!["TInput", "TOutput"],
            _ => return,
        };
        let type_params = parameters
            .iter()
            .map(|name| TypeParam {
                name: (*name).to_string(),
                constraint: None,
            })
            .collect::<Vec<_>>();
        let args = parameters
            .iter()
            .map(|name| Type::Generic((*name).to_string()))
            .collect();
        self.aliases.insert(
            alias.to_string(),
            AliasDef {
                type_params,
                body: Type::Apply {
                    name: canonical.to_string(),
                    args,
                },
            },
        );
    }

    fn arity(&self, name: &str) -> Option<usize> {
        self.interfaces
            .get(name)
            .map(|def| def.type_params.len())
            .or_else(|| self.aliases.get(name).map(|def| def.type_params.len()))
    }

    pub fn resolve(&self, ty: &Type, diags: &mut Vec<ResolveDiag>) -> Type {
        self.resolve_depth(ty, diags, 0)
    }

    fn resolve_depth(&self, ty: &Type, diags: &mut Vec<ResolveDiag>, depth: usize) -> Type {
        if depth > 64 {
            diags.push(ResolveDiag {
                code: "E3206",
                message: "type resolution exceeded the recursion limit".to_string(),
                subject: None,
            });
            return Type::Unknown;
        }
        let recur =
            |t: &Type, diags: &mut Vec<ResolveDiag>| self.resolve_depth(t, diags, depth + 1);
        match ty {
            Type::Apply { name, args } => {
                let args: Vec<Type> = args.iter().map(|arg| recur(arg, diags)).collect();
                self.resolve_apply(name, args, diags, depth)
            }
            Type::Named(name) => match name.as_str() {
                "FenwickTree" => Type::FenwickTree,
                "BubbleSort" => Type::BubbleSort,
                "SegmentTree" => Type::SegmentTree,
                "SparseTable" => Type::SparseTable,
                "SegmentTreeLazy" => Type::SegmentTreeLazy,
                "BitSet" => Type::BitSet,
                "Trie" => Type::Trie,
                "BloomFilter" => Type::BloomFilter,
                "SuffixTree" => Type::SuffixTree,
                "Lca" => Type::Lca,
                "Hld" => Type::Hld,
                "Centroid" => Type::Centroid,
                "UnionFind" => Type::UnionFind(Box::new(Type::Usize)),
                _ => {
                    if let Some(def) = self.aliases.get(name)
                        && def.type_params.is_empty()
                    {
                        return self.resolve_depth(&def.body, diags, depth + 1);
                    }
                    if let Some(arity) = self.arity(name)
                        && arity > 0
                    {
                        diags.push(ResolveDiag {
                            code: "E3201",
                            message: format!("type `{name}` expects {arity} type argument(s)"),
                            subject: Some(name.clone()),
                        });
                    }
                    ty.clone()
                }
            },
            Type::KeyOf(inner) => {
                let inner = recur(inner, diags);
                self.resolve_keyof(&inner, diags)
            }
            Type::Array(inner) => Type::Array(Box::new(recur(inner, diags))),
            Type::Map(key, value) => {
                Type::Map(Box::new(recur(key, diags)), Box::new(recur(value, diags)))
            }
            Type::Set(value) => Type::Set(Box::new(recur(value, diags))),
            Type::BinaryHeap(value) => Type::BinaryHeap(Box::new(recur(value, diags))),
            Type::BinomialHeap(value) => Type::BinomialHeap(Box::new(recur(value, diags))),
            Type::UnionFind(value) => Type::UnionFind(Box::new(recur(value, diags))),
            Type::Counter(value) => Type::Counter(Box::new(recur(value, diags))),
            Type::Deque(value) => Type::Deque(Box::new(recur(value, diags))),
            Type::LinkedList(value) => Type::LinkedList(Box::new(recur(value, diags))),
            Type::DoubleLinkedList(value) => Type::DoubleLinkedList(Box::new(recur(value, diags))),
            Type::Readonly(inner) => Type::Readonly(Box::new(recur(inner, diags))),
            Type::Ref(inner) => Type::Ref(Box::new(recur(inner, diags))),
            Type::MutRef(inner) => Type::MutRef(Box::new(recur(inner, diags))),
            Type::Optional(inner) => Type::Optional(Box::new(recur(inner, diags))),
            Type::Result { success, error } => Type::Result {
                success: Box::new(recur(success, diags)),
                error: Box::new(recur(error, diags)),
            },
            Type::Tuple(elements) => {
                Type::Tuple(elements.iter().map(|e| recur(e, diags)).collect())
            }
            Type::Union(variants) => {
                Type::Union(variants.iter().map(|v| recur(v, diags)).collect())
            }
            Type::Function { params, ret } => Type::Function {
                params: params.iter().map(|p| recur(p, diags)).collect(),
                ret: Box::new(recur(ret, diags)),
            },
            Type::Object(fields) => Type::Object(
                fields
                    .iter()
                    .map(|field| Field {
                        ty: recur(&field.ty, diags),
                        ..field.clone()
                    })
                    .collect(),
            ),
            Type::Conditional {
                check,
                bound,
                then,
                otherwise,
            } => self.resolve_conditional(check, bound, then, otherwise, diags, depth),
            _ => ty.clone(),
        }
    }

    #[allow(clippy::too_many_lines)]
    fn resolve_apply(
        &self,
        name: &str,
        args: Vec<Type>,
        diags: &mut Vec<ResolveDiag>,
        depth: usize,
    ) -> Type {
        match name {
            "Partial" => return self.map_fields(name, &args, diags, Modifier::Optional),
            "Required" => return self.map_fields(name, &args, diags, Modifier::Required),
            "Readonly" => return self.map_fields(name, &args, diags, Modifier::Readonly),
            "Pick" => return self.pick_omit(name, &args, diags, true),
            "Omit" => return self.pick_omit(name, &args, diags, false),
            "Optional" if args.len() == 2 => return self.optional_fields(&args, diags),
            "Record" => return self.record(&args, diags),
            "Exclude" => return self.exclude_extract(name, &args, diags, false),
            "Extract" => return self.exclude_extract(name, &args, diags, true),
            "ReturnType" => return self.return_type(&args, diags),
            "Parameters" => return self.parameters(&args, diags),
            "KeysOfByType" => return self.keys_of_by_type(&args, diags),
            "PickByType" => return self.pick_by_type(&args, diags),
            "OmitByType" => return self.omit_by_type(&args, diags),
            "CamelCaseKeys" => return self.camel_case_keys(&args, diags),
            "SnakeCaseKeys" => return self.snake_case_keys(&args, diags),
            "DataOnly" => return self.data_only(&args, diags),
            "DeepPartial" => return self.deep_partial(&args, diags),
            "Require" => return self.require_fields(&args, diags),
            "AtLeastOne" => return self.at_least_one(&args, diags),
            "HttpReply" if args.len() == 1 => {
                return Type::Apply {
                    name: name.to_string(),
                    args,
                };
            }
            "HttpReply" => {
                diags.push(ResolveDiag {
                    code: "E3200",
                    message: format!(
                        "type `HttpReply` expects 1 type argument(s), but {} were provided",
                        args.len()
                    ),
                    subject: Some(name.to_string()),
                });
                return Type::Unknown;
            }
            "Promise" if args.len() == 1 => {
                return Type::Apply {
                    name: name.to_string(),
                    args,
                };
            }
            "DbPromise" if args.len() == 1 => {
                return Type::Apply {
                    name: name.to_string(),
                    args,
                };
            }
            "Promise" => {
                diags.push(ResolveDiag {
                    code: "E3048",
                    message: format!(
                        "type `Promise` expects 1 type argument(s), but {} were provided",
                        args.len()
                    ),
                    subject: Some(name.to_string()),
                });
                return Type::Unknown;
            }
            "JoinHandle" if args.len() == 1 => {
                return Type::Apply {
                    name: name.to_string(),
                    args,
                };
            }
            "JoinHandle" => {
                diags.push(ResolveDiag {
                    code: "E3041",
                    message: format!(
                        "type `JoinHandle` expects 1 type argument(s), but {} were provided",
                        args.len()
                    ),
                    subject: Some(name.to_string()),
                });
                return Type::Unknown;
            }
            "Sender" | "Receiver" if args.len() == 1 => {
                return Type::Apply {
                    name: name.to_string(),
                    args,
                };
            }
            "Sender" | "Receiver" => {
                diags.push(ResolveDiag {
                    code: "E3039",
                    message: format!(
                        "type `{name}` expects 1 type argument(s), but {} were provided",
                        args.len()
                    ),
                    subject: Some(name.to_string()),
                });
                return Type::Unknown;
            }
            "ReadableStream" | "WritableStream" if args.len() == 1 => {
                return Type::Apply {
                    name: name.to_string(),
                    args,
                };
            }
            "ReadableStream" | "WritableStream" => {
                diags.push(ResolveDiag {
                    code: "E3200",
                    message: format!(
                        "type `{name}` expects 1 type argument(s), but {} were provided",
                        args.len()
                    ),
                    subject: Some(name.to_string()),
                });
                return Type::Unknown;
            }
            "DuplexStream" | "TransformStream" if args.len() == 2 => {
                return Type::Apply {
                    name: name.to_string(),
                    args,
                };
            }
            "DuplexStream" | "TransformStream" => {
                diags.push(ResolveDiag {
                    code: "E3200",
                    message: format!(
                        "type `{name}` expects 2 type argument(s), but {} were provided",
                        args.len()
                    ),
                    subject: Some(name.to_string()),
                });
                return Type::Unknown;
            }
            "Mutex" | "MutexGuard" if args.len() == 1 => {
                return Type::Apply {
                    name: name.to_string(),
                    args,
                };
            }
            "Mutex" | "MutexGuard" => {
                diags.push(ResolveDiag {
                    code: "E3043",
                    message: format!(
                        "type `{name}` expects 1 type argument(s), but {} were provided",
                        args.len()
                    ),
                    subject: Some(name.to_string()),
                });
                return Type::Unknown;
            }
            "RwLock" | "RwReadGuard" | "RwWriteGuard" if args.len() == 1 => {
                return Type::Apply {
                    name: name.to_string(),
                    args,
                };
            }
            "RwLock" | "RwReadGuard" | "RwWriteGuard" => {
                diags.push(ResolveDiag {
                    code: "E3045",
                    message: format!(
                        "type `{name}` expects 1 type argument(s), but {} were provided",
                        args.len()
                    ),
                    subject: Some(name.to_string()),
                });
                return Type::Unknown;
            }
            "Tuple" => return Type::Tuple(args),
            "Optional" if args.len() == 1 => {
                return Type::Optional(Box::new(args.into_iter().next().unwrap()));
            }
            "Optional" if args.len() == 2 && args[1] == Type::Void => {
                return Type::Optional(Box::new(args[0].clone()));
            }
            "Optional" => {
                diags.push(ResolveDiag {
                    code: "E3200",
                    message: format!(
                        "type `Optional` expects 1 or 2 type argument(s), but {} were provided",
                        args.len()
                    ),
                    subject: Some(name.to_string()),
                });
                return Type::Unknown;
            }
            "Result" if args.len() == 2 => {
                if matches!(args[1], Type::Optional(_)) {
                    diags.push(ResolveDiag {
                        code: "E3072",
                        message: "`Result<T, E>` does not accept an optional error type; place absence in the success type, as in `Result<Optional<T, void>, E>`".to_string(),
                        subject: Some(name.to_string()),
                    });
                    return Type::Unknown;
                }
                return Type::Result {
                    success: Box::new(args[0].clone()),
                    error: Box::new(args[1].clone()),
                };
            }
            "Result" => {
                diags.push(ResolveDiag {
                    code: "E3200",
                    message: format!(
                        "type `Result` expects 2 type argument(s), but {} were provided",
                        args.len()
                    ),
                    subject: Some(name.to_string()),
                });
                return Type::Unknown;
            }
            "Range" if args.len() == 1 => {
                return Type::Range(Box::new(args.into_iter().next().unwrap()));
            }
            "Range" => {
                diags.push(ResolveDiag {
                    code: "E3200",
                    message: format!(
                        "type `Range` expects 1 type argument(s), but {} were provided",
                        args.len()
                    ),
                    subject: Some(name.to_string()),
                });
                return Type::Unknown;
            }
            "Set" if args.len() == 1 => {
                return Type::Set(Box::new(args.into_iter().next().unwrap()));
            }
            "BinaryHeap" if args.len() == 1 => {
                return Type::BinaryHeap(Box::new(args.into_iter().next().unwrap()));
            }
            "BinomialHeap" if args.len() == 1 => {
                return Type::BinomialHeap(Box::new(args.into_iter().next().unwrap()));
            }
            "UnionFind" if args.len() == 1 => {
                return Type::UnionFind(Box::new(args.into_iter().next().unwrap()));
            }
            "Counter" if args.len() == 1 => {
                return Type::Counter(Box::new(args.into_iter().next().unwrap()));
            }
            "Deque" if args.len() == 1 => {
                return Type::Deque(Box::new(args.into_iter().next().unwrap()));
            }
            "SortedSet" if args.len() == 1 => {
                return Type::SortedSet(Box::new(args.into_iter().next().unwrap()));
            }
            "SkipList" if args.len() == 1 => {
                return Type::SkipList(Box::new(args.into_iter().next().unwrap()));
            }
            "SortedMap" if args.len() == 2 => {
                let mut args = args.into_iter();
                return Type::SortedMap(
                    Box::new(args.next().unwrap()),
                    Box::new(args.next().unwrap()),
                );
            }
            "Graph" if args.len() == 2 => {
                let mut args = args.into_iter();
                return Type::Graph(
                    Box::new(args.next().unwrap()),
                    Box::new(args.next().unwrap()),
                );
            }
            "LinkedList" if args.len() == 1 => {
                return Type::LinkedList(Box::new(args.into_iter().next().unwrap()));
            }
            "DoubleLinkedList" if args.len() == 1 => {
                return Type::DoubleLinkedList(Box::new(args.into_iter().next().unwrap()));
            }
            "AvlTree" if args.len() == 1 => {
                return Type::AvlTree(Box::new(args.into_iter().next().unwrap()));
            }
            "RedBlackTree" if args.len() == 1 => {
                return Type::RedBlackTree(Box::new(args.into_iter().next().unwrap()));
            }
            "SplayTree" if args.len() == 1 => {
                return Type::SplayTree(Box::new(args.into_iter().next().unwrap()));
            }
            "Treap" if args.len() == 1 => {
                return Type::Treap(Box::new(args.into_iter().next().unwrap()));
            }
            "Map" if args.len() == 2 => {
                let mut args = args.into_iter();
                return Type::Map(
                    Box::new(args.next().unwrap()),
                    Box::new(args.next().unwrap()),
                );
            }
            _ => {}
        }
        let Some(expected) = self.arity(name) else {
            diags.push(ResolveDiag {
                code: "E3202",
                message: format!("cannot find generic type `{name}`"),
                subject: Some(name.to_string()),
            });
            return Type::Unknown;
        };
        if args.len() != expected {
            diags.push(ResolveDiag {
                code: "E3200",
                message: format!(
                    "type `{name}` expects {expected} type argument(s), but {} were provided",
                    args.len()
                ),
                subject: Some(name.to_string()),
            });
            return Type::Unknown;
        }
        if let Some(def) = self.aliases.get(name) {
            let map = bind(&def.type_params, &args);
            let body = substitute(&def.body, &map);
            return self.resolve_depth(&body, diags, depth + 1);
        }
        if let Some(def) = self.interfaces.get(name) {
            let map = bind(&def.type_params, &args);
            let mut fields = Vec::new();
            let mut chain = vec![name.to_string()];
            self.collect_fields_of(def, &map, &mut fields, diags, depth, &mut chain);
            return Type::Object(fields);
        }
        Type::Unknown
    }

    fn object_fields(&self, ty: &Type, diags: &mut Vec<ResolveDiag>) -> Option<Vec<Field>> {
        match ty {
            Type::Object(fields) => Some(fields.clone()),
            Type::Named(name) => self.interface_fields(name, diags),
            _ => None,
        }
    }

    fn map_fields(
        &self,
        name: &str,
        args: &[Type],
        diags: &mut Vec<ResolveDiag>,
        modifier: Modifier,
    ) -> Type {
        if !self.check_utility_arity(name, args, 1, diags) {
            return Type::Unknown;
        }
        let Some(fields) = self.object_fields(&args[0], diags) else {
            return self.utility_non_object(name, &args[0], diags);
        };
        if matches!(modifier, Modifier::Optional) {
            return Type::Object(
                ObjectProjection::all_fields(fields, ObjectProjectionMode::Optional).apply(),
            );
        }
        let mut fields: Vec<_> = fields
            .into_iter()
            .map(|field| match modifier {
                Modifier::Optional => unreachable!("optional fields are projected above"),
                Modifier::Required => Field {
                    optional: false,
                    ..field
                },
                Modifier::Readonly => Field {
                    readonly: true,
                    ty: Self::deeply_readonly(field.ty),
                    ..field
                },
            })
            .collect();
        // User-declared interfaces use a compact native storage order. A
        // `Readonly<T>` result is structural, but it can receive a `T` value
        // without a conversion, so it must preserve that exact ABI order.
        if matches!(modifier, Modifier::Readonly) {
            self.readonly_storage_order(&mut fields);
        }
        Type::Object(fields)
    }

    fn readonly_storage_order(&self, fields: &mut [Field]) {
        fields.sort_by_key(|field| {
            let mut seen = HashSet::default();
            let (align, size) = self.readonly_storage_weight(field, &mut seen);
            std::cmp::Reverse((align, size))
        });
    }

    fn readonly_storage_weight(&self, field: &Field, seen: &mut HashSet<String>) -> (u32, u32) {
        if field.optional {
            return (8, 8);
        }
        self.readonly_type_storage_weight(&field.ty, seen)
    }

    fn readonly_type_storage_weight(&self, ty: &Type, seen: &mut HashSet<String>) -> (u32, u32) {
        match ty {
            Type::Readonly(inner) | Type::Ref(inner) | Type::MutRef(inner) => {
                self.readonly_type_storage_weight(inner, seen)
            }
            Type::Bool | Type::I8 | Type::U8 => (1, 1),
            Type::I16 | Type::U16 => (2, 2),
            Type::I32 | Type::U32 | Type::F32 | Type::Char => (4, 4),
            Type::I64 | Type::U64 | Type::F64 | Type::Isize | Type::Usize => (8, 8),
            Type::I128 | Type::U128 | Type::F128 => (16, 16),
            Type::Decimal => (16, 32),
            Type::Array(_) => (8, 24),
            Type::Str | Type::Regex => (8, 16),
            Type::Tuple(elements) => elements.iter().fold((1, 0), |(align, size), element| {
                let (element_align, element_size) =
                    self.readonly_type_storage_weight(element, seen);
                (align.max(element_align), size + element_size)
            }),
            Type::Object(fields) => fields.iter().fold((1, 0), |(align, size), field| {
                let (field_align, field_size) = self.readonly_storage_weight(field, seen);
                (align.max(field_align), size + field_size)
            }),
            Type::Named(name) => {
                if !seen.insert(name.clone()) {
                    return (8, 8);
                }
                self.interfaces.get(name).map_or((8, 8), |definition| {
                    definition
                        .fields
                        .iter()
                        .fold((1, 0), |(align, size), field| {
                            let (field_align, field_size) =
                                self.readonly_storage_weight(field, seen);
                            (align.max(field_align), size + field_size)
                        })
                })
            }
            _ => (8, 8),
        }
    }

    fn deeply_readonly(ty: Type) -> Type {
        match &ty {
            Type::Readonly(_) => ty,
            Type::Named(_)
            | Type::Object(_)
            | Type::Array(_)
            | Type::Tuple(_)
            | Type::Apply { .. } => Type::Readonly(Box::new(ty)),
            _ => ty,
        }
    }

    fn pick_omit(
        &self,
        name: &str,
        args: &[Type],
        diags: &mut Vec<ResolveDiag>,
        keep: bool,
    ) -> Type {
        if !self.check_utility_arity(name, args, 2, diags) {
            return Type::Unknown;
        }
        let Some(fields) = self.object_fields(&args[0], diags) else {
            return self.utility_non_object(name, &args[0], diags);
        };
        let Some(keys) = self.projection_keys(name, &args[1], &fields, diags) else {
            return Type::Unknown;
        };
        let mode = if keep {
            ObjectProjectionMode::Pick
        } else {
            ObjectProjectionMode::Omit
        };
        Type::Object(ObjectProjection::new(fields, keys, mode).apply())
    }

    fn optional_fields(&self, args: &[Type], diags: &mut Vec<ResolveDiag>) -> Type {
        debug_assert_eq!(args.len(), 2);
        let Some(fields) = self.object_fields(&args[0], diags) else {
            return self.utility_non_object("Optional", &args[0], diags);
        };
        let Some(keys) = self.projection_keys("Optional", &args[1], &fields, diags) else {
            return Type::Unknown;
        };
        Type::Object(ObjectProjection::new(fields, keys, ObjectProjectionMode::Optional).apply())
    }

    fn projection_keys(
        &self,
        name: &str,
        key_type: &Type,
        fields: &[Field],
        diags: &mut Vec<ResolveDiag>,
    ) -> Option<Vec<String>> {
        let Some(keys) = literal_key_names(key_type) else {
            diags.push(ResolveDiag {
                code: "E3209",
                message: format!("the keys of `{name}` must be string literal types"),
                subject: None,
            });
            return None;
        };
        for key in &keys {
            if !fields.iter().any(|field| &field.name == key) {
                diags.push(ResolveDiag {
                    code: "E3212",
                    message: format!("`{key}` is not a property of the type given to `{name}`"),
                    subject: Some(key.clone()),
                });
            }
        }
        Some(keys)
    }

    fn pick_by_type(&self, args: &[Type], diags: &mut Vec<ResolveDiag>) -> Type {
        if !self.check_utility_arity("PickByType", args, 2, diags) {
            return Type::Unknown;
        }
        let Some(fields) = self.object_fields(&args[0], diags) else {
            return self.utility_non_object("PickByType", &args[0], diags);
        };
        let filter = &args[1];
        let fields = fields
            .into_iter()
            .filter(|field| type_matches(&field.ty, filter))
            .collect();
        Type::Object(fields)
    }

    fn omit_by_type(&self, args: &[Type], diags: &mut Vec<ResolveDiag>) -> Type {
        if !self.check_utility_arity("OmitByType", args, 2, diags) {
            return Type::Unknown;
        }
        let Some(fields) = self.object_fields(&args[0], diags) else {
            return self.utility_non_object("OmitByType", &args[0], diags);
        };
        let filter = &args[1];
        let fields = fields
            .into_iter()
            .filter(|field| !type_matches(&field.ty, filter))
            .collect();
        Type::Object(fields)
    }

    fn camel_case_keys(&self, args: &[Type], diags: &mut Vec<ResolveDiag>) -> Type {
        if !self.check_utility_arity("CamelCaseKeys", args, 1, diags) {
            return Type::Unknown;
        }
        let Some(fields) = self.object_fields(&args[0], diags) else {
            return self.utility_non_object("CamelCaseKeys", &args[0], diags);
        };
        let fields = fields
            .into_iter()
            .map(|field| Field {
                name: snake_to_camel(&field.name),
                ..field
            })
            .collect();
        Type::Object(fields)
    }

    fn snake_case_keys(&self, args: &[Type], diags: &mut Vec<ResolveDiag>) -> Type {
        if !self.check_utility_arity("SnakeCaseKeys", args, 1, diags) {
            return Type::Unknown;
        }
        let Some(fields) = self.object_fields(&args[0], diags) else {
            return self.utility_non_object("SnakeCaseKeys", &args[0], diags);
        };
        let fields = fields
            .into_iter()
            .map(|field| Field {
                name: camel_to_snake(&field.name),
                ..field
            })
            .collect();
        Type::Object(fields)
    }

    fn data_only(&self, args: &[Type], diags: &mut Vec<ResolveDiag>) -> Type {
        if !self.check_utility_arity("DataOnly", args, 1, diags) {
            return Type::Unknown;
        }
        let Some(fields) = self.object_fields(&args[0], diags) else {
            return self.utility_non_object("DataOnly", &args[0], diags);
        };
        let fields = fields
            .into_iter()
            .filter(|field| {
                !matches!(
                    field.ty,
                    Type::Function { .. } | Type::GenericFunction { .. }
                )
            })
            .collect();
        Type::Object(fields)
    }

    fn deep_partial(&self, args: &[Type], diags: &mut Vec<ResolveDiag>) -> Type {
        if !self.check_utility_arity("DeepPartial", args, 1, diags) {
            return Type::Unknown;
        }
        let Some(fields) = self.object_fields(&args[0], diags) else {
            return self.utility_non_object("DeepPartial", &args[0], diags);
        };
        let fields = fields
            .into_iter()
            .map(|field| Field {
                optional: true,
                ty: self.deep_partial_type(field.ty, 5),
                ..field
            })
            .collect();
        Type::Object(fields)
    }

    fn deep_partial_type(&self, ty: Type, depth: usize) -> Type {
        if depth == 0 {
            return ty;
        }
        match ty {
            Type::Object(fields) => {
                let fields = fields
                    .into_iter()
                    .map(|field| Field {
                        optional: true,
                        ty: self.deep_partial_type(field.ty, depth - 1),
                        ..field
                    })
                    .collect();
                Type::Object(fields)
            }
            Type::Named(name) => {
                let mut ignored = Vec::new();
                self.interface_fields(&name, &mut ignored)
                    .map(|fields| {
                        Type::Object(
                            fields
                                .into_iter()
                                .map(|field| Field {
                                    optional: true,
                                    ty: self.deep_partial_type(field.ty, depth - 1),
                                    ..field
                                })
                                .collect(),
                        )
                    })
                    .unwrap_or(Type::Named(name))
            }
            other => other,
        }
    }

    fn require_fields(&self, args: &[Type], diags: &mut Vec<ResolveDiag>) -> Type {
        if !self.check_utility_arity("Require", args, 2, diags) {
            return Type::Unknown;
        }
        let Some(fields) = self.object_fields(&args[0], diags) else {
            return self.utility_non_object("Require", &args[0], diags);
        };
        let Some(keys) = literal_key_names(&args[1]) else {
            diags.push(ResolveDiag {
                code: "E3209",
                message: "the keys of `Require` must be string literal types".to_string(),
                subject: None,
            });
            return Type::Unknown;
        };
        let fields = fields
            .into_iter()
            .map(|field| {
                if keys.contains(&field.name) {
                    Field {
                        optional: false,
                        ..field
                    }
                } else {
                    field
                }
            })
            .collect();
        Type::Object(fields)
    }

    fn at_least_one(&self, args: &[Type], diags: &mut Vec<ResolveDiag>) -> Type {
        if !self.check_utility_arity("AtLeastOne", args, 2, diags) {
            return Type::Unknown;
        }
        let Some(fields) = self.object_fields(&args[0], diags) else {
            return self.utility_non_object("AtLeastOne", &args[0], diags);
        };
        let Some(keys) = self.projection_keys("AtLeastOne", &args[1], &fields, diags) else {
            return Type::Unknown;
        };
        Type::Union(
            keys.iter()
                .map(|required| {
                    Type::Object(
                        fields
                            .iter()
                            .cloned()
                            .map(|field| {
                                if keys.contains(&field.name) {
                                    Field {
                                        optional: field.name != *required,
                                        ..field
                                    }
                                } else {
                                    field
                                }
                            })
                            .collect(),
                    )
                })
                .collect(),
        )
    }

    fn record(&self, args: &[Type], diags: &mut Vec<ResolveDiag>) -> Type {
        if !self.check_utility_arity("Record", args, 2, diags) {
            return Type::Unknown;
        }
        let value = &args[1];
        if let Some(keys) = literal_key_names(&args[0]) {
            let fields = keys
                .into_iter()
                .map(|name| Field {
                    name,
                    ty: value.clone(),
                    readonly: false,
                    optional: false,
                    decorators: Vec::new(),
                    span: Span::DUMMY,
                })
                .collect();
            return Type::Object(fields);
        }
        Type::Map(Box::new(args[0].clone()), Box::new(value.clone()))
    }

    fn check_utility_arity(
        &self,
        name: &str,
        args: &[Type],
        expected: usize,
        diags: &mut Vec<ResolveDiag>,
    ) -> bool {
        if args.len() == expected {
            return true;
        }
        diags.push(ResolveDiag {
            code: "E3208",
            message: format!(
                "`{name}` expects {expected} type argument(s), but {} were provided",
                args.len()
            ),
            subject: Some(name.to_string()),
        });
        false
    }

    fn utility_non_object(&self, name: &str, arg: &Type, diags: &mut Vec<ResolveDiag>) -> Type {
        diags.push(ResolveDiag {
            code: "E3211",
            message: format!("`{name}` requires an object type, found `{arg}`"),
            subject: None,
        });
        Type::Unknown
    }

    fn resolve_conditional(
        &self,
        check: &Type,
        bound: &Type,
        then: &Type,
        otherwise: &Type,
        diags: &mut Vec<ResolveDiag>,
        depth: usize,
    ) -> Type {
        let check = self.resolve_depth(check, diags, depth + 1);
        if let Type::Union(members) = &check {
            let branches = members
                .iter()
                .map(|member| {
                    self.resolve_conditional_one(member, bound, then, otherwise, diags, depth)
                })
                .collect();
            return union_from(branches);
        }
        self.resolve_conditional_one(&check, bound, then, otherwise, diags, depth)
    }

    fn resolve_conditional_one(
        &self,
        check: &Type,
        bound: &Type,
        then: &Type,
        otherwise: &Type,
        diags: &mut Vec<ResolveDiag>,
        depth: usize,
    ) -> Type {
        let bound = self.resolve_depth(bound, diags, depth + 1);
        let mut bindings = HashMap::default();
        if self.type_extends(check, &bound, &mut bindings) {
            let then = substitute(then, &bindings);
            self.resolve_depth(&then, diags, depth + 1)
        } else {
            self.resolve_depth(otherwise, diags, depth + 1)
        }
    }

    fn type_extends(
        &self,
        check: &Type,
        bound: &Type,
        bindings: &mut HashMap<String, Type>,
    ) -> bool {
        match bound {
            Type::InferVar(name) => {
                bindings.insert(name.clone(), check.clone());
                true
            }
            Type::Unknown => true,
            Type::Function {
                params: bound_params,
                ret: bound_ret,
            } => match self.as_function(check) {
                Some((check_params, check_ret)) => {
                    for (c, b) in check_params.iter().zip(bound_params) {
                        self.type_extends(c, b, bindings);
                    }
                    self.type_extends(&check_ret, bound_ret, bindings)
                }
                None => false,
            },
            Type::Union(variants) => variants
                .iter()
                .any(|variant| self.type_extends(check, variant, bindings)),
            _ => self.simple_extends(check, bound),
        }
    }

    fn simple_extends(&self, check: &Type, bound: &Type) -> bool {
        if check == bound {
            return true;
        }
        match (check, bound) {
            (Type::Never, _) => true,
            (_, Type::Unknown) => true,
            (Type::Lit(lit), primitive) => literal_underlying(lit).as_ref() == Some(primitive),
            _ => false,
        }
    }

    fn as_function(&self, ty: &Type) -> Option<(Vec<Type>, Type)> {
        match ty {
            Type::Function { params, ret } => Some((params.clone(), (**ret).clone())),
            Type::Named(name) => {
                let def = self.aliases.get(name)?;
                if def.type_params.is_empty() {
                    self.as_function(&def.body)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn exclude_extract(
        &self,
        name: &str,
        args: &[Type],
        diags: &mut Vec<ResolveDiag>,
        keep: bool,
    ) -> Type {
        if !self.check_utility_arity(name, args, 2, diags) {
            return Type::Unknown;
        }
        let members = match &args[0] {
            Type::Union(variants) => variants.clone(),
            other => vec![other.clone()],
        };
        let kept = members
            .into_iter()
            .filter(|member| {
                let mut bindings = HashMap::default();
                self.type_extends(member, &args[1], &mut bindings) == keep
            })
            .collect();
        union_from(kept)
    }

    fn return_type(&self, args: &[Type], diags: &mut Vec<ResolveDiag>) -> Type {
        if !self.check_utility_arity("ReturnType", args, 1, diags) {
            return Type::Unknown;
        }
        match self.as_function(&args[0]) {
            Some((_, ret)) => ret,
            None => self.utility_non_function("ReturnType", &args[0], diags),
        }
    }

    fn parameters(&self, args: &[Type], diags: &mut Vec<ResolveDiag>) -> Type {
        if !self.check_utility_arity("Parameters", args, 1, diags) {
            return Type::Unknown;
        }
        match self.as_function(&args[0]) {
            Some((params, _)) => Type::Tuple(params),
            None => self.utility_non_function("Parameters", &args[0], diags),
        }
    }

    fn utility_non_function(&self, name: &str, arg: &Type, diags: &mut Vec<ResolveDiag>) -> Type {
        diags.push(ResolveDiag {
            code: "E3210",
            message: format!("`{name}` requires a function type, found `{arg}`"),
            subject: None,
        });
        Type::Never
    }

    fn keys_of_by_type(&self, args: &[Type], diags: &mut Vec<ResolveDiag>) -> Type {
        if !self.check_utility_arity("KeysOfByType", args, 2, diags) {
            return Type::Unknown;
        }
        let Some(fields) = self.object_fields(&args[0], diags) else {
            return self.utility_non_object("KeysOfByType", &args[0], diags);
        };
        let wanted = &args[1];
        let variants = fields
            .into_iter()
            .filter(|field| self.resolve_depth(&field.ty, &mut Vec::new(), 0) == *wanted)
            .map(|field| Type::Lit(Box::new(Expr::synthetic(ExprKind::StrLit(field.name)))))
            .collect::<Vec<_>>();
        if variants.is_empty() {
            Type::Never
        } else {
            Type::Union(variants)
        }
    }

    fn resolve_keyof(&self, inner: &Type, diags: &mut Vec<ResolveDiag>) -> Type {
        let fields = match inner {
            Type::Object(fields) => Some(fields.clone()),
            Type::Named(name) => self.interface_fields(name, diags),
            _ => None,
        };
        let Some(fields) = fields else {
            diags.push(ResolveDiag {
                code: "E3207",
                message: format!("`keyof` requires an object type, found `{inner}`"),
                subject: None,
            });
            return Type::Never;
        };
        if fields.is_empty() {
            return Type::Never;
        }
        let variants = fields
            .iter()
            .map(|field| {
                Type::Lit(Box::new(Expr::synthetic(ExprKind::StrLit(
                    field.name.clone(),
                ))))
            })
            .collect();
        Type::Union(variants)
    }

    pub fn interface_extends(&self, name: &str, base: &str) -> bool {
        fn walk(defs: &GenericDefs, name: &str, base: &str, depth: usize) -> bool {
            if depth > 64 {
                return false;
            }
            let Some(def) = defs.interfaces.get(name) else {
                return false;
            };
            def.extends.iter().any(|supertype| match supertype {
                Type::Named(parent) => parent == base || walk(defs, parent, base, depth + 1),
                _ => false,
            })
        }
        walk(self, name, base, 0)
    }

    pub fn interface_fields(&self, name: &str, diags: &mut Vec<ResolveDiag>) -> Option<Vec<Field>> {
        self.interface_fields_at_depth(name, diags, 0, &mut Vec::new())
    }

    fn interface_fields_at_depth(
        &self,
        name: &str,
        diags: &mut Vec<ResolveDiag>,
        depth: usize,
        chain: &mut Vec<String>,
    ) -> Option<Vec<Field>> {
        let def = self.interfaces.get(name)?;
        if !def.type_params.is_empty() {
            return None;
        }
        if chain.iter().any(|visited| visited == name) {
            let origin = FieldMergeOrigin::of(def);
            diags.push(ResolveDiag {
                code: origin.cycle_code(),
                message: format!("{} through `{name}`", origin.cycle_message()),
                subject: Some(name.to_string()),
            });
            return Some(Vec::new());
        }
        chain.push(name.to_string());
        let mut fields = Vec::new();
        self.collect_fields_of(def, &HashMap::default(), &mut fields, diags, depth, chain);
        chain.pop();
        Some(fields)
    }

    fn collect_fields_of(
        &self,
        def: &InterfaceDef,
        map: &HashMap<String, Type>,
        out: &mut Vec<Field>,
        diags: &mut Vec<ResolveDiag>,
        depth: usize,
        chain: &mut Vec<String>,
    ) {
        let origin = FieldMergeOrigin::of(def);
        if depth > 64 {
            diags.push(ResolveDiag {
                code: origin.cycle_code(),
                message: origin.cycle_message().to_string(),
                subject: None,
            });
            return;
        }
        for supertype in &def.extends {
            let resolved = self.resolve_depth(&substitute(supertype, map), diags, depth + 1);
            let super_fields = match &resolved {
                Type::Object(fields) => fields.clone(),
                Type::Named(name) => self
                    .interface_fields_at_depth(name, diags, depth + 1, chain)
                    .unwrap_or_else(|| {
                        crate::async_types::error_interfaces()
                            .remove(name.as_str())
                            .unwrap_or_default()
                    }),
                _ => {
                    let message = match origin {
                        FieldMergeOrigin::Extends => {
                            format!("an interface can only extend interfaces, found `{resolved}`")
                        }
                        FieldMergeOrigin::Intersection => {
                            format!("`&` can only combine object types, found `{resolved}`")
                        }
                    };
                    diags.push(ResolveDiag {
                        code: origin.operand_code(),
                        message,
                        subject: None,
                    });
                    Vec::new()
                }
            };
            for field in super_fields {
                merge_field(out, field, origin, diags);
            }
        }
        for field in &def.fields {
            merge_field(
                out,
                Field {
                    ty: substitute(&field.ty, map),
                    ..field.clone()
                },
                origin,
                diags,
            );
        }
    }
}

enum Modifier {
    Optional,
    Required,
    Readonly,
}

struct ObjectProjection {
    source_fields: Vec<Field>,
    keys: HashSet<String>,
    mode: ObjectProjectionMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ObjectProjectionMode {
    Pick,
    Omit,
    Optional,
}

impl ObjectProjection {
    fn new(
        source_fields: Vec<Field>,
        keys: impl IntoIterator<Item = String>,
        mode: ObjectProjectionMode,
    ) -> Self {
        Self {
            source_fields,
            keys: keys.into_iter().collect(),
            mode,
        }
    }

    fn all_fields(source_fields: Vec<Field>, mode: ObjectProjectionMode) -> Self {
        let keys = source_fields
            .iter()
            .map(|field| field.name.clone())
            .collect::<Vec<_>>();
        Self::new(source_fields, keys, mode)
    }

    fn apply(self) -> Vec<Field> {
        match self.mode {
            ObjectProjectionMode::Pick => self
                .source_fields
                .into_iter()
                .filter(|field| self.keys.contains(&field.name))
                .collect(),
            ObjectProjectionMode::Omit => self
                .source_fields
                .into_iter()
                .filter(|field| !self.keys.contains(&field.name))
                .collect(),
            ObjectProjectionMode::Optional => self
                .source_fields
                .into_iter()
                .map(|field| Field {
                    optional: field.optional || self.keys.contains(&field.name),
                    ..field
                })
                .collect(),
        }
    }
}

pub(crate) fn project_object_fields(
    source_fields: Vec<Field>,
    keys: impl IntoIterator<Item = String>,
    mode: ObjectProjectionMode,
) -> Vec<Field> {
    ObjectProjection::new(source_fields, keys, mode).apply()
}

fn literal_key_names(ty: &Type) -> Option<Vec<String>> {
    fn one(ty: &Type) -> Option<String> {
        match ty {
            Type::Lit(expr) => match expr.as_ref() {
                Expr {
                    kind: ExprKind::StrLit(name),
                    ..
                } => Some(name.clone()),
                _ => None,
            },
            _ => None,
        }
    }
    match ty {
        Type::Never => Some(Vec::new()),
        Type::Union(variants) => variants.iter().map(one).collect(),
        other => one(other).map(|name| vec![name]),
    }
}

fn resolve_named_type_alias(ty: &Type) -> Type {
    match ty {
        Type::Named(name) => match name.as_str() {
            "bool" | "boolean" => Type::Bool,
            "string" => Type::Str,
            "void" => Type::Void,
            "number" => Type::F64,
            "any" => Type::Unknown,
            "never" => Type::Never,
            "undefined" | "null" => Type::Void,
            _ => ty.clone(),
        },
        _ => ty.clone(),
    }
}

fn type_matches(a: &Type, b: &Type) -> bool {
    let a = resolve_named_type_alias(a);
    let b = resolve_named_type_alias(b);
    match (&a, &b) {
        (Type::I8, Type::I8)
        | (Type::I16, Type::I16)
        | (Type::I32, Type::I32)
        | (Type::I64, Type::I64)
        | (Type::I128, Type::I128)
        | (Type::Isize, Type::Isize)
        | (Type::U8, Type::U8)
        | (Type::U16, Type::U16)
        | (Type::U32, Type::U32)
        | (Type::U64, Type::U64)
        | (Type::U128, Type::U128)
        | (Type::Usize, Type::Usize)
        | (Type::F32, Type::F32)
        | (Type::F64, Type::F64)
        | (Type::F128, Type::F128)
        | (Type::Decimal, Type::Decimal)
        | (Type::Bool, Type::Bool)
        | (Type::Char, Type::Char)
        | (Type::Str, Type::Str)
        | (Type::Void, Type::Void) => true,
        (Type::Array(a_inner), Type::Array(b_inner)) => type_matches(a_inner, b_inner),
        (Type::Optional(a_inner), Type::Optional(b_inner)) => type_matches(a_inner, b_inner),
        (Type::Named(a_name), Type::Named(b_name)) => a_name == b_name,
        (Type::Object(a_fields), Type::Object(b_fields)) => {
            a_fields.len() == b_fields.len()
                && a_fields.iter().zip(b_fields.iter()).all(|(af, bf)| {
                    af.name == bf.name && af.optional == bf.optional && type_matches(&af.ty, &bf.ty)
                })
        }
        (Type::Tuple(a_items), Type::Tuple(b_items)) => {
            a_items.len() == b_items.len()
                && a_items
                    .iter()
                    .zip(b_items.iter())
                    .all(|(a, b)| type_matches(a, b))
        }
        (Type::Union(a_variants), Type::Union(b_variants)) => {
            a_variants.len() == b_variants.len()
                && a_variants
                    .iter()
                    .all(|a| b_variants.iter().any(|b| type_matches(a, b)))
        }
        (
            Type::Function {
                params: ap,
                ret: ar,
            },
            Type::Function {
                params: bp,
                ret: br,
            },
        ) => {
            ap.len() == bp.len()
                && ap.iter().zip(bp.iter()).all(|(a, b)| type_matches(a, b))
                && type_matches(ar, br)
        }
        _ => false,
    }
}

fn snake_to_camel(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    let mut capitalize_next = false;
    for ch in name.chars() {
        if ch == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.extend(ch.to_uppercase());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }
    result
}

fn camel_to_snake(name: &str) -> String {
    let mut result = String::with_capacity(name.len() + 4);
    for ch in name.chars() {
        if ch.is_uppercase() {
            result.push('_');
            for lower in ch.to_lowercase() {
                result.push(lower);
            }
        } else {
            result.push(ch);
        }
    }
    result
}

fn literal_underlying(lit: &Expr) -> Option<Type> {
    match lit.kind() {
        ExprKind::StrLit(_) => Some(Type::Str),
        ExprKind::BoolLit(_) => Some(Type::Bool),
        ExprKind::IntLit(_) => Some(Type::I32),
        ExprKind::FloatLit(_) => Some(Type::F64),
        _ => None,
    }
}

fn union_from(members: Vec<Type>) -> Type {
    let mut result: Vec<Type> = Vec::new();
    let push = |ty: Type, result: &mut Vec<Type>| {
        if !matches!(ty, Type::Never) && !result.contains(&ty) {
            result.push(ty);
        }
    };
    for member in members {
        match member {
            Type::Union(inner) => inner.into_iter().for_each(|ty| push(ty, &mut result)),
            other => push(other, &mut result),
        }
    }
    match result.len() {
        0 => Type::Never,
        1 => result.pop().unwrap(),
        _ => Type::Union(result),
    }
}

fn bind(type_params: &[TypeParam], args: &[Type]) -> HashMap<String, Type> {
    type_params
        .iter()
        .zip(args)
        .map(|(param, arg)| (param.name.clone(), arg.clone()))
        .collect()
}

fn merge_field(
    out: &mut Vec<Field>,
    field: Field,
    origin: FieldMergeOrigin,
    diags: &mut Vec<ResolveDiag>,
) {
    if let Some(existing) = out.iter_mut().find(|f| f.name == field.name) {
        if existing.ty != field.ty {
            diags.push(ResolveDiag {
                code: origin.conflict_code(),
                message: format!(
                    "field `{}` has conflicting types `{}` and `{}` {}",
                    field.name,
                    existing.ty,
                    field.ty,
                    origin.conflict_context()
                ),
                subject: Some(field.name.clone()),
            });
        }
        match origin {
            // Inheritance is directional: the child refines what the parent
            // declared, so the last field wins.
            FieldMergeOrigin::Extends => *existing = field,
            // An intersection is symmetric, so the modifiers have to combine the
            // same way whichever side is written first. A value of `A & B` has to
            // satisfy both, so the field is required as soon as one side requires
            // it, and read-only as soon as one side freezes it.
            FieldMergeOrigin::Intersection => {
                *existing = Field {
                    optional: existing.optional && field.optional,
                    readonly: existing.readonly || field.readonly,
                    ..field
                };
            }
        }
    } else {
        out.push(field);
    }
}

pub fn substitute(ty: &Type, map: &HashMap<String, Type>) -> Type {
    if map.is_empty() {
        return ty.clone();
    }
    match ty {
        Type::Generic(name) | Type::Named(name) | Type::InferVar(name) => {
            map.get(name).cloned().unwrap_or_else(|| ty.clone())
        }
        Type::Array(inner) => Type::Array(Box::new(substitute(inner, map))),
        Type::Map(key, value) => Type::Map(
            Box::new(substitute(key, map)),
            Box::new(substitute(value, map)),
        ),
        Type::Set(value) => Type::Set(Box::new(substitute(value, map))),
        Type::BinaryHeap(value) => Type::BinaryHeap(Box::new(substitute(value, map))),
        Type::BinomialHeap(value) => Type::BinomialHeap(Box::new(substitute(value, map))),
        Type::UnionFind(value) => Type::UnionFind(Box::new(substitute(value, map))),
        Type::Counter(value) => Type::Counter(Box::new(substitute(value, map))),
        Type::Deque(value) => Type::Deque(Box::new(substitute(value, map))),
        Type::LinkedList(value) => Type::LinkedList(Box::new(substitute(value, map))),
        Type::DoubleLinkedList(value) => Type::DoubleLinkedList(Box::new(substitute(value, map))),
        Type::Readonly(inner) => Type::Readonly(Box::new(substitute(inner, map))),
        Type::Ref(inner) => Type::Ref(Box::new(substitute(inner, map))),
        Type::MutRef(inner) => Type::MutRef(Box::new(substitute(inner, map))),
        Type::Optional(inner) => Type::Optional(Box::new(substitute(inner, map))),
        Type::Result { success, error } => Type::Result {
            success: Box::new(substitute(success, map)),
            error: Box::new(substitute(error, map)),
        },
        Type::KeyOf(inner) => Type::KeyOf(Box::new(substitute(inner, map))),
        Type::Tuple(elements) => Type::Tuple(elements.iter().map(|e| substitute(e, map)).collect()),
        Type::Union(variants) => Type::Union(variants.iter().map(|v| substitute(v, map)).collect()),
        Type::Apply { name, args } => Type::Apply {
            name: name.clone(),
            args: args.iter().map(|a| substitute(a, map)).collect(),
        },
        Type::Function { params, ret } => Type::Function {
            params: params.iter().map(|p| substitute(p, map)).collect(),
            ret: Box::new(substitute(ret, map)),
        },
        Type::Object(fields) => Type::Object(
            fields
                .iter()
                .map(|field| Field {
                    ty: substitute(&field.ty, map),
                    ..field.clone()
                })
                .collect(),
        ),
        Type::Conditional {
            check,
            bound,
            then,
            otherwise,
        } => Type::Conditional {
            check: Box::new(substitute(check, map)),
            bound: Box::new(substitute(bound, map)),
            then: Box::new(substitute(then, map)),
            otherwise: Box::new(substitute(otherwise, map)),
        },
        _ => ty.clone(),
    }
}

pub fn normalize_program_with_imports(
    program: &mut Program,
    imported_aliases: &HashMap<String, Type>,
    imported_types: &HashMap<String, Type>,
) -> Vec<ResolveDiag> {
    let mut defs = GenericDefs::collect(program);
    if imported_types.contains_key("Page") {
        defs.register_db_page();
    }
    for (alias, ty) in imported_aliases {
        if let Type::Named(canonical) = ty {
            defs.register_stream_alias(alias, canonical);
        }
    }
    normalize_program_with(program, &defs)
}

pub fn normalize_program_for_backend(program: &mut Program) -> Vec<ResolveDiag> {
    let mut defs = GenericDefs::collect(program);
    defs.register_db_page();
    defs.lower_unknown = true;
    normalize_program_with(program, &defs)
}

fn normalize_program_with(program: &mut Program, defs: &GenericDefs) -> Vec<ResolveDiag> {
    let mut diags = Vec::new();
    for item in &mut program.items {
        normalize_item(item, defs, &mut diags);
    }
    diags
}

fn resolve_in_place(ty: &mut Type, defs: &GenericDefs, diags: &mut Vec<ResolveDiag>) {
    *ty = defs.resolve(ty, diags);
    if defs.lower_unknown {
        lower_unknown(ty);
    }
}

fn lower_unknown(ty: &mut Type) {
    match ty {
        Type::Unknown => *ty = Type::Union(crate::typeof_category::unknown_runtime_variants()),
        Type::Union(variants) => {
            if variants
                .iter()
                .any(|variant| matches!(variant, Type::Unknown))
            {
                *ty = Type::Union(crate::typeof_category::unknown_runtime_variants());
            } else {
                variants.iter_mut().for_each(lower_unknown);
            }
        }
        Type::Array(inner)
        | Type::Readonly(inner)
        | Type::Ref(inner)
        | Type::MutRef(inner)
        | Type::Optional(inner)
        | Type::Range(inner)
        | Type::KeyOf(inner)
        | Type::Set(inner)
        | Type::BinaryHeap(inner)
        | Type::BinomialHeap(inner)
        | Type::UnionFind(inner)
        | Type::Counter(inner)
        | Type::Deque(inner)
        | Type::LinkedList(inner)
        | Type::DoubleLinkedList(inner)
        | Type::AvlTree(inner)
        | Type::RedBlackTree(inner)
        | Type::SplayTree(inner)
        | Type::Treap(inner)
        | Type::SortedSet(inner)
        | Type::SkipList(inner) => lower_unknown(inner),
        Type::Map(first, second) | Type::SortedMap(first, second) | Type::Graph(first, second) => {
            lower_unknown(first);
            lower_unknown(second);
        }
        Type::Result { success, error } => {
            lower_unknown(success);
            lower_unknown(error);
        }
        Type::Tuple(elements) => elements.iter_mut().for_each(lower_unknown),
        Type::Throws { function, effects } => {
            lower_unknown(function);
            effects.iter_mut().for_each(lower_unknown);
        }
        Type::Function { params, ret } | Type::GenericFunction { params, ret, .. } => {
            params.iter_mut().for_each(lower_unknown);
            lower_unknown(ret);
        }
        Type::Object(fields) => fields
            .iter_mut()
            .for_each(|field| lower_unknown(&mut field.ty)),
        Type::Apply { args, .. } => args.iter_mut().for_each(lower_unknown),
        Type::Conditional {
            check,
            bound,
            then,
            otherwise,
        } => {
            lower_unknown(check);
            lower_unknown(bound);
            lower_unknown(then);
            lower_unknown(otherwise);
        }
        Type::Infer
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
        | Type::Char
        | Type::Regex
        | Type::Str
        | Type::Void
        | Type::Named(_)
        | Type::Generic(_)
        | Type::Never
        | Type::Lit(_)
        | Type::TypeQuery(_)
        | Type::InferVar(_)
        | Type::FenwickTree
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
        | Type::Centroid => {}
    }
}

fn normalize_type_params(
    params: &mut [TypeParam],
    defs: &GenericDefs,
    diags: &mut Vec<ResolveDiag>,
) {
    for param in params {
        if let Some(constraint) = &mut param.constraint {
            resolve_in_place(constraint, defs, diags);
        }
    }
}

fn normalize_params(params: &mut [Param], defs: &GenericDefs, diags: &mut Vec<ResolveDiag>) {
    for param in params {
        resolve_in_place(&mut param.ty, defs, diags);
        if let Some(default) = &mut param.default_value {
            normalize_expr(default, defs, diags);
        }
    }
}

fn normalize_block(body: &mut [Stmt], defs: &GenericDefs, diags: &mut Vec<ResolveDiag>) {
    for stmt in body {
        normalize_stmt(stmt, defs, diags);
    }
}

fn normalize_item(item: &mut Item, defs: &GenericDefs, diags: &mut Vec<ResolveDiag>) {
    match item {
        Item::InterfaceDecl {
            type_params,
            extends,
            fields,
            ..
        } => {
            normalize_type_params(type_params, defs, diags);
            for supertype in extends.iter_mut() {
                resolve_in_place(supertype, defs, diags);
            }
            for field in fields {
                resolve_in_place(&mut field.ty, defs, diags);
            }
        }
        Item::BehaviorInterfaceDecl {
            type_params,
            targets,
            extends,
            methods,
            ..
        } => {
            normalize_type_params(type_params, defs, diags);
            for target in targets {
                resolve_in_place(target, defs, diags);
            }
            for supertype in extends {
                resolve_in_place(supertype, defs, diags);
            }
            for method in methods {
                normalize_type_params(&mut method.type_params, defs, diags);
                normalize_params(&mut method.params, defs, diags);
                resolve_in_place(&mut method.ret_type, defs, diags);
                for ty in &mut method.throws {
                    resolve_in_place(ty, defs, diags);
                }
                normalize_block(&mut method.body, defs, diags);
            }
        }
        Item::TypeAlias {
            type_params, ty, ..
        } => {
            normalize_type_params(type_params, defs, diags);
            resolve_in_place(ty, defs, diags);
        }
        Item::EnumDecl { repr, .. } => resolve_in_place(repr, defs, diags),
        Item::FnDecl {
            type_params,
            params,
            ret_type,
            throws,
            body,
            ..
        } => {
            normalize_type_params(type_params, defs, diags);
            normalize_params(params, defs, diags);
            resolve_in_place(ret_type, defs, diags);
            for ty in throws {
                resolve_in_place(ty, defs, diags);
            }
            normalize_block(body, defs, diags);
        }
        Item::Stmt(stmt) => normalize_stmt(stmt, defs, diags),
    }
}

fn normalize_stmt(stmt: &mut Stmt, defs: &GenericDefs, diags: &mut Vec<ResolveDiag>) {
    match &mut stmt.kind {
        StmtKind::VarDecl { ty, value, .. } => {
            resolve_in_place(ty, defs, diags);
            if let Some(value) = value {
                normalize_expr(value, defs, diags);
            }
        }
        StmtKind::ArrayDestructure {
            bindings, value, ..
        }
        | StmtKind::ObjectDestructure {
            bindings, value, ..
        } => {
            for binding in bindings {
                resolve_in_place(&mut binding.ty, defs, diags);
            }
            normalize_expr(value, defs, diags);
        }
        StmtKind::Assign { value, .. } => normalize_expr(value, defs, diags),
        StmtKind::IndexAssign { index, value, .. } => {
            normalize_expr(index, defs, diags);
            normalize_expr(value, defs, diags);
        }
        StmtKind::FieldAssign { object, value, .. } => {
            normalize_expr(object, defs, diags);
            normalize_expr(value, defs, diags);
        }
        StmtKind::Return(Some(expr)) | StmtKind::Throw(expr) | StmtKind::Expr(expr) => {
            normalize_expr(expr, defs, diags);
        }
        StmtKind::Return(None) | StmtKind::Continue => {}
        StmtKind::Break(Some(expr)) => normalize_expr(expr, defs, diags),
        StmtKind::Break(None) => {}
        StmtKind::TryCatch {
            try_body,
            catch_type,
            catch_type_span: _,
            catch_body,
            finally_body,
            ..
        } => {
            normalize_block(try_body, defs, diags);
            resolve_in_place(catch_type, defs, diags);
            normalize_block(catch_body, defs, diags);
            if let Some(finally_body) = finally_body {
                normalize_block(finally_body, defs, diags);
            }
        }
        StmtKind::Defer { body, .. } => normalize_block(body, defs, diags),
        StmtKind::While { cond, body } | StmtKind::DoWhile { body, cond } => {
            normalize_expr(cond, defs, diags);
            normalize_block(body, defs, diags);
        }
        StmtKind::For {
            init,
            cond,
            update,
            body,
        } => {
            normalize_stmt(init, defs, diags);
            normalize_expr(cond, defs, diags);
            normalize_stmt(update, defs, diags);
            normalize_block(body, defs, diags);
        }
        StmtKind::ForOf { iterable, body, .. } => {
            normalize_expr(iterable, defs, diags);
            normalize_block(body, defs, diags);
        }
        StmtKind::ForIn { object, body, .. } => {
            normalize_expr(object, defs, diags);
            normalize_block(body, defs, diags);
        }
        StmtKind::Switch {
            expr,
            cases,
            default,
            ..
        } => {
            normalize_expr(expr, defs, diags);
            for case in cases {
                normalize_expr(&mut case.value, defs, diags);
                normalize_block(&mut case.body, defs, diags);
            }
            if let Some(default) = default {
                normalize_block(default, defs, diags);
            }
        }
        StmtKind::Match {
            expr,
            arms,
            default,
        } => {
            normalize_expr(expr, defs, diags);
            for arm in arms {
                if let Some(guard) = &mut arm.guard {
                    normalize_expr(guard, defs, diags);
                }
                normalize_block(&mut arm.body, defs, diags);
            }
            if let Some(default) = default {
                normalize_block(default, defs, diags);
            }
        }
    }
}

fn normalize_expr(expr: &mut Expr, defs: &GenericDefs, diags: &mut Vec<ResolveDiag>) {
    match &mut expr.kind {
        ExprKind::TypeAssert { value, ty } => {
            resolve_in_place(ty, defs, diags);
            normalize_expr(value, defs, diags);
        }
        ExprKind::Is { expr, ty } => {
            resolve_in_place(ty, defs, diags);
            normalize_expr(expr, defs, diags);
        }
        ExprKind::CallValue {
            callee,
            type_args,
            args,
        } => {
            normalize_expr(callee, defs, diags);
            for ty in type_args {
                resolve_in_place(ty, defs, diags);
            }
            for arg in args {
                normalize_expr(arg, defs, diags);
            }
        }
        ExprKind::FnCall {
            type_args, args, ..
        } => {
            for ty in type_args {
                resolve_in_place(ty, defs, diags);
            }
            for arg in args {
                normalize_expr(arg, defs, diags);
            }
        }
        ExprKind::ArrowFn {
            is_async: _,
            type_params,
            params,
            ret_type,
            throws,
            body,
        } => {
            normalize_type_params(type_params, defs, diags);
            normalize_params(params, defs, diags);
            resolve_in_place(ret_type, defs, diags);
            for ty in throws {
                resolve_in_place(ty, defs, diags);
            }
            normalize_block(body, defs, diags);
        }
        ExprKind::ObjectLit(fields) => {
            for (_, value) in fields {
                normalize_expr(value, defs, diags);
            }
        }
        ExprKind::ObjectUpdate { base, fields } => {
            normalize_expr(base, defs, diags);
            for (_, value) in fields {
                normalize_expr(value, defs, diags);
            }
        }
        ExprKind::ArrayLit(values) => {
            for value in values {
                normalize_expr(value, defs, diags);
            }
        }
        ExprKind::ArrayRepeat { value, count } => {
            normalize_expr(value, defs, diags);
            normalize_expr(count, defs, diags);
        }
        ExprKind::TemplateLit(parts) => {
            for part in parts {
                if let TemplatePart::Expr(expr, _) = part {
                    normalize_expr(expr, defs, diags);
                }
            }
        }
        ExprKind::MethodCall { object, args, .. }
        | ExprKind::OptionalMethodCall { object, args, .. } => {
            normalize_expr(object, defs, diags);
            for arg in args {
                normalize_expr(arg, defs, diags);
            }
        }
        ExprKind::FieldAccess { object, .. } | ExprKind::OptionalFieldAccess { object, .. } => {
            normalize_expr(object, defs, diags);
        }
        ExprKind::IndexAccess { object, index } => {
            normalize_expr(object, defs, diags);
            normalize_expr(index, defs, diags);
        }
        ExprKind::BinOp { left, right, .. } | ExprKind::Pipeline { left, right } => {
            normalize_expr(left, defs, diags);
            normalize_expr(right, defs, diags);
        }
        ExprKind::ChainedComparison { terms, .. } => {
            for term in terms {
                normalize_expr(term, defs, diags);
            }
        }
        ExprKind::Ternary {
            cond,
            then_expr,
            else_expr,
        } => {
            normalize_expr(cond, defs, diags);
            normalize_expr(then_expr, defs, diags);
            normalize_expr(else_expr, defs, diags);
        }
        ExprKind::If {
            cond,
            then_block,
            else_block,
        } => {
            normalize_expr(cond, defs, diags);
            normalize_block(then_block, defs, diags);
            if let Some(else_block) = else_block {
                normalize_block(else_block, defs, diags);
            }
        }
        ExprKind::Loop { body } => normalize_block(body, defs, diags),
        ExprKind::In { key, object } => {
            normalize_expr(key, defs, diags);
            normalize_expr(object, defs, diags);
        }
        ExprKind::Range { start, end, .. } => {
            normalize_expr(start, defs, diags);
            normalize_expr(end, defs, diags);
        }
        ExprKind::NewSet { ty, initial } => {
            if let Some(ty) = ty {
                resolve_in_place(ty, defs, diags);
            }
            if let Some(initial) = initial {
                normalize_expr(initial, defs, diags);
            }
        }
        ExprKind::NewMap {
            key,
            value,
            initial,
        } => {
            if let Some(key) = key {
                resolve_in_place(key, defs, diags);
            }
            if let Some(value) = value {
                resolve_in_place(value, defs, diags);
            }
            if let Some(initial) = initial {
                normalize_expr(initial, defs, diags);
            }
        }
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
        | ExprKind::ConstAssert(inner)
        | ExprKind::Comptime(inner) => normalize_expr(inner, defs, diags),
        ExprKind::StrLit(_)
        | ExprKind::RegexLit { .. }
        | ExprKind::IntLit(_)
        | ExprKind::UIntLit(_)
        | ExprKind::FloatLit(_)
        | ExprKind::DecimalLit(_)
        | ExprKind::BoolLit(_)
        | ExprKind::Null
        | ExprKind::None
        | ExprKind::Topic
        | ExprKind::Ident(_)
        | ExprKind::Borrow { .. } => {}
    }
}

#[cfg(test)]
#[path = "tests/tests.rs"]
mod tests;
