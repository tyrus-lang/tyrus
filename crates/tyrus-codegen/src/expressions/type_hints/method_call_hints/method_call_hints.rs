use super::*;

impl Codegen<'_> {
    #[allow(clippy::too_many_lines)]
    pub(super) fn declared_method_call_type_hint(&self, expr: &Expr) -> Option<Type> {
        match expr.kind() {
            ExprKind::MethodCall {
                object,
                method,
                args,
                ..
            }
            | ExprKind::OptionalMethodCall {
                object,
                method,
                args,
                ..
            } => {
                if let Some(Type::Named(name)) = self.expr_type_hint(object) {
                    let fields = if name == "DigestStream" {
                        Some(tyrus_core::crypto::digest_stream_methods())
                    } else {
                        tyrus_core::crypto::crypto_namespace_interface_fields()
                            .into_iter()
                            .find(|(namespace, _)| namespace == &name)
                            .map(|(_, fields)| fields)
                    };
                    if let Some(returned) = fields
                        .and_then(|fields| fields.into_iter().find(|field| field.name == *method))
                        .and_then(|field| match field.ty {
                            Type::Function { ret, .. } => Some(*ret),
                            Type::GenericFunction { ret, .. } => Some(*ret),
                            _ => None,
                        })
                    {
                        return Some(self.resolve_type(&returned));
                    }
                    if let Some(returned) =
                        tyrus_core::encoding::encoding_namespace_interface_fields()
                            .into_iter()
                            .find(|(namespace, _)| namespace == &name)
                            .and_then(|(_, fields)| {
                                fields.into_iter().find(|field| field.name == *method)
                            })
                            .and_then(|field| match field.ty {
                                Type::Function { ret, .. } | Type::GenericFunction { ret, .. } => {
                                    Some(*ret)
                                }
                                _ => None,
                            })
                    {
                        return Some(self.resolve_type(&returned));
                    }
                }
                if method == "findOne"
                    && let ExprKind::Ident(model_name) = object.kind()
                {
                    let model_name = self
                        .db_models
                        .contains_key(model_name)
                        .then_some(model_name.as_str())
                        .or_else(|| {
                            self.db_models
                                .keys()
                                .find(|name| name.ends_with(&format!("_{model_name}")))
                                .map(String::as_str)
                        });
                    if let Some(model_name) = model_name {
                        return self
                            .db_find_one_result_type(&[Type::Named(model_name.to_string())]);
                    }
                }
                if method == "count"
                    && let ExprKind::Ident(model_name) = object.kind()
                {
                    let model_exists = self.db_models.contains_key(model_name)
                        || self
                            .db_models
                            .keys()
                            .any(|name| name.ends_with(&format!("_{model_name}")));
                    if model_exists {
                        return Some(self.db_count_result_type());
                    }
                }
                if method != "count"
                    && tyrus_core::db::model_method_target(method).is_some()
                    && matches!(object.kind(), ExprKind::Ident(name) if self.db_models.contains_key(name))
                {
                    let ExprKind::Ident(model_name) = object.kind() else {
                        return None;
                    };
                    return Some(Type::Result {
                        success: Box::new(
                            tyrus_core::db::model_mutation_result_type(method, model_name)
                                .unwrap_or(Type::Void),
                        ),
                        error: Box::new(Type::Named("DbError".to_string())),
                    });
                }
                if method == "findMany"
                    && let ExprKind::Ident(model_name) = object.kind()
                {
                    let model_name = self
                        .db_models
                        .contains_key(model_name)
                        .then_some(model_name.as_str())
                        .or_else(|| {
                            self.db_models
                                .keys()
                                .find(|name| name.ends_with(&format!("_{model_name}")))
                                .map(String::as_str)
                        });
                    if let Some(model_name) = model_name {
                        return self
                            .db_find_many_result_type(&[Type::Named(model_name.to_string())]);
                    }
                }
                if method == "fromArray"
                    && let ExprKind::Ident(type_name) = object.kind()
                {
                    let element = args
                        .first()
                        .and_then(|source| self.expr_type_hint(source))
                        .and_then(|source_ty| match transparent_codegen_type(&source_ty) {
                            Type::Array(element) => Some((**element).clone()),
                            _ => None,
                        })
                        .unwrap_or(Type::Unknown);
                    return Some(match type_name.as_str() {
                        "BitSet" | tyrus_core::algorithm::BIT_SET_NEW_TARGET => Type::BitSet,
                        "LinkedList" | tyrus_core::algorithm::LINKED_LIST_NEW_TARGET => {
                            Type::LinkedList(Box::new(element))
                        }
                        "DoubleLinkedList"
                        | tyrus_core::algorithm::DOUBLE_LINKED_LIST_NEW_TARGET => {
                            Type::DoubleLinkedList(Box::new(element))
                        }
                        _ => return None,
                    });
                }
                if method == "toString"
                    && args.is_empty()
                    && let Some(ty) = self.expr_type_hint(object)
                {
                    let scalar = transparent_codegen_type(&ty);
                    if matches!(scalar, Type::Str | Type::Bool)
                        || is_integer_type(scalar)
                        || is_float_type(scalar)
                    {
                        return Some(Type::Str);
                    }
                }
                let object_type = self.expr_type_hint(object)?;
                if method == "copy" && args.is_empty() {
                    return Some(object_type);
                }
                let (object_type, optional) = match object_type {
                    Type::Optional(inner) => (*inner, true),
                    other => (other, false),
                };
                if matches!(method.as_str(), "pick" | "omit")
                    && let (Some(fields), Some(keys)) = (
                        self.declared_fields(&object_type),
                        args.first().and_then(|argument| {
                            crate::methods::static_object_projection_keys(
                                argument,
                                self.expr_type_hint(argument),
                            )
                        }),
                    )
                {
                    let fields = fields
                        .into_iter()
                        .filter(|field| {
                            let selected = keys.iter().any(|key| key == &field.name);
                            if method == "pick" {
                                selected
                            } else {
                                !selected
                            }
                        })
                        .collect();
                    return Some(Type::Object(fields));
                }
                if let Type::Apply {
                    name,
                    args: type_args,
                } = &object_type
                {
                    if name == "HttpReply"
                        && matches!(
                            method.as_str(),
                            "setCookie"
                                | "clearCookie"
                                | "setCookies"
                                | "clearCookies"
                                | "header"
                                | "appendHeader"
                                | "removeHeader"
                                | "type"
                        )
                    {
                        return Some(object_type.clone());
                    }
                    if name == "HttpReply" && method == "status" {
                        let status = args.first().and_then(|arg| self.http_status_arg(arg));
                        let schema = status.and_then(|status| {
                            type_args
                                .first()
                                .and_then(|reply| http_reply_schema_for_status(reply, status))
                        });
                        if let (Some(status), Some(schema)) = (status, schema) {
                            return Some(Type::Apply {
                                name: "__HttpReplyStatus".to_string(),
                                args: vec![
                                    Type::Lit(Box::new(Expr::synthetic(ExprKind::IntLit(
                                        status as i128,
                                    )))),
                                    schema,
                                ],
                            });
                        }
                    }
                    if (name == "HttpReply" || name == "__HttpReplyStatus") && method == "send" {
                        return Some(Type::Void);
                    }
                }
                if let Type::Map(key, value) = transparent_codegen_type(&object_type) {
                    let (key, value) = (key.as_ref().clone(), value.as_ref().clone());
                    return match method.as_str() {
                        "get" => Some(Type::Optional(Box::new(value))),
                        "set" => Some(Type::Map(Box::new(key), Box::new(value))),
                        "getOrInsert" => Some(Type::MutRef(Box::new(value))),
                        "has" | "delete" => Some(Type::Bool),
                        "keys" => Some(Type::Array(Box::new(key))),
                        "values" => Some(Type::Array(Box::new(value))),
                        "entries" => Some(Type::Array(Box::new(Type::Tuple(vec![key, value])))),
                        _ => None,
                    };
                }
                if let Type::Set(element) = transparent_codegen_type(&object_type) {
                    let element = element.as_ref().clone();
                    return match method.as_str() {
                        "add" => Some(Type::Set(Box::new(element))),
                        "has" | "delete" => Some(Type::Bool),
                        "values" => Some(Type::Array(Box::new(element))),
                        _ => None,
                    };
                }
                if let Type::BinaryHeap(element) = transparent_codegen_type(&object_type) {
                    let element = element.as_ref().clone();
                    return match method.as_str() {
                        "pop" | "peek" => Some(Type::Optional(Box::new(element))),
                        "len" => Some(Type::Usize),
                        "isEmpty" => Some(Type::Bool),
                        "push" | "clear" => Some(Type::Void),
                        _ => None,
                    };
                }
                if let Type::BinomialHeap(element) = transparent_codegen_type(&object_type) {
                    let element = element.as_ref().clone();
                    return match method.as_str() {
                        "pop" | "peek" => Some(Type::Optional(Box::new(element))),
                        "len" => Some(Type::Usize),
                        "isEmpty" => Some(Type::Bool),
                        "push" | "clear" => Some(Type::Void),
                        _ => None,
                    };
                }
                if let Type::UnionFind(element) = transparent_codegen_type(&object_type) {
                    let element = element.as_ref().clone();
                    return match method.as_str() {
                        "find" | "setCount" => Some(element),
                        "union" | "connected" => Some(Type::Bool),
                        _ => None,
                    };
                }
                if let Type::Counter(element) = transparent_codegen_type(&object_type) {
                    let element = element.as_ref().clone();
                    return match method.as_str() {
                        "count" => Some(Type::I32),
                        "total" => Some(Type::I32),
                        "unique" => Some(Type::Usize),
                        "isEmpty" => Some(Type::Bool),
                        "add" | "remove" | "clear" => Some(Type::Void),
                        "mostCommon" => {
                            Some(Type::Array(Box::new(Type::Tuple(vec![element, Type::I32]))))
                        }
                        _ => None,
                    };
                }
                if let Type::Deque(element) = transparent_codegen_type(&object_type) {
                    let element = element.as_ref().clone();
                    return match method.as_str() {
                        "pushFront" | "pushBack" | "clear" => Some(Type::Void),
                        "popFront" | "popBack" => Some(Type::Optional(Box::new(element))),
                        "peekFront" | "peekBack" => Some(Type::Optional(Box::new(element))),
                        "len" => Some(Type::Usize),
                        "isEmpty" => Some(Type::Bool),
                        _ => None,
                    };
                }
                if let Type::FenwickTree = transparent_codegen_type(&object_type) {
                    return match method.as_str() {
                        "add" => Some(Type::Void),
                        "sum" => Some(Type::I32),
                        "len" => Some(Type::Usize),
                        "isEmpty" => Some(Type::Bool),
                        _ => None,
                    };
                }
                if let Type::BubbleSort = transparent_codegen_type(&object_type) {
                    return match method.as_str() {
                        "step" | "done" | "swapped" => Some(Type::Bool),
                        "run" | "values" => Some(Type::Array(Box::new(Type::I32))),
                        "left" | "right" | "pass" | "comparisons" | "swaps" => Some(Type::Usize),
                        "reset" => Some(Type::Void),
                        _ => None,
                    };
                }
                if let Type::SegmentTree = transparent_codegen_type(&object_type) {
                    return match method.as_str() {
                        "update" => Some(Type::Void),
                        "query" => Some(Type::I32),
                        "len" => Some(Type::Usize),
                        "isEmpty" => Some(Type::Bool),
                        _ => None,
                    };
                }
                if let Type::SegmentTreeLazy = transparent_codegen_type(&object_type) {
                    return match method.as_str() {
                        "updateRange" => Some(Type::Void),
                        "query" => Some(Type::I32),
                        "len" => Some(Type::Usize),
                        "isEmpty" => Some(Type::Bool),
                        _ => None,
                    };
                }
                if let Type::AvlTree(_)
                | Type::RedBlackTree(_)
                | Type::SplayTree(_)
                | Type::Treap(_) = transparent_codegen_type(&object_type)
                {
                    return match method.as_str() {
                        "insert" => Some(Type::Void),
                        "contains" => Some(Type::Bool),
                        "len" => Some(Type::Usize),
                        "isEmpty" => Some(Type::Bool),
                        _ => None,
                    };
                }
                if let Type::SortedSet(element) = transparent_codegen_type(&object_type) {
                    return match method.as_str() {
                        "insert" | "remove" | "contains" => Some(Type::Bool),
                        "min" | "max" => Some(Type::Optional(Box::new((**element).clone()))),
                        "len" => Some(Type::Usize),
                        "isEmpty" => Some(Type::Bool),
                        "clear" => Some(Type::Void),
                        _ => None,
                    };
                }
                if let Type::SkipList(element) = transparent_codegen_type(&object_type) {
                    return match method.as_str() {
                        "insert" | "remove" | "contains" => Some(Type::Bool),
                        "min" | "max" => Some(Type::Optional(Box::new((**element).clone()))),
                        "len" => Some(Type::Usize),
                        "isEmpty" => Some(Type::Bool),
                        "clear" => Some(Type::Void),
                        _ => None,
                    };
                }
                if let Type::SortedMap(key, value) = transparent_codegen_type(&object_type) {
                    return match method.as_str() {
                        "set" => Some(Type::Void),
                        "get" => Some(Type::Optional(Box::new((**value).clone()))),
                        "has" | "delete" => Some(Type::Bool),
                        "min" | "max" => Some(Type::Optional(Box::new((**key).clone()))),
                        "len" => Some(Type::Usize),
                        "isEmpty" => Some(Type::Bool),
                        "clear" => Some(Type::Void),
                        _ => None,
                    };
                }
                if let Type::Graph(node, weight) = transparent_codegen_type(&object_type) {
                    return match method.as_str() {
                        "addEdge" | "addNode" => Some(Type::Void),
                        "removeNode" | "removeEdge" | "hasNode" | "hasEdge" => Some(Type::Bool),
                        "edge" => Some(Type::Optional(Box::new((**weight).clone()))),
                        "neighbors" | "nodes" => Some(Type::Array(Box::new((**node).clone()))),
                        "degree" | "nodeCount" | "edgeCount" => Some(Type::Usize),
                        "isEmpty" => Some(Type::Bool),
                        "edges" => Some(Type::Array(Box::new(Type::Tuple(vec![
                            (**node).clone(),
                            (**node).clone(),
                            (**weight).clone(),
                        ])))),
                        "reverse" => Some(Type::Graph(
                            Box::new((**node).clone()),
                            Box::new((**weight).clone()),
                        )),
                        "mapEdges" => {
                            let new_weight = args
                                .first()
                                .and_then(|callback| {
                                    self.infer_callback_return_type(callback, &[(**weight).clone()])
                                })
                                .unwrap_or_else(|| (**weight).clone());
                            Some(Type::Graph(
                                Box::new((**node).clone()),
                                Box::new(new_weight),
                            ))
                        }
                        "mapNodes" => {
                            let new_node = args
                                .first()
                                .and_then(|callback| {
                                    self.infer_callback_return_type(callback, &[(**node).clone()])
                                })
                                .unwrap_or_else(|| (**node).clone());
                            Some(Type::Graph(
                                Box::new(new_node),
                                Box::new((**weight).clone()),
                            ))
                        }
                        "toDot" | "toString" => Some(Type::Str),
                        _ => None,
                    };
                }
                if let Type::SparseTable = transparent_codegen_type(&object_type) {
                    return match method.as_str() {
                        "query" => Some(Type::I32),
                        "len" => Some(Type::Usize),
                        "isEmpty" => Some(Type::Bool),
                        _ => None,
                    };
                }
                if let Type::BitSet = transparent_codegen_type(&object_type) {
                    return match method.as_str() {
                        "set" | "flip" | "clear" => Some(Type::Void),
                        "get" | "isSubset" | "isSuperset" => Some(Type::Bool),
                        "count" => Some(Type::Usize),
                        "isEmpty" => Some(Type::Bool),
                        "union" | "intersection" | "difference" | "complement" => {
                            Some(Type::BitSet)
                        }
                        _ => None,
                    };
                }
                if let Type::Trie = transparent_codegen_type(&object_type) {
                    return match method.as_str() {
                        "insert" | "clear" => Some(Type::Void),
                        "search" | "startsWith" | "isEmpty" => Some(Type::Bool),
                        "len" => Some(Type::Usize),
                        _ => None,
                    };
                }
                if let Type::Lca = transparent_codegen_type(&object_type) {
                    return match method.as_str() {
                        "query" => Some(Type::Usize),
                        _ => None,
                    };
                }
                if let Type::Hld = transparent_codegen_type(&object_type) {
                    return match method.as_str() {
                        "queryPath" => Some(Type::Array(Box::new(Type::Tuple(vec![
                            Type::Usize,
                            Type::Usize,
                        ])))),
                        "position" | "head" => Some(Type::Usize),
                        _ => None,
                    };
                }
                if let Type::Centroid = transparent_codegen_type(&object_type) {
                    return match method.as_str() {
                        "parentOf" => Some(Type::Optional(Box::new(Type::Usize))),
                        "depth" => Some(Type::Usize),
                        _ => None,
                    };
                }
                if let Type::SuffixTree = transparent_codegen_type(&object_type) {
                    return match method.as_str() {
                        "getSuffixArray" => Some(Type::Array(Box::new(Type::Usize))),
                        _ => None,
                    };
                }
                if let Type::BloomFilter = transparent_codegen_type(&object_type) {
                    return match method.as_str() {
                        "insert" => Some(Type::Void),
                        "contains" => Some(Type::Bool),
                        _ => None,
                    };
                }
                if let Type::LinkedList(element) | Type::DoubleLinkedList(element) =
                    transparent_codegen_type(&object_type)
                {
                    let element = element.as_ref().clone();
                    return match method.as_str() {
                        "pushFront" | "pushBack" | "clear" => Some(Type::Void),
                        "popFront" | "popBack" | "front" | "back" => {
                            Some(Type::Optional(Box::new(element)))
                        }
                        "len" => Some(Type::Usize),
                        "isEmpty" => Some(Type::Bool),
                        _ => None,
                    };
                }
                if let Type::Range(_) = transparent_codegen_type(&object_type) {
                    return match method.as_str() {
                        "contains" => Some(Type::Bool),
                        _ => None,
                    };
                }
                if let Type::Char = transparent_codegen_type(&object_type) {
                    return match method.as_str() {
                        "toUpperCase" | "toLowerCase" => Some(Type::Char),
                        "isDigit" | "isAlphabetic" | "isAlphanumeric" | "isWhitespace" => {
                            Some(Type::Bool)
                        }
                        "toString" => Some(Type::Str),
                        _ => None,
                    };
                }
                if let Type::Str = transparent_codegen_type(&object_type) {
                    let result = match method.as_str() {
                        "charAt" => Some(Type::Str),
                        "chars" => Some(Type::Array(Box::new(Type::Char))),
                        "graphemes" => Some(Type::Array(Box::new(Type::Str))),
                        "charCodeAt" => Some(Type::I32),
                        "indexOf" | "lastIndexOf" => Some(Type::I64),
                        "includes" | "startsWith" | "endsWith" => Some(Type::Bool),
                        "match" => Some(Type::Optional(Box::new(Type::Array(Box::new(Type::Str))))),
                        "search" => Some(Type::I32),
                        "slice" | "substring" => Some(Type::Str),
                        "trim" | "trimStart" | "trimEnd" => Some(Type::Str),
                        "toLowerCase" | "toUpperCase" => Some(Type::Str),
                        "split" => Some(Type::Array(Box::new(Type::Str))),
                        "matchAll" => Some(Type::Array(Box::new(Type::Array(Box::new(Type::Str))))),
                        "concat" => Some(Type::Str),
                        "replace" => Some(Type::Str),
                        "repeat" => Some(Type::Str),
                        "padStart" | "padEnd" => Some(Type::Str),
                        "count" => Some(Type::Usize),
                        _ => None,
                    };
                    return result.map(|ty| {
                        if optional {
                            match ty {
                                Type::Optional(_) => ty,
                                other => Type::Optional(Box::new(other)),
                            }
                        } else {
                            ty
                        }
                    });
                }
                if let Type::Regex = transparent_codegen_type(&object_type) {
                    let result = match method.as_str() {
                        "test" => Some(Type::Bool),
                        "exec" => Some(Type::Optional(Box::new(Type::Array(Box::new(Type::Str))))),
                        "execAll" => Some(Type::Optional(Box::new(Type::Array(Box::new(
                            Type::Array(Box::new(Type::Str)),
                        ))))),
                        "toString" => Some(Type::Str),
                        _ => None,
                    };
                    return result.map(|ty| {
                        if optional {
                            match ty {
                                Type::Optional(_) => ty,
                                other => Type::Optional(Box::new(other)),
                            }
                        } else {
                            ty
                        }
                    });
                }
                if let Type::Apply {
                    name,
                    args: promise_args,
                } = &object_type
                {
                    if name == "Promise" && promise_args.len() == 1 {
                        let payload = promise_args[0].clone();
                        let result = match method.as_str() {
                            "then" => self
                                .infer_callback_return_type(args.first()?, &[payload])
                                .map(|returned| match returned {
                                    Type::Apply { name, args }
                                        if name == "Promise" && args.len() == 1 =>
                                    {
                                        Type::Apply { name, args }
                                    }
                                    returned => Type::Apply {
                                        name: "Promise".to_string(),
                                        args: vec![returned],
                                    },
                                }),
                            "catch" => {
                                let recovered = self.infer_callback_return_type(
                                    args.first()?,
                                    &[Type::Named("Error".to_string())],
                                )?;
                                let recovered = match recovered {
                                    Type::Apply { name, args }
                                        if name == "Promise" && args.len() == 1 =>
                                    {
                                        args[0].clone()
                                    }
                                    recovered => recovered,
                                };
                                Some(Type::Apply {
                                    name: "Promise".to_string(),
                                    args: vec![if recovered == payload {
                                        payload
                                    } else {
                                        Type::Union(vec![payload, recovered])
                                    }],
                                })
                            }
                            "finally" => Some(Type::Apply {
                                name: "Promise".to_string(),
                                args: vec![payload],
                            }),
                            _ => None,
                        };
                        return result;
                    }
                    if name == "JoinHandle" && promise_args.len() == 1 && method == "join" {
                        return Some(Type::Apply {
                            name: "Promise".to_string(),
                            args: vec![promise_args[0].clone()],
                        });
                    }
                    if name == "Sender" && promise_args.len() == 1 {
                        return match method.as_str() {
                            "send" => Some(Type::Apply {
                                name: "Promise".to_string(),
                                args: vec![Type::Void],
                            }),
                            "trySend" => Some(Type::Bool),
                            _ => None,
                        };
                    }
                    if name == "Receiver" && promise_args.len() == 1 {
                        let payload = promise_args[0].clone();
                        return match method.as_str() {
                            "recv" => Some(Type::Apply {
                                name: "Promise".to_string(),
                                args: vec![Type::Optional(Box::new(payload))],
                            }),
                            "tryRecv" => Some(Type::Optional(Box::new(payload))),
                            "close" => Some(Type::Void),
                            _ => None,
                        };
                    }
                    if name == "Mutex" && promise_args.len() == 1 {
                        let guard = Type::Apply {
                            name: "MutexGuard".to_string(),
                            args: vec![promise_args[0].clone()],
                        };
                        return match method.as_str() {
                            "lock" => Some(Type::Apply {
                                name: "Promise".to_string(),
                                args: vec![guard],
                            }),
                            "tryLock" => Some(Type::Optional(Box::new(guard))),
                            _ => None,
                        };
                    }
                    if name == "RwLock" && promise_args.len() == 1 {
                        let read_guard = Type::Apply {
                            name: "RwReadGuard".to_string(),
                            args: vec![promise_args[0].clone()],
                        };
                        let write_guard = Type::Apply {
                            name: "RwWriteGuard".to_string(),
                            args: vec![promise_args[0].clone()],
                        };
                        return match method.as_str() {
                            "read" => Some(Type::Apply {
                                name: "Promise".to_string(),
                                args: vec![read_guard],
                            }),
                            "write" => Some(Type::Apply {
                                name: "Promise".to_string(),
                                args: vec![write_guard],
                            }),
                            "tryRead" => Some(Type::Optional(Box::new(read_guard))),
                            "tryWrite" => Some(Type::Optional(Box::new(write_guard))),
                            _ => None,
                        };
                    }
                }
                if matches!(transparent_codegen_type(&object_type), Type::Named(name) if name == "HttpRouter")
                {
                    if method == "listen" {
                        return Some(Type::Result {
                            success: Box::new(Type::Named("HttpServer".to_string())),
                            error: Box::new(Type::Named("HttpError".to_string())),
                        });
                    }
                    if matches!(
                        method.as_str(),
                        "onResponse"
                            | "cors"
                            | "csrf"
                            | "fetchPolicy"
                            | "rateLimit"
                            | "trustedProxies"
                            | "get"
                            | "post"
                            | "put"
                            | "patch"
                            | "delete"
                            | "head"
                            | "options"
                            | "connect"
                            | "query"
                            | "trace"
                            | "fallback"
                            | "ws"
                            | "docs"
                    ) {
                        return Some(Type::Result {
                            success: Box::new(Type::Void),
                            error: Box::new(Type::Named("HttpError".to_string())),
                        });
                    }
                }
                if matches!(transparent_codegen_type(&object_type), Type::Named(name) if name == "HttpServer")
                {
                    return match method.as_str() {
                        "close" => Some(Type::Apply {
                            name: "Promise".to_string(),
                            args: vec![Type::Void],
                        }),
                        "port" => Some(Type::I32),
                        _ => None,
                    };
                }
                if matches!(transparent_codegen_type(&object_type), Type::Named(name) if name == "WebSocket")
                {
                    let http_result_of = |success: Type| Type::Result {
                        success: Box::new(success),
                        error: Box::new(Type::Named("HttpError".to_string())),
                    };
                    let promise_of = |payload: Type| Type::Apply {
                        name: "Promise".to_string(),
                        args: vec![payload],
                    };
                    return match method.as_str() {
                        "stream" => Some(promise_of(http_result_of(Type::Apply {
                            name: "DuplexStream".to_string(),
                            args: vec![
                                Type::Array(Box::new(Type::U8)),
                                Type::Array(Box::new(Type::U8)),
                            ],
                        }))),
                        "receive" => Some(promise_of(http_result_of(Type::Optional(Box::new(
                            Type::Named("WebSocketMessage".to_string()),
                        ))))),
                        "sendText" | "sendBytes" | "close" => {
                            Some(promise_of(http_result_of(Type::Void)))
                        }
                        "protocol" => Some(Type::Str),
                        "closeCode" => Some(Type::Optional(Box::new(Type::I32))),
                        _ => None,
                    };
                }
                if matches!(transparent_codegen_type(&object_type), Type::Named(name) if name == "ChildProcess")
                {
                    return match method.as_str() {
                        "wait" => Some(Type::Apply {
                            name: "Promise".to_string(),
                            args: vec![Type::Result {
                                success: Box::new(Type::I32),
                                error: Box::new(Type::Named("ProcessError".to_string())),
                            }],
                        }),
                        "kill" => Some(Type::Apply {
                            name: "Promise".to_string(),
                            args: vec![Type::Result {
                                success: Box::new(Type::Void),
                                error: Box::new(Type::Named("ProcessError".to_string())),
                            }],
                        }),
                        _ => None,
                    };
                }
                if matches!(transparent_codegen_type(&object_type), Type::Named(name) if name == "TcpStream")
                {
                    let result = match method.as_str() {
                        "read" => Some(Type::Array(Box::new(Type::U8))),
                        "write" => Some(Type::U64),
                        "shutdown" => Some(Type::Void),
                        "stream" => Some(Type::Apply {
                            name: "DuplexStream".to_string(),
                            args: vec![
                                Type::Array(Box::new(Type::U8)),
                                Type::Array(Box::new(Type::U8)),
                            ],
                        }),
                        _ => None,
                    }?;
                    return Some(Type::Apply {
                        name: "Promise".to_string(),
                        args: vec![Type::Result {
                            success: Box::new(result),
                            error: Box::new(Type::Named("NetError".to_string())),
                        }],
                    });
                }
                if matches!(transparent_codegen_type(&object_type), Type::Named(name) if name == "TcpListener")
                    && method == "accept"
                {
                    return Some(Type::Apply {
                        name: "Promise".to_string(),
                        args: vec![Type::Result {
                            success: Box::new(Type::Named("TcpStream".to_string())),
                            error: Box::new(Type::Named("NetError".to_string())),
                        }],
                    });
                }
                if matches!(transparent_codegen_type(&object_type), Type::Named(name) if name == "UdpSocket")
                {
                    let result = match method.as_str() {
                        "sendTo" => Some(Type::U64),
                        "receiveFrom" => Some(Type::Named("UdpPacket".to_string())),
                        "setBroadcast" | "setTtl" => Some(Type::Void),
                        _ => None,
                    }?;
                    return Some(Type::Apply {
                        name: "Promise".to_string(),
                        args: vec![Type::Result {
                            success: Box::new(result),
                            error: Box::new(Type::Named("NetError".to_string())),
                        }],
                    });
                }
                if let Type::Apply { name, args } = transparent_codegen_type(&object_type) {
                    let promise_of = |payload: Type| {
                        Some(Type::Apply {
                            name: "Promise".to_string(),
                            args: vec![payload],
                        })
                    };
                    match (name.as_str(), args.as_slice(), method.as_str()) {
                        ("ReadableStream", [item], "read") => {
                            return promise_of(Type::Optional(Box::new(item.clone())));
                        }
                        ("ReadableStream", [_], "pipeTo" | "close")
                        | ("WritableStream", [_], "write" | "close")
                        | ("DuplexStream" | "TransformStream", [_, _], "close") => {
                            return promise_of(Type::Void);
                        }
                        _ => {}
                    }
                }
                if matches!(transparent_codegen_type(&object_type), Type::Named(name) if name == "Semaphore")
                {
                    let permit = Type::Named("SemaphorePermit".to_string());
                    return match method.as_str() {
                        "acquire" => Some(Type::Apply {
                            name: "Promise".to_string(),
                            args: vec![permit],
                        }),
                        "tryAcquire" => Some(Type::Optional(Box::new(permit))),
                        _ => None,
                    };
                }
                if let Type::Named(name) = transparent_codegen_type(&object_type) {
                    let method_return = tyrus_core::algorithm::algorithm_interface_fields()
                        .iter()
                        .find(|(interface, _)| interface == name)
                        .and_then(|(_, fields)| fields.iter().find(|field| field.name == *method))
                        .and_then(|field| match &field.ty {
                            Type::Function { ret, .. } => Some(self.resolve_type(ret)),
                            _ => None,
                        });
                    if let Some(returned) = method_return {
                        return Some(returned);
                    }
                }
                let element: Box<Type> = match transparent_codegen_type(&object_type) {
                    Type::Array(element) => element.clone(),
                    Type::Tuple(elements) => {
                        let resolved = elements
                            .iter()
                            .map(|element| self.resolve_type(element))
                            .collect::<Vec<_>>();
                        Box::new(
                            tyrus_core::type_checker::tuple_sequence(&resolved)
                                .element()?
                                .clone(),
                        )
                    }
                    _ => return None,
                };
                let result = match method.as_str() {
                    "map" => self
                        .infer_callback_return_type(
                            args.first()?,
                            &[
                                (*element).clone(),
                                Type::Usize,
                                Type::Array(element.clone()),
                            ],
                        )
                        .map(|result| Type::Array(Box::new(result))),
                    "flatMap" => self
                        .infer_callback_return_type(
                            args.first()?,
                            &[
                                (*element).clone(),
                                Type::Usize,
                                Type::Array(element.clone()),
                            ],
                        )
                        .and_then(|result| match result {
                            Type::Array(inner) => Some(Type::Array(inner)),
                            _ => None,
                        }),
                    "filter" | "sort" | "toSorted" | "sortedBy" | "unique" | "uniqueBy" => {
                        Some(Type::Array(element))
                    }
                    "partition" => Some(Type::Tuple(vec![
                        Type::Array(element.clone()),
                        Type::Array(element),
                    ])),
                    "windows" => Some(Type::Array(Box::new(Type::Array(element)))),
                    "scan" => self
                        .codegen_expr_type(args.first()?)
                        .map(|accumulator| Type::Array(Box::new(accumulator))),
                    "groupBy" => self
                        .infer_callback_return_type(args.first()?, &[(*element).clone()])
                        .map(|key| Type::Map(Box::new(key), Box::new(Type::Array(element)))),
                    "cartesianProduct" => {
                        match transparent_codegen_type(&self.expr_type_hint(args.first()?)?) {
                            Type::Array(other) => Some(Type::Array(Box::new(Type::Tuple(vec![
                                (*element).clone(),
                                (**other).clone(),
                            ])))),
                            _ => None,
                        }
                    }
                    "zip" => match transparent_codegen_type(&self.expr_type_hint(args.first()?)?) {
                        Type::Array(other) => Some(Type::Array(Box::new(Type::Tuple(vec![
                            (*element).clone(),
                            (**other).clone(),
                        ])))),
                        _ => None,
                    },
                    "transpose" => match transparent_codegen_type(element.as_ref()) {
                        Type::Array(inner) => {
                            Some(Type::Array(Box::new(Type::Array(inner.clone()))))
                        }
                        _ => None,
                    },
                    "some" | "every" => Some(Type::Bool),
                    "find" | "findLast" => Some(Type::Optional(element)),
                    "findIndex" | "findLastIndex" => Some(Type::I64),
                    "forEach" => Some(Type::Void),
                    "reduce" | "reduceRight" => args
                        .get(1)
                        .and_then(|initial| self.codegen_expr_type(initial))
                        .or_else(|| Some((*element).clone())),
                    "flat" => match *element {
                        Type::Array(inner) => Some(Type::Array(inner)),
                        other => Some(Type::Array(Box::new(other))),
                    },
                    "push" | "unshift" => Some(Type::Usize),
                    "pop" | "shift" => Some(Type::Optional(element)),
                    "splice" => Some(Type::Array(element)),
                    "indexOf" | "lastIndexOf" => Some(Type::I64),
                    "includes" | "equals" => Some(Type::Bool),
                    "count" => Some(Type::Usize),
                    "toggle" | "toggleAll" => Some(Type::Array(element)),
                    "reverse" | "slice" | "concat" | "fill" | "copyWithin" | "toReversed"
                    | "shuffle" | "sample" | "with" | "values" => Some(Type::Array(element)),
                    "splitAt" => Some(Type::Tuple(vec![
                        Type::Array(element.clone()),
                        Type::Array(element),
                    ])),
                    "at" => Some(Type::Optional(element)),
                    "keys" => Some(Type::Array(Box::new(Type::Usize))),
                    "entries" => Some(Type::Array(Box::new(Type::Tuple(vec![
                        Type::Usize,
                        *element,
                    ])))),
                    "join" => Some(Type::Str),
                    "first" | "last" => Some(Type::Optional(element)),
                    "random" => Some(*element),
                    "sum" | "product" => Some(*element),
                    "avg" => Some(Type::Optional(Box::new(
                        if matches!(transparent_codegen_type(element.as_ref()), Type::F128) {
                            Type::F128
                        } else {
                            Type::F64
                        },
                    ))),
                    "rotate" => Some(Type::Array(element)),
                    "sumBy" => None,
                    "avgBy" => Some(Type::Optional(Box::new(Type::F64))),
                    _ => None,
                };
                result.map(|ty| {
                    if optional {
                        match ty {
                            Type::Optional(_) => ty,
                            other => Type::Optional(Box::new(other)),
                        }
                    } else {
                        ty
                    }
                })
            }
            _ => None,
        }
    }
}
