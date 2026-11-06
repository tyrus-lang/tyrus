use super::{codegen, project};
use std::fs;
use std::io::IsTerminal;
use std::path::Path;
use std::process::{self, Command, ExitStatus};
use tyrus_core::env as tyrus_env;

pub(super) fn run_project_script(
    dir: &Path,
    name: &str,
    args: &[String],
) -> Result<Option<ExitStatus>, String> {
    if !dir.join("tyrus.json").is_file() {
        return Ok(None);
    }
    let config = project::load_project(dir)?;
    let Some(script) = config.scripts.get(name) else {
        return Ok(None);
    };
    let separator = args.iter().position(|argument| argument == "--");
    let before_separator = &args[..separator.unwrap_or(args.len())];
    let script_args = &args[separator.map_or(args.len(), |index| index + 1)..];
    if !before_separator.is_empty() {
        return Err("script arguments must follow `--`".to_string());
    }
    let status = project_script_command(script, script_args)
        .current_dir(dir)
        .status()
        .map_err(|error| format!("could not run script `{name}`: {error}"))?;
    Ok(Some(status))
}

#[cfg(windows)]
fn project_script_command(script: &str, script_args: &[String]) -> Command {
    let mut command = Command::new("cmd");
    command.arg("/D").arg("/S").arg("/C");
    let mut command_line = script.to_string();
    for argument in script_args {
        command_line.push(' ');
        command_line.push_str(&quote_windows_command_argument(argument));
    }
    command.arg(command_line);
    command
}

#[cfg(not(windows))]
fn project_script_command(script: &str, script_args: &[String]) -> Command {
    let mut command = Command::new("sh");
    command
        .arg("-c")
        .arg(format!("{script} \"$@\""))
        .arg("tyrus");
    command.args(script_args);
    command
}

#[cfg(windows)]
fn quote_windows_command_argument(argument: &str) -> String {
    format!("\"{}\"", argument.replace('"', "\"\""))
}

pub(super) fn cmd_info(dir: &Path) {
    let config = project::load_project(dir).unwrap_or_else(|e| {
        eprintln!("{e}");
        process::exit(1);
    });
    let project = config.project;
    let authors = if project.authors.is_empty() {
        "-".to_string()
    } else {
        project.authors.join(", ")
    };
    let description = if project.description.is_empty() {
        "-"
    } else {
        &project.description
    };

    println!("Name: {}", project.name);
    println!("Version: {}", project.version);
    println!("Description: {description}");
    println!("Authors: {authors}");
}

pub(super) fn cmd_init(dir: &Path) {
    let path = dir.join("tyrus.json");
    let existed = path.exists();
    let project_name = dir
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("tyrus-app");
    let contents = match fs::read_to_string(&path) {
        Ok(existing) => {
            project::merge_project_config_json(&existing, project_name).unwrap_or_else(|error| {
                eprintln!("error: {error}");
                process::exit(1);
            })
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            project::default_project_config_json(project_name)
        }
        Err(error) => {
            eprintln!("error: could not read tyrus.json: {error}");
            process::exit(1);
        }
    };
    fs::write(&path, contents).unwrap_or_else(|error| {
        eprintln!("error: could not write tyrus.json: {error}");
        process::exit(1);
    });
    println!(
        "  {} tyrus.json",
        if existed { "Updated" } else { "Created" }
    );
}

pub(super) fn cmd_default(dir: &Path) {
    let path = dir.join("tyrus.json");
    if !path.is_file() {
        eprintln!(
            "error: tyrus.json not found in `{}`; run `tyrus --init` to create it",
            dir.display()
        );
        process::exit(1);
    }
    let project_name = dir
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("tyrus-app");
    fs::write(&path, project::default_project_config_json(project_name)).unwrap_or_else(|error| {
        eprintln!("error: could not reset tyrus.json: {error}");
        process::exit(1);
    });
    println!("  Reset tyrus.json to defaults");
}

pub(super) fn parse_compile_target(args: &[String]) -> codegen::CompileTarget {
    let value = args
        .windows(2)
        .find(|w| w[0] == "--compile-target")
        .map(|w| w[1].as_str());
    match value {
        Some("wasm") => codegen::CompileTarget::Wasm,
        Some(other) => {
            eprintln!("error: unknown compile target `{other}` (expected `native` or `wasm`)");
            process::exit(1);
        }
        None => codegen::CompileTarget::Native,
    }
}

#[derive(Clone, Copy)]
pub(super) enum Target {
    Script,
    Cli,
    Api,
}

#[derive(Clone, Copy)]
pub(super) enum NewDatabase {
    None,
    Sqlite,
    Postgresql,
    Mysql,
    Mongodb,
}

impl NewDatabase {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "none" => Some(Self::None),
            "sqlite" => Some(Self::Sqlite),
            "postgresql" => Some(Self::Postgresql),
            "mysql" => Some(Self::Mysql),
            "mongodb" => Some(Self::Mongodb),
            _ => None,
        }
    }

    fn connection_backend(self) -> Option<&'static str> {
        match self {
            Self::Postgresql => Some("postgresql"),
            Self::Mysql => Some("mysql"),
            Self::Mongodb => Some("mongodb"),
            Self::None | Self::Sqlite => None,
        }
    }

    fn project_config(self) -> Option<serde_json::Value> {
        match self {
            Self::None => None,
            Self::Sqlite => Some(serde_json::json!({
                "backend": "sqlite",
                "path": "./data/app.db",
                "sqlite": {
                    "journalMode": "wal",
                    "synchronous": "full",
                    "busyTimeoutMs": 5000
                }
            })),
            Self::Postgresql | Self::Mysql | Self::Mongodb => Some(serde_json::json!({
                "backend": self.connection_backend().unwrap(),
                "connectionEnv": "DATABASE_URL"
            })),
        }
    }
}

impl Target {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Target::Script => "script",
            Target::Cli => "cli",
            Target::Api => "api",
        }
    }

    pub(super) fn parse(value: &str) -> Option<Target> {
        match value {
            "script" => Some(Target::Script),
            "cli" => Some(Target::Cli),
            "api" => Some(Target::Api),
            _ => None,
        }
    }
}

pub(super) fn validate_project_name(name: &str) -> Result<(), &'static str> {
    let name = name.trim();
    if name.is_empty() {
        return Err("the project name cannot be empty");
    }
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        return Err("the project name cannot start with a digit");
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err("the project name may only contain letters, digits, `-` and `_`");
    }
    Ok(())
}

pub(super) fn resolve_project_name(name_arg: Option<&str>) -> String {
    if let Some(name) = name_arg {
        if let Err(reason) = validate_project_name(name) {
            eprintln!("error: {reason}");
            process::exit(1);
        }
        return name.to_string();
    }
    if !std::io::stdin().is_terminal() {
        eprintln!(
            "error: missing project name\nUsage: tyrus new <name> [--target script|cli|api] [--database none|sqlite|postgresql|mysql|mongodb]"
        );
        process::exit(1);
    }
    inquire::Text::new("Project name?")
        .with_validator(|input: &str| match validate_project_name(input) {
            Ok(()) => Ok(inquire::validator::Validation::Valid),
            Err(reason) => Ok(inquire::validator::Validation::Invalid(reason.into())),
        })
        .prompt()
        .unwrap_or_else(|_| {
            eprintln!("error: project creation cancelled");
            process::exit(1);
        })
}

pub(super) fn resolve_target(target_arg: Option<&str>) -> Target {
    if let Some(target) = target_arg {
        return Target::parse(target).unwrap_or_else(|| {
            eprintln!("error: unknown target `{target}` (expected script, cli or api)");
            process::exit(1);
        });
    }
    if !std::io::stdin().is_terminal() {
        return Target::Script;
    }
    let choice = inquire::Select::new("Target?", vec!["script", "cli", "api"])
        .with_starting_cursor(0)
        .with_help_message(
            "script: a plain program · cli: interactive prompts · api: an HTTP server",
        )
        .prompt()
        .unwrap_or_else(|_| {
            eprintln!("error: project creation cancelled");
            process::exit(1);
        });
    Target::parse(choice).unwrap_or(Target::Script)
}

pub(super) fn resolve_new_database(target: Target, database_arg: Option<&str>) -> NewDatabase {
    if let Some(value) = database_arg {
        return NewDatabase::parse(value).unwrap_or_else(|| {
            eprintln!("error: unknown database `{value}` (expected none, sqlite, postgresql, mysql or mongodb)");
            process::exit(1);
        });
    }
    if !matches!(target, Target::Api) || !std::io::stdin().is_terminal() {
        return NewDatabase::None;
    }
    match inquire::Select::new(
        "Database?",
        vec!["none", "sqlite", "postgresql", "mysql", "mongodb"],
    )
    .with_starting_cursor(0)
    .with_help_message("Choose a backend now, or add database configuration later.")
    .prompt()
    {
        Ok(value) => NewDatabase::parse(value).unwrap_or(NewDatabase::None),
        Err(_) => {
            eprintln!("error: project creation cancelled");
            process::exit(1);
        }
    }
}

pub(super) fn cmd_new(
    name_arg: Option<&str>,
    target_arg: Option<&str>,
    database_arg: Option<&str>,
) {
    let name = resolve_project_name(name_arg);
    let name = name.trim();
    let target = resolve_target(target_arg);
    let database = resolve_new_database(target, database_arg);
    let root = Path::new(name);

    if root.exists() {
        eprintln!("error: directory `{name}` already exists");
        process::exit(1);
    }

    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("target")).unwrap();

    let project_config = project::project_config_json_with_database(
        name,
        Some(target.as_str()),
        database.project_config(),
    );
    fs::write(root.join("tyrus.json"), project_config).unwrap();
    if matches!(database, NewDatabase::Sqlite) {
        fs::create_dir_all(root.join("data")).unwrap();
    }

    let mut env = concat!(
        "# Every variable needs a `@type` and a value. Validator Decorations may be raw\n",
        "# (`@type(u32)`) or commented (`# @type(u32)`); Tyrus accepts both forms.\n",
        "# Use the commented form when another .env loader, e.g. Docker, reads this file.\n",
        "@type(string)\n",
        "@in([\"development\", \"production\", \"test\", \"staging\"])\n",
        "TYRUS_ENV=development\n",
    )
    .to_string();
    if database.connection_backend().is_some() {
        env.push_str("\n@type(string)\nDATABASE_URL=\n");
    }
    fs::write(root.join(".env"), &env).unwrap();
    fs::write(
        root.join(".env.example"),
        tyrus_env::render_env_example(&env),
    )
    .unwrap();

    fs::write(
        root.join("tyrus-fmt.json"),
        tyrus_fmt::config::default_config_json(),
    )
    .unwrap();

    fs::write(
        root.join("tyrus-lint.json"),
        match target {
            Target::Api => api_lint_config_json(),
            Target::Script | Target::Cli => tyrus_lint::config::default_config_json(),
        },
    )
    .unwrap();

    write_target_sources(root, target);
    write_test_scaffold(root, target);

    fs::write(
        root.join(".gitignore"),
        "target/\n.env\n.env.*\n!.env*.example\n",
    )
    .unwrap();

    println!("  Created project `{name}` ({} target)", target.as_str());
    println!();
    println!("  Get started:");
    println!("    cd {name}");
    println!("    tyrus run");
}

pub(super) fn write_test_scaffold(root: &Path, target: Target) {
    fs::create_dir_all(root.join("__tests__")).unwrap();
    let header = concat!(
        "// Any file named `*.test.tyr` is a test, wherever it is in the\n",
        "// project, and `tyrus test` runs it. `__tests__/` is the conventional\n",
        "// folder; a test may also sit next to the code it covers.\n",
    );
    let body = match target {
        Target::Script => concat!(
            "import { describe, it, expect } from \"tyrus:test\";\n",
            "\n",
            "const total = (prices: i32[]): i32 => {\n",
            "    let sum: i32 = 0;\n",
            "    for (const price of prices) {\n",
            "        sum += price;\n",
            "    }\n",
            "    return sum;\n",
            "};\n",
            "\n",
            "describe(\"total\", () => {\n",
            "    it(\"adds the prices\", () => {\n",
            "        expect(total([10, 20, 12])).toBe(42);\n",
            "    });\n",
            "});\n",
        ),
        Target::Cli | Target::Api => concat!(
            "import { describe, it, expect } from \"tyrus:test\";\n",
            "\n",
            "const total = (prices: i32[]): i32 => {\n",
            "    let sum: i32 = 0;\n",
            "    for (const price of prices) {\n",
            "        sum += price;\n",
            "    }\n",
            "    return sum;\n",
            "};\n",
            "\n",
            "describe(\"total\", () => {\n",
            "    it(\"adds the prices\", () => {\n",
            "        expect(total([10, 20, 12])).toBe(42);\n",
            "    });\n",
            "});\n",
        ),
    };
    let name = match target {
        Target::Script => "__tests__/example.test.tyr",
        Target::Cli | Target::Api => "__tests__/example.test.tyr",
    };
    fs::write(root.join(name), format!("{header}{body}")).unwrap();
}

pub(super) fn write_target_sources(root: &Path, target: Target) {
    match target {
        Target::Script => {
            fs::write(
                root.join("src/main.tyr"),
                "const main = () => {\n    print(\"Hello World!\");\n};\n",
            )
            .unwrap();
        }
        Target::Cli => {
            fs::create_dir_all(root.join("src/prompts")).unwrap();
            fs::write(
                root.join("src/prompts/setup.tyr"),
                concat!(
                    "import { input, select, multiselect, confirm, integer } from \"tyrus:cli\";\n",
                    "\n",
                    "// Interactive setup flow. Each prompt takes an options object;\n",
                    "// the prompt UI renders on stderr, so stdout stays clean.\n",
                    "export const runSetup = (): void => {\n",
                    "    const name: string = input({\n",
                    "        message: \"Project name?\",\n",
                    "        default: \"my-app\",\n",
                    "        minLength: 1,\n",
                    "    });\n",
                    "    const target: string = select({\n",
                    "        message: \"Which target?\",\n",
                    "        choices: [\"script\", \"cli\", \"api\"],\n",
                    "        default: \"cli\",\n",
                    "    });\n",
                    "    const features: string[] = multiselect({\n",
                    "        message: \"Enable which features?\",\n",
                    "        choices: [\"tyrus-lint\", \"tests\", \"ci\"],\n",
                    "        required: true,\n",
                    "    });\n",
                    "    const port: i64 = integer({\n",
                    "        message: \"Port?\",\n",
                    "        default: 3000,\n",
                    "        min: 1,\n",
                    "        max: 65535,\n",
                    "    });\n",
                    "    const git: boolean = confirm({\n",
                    "        message: \"Initialize a git repository?\",\n",
                    "        default: true,\n",
                    "    });\n",
                    "    console.log(\"name:\");\n",
                    "    console.log(name);\n",
                    "    console.log(\"target:\");\n",
                    "    console.log(target);\n",
                    "    console.log(\"features:\");\n",
                    "    console.log(features);\n",
                    "    console.log(\"port:\");\n",
                    "    console.log(port);\n",
                    "    console.log(\"git:\");\n",
                    "    console.log(git);\n",
                    "};\n",
                ),
            )
            .unwrap();
            fs::write(
                root.join("src/main.tyr"),
                concat!(
                    "import { runSetup } from \"./prompts/setup\";\n",
                    "\n",
                    "const main = (): void => {\n",
                    "    runSetup();\n",
                    "};\n",
                ),
            )
            .unwrap();
        }
        Target::Api => {
            fs::create_dir_all(root.join("src/routes")).unwrap();
            fs::create_dir_all(root.join("src/controllers")).unwrap();
            fs::create_dir_all(root.join("src/services")).unwrap();
            fs::create_dir_all(root.join("src/repositories")).unwrap();
            fs::create_dir_all(root.join("src/models")).unwrap();
            fs::create_dir_all(root.join("src/schemas")).unwrap();
            fs::write(
                root.join("src/main.tyr"),
                concat!(
                    "import { createRouter, HttpError, HttpRouter } from \"tyrus:http\";\n",
                    "\n",
                    "const main = (): void => {\n",
                    "    const app: HttpRouter = createRouter();\n",
                    "    try {\n",
                    "        app.listen(3000);\n",
                    "    } catch (error: HttpError) {\n",
                    "        console.error(`${error.code}: ${error.message}`);\n",
                    "    }\n",
                    "};\n",
                ),
            )
            .unwrap();
            fs::write(
                root.join("src/routes/user.tyr"),
                concat!(
                    "import { HttpRouter } from \"tyrus:http\";\n",
                    "import { createUserController, getUserController } from \"../controllers/user\";\n",
                    "import { CreateUserBody, CreateUserRequest, GetUserRequest, UserParams, UserResponse } from \"../schemas/user\";\n",
                    "\n",
                    "export default (router: HttpRouter): void => {\n",
                    "    router.get(\"/users/:id\", {\n",
                    "        params: UserParams,\n",
                    "        response: { 200: UserResponse },\n",
                    "        handler: getUserController,\n",
                    "    });\n",
                    "\n",
                    "    router.post(\"/users\", {\n",
                    "        body: CreateUserBody,\n",
                    "        response: { 201: UserResponse },\n",
                    "        handler: createUserController,\n",
                    "    });\n",
                    "};\n",
                ),
            )
            .unwrap();
            fs::write(
                root.join("src/controllers/user.tyr"),
                concat!(
                    "import { HttpReply, HttpStatus } from \"tyrus:http\";\n",
                    "import { createUserService, getUserService } from \"../services/user\";\n",
                    "import { CreateUserBody, CreateUserRequest, GetUserRequest, UserParams, UserResponse } from \"../schemas/user\";\n",
                    "\n",
                    "export function getUserController(request: GetUserRequest, reply: HttpReply<UserResponse>): void {\n",
                    "    const user: UserResponse = getUserService(request.params.id);\n",
                    "    reply.status(HttpStatus.Ok).send(user);\n",
                    "}\n",
                    "\n",
                    "export function createUserController(request: CreateUserRequest, reply: HttpReply<UserResponse>): void {\n",
                    "    const user: UserResponse = createUserService(request.body);\n",
                    "    reply.status(HttpStatus.Created).send(user);\n",
                    "}\n",
                ),
            )
            .unwrap();
            fs::write(
                root.join("src/services/user.tyr"),
                concat!(
                    "import { User } from \"../models/user\";\n",
                    "import { createUserRepository, findUserByIdRepository } from \"../repositories/user\";\n",
                    "import { CreateUserBody, UserResponse } from \"../schemas/user\";\n",
                    "\n",
                    "export const getUserService = (id: string): UserResponse => {\n",
                    "    const user: User = findUserByIdRepository(id);\n",
                    "    return { id: user.id, name: user.name };\n",
                    "};\n",
                    "\n",
                    "export const createUserService = (body: CreateUserBody): UserResponse => {\n",
                    "    const user: User = createUserRepository(body);\n",
                    "    return { id: user.id, name: user.name };\n",
                    "};\n",
                ),
            )
            .unwrap();
            fs::write(
                root.join("src/repositories/user.tyr"),
                concat!(
                    "import { User } from \"../models/user\";\n",
                    "import { CreateUserBody } from \"../schemas/user\";\n",
                    "\n",
                    "export const findUserByIdRepository = (id: string): User => {\n",
                    "    return { id: id, name: \"Ada Lovelace\" };\n",
                    "};\n",
                    "\n",
                    "export const createUserRepository = (body: CreateUserBody): User => {\n",
                    "    return { id: \"user-1\", name: body.name };\n",
                    "};\n",
                ),
            )
            .unwrap();
            fs::write(
                root.join("src/models/user.tyr"),
                concat!(
                    "export interface User {\n",
                    "    id: string;\n",
                    "    name: string;\n",
                    "}\n",
                ),
            )
            .unwrap();
            fs::write(
                root.join("src/schemas/user.tyr"),
                concat!(
                    "export interface UserParams {\n",
                    "    id: string;\n",
                    "}\n",
                    "\n",
                    "export interface GetUserRequest {\n",
                    "    readonly params: UserParams;\n",
                    "}\n",
                    "\n",
                    "export interface CreateUserBody {\n",
                    "    @minLength(1)\n",
                    "    name: string;\n",
                    "}\n",
                    "\n",
                    "export interface CreateUserRequest {\n",
                    "    readonly body: CreateUserBody;\n",
                    "}\n",
                    "\n",
                    "export interface UserResponse {\n",
                    "    id: string;\n",
                    "    name: string;\n",
                    "}\n",
                ),
            )
            .unwrap();
        }
    }
}

pub(super) fn api_lint_config_json() -> String {
    concat!(
        "{\n",
        "    \"api-no-verb-in-path\": \"warn\",\n",
        "    \"api-prefer-resource-nouns\": \"warn\",\n",
        "    \"controller-must-send-response\": \"error\",\n",
        "    \"controller-no-db-access\": \"error\",\n",
        "    \"controller-no-try-catch\": \"warn\",\n",
        "    \"http-no-raw-status-code\": \"warn\",\n",
        "    \"no-layer-skip\": \"warn\",\n",
        "    \"no-upward-imports\": \"error\",\n",
        "    \"repository-no-http-error\": \"warn\",\n",
        "    \"repository-only-db-access\": \"error\",\n",
        "    \"route-no-business-logic\": \"warn\",\n",
        "    \"service-no-reply\": \"error\"\n",
        "}\n",
    )
    .to_string()
}
