use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use sqlx::Connection;
use sqlx::sqlite::{SqliteConnectOptions, SqliteConnection};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

struct TestProject(PathBuf);

impl TestProject {
    fn new() -> Self {
        let id = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("tyrus-db-cli-{}-{id}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("tyrus.json"),
            r#"{"project":{"name":"db-test","version":"0.1.0"},"database":{"backend":"sqlite","path":"data.sqlite"}}"#,
        )
        .unwrap();
        Self(root)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn database(&self) -> PathBuf {
        self.0.join("data.sqlite")
    }

    fn command(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_tyrus"))
            .args(args)
            .current_dir(self.path())
            .env(
                "TYRUS_RUNTIME_LIB",
                std::env::current_exe()
                    .unwrap()
                    .parent()
                    .and_then(Path::parent)
                    .unwrap()
                    .join("libtyrus_runtime.a"),
            )
            .output()
            .unwrap()
    }
}

fn normalized_stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n")
}

impl Drop for TestProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

async fn create_database(path: &Path, schema: &str) {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true);
    let mut connection = SqliteConnection::connect_with(&options).await.unwrap();
    sqlx::query(sqlx::AssertSqlSafe(schema.to_owned()))
        .execute(&mut connection)
        .await
        .unwrap();
    connection.close().await.unwrap();
}

#[test]
fn introspect_writes_snapshot_and_check_confirms_it_without_mutating_database() {
    let project = TestProject::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(create_database(
        &project.database(),
        "CREATE TABLE users (id INTEGER PRIMARY KEY, email TEXT NOT NULL UNIQUE); CREATE INDEX users_email_idx ON users(email); CREATE TABLE posts (id INTEGER PRIMARY KEY, user_id INTEGER REFERENCES users);",
    ));

    let introspect = project.command(&["db", "introspect"]);
    assert!(
        introspect.status.success(),
        "{}",
        String::from_utf8_lossy(&introspect.stderr)
    );
    let snapshot = fs::read(project.path().join("tyrus-db-schema.json")).unwrap();
    let before = fs::read(project.database()).unwrap();

    let check = project.command(&["db", "check"]);
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    assert_eq!(fs::read(project.database()).unwrap(), before);
    let snapshot_text = String::from_utf8_lossy(&snapshot);
    assert!(snapshot_text.contains("\"schemaHash\""));
    let snapshot_json: serde_json::Value = serde_json::from_str(&snapshot_text).unwrap();
    let referenced = &snapshot_json["tables"][0]["foreignKeys"][0]["referencedColumns"][0];
    assert_eq!(referenced, "id");
}

#[test]
fn check_reports_drift_and_does_not_modify_the_database() {
    let project = TestProject::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(create_database(
        &project.database(),
        "CREATE TABLE users (id INTEGER PRIMARY KEY);",
    ));
    let introspect = project.command(&["db", "introspect"]);
    assert!(
        introspect.status.success(),
        "{}",
        String::from_utf8_lossy(&introspect.stderr)
    );
    runtime.block_on(create_database(
        &project.database(),
        "ALTER TABLE users ADD COLUMN display_name TEXT;",
    ));
    let before = fs::read(project.database()).unwrap();

    let check = project.command(&["db", "check"]);
    assert!(!check.status.success());
    let stderr = String::from_utf8_lossy(&check.stderr);
    assert!(stderr.contains("schema drift detected"), "{stderr}");
    assert_eq!(fs::read(project.database()).unwrap(), before);
}

#[test]
fn explain_sql_accepts_a_quoted_sql_argument() {
    let project = TestProject::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(create_database(
        &project.database(),
        "CREATE TABLE db_examples (id INTEGER PRIMARY KEY);",
    ));
    let introspect = project.command(&["db", "introspect"]);
    assert!(
        introspect.status.success(),
        "{}",
        String::from_utf8_lossy(&introspect.stderr)
    );

    let output = project.command(&[
        "db",
        "explain-sql",
        "SELECT COUNT(*) AS total FROM db_examples WHERE id >= ?",
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("?1: i64 (value hidden)"), "{stdout}");
    assert!(stdout.contains("total: i64"), "{stdout}");
}

#[test]
fn diff_reports_model_changes_against_snapshot_without_mutating_either() {
    let project = TestProject::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(create_database(
        &project.database(),
        "CREATE TABLE users (id INTEGER PRIMARY KEY);",
    ));
    let introspected = project.command(&["db", "introspect"]);
    assert!(
        introspected.status.success(),
        "{}",
        String::from_utf8_lossy(&introspected.stderr)
    );
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("src/main.tyr"),
        r#"@model("users", { timestamps: false })
interface User {
    @id
    id: i64;
    nickname: string?;
}

const main = (): void => {};
"#,
    )
    .unwrap();
    let database_before = fs::read(project.database()).unwrap();
    let snapshot_before = fs::read(project.path().join("tyrus-db-schema.json")).unwrap();

    let diff = project.command(&["db", "diff"]);
    assert!(
        diff.status.success(),
        "{}",
        String::from_utf8_lossy(&diff.stderr)
    );
    let stdout = String::from_utf8_lossy(&diff.stdout);
    assert!(stdout.contains("Differences between Models and the database schema snapshot"));
    assert!(stdout.contains("+ column `users.nickname` (string)"));
    assert_eq!(fs::read(project.database()).unwrap(), database_before);
    assert_eq!(
        fs::read(project.path().join("tyrus-db-schema.json")).unwrap(),
        snapshot_before
    );
}

#[test]
fn check_requires_a_snapshot_without_leaking_configuration_values() {
    let project = TestProject::new();
    fs::write(
        project.path().join("tyrus.json"),
        r#"{"project":{"name":"db-test","version":"0.1.0"},"database":{"backend":"sqlite","path":"private-secret.db","password":"dont-print-this"}}"#,
    )
    .unwrap();
    let output = project.command(&["db", "check"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("private-secret"));
    assert!(!stderr.contains("dont-print-this"));
}

#[test]
fn migrate_generate_writes_reviewable_sql_from_models_without_applying_it() {
    let project = TestProject::new();
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("src/main.tyr"),
        r#"import { db } from "tyrus:db";

interface UserRow {
    nickname: string;
}

@model("users")
export interface User {
    @id
    id: i64;
    @unique
    email: string;
    @default("Anonymous")
    displayName: string;
}

const main = async (): Promise<void> => {
    const rows = await db.query<UserRow>("SELECT nickname FROM users", []);
};
"#,
    )
    .unwrap();

    let generated = project.command(&["db", "migrate", "generate", "initial_schema"]);
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let migration =
        fs::read_to_string(project.path().join("db/migrations/0001_initial_schema.sql")).unwrap();
    assert!(migration.contains("CREATE TABLE \"users\""));
    assert!(migration.contains("PRIMARY KEY (\"id\")"));
    assert!(migration.contains("DEFAULT 'Anonymous'"));
    assert!(migration.contains("\"createdAt\" TEXT DEFAULT CURRENT_TIMESTAMP NOT NULL"));
    assert!(migration.contains("\"updatedAt\" TEXT DEFAULT CURRENT_TIMESTAMP NOT NULL"));
    assert!(migration.contains("CREATE TRIGGER"));
    assert!(!project.database().exists());
}

#[test]
fn sqlite_migration_generation_creates_an_implicit_id_for_models_without_one() {
    let project = TestProject::new();
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("src/main.tyr"),
        r#"@model("users", { timestamps: false })
export interface User {
    name: string;
}

const main = (): void => {};
"#,
    )
    .unwrap();

    let generated = project.command(&["db", "migrate", "generate", "initial_users"]);
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let migration =
        fs::read_to_string(project.path().join("db/migrations/0001_initial_users.sql")).unwrap();
    assert!(migration.contains("\"id\" INTEGER NOT NULL"), "{migration}");
    assert!(migration.contains("PRIMARY KEY (\"id\")"), "{migration}");
    assert!(migration.contains("\"name\" TEXT NOT NULL"), "{migration}");
}

#[test]
fn sqlite_model_insert_and_read_use_the_implicit_id() {
    let project = TestProject::new();
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::create_dir_all(project.path().join("src/models")).unwrap();
    fs::write(
        project.path().join("src/models/User.tyr"),
        r#"@model("users", { timestamps: false })
export interface User {
    name: string;
}
"#,
    )
    .unwrap();
    fs::write(
        project.path().join("src/main.tyr"),
        r#"import { DbError } from "tyrus:db";
import { User } from "./models/User";

const main = async (): Promise<void> => {
    const inserted = await User.insertOne({ name: "Ada" }) onfail {
        printError(inserted.code);
        return;
    };
    print(inserted.id);

    const user = await User.findOne({ id: inserted.id }) onfail {
        printError(user.code);
        return;
    };
    if (user) {
        print(user.id);
        print(user.name);
    }
};
"#,
    )
    .unwrap();

    let generated = project.command(&["db", "migrate", "generate", "initial_users"]);
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let applied = project.command(&["db", "migrate", "apply"]);
    assert!(
        applied.status.success(),
        "{}",
        String::from_utf8_lossy(&applied.stderr)
    );
    let introspected = project.command(&["db", "introspect"]);
    assert!(
        introspected.status.success(),
        "{}",
        String::from_utf8_lossy(&introspected.stderr)
    );

    let output = project.command(&["run", "src/main.tyr"]);
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        normalized_stdout(&output).ends_with("1\n1\nAda\n"),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn initial_migrations_validate_model_foreign_key_targets() {
    let cases = [
        (
            r#"@model("users", { timestamps: false })
interface User {
    @id
    id: i64;
    @foreignKey("teams", "id")
    teamId: i64?;
}
"#,
            "that table has no project Model",
        ),
        (
            r#"@model("teams", { timestamps: false })
interface Team {
    @id
    id: i64;
}
@model("users", { timestamps: false })
interface User {
    @id
    id: i64;
    @foreignKey("teams", "missing")
    teamId: i64?;
}
"#,
            "references missing Model column",
        ),
        (
            r#"@model("teams", { timestamps: false })
interface Team {
    id: i64;
    code: i64;
}
@model("users", { timestamps: false })
interface User {
    @id
    id: i64;
    @foreignKey("teams", "code")
    teamCode: i64?;
}
"#,
            "must be a single-column primary key or unique column",
        ),
        (
            r#"@model("teams", { timestamps: false })
interface Team {
    id: i64;
    @unique
    email: string;
}
@model("users", { timestamps: false })
interface User {
    @id
    id: i64;
    @foreignKey("teams", "email")
    teamId: i64?;
}
"#,
            "has type `i64` but referenced Model column `teams.email` has type `string`",
        ),
        (
            r#"@model("Teams", { timestamps: false })
interface Team {
    @id
    id: i64;
}
@model("teams", { timestamps: false })
interface OtherTeam {
    @id
    id: i64;
}
"#,
            "multiple models map to table `teams`",
        ),
        (
            r#"@model("users", { timestamps: false })
interface User {
    @id
    id: i64;
    @column("name")
    firstName: string;
    @column("NAME")
    lastName: string;
}
"#,
            "maps multiple fields to column `NAME`",
        ),
    ];

    for (source, expected_error) in cases {
        let project = TestProject::new();
        fs::create_dir_all(project.path().join("src")).unwrap();
        fs::write(
            project.path().join("src/main.tyr"),
            format!("{source}\nconst main = (): void => {{}};\n"),
        )
        .unwrap();
        let generated = project.command(&["db", "migrate", "generate", "initial_schema"]);
        assert!(!generated.status.success());
        assert!(
            String::from_utf8_lossy(&generated.stderr).contains(expected_error),
            "expected `{expected_error}`, got: {}",
            String::from_utf8_lossy(&generated.stderr)
        );
        assert!(!project.path().join("db/migrations").exists());
        assert!(!project.database().exists());
    }

    let project = TestProject::new();
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("src/main.tyr"),
        r#"@model("Teams", { timestamps: false })
interface Team {
    @id
    ID: i64;
}
@model("users", { timestamps: false })
interface User {
    @id
    id: i64;
    @foreignKey("teams", "id")
    teamId: i64?;
}

const main = (): void => {};
"#,
    )
    .unwrap();
    let generated = project.command(&["db", "migrate", "generate", "initial_schema"]);
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let migration =
        fs::read_to_string(project.path().join("db/migrations/0001_initial_schema.sql")).unwrap();
    assert!(migration.contains("REFERENCES \"teams\" (\"id\")"));
    assert!(!project.database().exists());
}

#[test]
fn introspect_accepts_case_insensitive_sqlite_foreign_key_identifiers() {
    let project = TestProject::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(create_database(
        &project.database(),
        "CREATE TABLE Teams (ID INTEGER PRIMARY KEY); CREATE TABLE Memberships (team_id INTEGER REFERENCES teams(id));",
    ));

    let introspected = project.command(&["db", "introspect"]);
    assert!(
        introspected.status.success(),
        "{}",
        String::from_utf8_lossy(&introspected.stderr)
    );
    let snapshot: serde_json::Value =
        serde_json::from_slice(&fs::read(project.path().join("tyrus-db-schema.json")).unwrap())
            .unwrap();
    let memberships = snapshot["tables"]
        .as_array()
        .unwrap()
        .iter()
        .find(|table| table["name"] == "Memberships")
        .unwrap();
    assert_eq!(memberships["foreignKeys"][0]["referencedTable"], "teams");
    assert_eq!(memberships["foreignKeys"][0]["referencedColumns"][0], "id");
}

#[test]
fn incremental_migrations_validate_foreign_key_targets_in_the_current_schema() {
    let cases = [
        (
            "CREATE TABLE unrelated (id INTEGER PRIMARY KEY);",
            r#"@foreignKey("missing", "id")
    teamId: i64?;"#,
            "references missing table `missing`",
        ),
        (
            "CREATE TABLE teams (id INTEGER PRIMARY KEY);",
            r#"@foreignKey("teams", "missing")
    teamId: i64?;"#,
            "references missing column `teams.missing`",
        ),
        (
            "CREATE TABLE teams (id INTEGER PRIMARY KEY, code TEXT);",
            r#"@foreignKey("teams", "code")
    teamCode: string?;"#,
            "which is not a primary key or unique key",
        ),
        (
            "CREATE TABLE teams (id TEXT PRIMARY KEY);",
            r#"@foreignKey("teams", "id")
    teamId: i64?;"#,
            "has a different SQLite type affinity",
        ),
    ];

    for (schema, foreign_key, expected_error) in cases {
        let project = TestProject::new();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(create_database(&project.database(), schema));
        let introspect = project.command(&["db", "introspect"]);
        assert!(
            introspect.status.success(),
            "{}",
            String::from_utf8_lossy(&introspect.stderr)
        );
        fs::create_dir_all(project.path().join("src")).unwrap();
        fs::write(
            project.path().join("src/main.tyr"),
            format!(
                "@model(\"users\", {{ timestamps: false }})\ninterface User {{\n    @id\n    id: i64;\n    {foreign_key}\n}}\nconst main = (): void => {{}};\n"
            ),
        )
        .unwrap();
        let generated = project.command(&["db", "migrate", "generate", "add_user"]);
        assert!(!generated.status.success());
        assert!(
            String::from_utf8_lossy(&generated.stderr).contains(expected_error),
            "expected `{expected_error}`, got: {}",
            String::from_utf8_lossy(&generated.stderr)
        );
        assert!(!project.path().join("db/migrations").exists());
    }

    let project = TestProject::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(create_database(
        &project.database(),
        "CREATE TABLE teams (id INTEGER PRIMARY KEY);",
    ));
    assert!(project.command(&["db", "introspect"]).status.success());
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("src/main.tyr"),
        r#"@model("users", { timestamps: false })
interface User {
    @id
    id: i64;
    @foreignKey("teams", "id")
    teamId: i64?;
}
const main = (): void => {};
"#,
    )
    .unwrap();
    let generated = project.command(&["db", "migrate", "generate", "add_user"]);
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );

    let project = TestProject::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(create_database(
        &project.database(),
        "CREATE TABLE teams (id INTEGER PRIMARY KEY, code TEXT NOT NULL); CREATE UNIQUE INDEX teams_code_unique ON teams(code);",
    ));
    assert!(project.command(&["db", "introspect"]).status.success());
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("src/main.tyr"),
        r#"@model("users", { timestamps: false })
interface User {
    @id
    id: i64;
    @foreignKey("teams", "code")
    teamCode: string?;
}
const main = (): void => {};
"#,
    )
    .unwrap();
    let generated = project.command(&["db", "migrate", "generate", "add_user"]);
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let migration =
        fs::read_to_string(project.path().join("db/migrations/0001_add_user.sql")).unwrap();
    assert!(migration.contains("REFERENCES \"teams\" (\"code\")"));

    let project = TestProject::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(create_database(
        &project.database(),
        "CREATE TABLE teams (region TEXT NOT NULL, code TEXT NOT NULL, PRIMARY KEY (region, code)); CREATE TABLE roster (region TEXT NOT NULL, team_code TEXT NOT NULL, FOREIGN KEY (region, team_code) REFERENCES teams (region, code));",
    ));
    let introspected = project.command(&["db", "introspect"]);
    assert!(
        introspected.status.success(),
        "{}",
        String::from_utf8_lossy(&introspected.stderr)
    );
    let snapshot: serde_json::Value =
        serde_json::from_slice(&fs::read(project.path().join("tyrus-db-schema.json")).unwrap())
            .unwrap();
    let roster = snapshot["tables"]
        .as_array()
        .unwrap()
        .iter()
        .find(|table| table["name"] == "roster")
        .unwrap();
    assert_eq!(roster["primaryKey"], serde_json::json!([]));
    assert_eq!(
        roster["foreignKeys"][0]["columns"],
        serde_json::json!(["region", "team_code"])
    );
    assert_eq!(
        roster["foreignKeys"][0]["referencedColumns"],
        serde_json::json!(["region", "code"])
    );
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("src/main.tyr"),
        r#"@model("users", { timestamps: false })
interface User {
    @id
    id: i64;
}
const main = (): void => {};
"#,
    )
    .unwrap();
    let generated = project.command(&["db", "migrate", "generate", "add_user"]);
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
}

#[test]
fn model_migrations_match_existing_sqlite_identifiers_without_case_sensitivity() {
    let project = TestProject::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(create_database(
        &project.database(),
        "CREATE TABLE Teams (ID INTEGER PRIMARY KEY); CREATE TABLE Users (ID INTEGER PRIMARY KEY, DisplayName TEXT NOT NULL);",
    ));
    assert!(project.command(&["db", "introspect"]).status.success());
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("src/main.tyr"),
        r#"@model("users", { timestamps: false })
interface User {
    @id
    id: i64;
    displayName: string;
}
const main = (): void => {};
"#,
    )
    .unwrap();
    let generated = project.command(&["db", "migrate", "generate", "no_change"]);
    assert!(!generated.status.success());
    assert!(
        String::from_utf8_lossy(&generated.stderr)
            .contains("models match the database schema; no migration is needed")
    );
    assert!(!project.path().join("db/migrations").exists());

    let project = TestProject::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(create_database(
        &project.database(),
        "CREATE TABLE Teams (ID INTEGER PRIMARY KEY);",
    ));
    assert!(project.command(&["db", "introspect"]).status.success());
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("src/main.tyr"),
        r#"@model("users", { timestamps: false })
interface User {
    @id
    id: i64;
    @foreignKey("teams", "id")
    teamId: i64?;
}
const main = (): void => {};
"#,
    )
    .unwrap();
    let generated = project.command(&["db", "migrate", "generate", "add_user"]);
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let migration =
        fs::read_to_string(project.path().join("db/migrations/0001_add_user.sql")).unwrap();
    assert!(migration.contains("REFERENCES \"teams\" (\"id\")"));

    let project = TestProject::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(create_database(
        &project.database(),
        "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL);",
    ));
    assert!(project.command(&["db", "introspect"]).status.success());
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("src/main.tyr"),
        r#"@model("users", { timestamps: false })
interface User {
    @id
    id: i64;
    @column("name")
    firstName: string;
    @column("NAME")
    lastName: string;
}
const main = (): void => {};
"#,
    )
    .unwrap();
    let generated = project.command(&["db", "migrate", "generate", "invalid_mapping"]);
    assert!(!generated.status.success());
    assert!(
        String::from_utf8_lossy(&generated.stderr)
            .contains("maps multiple fields to column `NAME`")
    );
    assert!(!project.path().join("db/migrations").exists());
}

#[test]
fn migrate_generate_adds_nullable_model_columns_against_the_snapshot() {
    let project = TestProject::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(create_database(
        &project.database(),
        "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL); INSERT INTO users (name) VALUES ('Ada');",
    ));
    let introspect = project.command(&["db", "introspect"]);
    assert!(
        introspect.status.success(),
        "{}",
        String::from_utf8_lossy(&introspect.stderr)
    );
    let database_before = fs::read(project.database()).unwrap();
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("src/main.tyr"),
        r#"import { db } from "tyrus:db";

interface UserRow {
    nickname: string;
}

@model("users", { timestamps: false })
export interface User {
    @id
    id: i64;
    name: string;
    nickname: string?;
    @default("untitled")
    title: string;
}

const main = async (): Promise<void> => {
    const rows = await db.query<UserRow>("SELECT nickname FROM users", []);
};
"#,
    )
    .unwrap();

    let generated = project.command(&["db", "migrate", "generate", "add_nickname"]);
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let migration =
        fs::read_to_string(project.path().join("db/migrations/0001_add_nickname.sql")).unwrap();
    assert!(migration.contains("ALTER TABLE \"users\" ADD COLUMN \"nickname\" TEXT;"));
    assert!(
        migration.contains(
            "ALTER TABLE \"users\" ADD COLUMN \"title\" TEXT DEFAULT 'untitled' NOT NULL;"
        )
    );
    assert_eq!(fs::read(project.database()).unwrap(), database_before);
}

#[test]
fn migrate_generate_adds_nullable_byte_array_columns_as_blobs() {
    let project = TestProject::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(create_database(
        &project.database(),
        "CREATE TABLE assets (id INTEGER PRIMARY KEY);",
    ));
    let introspect = project.command(&["db", "introspect"]);
    assert!(
        introspect.status.success(),
        "{}",
        String::from_utf8_lossy(&introspect.stderr)
    );
    let database_before = fs::read(project.database()).unwrap();
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("src/main.tyr"),
        r#"@model("assets", { timestamps: false })
export interface Asset {
    @id
    id: i64;
    payload: u8[]?;
}

const main = (): void => {};
"#,
    )
    .unwrap();

    let generated = project.command(&["db", "migrate", "generate", "add_payload"]);
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let migration =
        fs::read_to_string(project.path().join("db/migrations/0001_add_payload.sql")).unwrap();
    assert!(migration.contains("ALTER TABLE \"assets\" ADD COLUMN \"payload\" BLOB;"));
    assert_eq!(fs::read(project.database()).unwrap(), database_before);

    let applied = project.command(&["db", "migrate", "apply"]);
    assert!(
        applied.status.success(),
        "{}",
        String::from_utf8_lossy(&applied.stderr)
    );
    let refreshed = project.command(&["db", "introspect"]);
    assert!(
        refreshed.status.success(),
        "{}",
        String::from_utf8_lossy(&refreshed.stderr)
    );
    let snapshot = fs::read_to_string(project.path().join("tyrus-db-schema.json")).unwrap();
    assert!(snapshot.contains("\"name\": \"payload\""));
    assert!(snapshot.contains("\"type\": \"bytes\""));
}

#[test]
fn migrate_generate_rebuilds_removed_columns_and_preserves_existing_rows() {
    let project = TestProject::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(create_database(
        &project.database(),
        "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL, obsolete TEXT); INSERT INTO users (id, name, obsolete) VALUES (7, 'Ada', 'legacy');",
    ));
    assert!(project.command(&["db", "introspect"]).status.success());
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("src/main.tyr"),
        r#"@model("users", { timestamps: false })
interface User {
    @id
    id: i64;
    name: string;
}

const main = (): void => {};
"#,
    )
    .unwrap();

    let generated = project.command(&["db", "migrate", "generate", "remove_obsolete"]);
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let migration = fs::read_to_string(
        project
            .path()
            .join("db/migrations/0001_remove_obsolete.sql"),
    )
    .unwrap();
    assert!(migration.contains("INSERT INTO \"__tyrus_migrate_users\""));
    assert!(migration.contains("SELECT \"id\", \"name\" FROM \"users\""));
    assert!(migration.contains("DROP TABLE \"users\""));

    let applied = project.command(&["db", "migrate", "apply"]);
    assert!(
        applied.status.success(),
        "{}",
        String::from_utf8_lossy(&applied.stderr)
    );
    let (row, obsolete_columns): ((i64, String), i64) = runtime.block_on(async {
        let mut connection = SqliteConnection::connect_with(
            &SqliteConnectOptions::new().filename(project.database()),
        )
        .await
        .unwrap();
        let row = sqlx::query_as("SELECT id, name FROM users")
            .fetch_one(&mut connection)
            .await
            .unwrap();
        let columns: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pragma_table_info('users') WHERE name = 'obsolete'",
        )
        .fetch_one(&mut connection)
        .await
        .unwrap();
        (row, columns)
    });
    assert_eq!(row, (7, "Ada".to_string()));
    assert_eq!(obsolete_columns, 0);
}

#[test]
fn migrate_generate_rebuilds_a_table_and_preserves_custom_indexes() {
    let project = TestProject::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(create_database(
        &project.database(),
        "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL, obsolete TEXT); CREATE INDEX users_name_idx ON users(name);",
    ));
    assert!(project.command(&["db", "introspect"]).status.success());
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("src/main.tyr"),
        r#"@model("users", { timestamps: false })
interface User {
    @id
    id: i64;
    name: string;
}

const main = (): void => {};
"#,
    )
    .unwrap();

    let generated = project.command(&["db", "migrate", "generate", "remove_obsolete"]);
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let migration = fs::read_to_string(
        project
            .path()
            .join("db/migrations/0001_remove_obsolete.sql"),
    )
    .unwrap();
    assert!(migration.contains("CREATE INDEX users_name_idx ON users(name)"));
    let applied = project.command(&["db", "migrate", "apply"]);
    assert!(
        applied.status.success(),
        "{}",
        String::from_utf8_lossy(&applied.stderr)
    );
    let index_exists: i64 = runtime.block_on(async {
        let mut connection = SqliteConnection::connect_with(
            &SqliteConnectOptions::new().filename(project.database()),
        )
        .await
        .unwrap();
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_schema WHERE type = 'index' AND name = 'users_name_idx'",
        )
        .fetch_one(&mut connection)
        .await
        .unwrap()
    });
    assert_eq!(index_exists, 1);
}

#[test]
fn migrate_generate_refuses_to_remove_a_column_used_by_a_custom_index() {
    let project = TestProject::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(create_database(
        &project.database(),
        "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL, obsolete TEXT); CREATE INDEX users_obsolete_idx ON users(obsolete);",
    ));
    assert!(project.command(&["db", "introspect"]).status.success());
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("src/main.tyr"),
        r#"@model("users", { timestamps: false })
interface User {
    @id
    id: i64;
    name: string;
}

const main = (): void => {};
"#,
    )
    .unwrap();

    let generated = project.command(&["db", "migrate", "generate", "remove_obsolete"]);
    assert!(!generated.status.success());
    assert!(
        String::from_utf8_lossy(&generated.stderr)
            .contains("removes columns referenced by custom indexes")
    );
    assert!(
        !project
            .path()
            .join("db/migrations/0001_remove_obsolete.sql")
            .exists()
    );
}

#[test]
fn migrate_generate_rebuilds_unique_constraints_and_preserves_rows() {
    let project = TestProject::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(create_database(
        &project.database(),
        "CREATE TABLE users (id INTEGER PRIMARY KEY, email TEXT NOT NULL UNIQUE); INSERT INTO users (id, email) VALUES (3, 'ada@example.com');",
    ));
    assert!(project.command(&["db", "introspect"]).status.success());
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("src/main.tyr"),
        r#"@model("users", { timestamps: false })
interface User {
    @id
    id: i64;
    email: string;
}

const main = (): void => {};
"#,
    )
    .unwrap();

    let generated = project.command(&["db", "migrate", "generate", "drop_email_unique"]);
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let migration = fs::read_to_string(
        project
            .path()
            .join("db/migrations/0001_drop_email_unique.sql"),
    )
    .unwrap();
    assert!(migration.contains("CREATE TABLE \"__tyrus_migrate_users\""));
    assert!(migration.contains("INSERT INTO \"__tyrus_migrate_users\""));
    let applied = project.command(&["db", "migrate", "apply"]);
    assert!(
        applied.status.success(),
        "{}",
        String::from_utf8_lossy(&applied.stderr)
    );

    let (rows, duplicate_insert_succeeded): (i64, bool) = runtime.block_on(async {
        let mut connection = SqliteConnection::connect_with(
            &SqliteConnectOptions::new().filename(project.database()),
        )
        .await
        .unwrap();
        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&mut connection)
            .await
            .unwrap();
        let duplicate = sqlx::query("INSERT INTO users (id, email) VALUES (4, 'ada@example.com')")
            .execute(&mut connection)
            .await
            .is_ok();
        (rows, duplicate)
    });
    assert_eq!(rows, 1);
    assert!(duplicate_insert_succeeded);
}

#[test]
fn migrate_generate_rebuilds_a_primary_key_and_preserves_rows() {
    let project = TestProject::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let mut connection = SqliteConnection::connect_with(
            &SqliteConnectOptions::new()
                .filename(project.database())
                .create_if_missing(true),
        )
        .await
        .unwrap();
        for sql in [
            "CREATE TABLE audit (id INTEGER PRIMARY KEY, value INTEGER NOT NULL)",
            "CREATE TABLE users (id INTEGER PRIMARY KEY, code INTEGER NOT NULL)",
            "CREATE TRIGGER users_code_audit AFTER UPDATE OF code ON users BEGIN INSERT INTO audit (value) VALUES (NEW.code); END",
            "INSERT INTO users (id, code) VALUES (1, 40), (2, 50)",
        ] {
            sqlx::query(sql).execute(&mut connection).await.unwrap();
        }
        connection.close().await.unwrap();
    });
    let introspect = project.command(&["db", "introspect"]);
    assert!(
        introspect.status.success(),
        "{}",
        String::from_utf8_lossy(&introspect.stderr)
    );
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("src/main.tyr"),
        r#"@model("users", { timestamps: false })
interface User {
    id: i64;
    @id
    code: i64;
}

const main = (): void => {};
"#,
    )
    .unwrap();

    let generated = project.command(&["db", "migrate", "generate", "change_primary_key"]);
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let migration = fs::read_to_string(
        project
            .path()
            .join("db/migrations/0001_change_primary_key.sql"),
    )
    .unwrap();
    assert!(migration.contains("PRIMARY KEY (\"code\")"));
    assert!(migration.contains("CREATE TRIGGER users_code_audit"));
    let applied = project.command(&["db", "migrate", "apply"]);
    assert!(
        applied.status.success(),
        "{}",
        String::from_utf8_lossy(&applied.stderr)
    );

    let (rows, primary_column, trigger_value): (i64, String, i64) = runtime.block_on(async {
        let mut connection = SqliteConnection::connect_with(
            &SqliteConnectOptions::new().filename(project.database()),
        )
        .await
        .unwrap();
        let rows: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE (id, code) IN ((1, 40), (2, 50))")
                .fetch_one(&mut connection)
                .await
                .unwrap();
        let primary: String =
            sqlx::query_scalar("SELECT name FROM pragma_table_info('users') WHERE pk = 1")
                .fetch_one(&mut connection)
                .await
                .unwrap();
        sqlx::query("UPDATE users SET code = 55 WHERE id = 1")
            .execute(&mut connection)
            .await
            .unwrap();
        let trigger_value: i64 = sqlx::query_scalar("SELECT value FROM audit")
            .fetch_one(&mut connection)
            .await
            .unwrap();
        (rows, primary, trigger_value)
    });
    assert_eq!(rows, 2);
    assert_eq!(primary_column, "code");
    assert_eq!(trigger_value, 55);
}

#[test]
fn migrate_generate_rebuilds_foreign_keys_and_preserves_rows() {
    let project = TestProject::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(create_database(
        &project.database(),
        "CREATE TABLE teams (id INTEGER PRIMARY KEY); CREATE TABLE users (id INTEGER PRIMARY KEY, team_id INTEGER); INSERT INTO teams (id) VALUES (4); INSERT INTO users (id, team_id) VALUES (7, 4);",
    ));
    assert!(project.command(&["db", "introspect"]).status.success());
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("src/main.tyr"),
        r#"@model("teams", { timestamps: false })
interface Team {
    @id
    id: i64;
}

@model("users", { timestamps: false })
interface User {
    @id
    id: i64;
    @foreignKey("teams", "id")
    @column("team_id")
    teamId: i64?;
}

const main = (): void => {};
"#,
    )
    .unwrap();

    let generated = project.command(&["db", "migrate", "generate", "add_team_reference"]);
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let migration = fs::read_to_string(
        project
            .path()
            .join("db/migrations/0001_add_team_reference.sql"),
    )
    .unwrap();
    assert!(migration.contains("FOREIGN KEY (\"team_id\") REFERENCES \"teams\" (\"id\")"));
    let applied = project.command(&["db", "migrate", "apply"]);
    assert!(
        applied.status.success(),
        "{}",
        String::from_utf8_lossy(&applied.stderr)
    );

    let (row_count, foreign_key_count): (i64, i64) = runtime.block_on(async {
        let mut connection = SqliteConnection::connect_with(
            &SqliteConnectOptions::new().filename(project.database()),
        )
        .await
        .unwrap();
        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE id = 7 AND team_id = 4")
            .fetch_one(&mut connection)
            .await
            .unwrap();
        let foreign_keys: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pragma_foreign_key_list('users') WHERE \"table\" = 'teams' AND \"from\" = 'team_id' AND \"to\" = 'id'",
        )
        .fetch_one(&mut connection)
        .await
        .unwrap();
        (rows, foreign_keys)
    });
    assert_eq!(row_count, 1);
    assert_eq!(foreign_key_count, 1);
}

#[test]
fn migrate_generate_refuses_rebuilding_a_referenced_table() {
    let project = TestProject::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(create_database(
        &project.database(),
        "CREATE TABLE teams (id INTEGER PRIMARY KEY, obsolete TEXT); CREATE TABLE users (id INTEGER PRIMARY KEY, team_id INTEGER REFERENCES teams(id));",
    ));
    assert!(project.command(&["db", "introspect"]).status.success());
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("src/main.tyr"),
        r#"@model("teams", { timestamps: false })
interface Team {
    @id
    id: i64;
}

@model("users", { timestamps: false })
interface User {
    @id
    id: i64;
    @foreignKey("teams", "id")
    @column("team_id")
    teamId: i64?;
}

const main = (): void => {};
"#,
    )
    .unwrap();

    let generated = project.command(&["db", "migrate", "generate", "remove_team_column"]);
    assert!(!generated.status.success());
    assert!(
        String::from_utf8_lossy(&generated.stderr).contains("while another table references it")
    );
    assert!(
        !project
            .path()
            .join("db/migrations/0001_remove_team_column.sql")
            .exists()
    );
}

#[test]
fn migrate_generate_preserves_views_when_rebuilding_without_column_removal() {
    let project = TestProject::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(create_database(
        &project.database(),
        "CREATE TABLE users (id INTEGER PRIMARY KEY, code INTEGER NOT NULL); CREATE VIEW user_codes AS SELECT code FROM users; INSERT INTO users (id, code) VALUES (1, 10);",
    ));
    let introspect = project.command(&["db", "introspect"]);
    assert!(
        introspect.status.success(),
        "{}",
        String::from_utf8_lossy(&introspect.stderr)
    );
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("src/main.tyr"),
        r#"@model("users", { timestamps: false })
interface User {
    id: i64;
    @id
    code: i64;
}

const main = (): void => {};
"#,
    )
    .unwrap();

    let generated = project.command(&["db", "migrate", "generate", "change_primary_key"]);
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let migration = fs::read_to_string(
        project
            .path()
            .join("db/migrations/0001_change_primary_key.sql"),
    )
    .unwrap();
    assert!(migration.contains("DROP VIEW \"user_codes\";"));
    assert!(migration.contains("CREATE VIEW user_codes AS SELECT code FROM users;"));
    let applied = project.command(&["db", "migrate", "apply"]);
    assert!(
        applied.status.success(),
        "{}",
        String::from_utf8_lossy(&applied.stderr)
    );
    let view_value: i64 = runtime.block_on(async {
        let mut connection = SqliteConnection::connect_with(
            &SqliteConnectOptions::new().filename(project.database()),
        )
        .await
        .unwrap();
        sqlx::query_scalar("SELECT code FROM user_codes")
            .fetch_one(&mut connection)
            .await
            .unwrap()
    });
    assert_eq!(view_value, 10);
}

#[test]
fn migrate_generate_refuses_removing_columns_referenced_by_a_view() {
    let project = TestProject::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(create_database(
        &project.database(),
        "CREATE TABLE users (id INTEGER PRIMARY KEY, obsolete TEXT); CREATE VIEW user_obsolete AS SELECT obsolete FROM users;",
    ));
    let introspect = project.command(&["db", "introspect"]);
    assert!(
        introspect.status.success(),
        "{}",
        String::from_utf8_lossy(&introspect.stderr)
    );
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("src/main.tyr"),
        r#"@model("users", { timestamps: false })
interface User {
    @id
    id: i64;
}

const main = (): void => {};
"#,
    )
    .unwrap();

    let generated = project.command(&["db", "migrate", "generate", "remove_obsolete"]);
    assert!(!generated.status.success());
    assert!(
        String::from_utf8_lossy(&generated.stderr)
            .contains("generated migration breaks view `user_obsolete`")
    );
    assert!(
        !project
            .path()
            .join("db/migrations/0001_remove_obsolete.sql")
            .exists()
    );
}

#[test]
fn migrate_generate_allows_removed_columns_when_views_and_triggers_are_unrelated() {
    let project = TestProject::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(create_database(
        &project.database(),
        "CREATE TABLE users (id INTEGER PRIMARY KEY, obsolete TEXT); CREATE TABLE teams (id INTEGER PRIMARY KEY, name TEXT NOT NULL); CREATE TABLE audit (id INTEGER PRIMARY KEY, value TEXT NOT NULL); CREATE VIEW team_names AS SELECT name FROM teams; CREATE TRIGGER teams_name_audit AFTER UPDATE OF name ON teams BEGIN INSERT INTO audit (value) VALUES (NEW.name); END;",
    ));
    let introspect = project.command(&["db", "introspect"]);
    assert!(
        introspect.status.success(),
        "{}",
        String::from_utf8_lossy(&introspect.stderr)
    );
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("src/main.tyr"),
        r#"@model("users", { timestamps: false })
interface User {
    @id
    id: i64;
}

const main = (): void => {};
"#,
    )
    .unwrap();

    let generated = project.command(&["db", "migrate", "generate", "remove_obsolete"]);
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let migration = fs::read_to_string(
        project
            .path()
            .join("db/migrations/0001_remove_obsolete.sql"),
    )
    .unwrap();
    assert!(migration.contains("DROP VIEW \"team_names\";"));
    assert!(migration.contains("CREATE VIEW team_names AS SELECT name FROM teams;"));
}

#[test]
fn migrate_generate_refuses_removed_columns_used_by_update_of_triggers() {
    let project = TestProject::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(create_database(
        &project.database(),
        "CREATE TABLE users (id INTEGER PRIMARY KEY, obsolete TEXT); CREATE TABLE audit (id INTEGER PRIMARY KEY, value TEXT); CREATE TRIGGER obsolete_audit AFTER UPDATE OF obsolete ON users BEGIN INSERT INTO audit (value) VALUES (NEW.obsolete); END;",
    ));
    let introspect = project.command(&["db", "introspect"]);
    assert!(
        introspect.status.success(),
        "{}",
        String::from_utf8_lossy(&introspect.stderr)
    );
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("src/main.tyr"),
        r#"@model("users", { timestamps: false })
interface User {
    @id
    id: i64;
}

const main = (): void => {};
"#,
    )
    .unwrap();

    let generated = project.command(&["db", "migrate", "generate", "remove_obsolete"]);
    assert!(!generated.status.success());
    assert!(
        String::from_utf8_lossy(&generated.stderr)
            .contains("removes a column referenced by a trigger")
    );
    assert!(
        !project
            .path()
            .join("db/migrations/0001_remove_obsolete.sql")
            .exists()
    );
}

#[test]
fn adding_unique_constraint_with_duplicate_data_rolls_back_the_rebuild() {
    let project = TestProject::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(create_database(
        &project.database(),
        "CREATE TABLE users (id INTEGER PRIMARY KEY, email TEXT NOT NULL); INSERT INTO users (id, email) VALUES (1, 'same@example.com'), (2, 'same@example.com');",
    ));
    assert!(project.command(&["db", "introspect"]).status.success());
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("src/main.tyr"),
        r#"@model("users", { timestamps: false })
interface User {
    @id
    id: i64;
    @unique
    email: string;
}

const main = (): void => {};
"#,
    )
    .unwrap();

    let generated = project.command(&["db", "migrate", "generate", "add_email_unique"]);
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let applied = project.command(&["db", "migrate", "apply"]);
    assert!(!applied.status.success());
    assert!(String::from_utf8_lossy(&applied.stderr).contains("will be rolled back"));

    let (rows, unique_indexes): (i64, i64) = runtime.block_on(async {
        let mut connection = SqliteConnection::connect_with(
            &SqliteConnectOptions::new().filename(project.database()),
        )
        .await
        .unwrap();
        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&mut connection)
            .await
            .unwrap();
        let indexes: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pragma_index_list('users') WHERE \"unique\" = 1",
        )
        .fetch_one(&mut connection)
        .await
        .unwrap();
        (rows, indexes)
    });
    assert_eq!(rows, 2);
    assert_eq!(unique_indexes, 0);
}

#[test]
fn migrate_generate_refuses_a_required_column_without_a_default() {
    let project = TestProject::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(create_database(
        &project.database(),
        "CREATE TABLE users (id INTEGER PRIMARY KEY);",
    ));
    assert!(project.command(&["db", "introspect"]).status.success());
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("src/main.tyr"),
        r#"@model("users", { timestamps: false })
interface User {
    @id
    id: i64;
    name: string;
}

const main = (): void => {};
"#,
    )
    .unwrap();

    let generated = project.command(&["db", "migrate", "generate", "add_name"]);
    assert!(!generated.status.success());
    assert!(String::from_utf8_lossy(&generated.stderr).contains("without a default"));
    assert!(
        !project
            .path()
            .join("db/migrations/0001_add_name.sql")
            .exists()
    );
}

#[test]
fn migrate_generate_refuses_to_use_a_stale_schema_snapshot() {
    let project = TestProject::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(create_database(
        &project.database(),
        "CREATE TABLE users (id INTEGER PRIMARY KEY);",
    ));
    assert!(project.command(&["db", "introspect"]).status.success());
    runtime.block_on(create_database(
        &project.database(),
        "ALTER TABLE users ADD COLUMN external_change TEXT;",
    ));
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("src/main.tyr"),
        r#"@model("users", { timestamps: false })
interface User {
    @id
    id: i64;
}

const main = (): void => {};
"#,
    )
    .unwrap();

    let generated = project.command(&["db", "migrate", "generate", "add_field"]);
    assert!(!generated.status.success());
    assert!(String::from_utf8_lossy(&generated.stderr).contains("schema drift detected"));
    assert!(
        !project
            .path()
            .join("db/migrations/0001_add_field.sql")
            .exists()
    );
}

#[test]
fn migrate_apply_is_atomic_and_can_retry_after_a_failed_batch() {
    let project = TestProject::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(create_database(
        &project.database(),
        "CREATE TABLE bootstrap (id INTEGER PRIMARY KEY);",
    ));
    let migrations = project.path().join("db/migrations");
    fs::create_dir_all(&migrations).unwrap();
    fs::write(
        migrations.join("0001_create_users.sql"),
        "CREATE TABLE users (id INTEGER PRIMARY KEY);\n",
    )
    .unwrap();
    fs::write(
        migrations.join("0002_invalid_statement.sql"),
        "INSERT INTO missing_table VALUES (1);\n",
    )
    .unwrap();

    let failed = project.command(&["db", "migrate", "apply"]);
    assert!(!failed.status.success());
    let users: i64 = runtime.block_on(async {
        let mut connection = SqliteConnection::connect_with(
            &SqliteConnectOptions::new().filename(project.database()),
        )
        .await
        .unwrap();
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_schema WHERE type = 'table' AND name = 'users'",
        )
        .fetch_one(&mut connection)
        .await
        .unwrap()
    });
    assert_eq!(users, 0, "the failed batch must roll back earlier DDL");

    fs::write(
        migrations.join("0002_invalid_statement.sql"),
        "CREATE TABLE roles (id INTEGER PRIMARY KEY);\n",
    )
    .unwrap();
    let applied = project.command(&["db", "migrate", "apply"]);
    assert!(
        applied.status.success(),
        "{}",
        String::from_utf8_lossy(&applied.stderr)
    );
    let repeated = project.command(&["db", "migrate", "apply"]);
    assert!(
        repeated.status.success(),
        "{}",
        String::from_utf8_lossy(&repeated.stderr)
    );
    assert!(String::from_utf8_lossy(&repeated.stdout).contains("0 migration(s) applied"));
    let rollback = project.command(&["db", "migrate", "rollback"]);
    assert!(
        !rollback.status.success(),
        "SQLite migrations have no automatic downgrade command"
    );
}

#[test]
fn concurrent_migrate_apply_processes_record_each_migration_once() {
    let project = TestProject::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(create_database(
        &project.database(),
        "CREATE TABLE bootstrap (id INTEGER PRIMARY KEY);",
    ));
    let migrations = project.path().join("db/migrations");
    fs::create_dir_all(&migrations).unwrap();
    fs::write(
        migrations.join("0001_create_concurrent.sql"),
        "CREATE TABLE concurrent_rows (id INTEGER PRIMARY KEY);\n",
    )
    .unwrap();

    let (first, second) = std::thread::scope(|scope| {
        let first = scope.spawn(|| project.command(&["db", "migrate", "apply"]));
        let second = scope.spawn(|| project.command(&["db", "migrate", "apply"]));
        (first.join().unwrap(), second.join().unwrap())
    });
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(
        second.status.success(),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );
    let applied_once = [first.stdout, second.stdout]
        .iter()
        .filter(|output| String::from_utf8_lossy(output).contains("1 migration(s) applied"))
        .count();
    assert_eq!(applied_once, 1);
}

#[test]
fn query_executes_typed_crud_join_and_count() {
    let project = TestProject::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(create_database(
        &project.database(),
        "CREATE TABLE roles (id INTEGER PRIMARY KEY, title TEXT NOT NULL); INSERT INTO roles (id, title) VALUES (1, 'Writer'); CREATE TABLE users (id INTEGER PRIMARY KEY, label TEXT NOT NULL, role_id INTEGER NOT NULL DEFAULT 1 REFERENCES roles(id)); INSERT INTO users (id, label) VALUES (1, 'Ada');",
    ));
    let introspect = project.command(&["db", "introspect"]);
    assert!(
        introspect.status.success(),
        "{}",
        String::from_utf8_lossy(&introspect.stderr)
    );
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("src/main.tyr"),
        r#"import { db } from "tyrus:db";

interface UserRow {
    id: i64;
    label: string;
}

interface JoinedRow {
    label: string;
    title: string;
}

interface CountRow {
    total: i64;
}

interface AggregateRow {
    total: i64;
    sum: i64?;
    average: f64?;
    minimum: i64?;
    maximum: i64?;
}

const main = async (): Promise<void> => {
    const inserted = await db.query("INSERT INTO users (label) VALUES (?)", ["Grace"]);
    match (inserted) {
        Ok(_) => { print("inserted"); }
        Err(error) => { printError(error.code); }
    }
    const selectedId: i64 = 2;
    const selected = await db.query<UserRow>("SELECT id, label FROM users WHERE id = ?", [selectedId]);
    match (selected) {
        Ok(rows) => { print(rows[0].label); }
        Err(error) => { printError(error.code); }
    }
    const selectedMany = await db.query<UserRow>("SELECT id, label FROM users WHERE id = ?", [selectedId]);
    match (selectedMany) {
        Ok(rows) => { print(rows.length); }
        Err(error) => { printError(error.code); }
    }
    const joined = await db.query<JoinedRow>("SELECT * FROM users JOIN roles ON users.role_id = roles.id WHERE users.id = ?", [selectedId]);
    match (joined) {
        Ok(rows) => { print(rows[0].label); print(rows[0].title); }
        Err(error) => { printError(error.code); }
    }
    const aliasedJoin = await db.query<JoinedRow>("SELECT r.title AS title, u.label AS label FROM users AS u INNER JOIN roles AS r ON u.role_id = r.id WHERE u.id = ?", [selectedId]);
    match (aliasedJoin) {
        Ok(rows) => { print(rows[0].label); print(rows[0].title); }
        Err(error) => { printError(error.code); }
    }
    const payload = "Grace'); DELETE FROM users; --";
    const updated = await db.query("UPDATE users SET label = ? WHERE id = ?", [payload, selectedId]);
    match (updated) {
        Ok(_) => { print("updated"); }
        Err(error) => { printError(error.code); }
    }
    const selectedAfterUpdate = await db.query<UserRow>("SELECT id, label FROM users WHERE id = ?", [selectedId]);
    match (selectedAfterUpdate) {
        Ok(rows) => { print(rows[0].label); }
        Err(error) => { printError(error.code); }
    }
    const firstId: i64 = 1;
    const filteredCount = await db.query<CountRow>("SELECT COUNT(*) AS total FROM users WHERE id >= ?", [firstId]);
    match (filteredCount) {
        Ok(rows) => { print(rows[0].total); }
        Err(error) => { printError(error.code); }
    }
    const totalCount = await db.query<CountRow>("SELECT COUNT(id) AS total FROM users", []);
    match (totalCount) {
        Ok(rows) => { print(rows[0].total); }
        Err(error) => { printError(error.code); }
    }
    const allAggregates = await db.query<AggregateRow>("SELECT COUNT(*) AS total, SUM(id) AS sum, AVG(id) AS average, MIN(id) AS minimum, MAX(id) AS maximum FROM users", []);
    match (allAggregates) {
        Ok(rows) => { print(rows[0].total); print(rows[0].sum ?? 0); print(rows[0].average ?? 0.0); print(rows[0].minimum ?? 0); print(rows[0].maximum ?? 0); }
        Err(error) => { printError(error.code); }
    }
    const firstRows = await db.query<UserRow>("SELECT id, label FROM users WHERE id = ?", [firstId]);
    match (firstRows) {
        Ok(rows) => { print(rows.length); }
        Err(error) => { printError(error.code); }
    }
    const deleted = await db.query("DELETE FROM users WHERE id = ?", [selectedId]);
    match (deleted) {
        Ok(_) => { print("deleted"); }
        Err(error) => { printError(error.code); }
    }
    const remainingAggregates = await db.query<AggregateRow>("SELECT COUNT(*) AS total, SUM(id) AS sum, AVG(id) AS average, MIN(id) AS minimum, MAX(id) AS maximum FROM users WHERE id = ?", [selectedId]);
    match (remainingAggregates) {
        Ok(rows) => { print(rows[0].total); print(rows[0].sum ?? 0); }
        Err(error) => { printError(error.code); }
    }
};
"#,
    )
    .unwrap();

    let output = project.command(&["run", "src/main.tyr"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        normalized_stdout(&output)
            .ends_with("inserted\nGrace\n1\nGrace\nWriter\nGrace\nWriter\nupdated\nGrace'); DELETE FROM users; --\n2\n2\n2\n3\n1.5\n1\n2\n1\ndeleted\n0\n0\n"),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    fs::write(
        project.path().join("src/main.tyr"),
        r#"import { db } from "tyrus:db";

interface AmbiguousRow {
    id: i64;
}

const main = (): void => {
    const rows = db.query<AmbiguousRow>("SELECT * FROM users JOIN roles ON users.role_id = roles.id", []);
};
"#,
    )
    .unwrap();
    let ambiguous = project.command(&["build", "src/main.tyr"]);
    assert!(!ambiguous.status.success());
    assert!(
        String::from_utf8_lossy(&ambiguous.stderr).contains("column `id` is ambiguous"),
        "{}",
        String::from_utf8_lossy(&ambiguous.stderr)
    );
}
#[test]
fn model_operations_share_explicit_sql_transaction_and_rollback() {
    let project = TestProject::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(create_database(
        &project.database(),
        "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL); INSERT INTO users (id, name) VALUES (1, 'Ada');",
    ));
    let introspect = project.command(&["db", "introspect"]);
    assert!(
        introspect.status.success(),
        "{}",
        String::from_utf8_lossy(&introspect.stderr)
    );
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::create_dir_all(project.path().join("src/models")).unwrap();
    fs::write(
        project.path().join("src/models/User.tyr"),
        r#"@model("users", { timestamps: false })
export interface User {
    @id
    id: i64;
    name: string;
}
"#,
    )
    .unwrap();
    fs::write(
        project.path().join("src/main.tyr"),
        r#"import { DbError, db } from "tyrus:db";
import { User } from "./models/User";

async function transactionScenario(): Promise<Result<boolean, DbError>> {
    const transaction = await db.beginTransaction() onfail {
        return Err(transaction);
    };
    const id: i64 = 1;
    const updated = await User.updateOne({ id }, { name: "Grace" }, { transaction }) onfail {
        return Err(updated);
    };
    print(updated?.id ?? 0);
    const rawUpdated = await db.query(
        "UPDATE users SET name = ? WHERE id = ?",
        ["Grace Byron", id],
        { transaction },
    ) onfail {
        return Err(rawUpdated);
    };
    print("raw update succeeded");
    const rawRows = await db.query<User>(
        "SELECT id, name FROM users WHERE id = ?",
        [id],
        { transaction },
    ) onfail {
        return Err(rawRows);
    };
    print(rawRows[0].name);
    const count = await User.count({ id }, { transaction }) onfail {
        return Err(count);
    };
    print(count);
    const inside = await User.findOne({ id }, { transaction }) onfail {
        return Err(inside);
    };
    print(inside?.name ?? "missing");
    const committed = await db.commitTransaction(transaction) onfail {
        printError(committed.message);
        return Err(committed);
    };
    print("committed");
    const visible = await User.findOne({ id }) onfail {
        return Err(visible);
    };
    print(visible?.name ?? "missing");
    const missingId: i64 = 2;
    const missing = await User.findOne({ id: missingId }) onfail {
        return Err(missing);
    };
    print(missing?.name ?? "missing");
    const second = await db.beginTransaction() onfail {
        return Err(second);
    };
    const rolledBackUpdate = await User.updateOne({ id }, { name: "Ada" }, { transaction: second }) onfail {
        return Err(rolledBackUpdate);
    };
    print(rolledBackUpdate?.id ?? 0);
    const rolledBack = await db.rollbackTransaction(second) onfail {
        return Err(rolledBack);
    };
    print("rolled back");
    const afterRollback = await User.findOne({ id }) onfail {
        return Err(afterRollback);
    };
    print(afterRollback?.name ?? "missing");
    return Ok(true);
}

const main = async (): Promise<void> => {
    const result = await transactionScenario();
    match (result) {
        Err(error) => { print(error.code); }
        Ok(_) => { print("scenario complete"); }
    }
};
"#,
    )
    .unwrap();

    let output = project.command(&["run", "src/main.tyr"]);
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        normalized_stdout(&output).ends_with(
                "1\nraw update succeeded\nGrace Byron\n1\nGrace Byron\ncommitted\nGrace Byron\nmissing\n1\nrolled back\nGrace Byron\nscenario complete\n"
        ),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn db_apply_runs_one_tyr_seed_and_records_its_checksum() {
    let project = TestProject::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(create_database(&project.database(), ""));
    let seeds = project.path().join("db/seeds");
    fs::create_dir_all(&seeds).unwrap();
    fs::write(seeds.join("sample_data.tyr"), r#"print("seed ran");"#).unwrap();

    let apply = project.command(&["db", "apply", "sample_data.tyr"]);
    assert!(
        apply.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&apply.stdout),
        String::from_utf8_lossy(&apply.stderr)
    );
    assert!(String::from_utf8_lossy(&apply.stdout).contains("seed ran"));
    assert!(String::from_utf8_lossy(&apply.stdout).contains("applied: sample_data"));

    let repeated = project.command(&["db", "apply", "sample_data.tyr"]);
    assert!(!repeated.status.success());
    assert!(String::from_utf8_lossy(&repeated.stderr).contains("already applied"));

    fs::write(seeds.join("sample_data.tyr"), r#"print("changed seed");"#).unwrap();
    let changed = project.command(&["db", "apply", "sample_data.tyr"]);
    assert!(!changed.status.success());
    assert!(String::from_utf8_lossy(&changed.stderr).contains("has changed"));
}

#[test]
fn db_apply_reports_changed_started_seed_as_incomplete() {
    let project = TestProject::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(create_database(
        &project.database(),
        "CREATE TABLE _tyrus_seeds (name TEXT PRIMARY KEY, checksum TEXT NOT NULL, applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP)",
    ));
    runtime.block_on(async {
        let options = SqliteConnectOptions::new().filename(project.database());
        let mut connection = SqliteConnection::connect_with(&options).await.unwrap();
        sqlx::query("INSERT INTO _tyrus_seeds (name, checksum) VALUES (?, ?)")
            .bind("sample_data")
            .bind("started:previous-checksum")
            .execute(&mut connection)
            .await
            .unwrap();
        connection.close().await.unwrap();
    });
    let seeds = project.path().join("db/seeds");
    fs::create_dir_all(&seeds).unwrap();
    fs::write(seeds.join("sample_data.tyr"), r#"print("updated seed");"#).unwrap();

    let apply = project.command(&["db", "apply", "sample_data.tyr"]);
    assert!(!apply.status.success());
    let stderr = String::from_utf8_lossy(&apply.stderr);
    assert!(stderr.contains("incomplete state"), "{stderr}");
    assert!(
        !stderr.contains("applied seed `sample_data` has changed"),
        "{stderr}"
    );
}

#[test]
fn db_apply_rejects_sql_seed_files_and_paths_outside_the_seed_directory() {
    let project = TestProject::new();
    let seeds = project.path().join("db/seeds");
    fs::create_dir_all(&seeds).unwrap();
    fs::write(seeds.join("legacy.sql"), "SELECT 1;").unwrap();

    let sql = project.command(&["db", "apply", "legacy.sql"]);
    assert!(!sql.status.success());
    assert!(String::from_utf8_lossy(&sql.stderr).contains(".tyr"));

    let traversal = project.command(&["db", "apply", "../legacy.tyr"]);
    assert!(!traversal.status.success());
    assert!(String::from_utf8_lossy(&traversal.stderr).contains("name such as"));
}

#[test]
fn db_query_result_survives_repository_and_service_return_forwarding() {
    let project = TestProject::new();
    fs::create_dir_all(project.path().join("src")).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(create_database(
        &project.database(),
        "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL);",
    ));
    let introspect = project.command(&["db", "introspect"]);
    assert!(
        introspect.status.success(),
        "{}",
        String::from_utf8_lossy(&introspect.stderr)
    );
    fs::write(
        project.path().join("src/main.tyr"),
        r#"
import { db, DbError } from "tyrus:db";

interface User {
    readonly id: i64;
    readonly name: string;
}

function findUsersRepository(): Promise<Result<User[], DbError>> {
    return db.query<User>("SELECT id, name FROM users", []);
}

function findUsersService(): Promise<Result<User[], DbError>> {
    return findUsersRepository();
}

const main = async (): Promise<void> => {
    const result = await findUsersService();
    match (result) {
        Ok(users) => { print(users.length); }
        Err(error) => { printError(error.code); }
    }
};
"#,
    )
    .unwrap();

    let output = project.command(&["run", "src/main.tyr"]);
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        normalized_stdout(&output).ends_with("0\n"),
        "the forwarded database promise must decode into an empty successful Result"
    );
}
