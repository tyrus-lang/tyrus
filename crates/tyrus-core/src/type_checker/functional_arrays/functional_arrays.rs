use super::*;
use crate::ast::ExprKind;

impl TypeChecker {
    pub(super) fn check_array_keyed_sort(
        &mut self,
        element: &Type,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        self.require_method_arity(method, args, 1, 2);
        let key = match args.first().map(super::super::ast::Expr::kind) {
            Some(ExprKind::StrLit(key)) => key.clone(),
            Some(argument) => {
                self.check_expr(&Expr {
                    kind: argument.clone(),
                    span: args[0].span,
                });
                self.error(
                    "E0308",
                    format!("`{method}` expects a string literal field name as its first argument"),
                    Some(method),
                );
                String::new()
            }
            None => String::new(),
        };
        let fields = match transparent_type(element) {
            Type::Named(name) => self.interfaces.get(name).cloned(),
            Type::Object(fields) => Some(fields.clone()),
            _ => None,
        };
        let field = fields
            .as_ref()
            .and_then(|fields| fields.iter().find(|field| field.name == key));
        let Some(field) = field else {
            self.error(
                "E0609",
                format!("`{method}` key `{key}` is not a field of `{element}`"),
                Some(method),
            );
            return ValueType::Known(Type::Array(Box::new(element.clone())));
        };
        if method == "sortedBy"
            && !fields.as_ref().is_some_and(|fields| {
                fields.iter().all(|field| {
                    self.resolve_type_annotation(&field_value_type(field))
                        .is_copy()
                })
            })
        {
            self.error(
                "E0277",
                "`sortedBy` currently requires an element type whose fields are all Copy; use `sortBy` for an in-place sort of owned values".to_string(),
                Some(method),
            );
        }
        let field_type = self.resolve_type_annotation(&field_value_type(field));
        if !is_keyed_sort_type(&field_type) {
            self.error(
                "E0277",
                format!("`{method}` cannot order field `{key}` of type `{field_type}`"),
                Some(method),
            );
        }
        if let Some(order) = args.get(1) {
            match order.kind() {
                ExprKind::StrLit(value) if value == "asc" || value == "desc" => {}
                _ => {
                    self.error(
                        "E0308",
                        format!("`{method}` order must be the string literal `asc` or `desc`"),
                        Some(method),
                    );
                }
            }
        }
        ValueType::Known(Type::Array(Box::new(element.clone())))
    }

    pub(super) fn check_array_keyed_aggregate(
        &mut self,
        element: &Type,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        self.require_method_arity(method, args, 1, 1);
        let key = match args.first().map(super::super::ast::Expr::kind) {
            Some(ExprKind::StrLit(key)) => key.clone(),
            Some(argument) => {
                self.check_expr(&Expr {
                    kind: argument.clone(),
                    span: args[0].span,
                });
                self.error(
                    "E0308",
                    format!("`{method}` expects a string literal field name as its argument"),
                    Some(method),
                );
                String::new()
            }
            None => String::new(),
        };
        let fields = match transparent_type(element) {
            Type::Named(name) => self.interfaces.get(name).cloned(),
            Type::Object(fields) => Some(fields.clone()),
            _ => None,
        };
        let field = fields
            .as_ref()
            .and_then(|fields| fields.iter().find(|field| field.name == key));
        let Some(field) = field else {
            self.error(
                "E0609",
                format!("`{method}` key `{key}` is not a field of `{element}`"),
                Some(method),
            );
            return ValueType::Known(if method == "avgBy" {
                Type::Optional(Box::new(Type::F64))
            } else {
                element.clone()
            });
        };
        let field_type = self.resolve_type_annotation(&field_value_type(field));
        if !is_numeric_type(&field_type) {
            self.error(
                "E0277",
                format!("`{method}` requires a numeric field, but `{key}` has type `{field_type}`"),
                Some(method),
            );
        }
        if method == "avgBy" {
            let average_type = if matches!(&field_type, Type::F128) {
                Type::F128
            } else {
                Type::F64
            };
            ValueType::Known(Type::Optional(Box::new(average_type)))
        } else {
            ValueType::Known(field_type)
        }
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn check_functional_array_method(
        &mut self,
        object: &Expr,
        element: &Type,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        let array = Type::Array(Box::new(element.clone()));
        let callback_parameters = vec![element.clone(), Type::Usize, array.clone()];
        match method {
            "map" | "flatMap" => {
                self.require_method_arity(method, args, 1, 1);
                let return_type = args.first().and_then(|callback| {
                    self.check_array_callback(callback, &callback_parameters, &Type::Infer, method)
                });
                let Some(return_type) = return_type else {
                    return ValueType::Error;
                };
                if method == "flatMap" {
                    match return_type {
                        Type::Array(inner) => ValueType::Known(Type::Array(inner)),
                        other => {
                            self.error(
                                "E0308",
                                format!("`flatMap` callback must return an array, found `{other}`"),
                                Some(method),
                            );
                            ValueType::Error
                        }
                    }
                } else {
                    ValueType::Known(Type::Array(Box::new(return_type)))
                }
            }
            "filter" => {
                self.require_method_arity(method, args, 1, 1);
                if let Some(callback) = args.first() {
                    let implicit_callback = self
                        .implicit_array_predicate(callback)
                        .unwrap_or_else(|| callback.clone());
                    self.check_array_callback(
                        &implicit_callback,
                        &callback_parameters,
                        &Type::Bool,
                        method,
                    );
                }
                ValueType::Known(array)
            }
            "partition" => {
                if matches!(
                    args.first().map(super::super::ast::Expr::kind),
                    Some(ExprKind::StrLit(_))
                ) {
                    self.require_method_arity(method, args, 1, usize::MAX);
                    let Some(field_type) = self.check_array_partition_field(element, method, args)
                    else {
                        return ValueType::Known(Type::Tuple(vec![array.clone(), array]));
                    };
                    if args.len() == 1 {
                        if !matches!(transparent_type(&field_type), Type::Bool) {
                            self.error(
                                "E0277",
                                format!("`partition` field must be `boolean` when no match values are provided, found `{field_type}`"),
                                Some(method),
                            );
                        }
                    } else {
                        if !matches!(transparent_type(&field_type), Type::Str) {
                            self.error(
                                "E0277",
                                format!("`partition` field `{}` must be `string` when match values are provided, found `{field_type}`", match args[0].kind() { ExprKind::StrLit(key) => key, _ => unreachable!() }),
                                Some(method),
                            );
                        }
                        for value in &args[1..] {
                            self.check_expr_as(value, &field_type);
                        }
                    }
                } else {
                    self.require_method_arity(method, args, 1, 1);
                    if let Some(callback) = args.first() {
                        let implicit_callback = self
                            .implicit_array_predicate(callback)
                            .unwrap_or_else(|| callback.clone());
                        self.check_array_callback(
                            &implicit_callback,
                            &callback_parameters,
                            &Type::Bool,
                            method,
                        );
                    }
                }
                if !self.partition_element_is_cloneable(element) {
                    self.error(
                        "E0277",
                        "`partition` requires Copy elements, `string`, or object elements whose fields are Copy or `string` so both output arrays can own their values safely".to_string(),
                        Some(method),
                    );
                }
                ValueType::Known(Type::Tuple(vec![array.clone(), array]))
            }
            "windows" => {
                self.require_method_arity(method, args, 1, 1);
                if let Some(size) = args.first() {
                    let size_type = self.check_expr_as(size, &Type::Usize);
                    if matches!(size.kind(), ExprKind::IntLit(value) if *value == 0) {
                        self.error(
                            "E0747",
                            "`windows` size must be greater than zero".to_string(),
                            Some(method),
                        );
                    }
                    let _ = size_type;
                }
                ValueType::Known(Type::Array(Box::new(array)))
            }
            "scan" => {
                self.require_method_arity(method, args, 2, 2);
                let accumulator = args
                    .first()
                    .map(|initial| self.check_expr(initial))
                    .and_then(|value| concrete_value_type(&value))
                    .unwrap_or(Type::Infer);
                if let Some(callback) = args.get(1) {
                    self.check_array_callback(
                        callback,
                        &[accumulator.clone(), element.clone()],
                        &accumulator,
                        method,
                    );
                }
                ValueType::Known(Type::Array(Box::new(accumulator)))
            }
            "unique" => {
                self.require_method_arity(method, args, 0, 0);
                if !self.resolve_type_annotation(element).is_copy()
                    && !matches!(transparent_type(element), Type::Str)
                {
                    self.error(
                        "E0277",
                        "`unique` currently requires Copy elements or `string`".to_string(),
                        Some(method),
                    );
                }
                ValueType::Known(array)
            }
            "uniqueBy" => {
                self.require_method_arity(method, args, 1, 1);
                let _key_type = self.check_array_hash_key(element, method, args);
                if !self.resolve_type_annotation(element).is_copy()
                    && !matches!(transparent_type(element), Type::Str)
                {
                    self.error(
                        "E0277",
                        "`uniqueBy` currently requires Copy elements or `string`".to_string(),
                        Some(method),
                    );
                }
                ValueType::Known(array)
            }
            "groupBy" => {
                self.require_method_arity(method, args, 1, 1);
                let key_type = self.check_array_hash_key(element, method, args);
                if !self.resolve_type_annotation(element).is_copy() {
                    self.error(
                        "E0277",
                        "`groupBy` currently requires Copy elements for ownership-safe groups"
                            .to_string(),
                        Some(method),
                    );
                }
                let key_type = key_type.unwrap_or(Type::Infer);
                ValueType::Known(Type::Map(Box::new(key_type), Box::new(array)))
            }
            "cartesianProduct" => {
                self.require_method_arity(method, args, 1, 1);
                let other = args
                    .first()
                    .map(|argument| self.check_expr(argument))
                    .and_then(|value| concrete_value_type(&value));
                let Some(Type::Array(other_element)) = other else {
                    self.error(
                        "E0277",
                        "`cartesianProduct` expects an array argument".to_string(),
                        Some(method),
                    );
                    return ValueType::Error;
                };
                if !self.resolve_type_annotation(element).is_copy()
                    || !self.resolve_type_annotation(&other_element).is_copy()
                {
                    self.error("E0277", "`cartesianProduct` currently requires Copy elements for ownership-safe tuple materialization".to_string(), Some(method));
                }
                ValueType::Known(Type::Array(Box::new(Type::Tuple(vec![
                    element.clone(),
                    *other_element,
                ]))))
            }
            "zip" => {
                self.require_method_arity(method, args, 1, 1);
                let other = args
                    .first()
                    .map(|argument| self.check_expr(argument))
                    .and_then(|value| concrete_value_type(&value));
                let Some(Type::Array(other_element)) = other else {
                    self.error(
                        "E0277",
                        "`zip` expects an array argument".to_string(),
                        Some(method),
                    );
                    return ValueType::Error;
                };
                self.check_unhandled_effect(
                    &Type::Named("ZipError".to_string()),
                    "call to `zip`",
                    object.span,
                    true,
                );
                ValueType::Known(Type::Array(Box::new(Type::Tuple(vec![
                    element.clone(),
                    *other_element,
                ]))))
            }
            "transpose" => {
                self.require_method_arity(method, args, 0, 0);
                let Type::Array(inner) = transparent_type(element) else {
                    self.error(
                        "E0277",
                        "`transpose` requires a two-dimensional array".to_string(),
                        Some(method),
                    );
                    return ValueType::Error;
                };
                ValueType::Known(Type::Array(Box::new(Type::Array(inner.clone()))))
            }
            "some" | "every" => {
                self.require_method_arity(method, args, 1, 1);
                if let Some(callback) = args.first() {
                    let implicit_callback = self
                        .implicit_array_predicate(callback)
                        .unwrap_or_else(|| callback.clone());
                    self.check_array_callback(
                        &implicit_callback,
                        &callback_parameters,
                        &Type::Bool,
                        method,
                    );
                }
                ValueType::Known(Type::Bool)
            }
            "find" | "findLast" | "findIndex" | "findLastIndex" => {
                self.require_method_arity(method, args, 1, 1);
                if let Some(callback) = args.first() {
                    let implicit_callback = self
                        .implicit_array_predicate(callback)
                        .unwrap_or_else(|| callback.clone());
                    self.check_array_callback(
                        &implicit_callback,
                        &callback_parameters,
                        &Type::Bool,
                        method,
                    );
                }
                if matches!(method, "findIndex" | "findLastIndex") {
                    ValueType::Known(Type::I64)
                } else {
                    ValueType::Known(Type::Optional(Box::new(element.clone())))
                }
            }
            "forEach" => {
                self.require_method_arity(method, args, 1, 1);
                if let Some(callback) = args.first() {
                    self.check_array_callback(callback, &callback_parameters, &Type::Void, method);
                }
                ValueType::Known(Type::Void)
            }
            "reduce" | "reduceRight" => {
                self.require_method_arity(method, args, 1, 2);
                let accumulator = args
                    .get(1)
                    .map(|initial| self.check_expr(initial))
                    .and_then(|value| concrete_value_type(&value))
                    .unwrap_or_else(|| element.clone());
                if let Some(callback) = args.first() {
                    self.check_array_callback(
                        callback,
                        &[accumulator.clone(), element.clone(), Type::Usize, array],
                        &accumulator,
                        method,
                    );
                }
                ValueType::Known(accumulator)
            }
            "sort" | "toSorted" => {
                self.require_method_arity(method, args, 0, 1);
                if method == "sort"
                    && let ExprKind::Ident(name) = object.kind()
                    && self.lookup(name).is_some_and(|symbol| !symbol.mutable)
                {
                    self.error(
                        "E0384",
                        format!("cannot call mutating method `sort` on immutable array `{name}`"),
                        Some(name),
                    );
                }
                if let Some(callback) = args.first() {
                    self.check_array_callback(
                        callback,
                        &[element.clone(), element.clone()],
                        &Type::I32,
                        method,
                    );
                }
                ValueType::Known(array)
            }
            "flat" => {
                self.require_method_arity(method, args, 0, 1);
                let depth = match args.first() {
                    None => 1,
                    Some(Expr {
                        kind: ExprKind::IntLit(depth),
                        ..
                    }) if *depth >= 0 => *depth as usize,
                    Some(depth) => {
                        self.check_expr(depth);
                        self.error(
                            "E0747",
                            "`flat` depth must be a non-negative integer literal so the native result type is known"
                                .to_string(),
                            Some(method),
                        );
                        1
                    }
                };
                ValueType::Known(Type::Array(Box::new(flatten_array_type(element, depth))))
            }
            _ => unreachable!(),
        }
    }

    pub(super) fn check_array_callback(
        &mut self,
        callback: &Expr,
        parameters: &[Type],
        return_type: &Type,
        method: &str,
    ) -> Option<Type> {
        let expected = Type::Function {
            params: parameters.to_vec(),
            ret: Box::new(return_type.clone()),
        };
        let actual = self.check_expr_as(callback, &expected);
        let ValueType::Known(Type::Function {
            params: actual_parameters,
            ret,
        }) = actual
        else {
            self.error(
                "E0277",
                format!("`{method}` expects a callback function"),
                Some(method),
            );
            return None;
        };
        if actual_parameters.len() > parameters.len() {
            self.error(
                "E0057",
                format!(
                    "`{method}` callback accepts at most {} parameter(s), found {}",
                    parameters.len(),
                    actual_parameters.len()
                ),
                Some(method),
            );
        }
        for (index, (actual, expected)) in actual_parameters.iter().zip(parameters).enumerate() {
            let subject = callback_parameter_subject(method, callback, index);
            self.require_assignable(
                expected,
                &ValueType::Known(actual.clone()),
                &subject,
                "E0308",
            );
        }
        if !matches!(return_type, Type::Infer) {
            self.require_assignable(
                return_type,
                &ValueType::Known((*ret).clone()),
                method,
                "E0308",
            );
        }
        Some(*ret)
    }

    fn implicit_array_predicate(&self, expression: &Expr) -> Option<Expr> {
        let mut free: Vec<String> = crate::ast::expr_referenced_identifiers(expression)
            .into_iter()
            .filter(|name| self.lookup(name).is_none())
            .collect();
        free.sort();
        (free.len() == 1).then(|| implicit_array_predicate(&free[0], expression.clone()))
    }

    pub(super) fn require_method_arity(
        &mut self,
        method: &str,
        args: &[Expr],
        minimum: usize,
        maximum: usize,
    ) {
        if args.len() < minimum || args.len() > maximum {
            self.error(
                "E0061",
                format!(
                    "method `{method}` expects {minimum} to {maximum} argument(s), but {} were provided",
                    args.len()
                ),
                Some(method),
            );
        }
    }
}

impl TypeChecker {
    fn partition_element_is_cloneable(&mut self, element: &Type) -> bool {
        let resolved = self.resolve_type_annotation(element);
        if resolved.is_copy() || matches!(transparent_type(&resolved), Type::Str) {
            return true;
        }
        let fields = match transparent_type(&resolved) {
            Type::Named(name) => self.interfaces.get(name).cloned(),
            Type::Object(fields) => Some(fields.clone()),
            _ => None,
        };
        fields.is_some_and(|fields| {
            fields.iter().all(|field| {
                let field_type = self.resolve_type_annotation(&field_value_type(field));
                field_type.is_copy() || matches!(transparent_type(&field_type), Type::Str)
            })
        })
    }

    fn check_array_partition_field(
        &mut self,
        element: &Type,
        method: &str,
        args: &[Expr],
    ) -> Option<Type> {
        let ExprKind::StrLit(key) = args.first()?.kind() else {
            unreachable!()
        };
        let fields = match transparent_type(element) {
            Type::Named(name) => self.interfaces.get(name).cloned(),
            Type::Object(fields) => Some(fields.clone()),
            _ => None,
        };
        let Some(fields) = fields else {
            self.error(
                "E0277",
                format!("`{method}` requires object elements, found `{element}`"),
                Some(method),
            );
            return None;
        };
        let Some(field) = fields.iter().find(|field| field.name == *key) else {
            self.error(
                "E0609",
                format!("`{method}` key `{key}` is not a field of `{element}`"),
                Some(method),
            );
            return None;
        };
        Some(self.resolve_type_annotation(&field_value_type(field)))
    }

    fn check_array_hash_key(
        &mut self,
        element: &Type,
        method: &str,
        args: &[Expr],
    ) -> Option<Type> {
        let argument = args.first()?;
        if !matches!(argument.kind(), ExprKind::StrLit(_)) {
            let callback = self
                .implicit_array_predicate(argument)
                .unwrap_or_else(|| argument.clone());
            let array = Type::Array(Box::new(element.clone()));
            return self.check_array_callback(
                &callback,
                &[element.clone(), Type::Usize, array],
                &Type::Infer,
                method,
            );
        }
        let ExprKind::StrLit(key) = argument.kind() else {
            unreachable!()
        };
        let fields = match transparent_type(element) {
            Type::Named(name) => self.interfaces.get(name).cloned(),
            Type::Object(fields) => Some(fields.clone()),
            _ => None,
        };
        let Some(fields) = fields else {
            self.error(
                "E0277",
                format!("`{method}` requires object elements, found `{element}`"),
                Some(method),
            );
            return None;
        };
        let Some(field) = fields.iter().find(|field| field.name == *key) else {
            self.error(
                "E0609",
                format!("`{method}` key `{key}` is not a field of `{element}`"),
                Some(method),
            );
            return None;
        };
        let field_type = self.resolve_type_annotation(&field.ty);
        if !is_hashable_array_key(&field_type) {
            self.error(
                "E0277",
                format!("`{method}` field `{key}` must be hashable, found `{field_type}`"),
                Some(method),
            );
            return None;
        }
        Some(field_type)
    }

    pub(super) fn check_array_toggle(
        &mut self,
        element: &Type,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        let array = Type::Array(Box::new(element.clone()));
        match method {
            "toggle" => {
                self.require_method_arity(method, args, 1, 2);
                if let Some(item) = args.first() {
                    let actual = self.check_expr(item);
                    self.require_assignable(element, &actual, method, "E0308");
                }
                if let Some(key_arg) = args.get(1) {
                    let key = match key_arg.kind() {
                        ExprKind::StrLit(key) => key.clone(),
                        _ => {
                            self.check_expr(key_arg);
                            self.error(
                                "E0308",
                                "`toggle` key must be a string literal field name".to_string(),
                                Some(method),
                            );
                            String::new()
                        }
                    };
                    if !key.is_empty() {
                        let fields = match transparent_type(element) {
                            Type::Named(name) => self.interfaces.get(name).cloned(),
                            Type::Object(fields) => Some(fields.clone()),
                            _ => None,
                        };
                        let field = fields
                            .as_ref()
                            .and_then(|fields| fields.iter().find(|f| f.name == key));
                        if field.is_none() {
                            self.error(
                                "E0609",
                                format!("`toggle` key `{key}` is not a field of `{element}`"),
                                Some(method),
                            );
                        }
                    }
                }
                ValueType::Known(array)
            }
            "toggleAll" => {
                self.require_method_arity(method, args, 1, 1);
                if let Some(items) = args.first() {
                    let actual = self.check_expr(items);
                    self.require_assignable(&array, &actual, method, "E0308");
                }
                ValueType::Known(array)
            }
            _ => unreachable!(),
        }
    }
}

fn callback_parameter_subject(method: &str, callback: &Expr, index: usize) -> String {
    let name = match callback.kind() {
        ExprKind::ArrowFn { params, .. } => params
            .get(index)
            .and_then(|param| param.pattern.binding_names().first().copied())
            .unwrap_or(method),
        _ => method,
    };
    format!("callback-param:{method}:{index}:{name}")
}

fn implicit_array_predicate(free_name: &str, body: Expr) -> Expr {
    let param = Param {
        decorators: Vec::new(),
        pattern: ParamPattern::Name(free_name.to_string()),
        ty: Type::Infer,
        mutable: false,
        optional: false,
        default_value: None,
        rest: false,
        span: Span::DUMMY,
    };
    Expr::synthetic(ExprKind::ArrowFn {
        is_async: false,
        type_params: Vec::new(),
        params: vec![param],
        ret_type: Type::Infer,
        throws: Vec::new(),
        body: vec![Stmt::synthetic(StmtKind::Return(Some(body)))],
    })
}

pub fn is_functional_array_method(method: &str) -> bool {
    matches!(
        method,
        "map"
            | "filter"
            | "partition"
            | "windows"
            | "scan"
            | "unique"
            | "uniqueBy"
            | "groupBy"
            | "cartesianProduct"
            | "zip"
            | "transpose"
            | "some"
            | "every"
            | "find"
            | "findLast"
            | "findIndex"
            | "findLastIndex"
            | "forEach"
            | "reduce"
            | "reduceRight"
            | "sort"
            | "toSorted"
            | "flat"
            | "flatMap"
    )
}

fn is_hashable_array_key(ty: &Type) -> bool {
    matches!(
        transparent_type(ty),
        Type::Str
            | Type::Bool
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
    )
}

pub(super) fn flatten_array_type(element: &Type, depth: usize) -> Type {
    if depth == 0 {
        return element.clone();
    }
    match element {
        Type::Array(inner) => flatten_array_type(inner, depth - 1),
        other => other.clone(),
    }
}

fn is_keyed_sort_type(ty: &Type) -> bool {
    matches!(
        transparent_type(ty),
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
            | Type::Bool
            | Type::Char
            | Type::Str
    )
}
