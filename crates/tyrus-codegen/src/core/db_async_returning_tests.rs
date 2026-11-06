use super::db_async_returning_functions;
use tyrus_core::parser::Parser;

#[test]
fn tracks_db_async_results_forwarded_through_const_arrow_functions() {
    let program = Parser::from_source(
        r#"
            const repository = (): Promise<Result<User[], DbError>> => User.findMany({});
            const service = (): Promise<Result<User[], DbError>> => repository();
            const controller = (): Promise<Result<User[], DbError>> => service();
        "#,
    )
    .unwrap()
    .parse_program()
    .unwrap();

    let forwarded = db_async_returning_functions(&program.items);

    assert!(forwarded.contains("repository"));
    assert!(forwarded.contains("service"));
    assert!(forwarded.contains("controller"));
}
