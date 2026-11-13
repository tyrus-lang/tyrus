#[test]
fn db_transaction_exports_are_explicit_and_query_accepts_a_transaction_context() {
    let (transaction_type, _, transaction_alias) =
        native_export_info("tyrus:db", "DbTransaction").unwrap();
    assert_eq!(transaction_type, Type::Named("DbTransaction".to_string()));
    assert_eq!(transaction_alias, Some(Type::U64));
    for name in [
        "beginTransaction",
        "commitTransaction",
        "rollbackTransaction",
    ] {
        assert!(
            native_export_info("tyrus:db", name).is_some(),
            "missing {name}"
        );
    }

    let (query_type, _, _) = native_export_info("tyrus:db", "query").unwrap();
    let Type::GenericFunction { params, .. } = query_type else {
        panic!("db.query must remain generic");
    };
    let Some(Type::Optional(options)) = params.get(2) else {
        panic!("db.query must accept optional transaction options");
    };
    let Type::Object(fields) = options.as_ref() else {
        panic!("db.query transaction options must be an object");
    };
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].name, "transaction");
    assert_eq!(fields[0].ty, Type::U64);
}
