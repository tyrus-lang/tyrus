use super::*;
use crate::lexer::lex;
use crate::parser::Parser;

fn defs(source: &str) -> (GenericDefs, Program) {
    let program = Parser::new(lex(source).unwrap()).parse_program().unwrap();
    (GenericDefs::collect(&program), program)
}

fn resolve_ok(defs: &GenericDefs, ty: &Type) -> Type {
    let mut diags = Vec::new();
    let resolved = defs.resolve(ty, &mut diags);
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    resolved
}

#[test]
fn readonly_reaches_past_the_first_level() {
    let (defs, _) = defs(
        "interface Inner { port: i32; }\ninterface Cfg { inner: Inner; tags: string[]; name: string; }",
    );
    let frozen = Type::Apply {
        name: "Readonly".into(),
        args: vec![Type::Named("Cfg".into())],
    };
    let Type::Object(fields) = resolve_ok(&defs, &frozen) else {
        panic!("expected an object");
    };
    assert!(fields.iter().all(|field| field.readonly));
    let field_type = |name: &str| {
        fields
            .iter()
            .find(|field| field.name == name)
            .unwrap_or_else(|| panic!("missing field `{name}`"))
            .ty
            .clone()
    };
    assert_eq!(
        field_type("inner"),
        Type::Readonly(Box::new(Type::Named("Inner".into()))),
        "a nested interface field must carry the guarantee into itself"
    );
    assert_eq!(
        field_type("tags"),
        Type::Readonly(Box::new(Type::Array(Box::new(Type::Str)))),
        "an array field must become a readonly array, so `push` is refused"
    );
    assert_eq!(field_type("name"), Type::Str);
}

#[test]
fn substitutes_generic_parameters() {
    let map = [("T".to_string(), Type::I32)].into_iter().collect();
    let ty = Type::Array(Box::new(Type::Generic("T".into())));
    assert_eq!(substitute(&ty, &map), Type::Array(Box::new(Type::I32)));
}

#[test]
fn resolves_generic_alias_and_interface_application() {
    let (defs, _) = defs("type Pair<A, B> = [A, B];\ninterface Box<T> { value: T; }");
    let pair = Type::Apply {
        name: "Pair".into(),
        args: vec![Type::I32, Type::Str],
    };
    assert_eq!(
        resolve_ok(&defs, &pair),
        Type::Tuple(vec![Type::I32, Type::Str])
    );
    let boxed = Type::Apply {
        name: "Box".into(),
        args: vec![Type::Str],
    };
    let Type::Object(fields) = resolve_ok(&defs, &boxed) else {
        panic!("expected object");
    };
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].name, "value");
    assert_eq!(fields[0].ty, Type::Str);
}

#[test]
fn resolves_builtin_generic_type_constructors() {
    let (defs, _) = defs("");
    let apply = |name: &str, args: Vec<Type>| Type::Apply {
        name: name.into(),
        args,
    };

    assert_eq!(
        resolve_ok(&defs, &apply("Tuple", vec![Type::Str, Type::I32])),
        Type::Tuple(vec![Type::Str, Type::I32])
    );
    assert_eq!(
        resolve_ok(&defs, &apply("Optional", vec![Type::I32])),
        Type::Optional(Box::new(Type::I32))
    );
    assert_eq!(
        resolve_ok(&defs, &apply("Range", vec![Type::I32])),
        Type::Range(Box::new(Type::I32))
    );
}

#[test]
fn resolves_keyof_to_a_literal_union() {
    let (defs, _) = defs("interface User { name: string; age: i32; }");
    let keys = resolve_ok(&defs, &Type::KeyOf(Box::new(Type::Named("User".into()))));
    let Type::Union(variants) = keys else {
        panic!("expected union");
    };
    let names: Vec<String> = variants
        .iter()
        .map(|v| match v {
            Type::Lit(expr) => match expr.as_ref() {
                Expr {
                    kind: ExprKind::StrLit(s),
                    ..
                } => s.clone(),
                _ => panic!("expected string literal"),
            },
            _ => panic!("expected literal type"),
        })
        .collect();
    assert_eq!(names, ["name", "age"]);
}

#[test]
fn resolves_keys_of_by_type_to_exact_field_matches() {
    let (defs, _) = defs("interface Product { id: i32; name: string; optional: string?; }");
    let keys = resolve_ok(
        &defs,
        &Type::Apply {
            name: "KeysOfByType".into(),
            args: vec![Type::Named("Product".into()), Type::Str],
        },
    );
    let Type::Union(variants) = keys else {
        panic!("expected union");
    };
    assert_eq!(variants.len(), 1);
    assert!(
        matches!(&variants[0], Type::Lit(expr) if matches!(&expr.kind, ExprKind::StrLit(name) if name == "name"))
    );
}

#[test]
fn merges_interface_inheritance() {
    let (defs, _) = defs(
        "interface User { name: string; }\n\
         interface Timestamps { createdAt: i64; }\n\
         interface Admin extends User, Timestamps { role: string; }",
    );
    let mut diags = Vec::new();
    let fields = defs.interface_fields("Admin", &mut diags).unwrap();
    assert!(diags.is_empty(), "unexpected: {diags:?}");
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["name", "createdAt", "role"]);
}

#[test]
fn merges_type_intersection_of_named_operands() {
    let (defs, _) = defs(
        "interface B { id: i32; }\n\
         interface C { name: string; }\n\
         type A = B & C;",
    );
    let mut diags = Vec::new();
    let fields = defs.interface_fields("A", &mut diags).unwrap();
    assert!(diags.is_empty(), "unexpected: {diags:?}");
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["id", "name"]);
}

#[test]
fn merges_type_intersection_with_inline_object_operands() {
    let (defs, _) = defs(
        "interface B { id: i32; }\n\
         interface C { age: i32; }\n\
         type A = { name: string; } & B & C;",
    );
    let mut diags = Vec::new();
    let fields = defs.interface_fields("A", &mut diags).unwrap();
    assert!(diags.is_empty(), "unexpected: {diags:?}");
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["name", "id", "age"]);
}

#[test]
fn intersection_modifiers_do_not_depend_on_operand_order() {
    let (defs, _) = defs(
        "interface B { tag?: string; }\n\
         interface C { tag: string; }\n\
         type A = B & C;\n\
         type D = C & B;",
    );
    for name in ["A", "D"] {
        let mut diags = Vec::new();
        let fields = defs.interface_fields(name, &mut diags).unwrap();
        assert!(diags.is_empty(), "{name}: {diags:?}");
        assert_eq!(fields.len(), 1);
        assert!(
            !fields[0].optional,
            "`{name}` must require `tag`, one side requires it"
        );
    }
}

#[test]
fn intersection_keeps_a_field_read_only_whichever_side_freezes_it() {
    let (defs, _) = defs(
        "interface B { readonly id: i32; }\n\
         interface C { id: i32; }\n\
         type A = B & C;\n\
         type D = C & B;",
    );
    for name in ["A", "D"] {
        let mut diags = Vec::new();
        let fields = defs.interface_fields(name, &mut diags).unwrap();
        assert!(diags.is_empty(), "{name}: {diags:?}");
        assert!(fields[0].readonly, "`{name}` must keep `id` read-only");
    }
}

#[test]
fn inheritance_still_lets_the_child_refine_an_optional_field() {
    let (defs, _) = defs(
        "interface Base { tag?: string; }\n\
         interface Child extends Base { tag: string; }",
    );
    let mut diags = Vec::new();
    let fields = defs.interface_fields("Child", &mut diags).unwrap();
    assert!(diags.is_empty(), "unexpected: {diags:?}");
    assert!(!fields[0].optional);
}

#[test]
fn intersection_accepts_a_field_repeated_with_the_same_type() {
    let (defs, _) = defs(
        "interface B { id: i32; name: string; }\n\
         interface C { id: i32; age: i32; }\n\
         type A = B & C;",
    );
    let mut diags = Vec::new();
    let fields = defs.interface_fields("A", &mut diags).unwrap();
    assert!(diags.is_empty(), "unexpected: {diags:?}");
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["id", "name", "age"]);
}

#[test]
fn intersection_accepts_an_object_alias_as_an_operand() {
    let (defs, _) = defs(
        "interface B { id: i32; }\n\
         type X = { name: string; };\n\
         type A = B & X;",
    );
    let mut diags = Vec::new();
    let fields = defs.interface_fields("A", &mut diags).unwrap();
    assert!(diags.is_empty(), "unexpected: {diags:?}");
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["id", "name"]);
}

#[test]
fn reports_a_union_or_array_intersection_operand() {
    let (defs, _) = defs(
        "interface B { id: i32; }\n\
         type U = i32 | string;\n\
         type A = B & U;\n\
         type D = B & i32[];",
    );
    for name in ["A", "D"] {
        let mut diags = Vec::new();
        defs.interface_fields(name, &mut diags);
        assert!(diags.iter().any(|d| d.code == "E3213"), "{name}: {diags:?}");
    }
}

#[test]
fn reports_intersection_field_conflict_with_its_own_code() {
    let (defs, _) = defs(
        "interface B { id: i32; }\n\
         interface C { id: string; }\n\
         type A = B & C;",
    );
    let mut diags = Vec::new();
    defs.interface_fields("A", &mut diags);
    let conflict = diags
        .iter()
        .find(|d| d.code == "E3214")
        .expect("expected E3214");
    assert_eq!(conflict.subject.as_deref(), Some("id"));
    assert!(conflict.message.contains("across the intersection"));
    assert!(
        !diags.iter().any(|d| d.code == "E3204"),
        "inheritance code must not leak into an intersection: {diags:?}"
    );
}

#[test]
fn reports_non_object_intersection_operand() {
    let (defs, _) = defs("interface B { id: i32; }\ntype A = B & i32;");
    let mut diags = Vec::new();
    defs.interface_fields("A", &mut diags);
    assert!(diags.iter().any(|d| d.code == "E3213"), "{diags:?}");
    assert!(!diags.iter().any(|d| d.code == "E3203"), "{diags:?}");
}

#[test]
fn reports_intersection_cycle_without_overflowing() {
    let (defs, _) = defs("interface B { id: i32; }\ntype A = A & B;");
    let mut diags = Vec::new();
    defs.interface_fields("A", &mut diags);
    let cycle = diags
        .iter()
        .find(|d| d.code == "E3216")
        .expect("expected E3216");
    assert_eq!(cycle.subject.as_deref(), Some("A"));
}

#[test]
fn reports_mutual_intersection_cycle_without_overflowing() {
    let (defs, _) = defs("type A = B & { x: i32; };\ntype B = A & { y: i32; };");
    let mut diags = Vec::new();
    defs.interface_fields("A", &mut diags);
    assert!(diags.iter().any(|d| d.code == "E3216"), "{diags:?}");
}

#[test]
fn reports_inheritance_cycle_without_overflowing() {
    let (defs, _) = defs("interface A extends A { id: i32; }");
    let mut diags = Vec::new();
    defs.interface_fields("A", &mut diags);
    let cycle = diags
        .iter()
        .find(|d| d.code == "E3205")
        .expect("expected E3205");
    assert_eq!(cycle.subject.as_deref(), Some("A"));
}

#[test]
fn reports_arity_unknown_and_conflict() {
    let (defs, _) = defs(
        "interface Box<T> { value: T; }\n\
         interface A { x: i32; }\n\
         interface B { x: string; }\n\
         interface C extends A, B {}",
    );
    let mut diags = Vec::new();
    defs.resolve(
        &Type::Apply {
            name: "Box".into(),
            args: vec![Type::I32, Type::Str],
        },
        &mut diags,
    );
    assert!(diags.iter().any(|d| d.code == "E3200"));
    let mut diags = Vec::new();
    defs.resolve(
        &Type::Apply {
            name: "Nope".into(),
            args: vec![Type::I32],
        },
        &mut diags,
    );
    assert!(diags.iter().any(|d| d.code == "E3202"));
    let mut diags = Vec::new();
    defs.interface_fields("C", &mut diags);
    assert!(diags.iter().any(|d| d.code == "E3204"));
}

#[test]
fn resolves_mapped_and_structural_utilities() {
    let (defs, _) = defs("interface User { name: string; age?: i32; }");
    let apply = |name: &str, args: Vec<Type>| Type::Apply {
        name: name.into(),
        args,
    };
    let user = || Type::Named("User".into());
    let lit = |s: &str| Type::Lit(Box::new(Expr::synthetic(ExprKind::StrLit(s.into()))));

    let Type::Object(fields) = resolve_ok(&defs, &apply("Partial", vec![user()])) else {
        panic!()
    };
    assert!(fields.iter().all(|f| f.optional));
    let Type::Object(fields) = resolve_ok(&defs, &apply("Required", vec![user()])) else {
        panic!()
    };
    assert!(fields.iter().all(|f| !f.optional));
    let Type::Object(fields) = resolve_ok(&defs, &apply("Readonly", vec![user()])) else {
        panic!()
    };
    assert!(fields.iter().all(|f| f.readonly));
    let Type::Object(fields) = resolve_ok(&defs, &apply("Pick", vec![user(), lit("name")])) else {
        panic!()
    };
    assert_eq!(
        fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
        ["name"]
    );
    let Type::Object(fields) = resolve_ok(&defs, &apply("Omit", vec![user(), lit("age")])) else {
        panic!()
    };
    assert_eq!(
        fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
        ["name"]
    );
    let Type::Object(fields) = resolve_ok(&defs, &apply("Optional", vec![user(), lit("name")]))
    else {
        panic!()
    };
    assert_eq!(
        fields
            .iter()
            .map(|field| (field.name.as_str(), field.optional))
            .collect::<Vec<_>>(),
        [("name", true), ("age", true)]
    );
    let Type::Object(fields) = resolve_ok(&defs, &apply("Optional", vec![user(), Type::Never]))
    else {
        panic!()
    };
    assert_eq!(
        fields
            .iter()
            .map(|field| (field.name.as_str(), field.optional))
            .collect::<Vec<_>>(),
        [("name", false), ("age", true)]
    );
    let keys = Type::Union(vec![lit("a"), lit("b")]);
    let Type::Object(fields) = resolve_ok(&defs, &apply("Record", vec![keys, Type::I32])) else {
        panic!()
    };
    assert_eq!(fields.len(), 2);
    assert!(fields.iter().all(|f| f.ty == Type::I32));
}

#[test]
fn canonical_object_projection_preserves_order_and_normalizes_keys() {
    let (defs, _) = defs("interface User { id: i32; name: string; email?: string; }");
    let fields = defs.interface_fields("User", &mut Vec::new()).unwrap();

    let picked = ObjectProjection::new(
        fields.clone(),
        vec!["name".into(), "id".into(), "name".into()],
        ObjectProjectionMode::Pick,
    )
    .apply();
    assert_eq!(
        picked
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>(),
        ["id", "name"]
    );

    let omitted = ObjectProjection::new(
        fields.clone(),
        vec!["name".into(), "name".into()],
        ObjectProjectionMode::Omit,
    )
    .apply();
    assert_eq!(
        omitted
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>(),
        ["id", "email"]
    );

    let optional = ObjectProjection::new(
        fields,
        vec!["id".into(), "email".into(), "id".into()],
        ObjectProjectionMode::Optional,
    )
    .apply();
    assert_eq!(
        optional
            .iter()
            .map(|field| (field.name.as_str(), field.optional))
            .collect::<Vec<_>>(),
        [("id", true), ("name", false), ("email", true)]
    );
}

#[test]
fn composes_key_projections_in_declaration_order() {
    let (defs, _) = defs("interface User { id: i32; name: string; email: string; }");
    let apply = |name: &str, args: Vec<Type>| Type::Apply {
        name: name.into(),
        args,
    };
    let lit = |name: &str| Type::Lit(Box::new(Expr::synthetic(ExprKind::StrLit(name.into()))));
    let source = apply("Omit", vec![Type::Named("User".into()), lit("name")]);
    let Type::Object(fields) = resolve_ok(&defs, &apply("Pick", vec![source, lit("email")])) else {
        panic!()
    };
    assert_eq!(
        fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>(),
        ["email"]
    );
}

#[test]
fn resolves_open_record_to_dynamic_map() {
    let (defs, _) = defs("");
    let apply = |name: &str, args: Vec<Type>| Type::Apply {
        name: name.into(),
        args,
    };
    let resolved = resolve_ok(&defs, &apply("Record", vec![Type::Str, Type::I32]));
    assert_eq!(
        resolved,
        Type::Map(Box::new(Type::Str), Box::new(Type::I32))
    );
}

#[test]
fn reports_utility_misuse() {
    let (defs, _) = defs("interface User { name: string; }");
    let apply = |name: &str, args: Vec<Type>| Type::Apply {
        name: name.into(),
        args,
    };
    let lit = |s: &str| Type::Lit(Box::new(Expr::synthetic(ExprKind::StrLit(s.into()))));
    let mut diags = Vec::new();
    defs.resolve(
        &apply("Pick", vec![Type::Named("User".into()), lit("missing")]),
        &mut diags,
    );
    assert!(diags.iter().any(|d| d.code == "E3212"));
    let mut diags = Vec::new();
    defs.resolve(&apply("Partial", vec![Type::I32]), &mut diags);
    assert!(diags.iter().any(|d| d.code == "E3211"));
    let mut diags = Vec::new();
    defs.resolve(&apply("Optional", vec![Type::I32, lit("name")]), &mut diags);
    assert!(diags.iter().any(|d| d.code == "E3211"));
    let mut diags = Vec::new();
    defs.resolve(
        &apply(
            "Optional",
            vec![Type::Named("User".into()), lit("name"), lit("age")],
        ),
        &mut diags,
    );
    assert!(diags.iter().any(|d| d.code == "E3200"));
}

#[test]
fn resolves_conditional_infer_and_function_utilities() {
    let (defs, _) =
        defs("type Fn = (i32) => string;\ntype Ret<T> = T extends (i32) => infer R ? R : never;");
    let apply = |name: &str, args: Vec<Type>| Type::Apply {
        name: name.into(),
        args,
    };
    let lit = |s: &str| Type::Lit(Box::new(Expr::synthetic(ExprKind::StrLit(s.into()))));
    let fn_alias = || Type::Named("Fn".into());

    assert_eq!(
        resolve_ok(&defs, &apply("ReturnType", vec![fn_alias()])),
        Type::Str
    );
    assert_eq!(
        resolve_ok(&defs, &apply("Parameters", vec![fn_alias()])),
        Type::Tuple(vec![Type::I32])
    );

    assert_eq!(
        resolve_ok(&defs, &apply("Ret", vec![fn_alias()])),
        Type::Str
    );

    let abc = Type::Union(vec![lit("a"), lit("b"), lit("c")]);
    assert_eq!(
        resolve_ok(&defs, &apply("Exclude", vec![abc.clone(), lit("b")])),
        Type::Union(vec![lit("a"), lit("c")])
    );
    assert_eq!(
        resolve_ok(
            &defs,
            &apply("Extract", vec![abc, Type::Union(vec![lit("b"), lit("z")])])
        ),
        lit("b")
    );

    let cond = Type::Conditional {
        check: Box::new(Type::Union(vec![lit("a"), lit("b")])),
        bound: Box::new(Type::Str),
        then: Box::new(Type::Lit(Box::new(Expr::synthetic(ExprKind::IntLit(1))))),
        otherwise: Box::new(Type::Lit(Box::new(Expr::synthetic(ExprKind::IntLit(0))))),
    };
    assert_eq!(
        resolve_ok(&defs, &cond),
        Type::Lit(Box::new(Expr::synthetic(ExprKind::IntLit(1))))
    );
}

#[test]
fn resolves_native_algorithm_type_annotations() {
    let (defs, _) = defs("");
    assert_eq!(
        resolve_ok(
            &defs,
            &Type::Apply {
                name: "BinaryHeap".into(),
                args: vec![Type::I32],
            }
        ),
        Type::BinaryHeap(Box::new(Type::I32))
    );
    assert_eq!(
        resolve_ok(
            &defs,
            &Type::Apply {
                name: "BinomialHeap".into(),
                args: vec![Type::I32],
            }
        ),
        Type::BinomialHeap(Box::new(Type::I32))
    );
    assert_eq!(
        resolve_ok(
            &defs,
            &Type::Apply {
                name: "AvlTree".into(),
                args: vec![Type::I32],
            }
        ),
        Type::AvlTree(Box::new(Type::I32))
    );
    assert_eq!(
        resolve_ok(
            &defs,
            &Type::Apply {
                name: "RedBlackTree".into(),
                args: vec![Type::I32],
            }
        ),
        Type::RedBlackTree(Box::new(Type::I32))
    );
    assert_eq!(
        resolve_ok(
            &defs,
            &Type::Apply {
                name: "SplayTree".into(),
                args: vec![Type::I32],
            }
        ),
        Type::SplayTree(Box::new(Type::I32))
    );
    assert_eq!(
        resolve_ok(
            &defs,
            &Type::Apply {
                name: "Treap".into(),
                args: vec![Type::I32],
            }
        ),
        Type::Treap(Box::new(Type::I32))
    );
    assert_eq!(
        resolve_ok(&defs, &Type::Named("FenwickTree".into())),
        Type::FenwickTree
    );
    assert_eq!(
        resolve_ok(&defs, &Type::Named("SegmentTree".into())),
        Type::SegmentTree
    );
    assert_eq!(
        resolve_ok(&defs, &Type::Named("SparseTable".into())),
        Type::SparseTable
    );
    assert_eq!(
        resolve_ok(&defs, &Type::Named("SegmentTreeLazy".into())),
        Type::SegmentTreeLazy
    );
    assert_eq!(
        resolve_ok(&defs, &Type::Named("BitSet".into())),
        Type::BitSet
    );
    assert_eq!(
        resolve_ok(&defs, &Type::Named("UnionFind".into())),
        Type::UnionFind(Box::new(Type::Usize))
    );
}

#[test]
fn resolves_pick_by_type_and_omit_by_type() {
    let (d, _) = defs("interface User { id: i32; name: string; email: string; active: bool; }");
    let apply = |name: &str, args: Vec<Type>| Type::Apply {
        name: name.into(),
        args,
    };
    let user = || Type::Named("User".into());

    let Type::Object(fields) = resolve_ok(&d, &apply("PickByType", vec![user(), Type::Str])) else {
        panic!()
    };
    assert_eq!(
        fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
        ["name", "email"]
    );
    let Type::Object(fields) = resolve_ok(&d, &apply("OmitByType", vec![user(), Type::Str])) else {
        panic!()
    };
    assert_eq!(
        fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
        ["id", "active"]
    );
    let Type::Object(fields) = resolve_ok(&d, &apply("PickByType", vec![user(), Type::I32])) else {
        panic!()
    };
    assert_eq!(
        fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
        ["id"]
    );
    let Type::Object(fields) = resolve_ok(&d, &apply("PickByType", vec![user(), Type::Bool]))
    else {
        panic!()
    };
    assert_eq!(
        fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
        ["active"]
    );
}

#[test]
fn resolves_camel_case_keys_and_snake_case_keys() {
    let (d1, _) = defs("interface Api { user_id: i32; first_name: string; is_active: bool; }");
    let apply = |name: &str, args: Vec<Type>| Type::Apply {
        name: name.into(),
        args,
    };
    let api = || Type::Named("Api".into());

    let Type::Object(fields) = resolve_ok(&d1, &apply("CamelCaseKeys", vec![api()])) else {
        panic!()
    };
    assert_eq!(
        fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
        ["userId", "firstName", "isActive"]
    );
    let (d2, _) = defs("interface Internal { userId: i32; firstName: string; }");
    let internal = || Type::Named("Internal".into());
    let Type::Object(fields) = resolve_ok(&d2, &apply("SnakeCaseKeys", vec![internal()])) else {
        panic!()
    };
    assert_eq!(
        fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
        ["user_id", "first_name"]
    );
}

#[test]
fn resolves_data_only() {
    let (d, _) = defs(
        "interface Service { id: i32; name: string; save: () => bool; notify: (string) => void; }",
    );
    let apply = |name: &str, args: Vec<Type>| Type::Apply {
        name: name.into(),
        args,
    };
    let service = || Type::Named("Service".into());

    let Type::Object(fields) = resolve_ok(&d, &apply("DataOnly", vec![service()])) else {
        panic!()
    };
    assert_eq!(
        fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
        ["id", "name"]
    );
}

#[test]
fn resolves_deep_partial() {
    let (defs, _) = defs("interface Config { db: { host: string; port: i32; }; active: bool; }");
    let apply = |name: &str, args: Vec<Type>| Type::Apply {
        name: name.into(),
        args,
    };
    let config = || Type::Named("Config".into());

    let Type::Object(fields) = resolve_ok(&defs, &apply("DeepPartial", vec![config()])) else {
        panic!()
    };
    assert!(fields.iter().all(|f| f.optional));
}

#[test]
fn deep_partial_expands_named_interface_fields() {
    let (defs, _) = defs(
        "interface Credentials { user: string; password: string; } interface Config { credentials: Credentials; }",
    );
    let Type::Object(fields) = resolve_ok(
        &defs,
        &Type::Apply {
            name: "DeepPartial".into(),
            args: vec![Type::Named("Config".into())],
        },
    ) else {
        panic!()
    };
    let credentials = fields
        .iter()
        .find(|field| field.name == "credentials")
        .unwrap();
    let Type::Object(nested) = &credentials.ty else {
        panic!("expected named interface to expand into a partial object")
    };
    assert!(nested.iter().all(|field| field.optional));
}

#[test]
fn resolves_require() {
    let (defs, _) =
        defs("interface Client { id: i32; name: string; email?: string; phone?: string; }");
    let apply = |name: &str, args: Vec<Type>| Type::Apply {
        name: name.into(),
        args,
    };
    let client = || Type::Named("Client".into());
    let lit = |s: &str| Type::Lit(Box::new(Expr::synthetic(ExprKind::StrLit(s.into()))));

    let Type::Object(fields) = resolve_ok(&defs, &apply("Require", vec![client(), lit("email")]))
    else {
        panic!()
    };
    let email_field = fields.iter().find(|f| f.name == "email").unwrap();
    assert!(!email_field.optional);
    let phone_field = fields.iter().find(|f| f.name == "phone").unwrap();
    assert!(phone_field.optional);
}

#[test]
fn resolves_at_least_one() {
    let (defs, _) = defs("interface Form { name: string; email?: string; phone?: string; }");
    let apply = |name: &str, args: Vec<Type>| Type::Apply {
        name: name.into(),
        args,
    };
    let form = || Type::Named("Form".into());
    let lit = |s: &str| Type::Lit(Box::new(Expr::synthetic(ExprKind::StrLit(s.into()))));

    let keys = Type::Union(vec![lit("email"), lit("phone")]);
    let Type::Union(variants) = resolve_ok(&defs, &apply("AtLeastOne", vec![form(), keys])) else {
        panic!()
    };
    assert_eq!(variants.len(), 2);
    for variant in variants {
        let Type::Object(fields) = variant else {
            panic!()
        };
        assert!(
            !fields
                .iter()
                .find(|field| field.name == "name")
                .unwrap()
                .optional
        );
        assert_eq!(
            fields
                .iter()
                .filter(|field| matches!(field.name.as_str(), "email" | "phone") && !field.optional)
                .count(),
            1
        );
    }
}
