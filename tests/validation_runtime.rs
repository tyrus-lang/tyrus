use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Tells this binary's scratch projects apart. A name built only from a
/// timestamp collided whenever two tests reached it inside the same tick, and
/// since `Drop` removes the whole directory, the loser's binary was gone by the
/// time it tried to run: `NotFound` from a build that had just succeeded. The
/// process id and this counter make the name unique by construction, and the
/// timestamp stays to keep a directory left behind by a crashed run, under a
/// process id the system later reused, from being adopted as if it were ours.
static PROJECT_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// One validation operation reports every rule the value broke, including the
/// rules broken inside the objects it contains.
///
/// A nested object literal used to be an operation of its own, and it threw as
/// soon as it failed. The fields the outer object declared before it had been
/// evaluated but never checked, so `{ title, members: [...] }` with a bad title
/// *and* a bad zip code reported only the zip code, and the aggregation the
/// README promises held just until the first nesting.
#[test]
fn one_operation_reports_every_rule_broken_inside_the_value() {
    let suffix = format!(
        "{}-{}-{}",
        std::process::id(),
        PROJECT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let root = std::env::temp_dir().join(format!("tyrus-validation-nested-{suffix}"));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("tyrus.json"),
        r#"{
  "project": {
    "name": "validation-nested-test",
    "version": "0.1.0",
    "authors": [],
    "description": "Aggregation across nested objects and arrays"
  },
  "dependencies": {}
}
"#,
    )
    .unwrap();
    fs::write(
        root.join("src/main.tyr"),
        r#"interface Address {
    @minLength(5)
    zipCode: string;
}

interface Profile {
    @minLength(3)
    name: string;
    address: Address;
}

interface Team {
    @minLength(4)
    title: string;
    members: Profile[];
}

const short = (): string => "a";

const main = (): void => {
    try {
        const team: Team = {
            title: short(),
            members: [{ name: short(), address: { zipCode: short() } }],
        };
        console.log(team.title);
    } catch (error: ValidationError) {
        console.log("issues:", error.issues.length);
        for (const issue of error.issues) {
            console.log(issue.target);
        }
    }
};
"#,
    )
    .unwrap();

    let build = Command::new(env!("CARGO_BIN_EXE_tyrus"))
        .arg("build")
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let output = Command::new(root.join("target/debug/validation-nested-test"))
        .output()
        .unwrap();
    assert!(output.status.success());
    // Three issues, in the order the fields are written: the outer field first,
    // then the ones inside the array element.
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "issues: 3\ntitle\nmembers[0].name\nmembers[0].address.zipCode\n"
    );
    assert!(output.stderr.is_empty());

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn aggregates_parameter_and_object_validation_issues() {
    let suffix = format!(
        "{}-{}-{}",
        std::process::id(),
        PROJECT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let root = std::env::temp_dir().join(format!("tyrus-validation-aggregate-{suffix}"));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("tyrus.json"),
        r#"{
  "project": {
    "name": "validation-aggregate-test",
    "version": "0.1.0",
    "authors": [],
    "description": "Validation aggregation integration test"
  },
  "dependencies": {}
}
"#,
    )
    .unwrap();
    fs::write(
        root.join("src/main.tyr"),
        r#"interface SignupInput {
    @minLength(3)
    @startsWith("user-")
    username: string;

    @email()
    email: string;
}

interface Address {
    @minLength(5)
    zipCode: string;
}

interface Profile {
    address: Address;
}

interface Team {
    members: Profile[];
}

const readUsername = (): string => "a";
const readEmail = (): string => "invalid";

const validateParameters = (@minLength(3) @startsWith("user-") username: string, @email() email: string): void => {
    console.log(username);
    console.log(email);
};

const collectIssues = (): readonly ValidationIssue[] => {
    try {
        validateParameters(readUsername(), readEmail());
    } catch (error: ValidationError) {
        return error.issues;
    }
    return [];
};

const main = (): void => {
    try {
        validateParameters(readUsername(), readEmail());
    } catch (error: ValidationError) {
        console.log("parameters:", error.issues.length);
    }

    try {
        const input: SignupInput = {
            username: readUsername(),
            email: readEmail(),
        };
        console.log(input.username);
    } catch (error: ValidationError) {
        console.log("object:", error.issues.length);
        console.log(error.issues[0].target);
        console.log(error.issues[1].target);
        console.log(error.issues[2].target);
    }

    const valid: SignupInput = {
        username: "user-valid",
        email: "valid@example.com",
    };
    try {
        const updated: SignupInput = {
            ...valid,
            username: readUsername(),
            email: readEmail(),
        };
        console.log(updated.username);
    } catch (error: ValidationError) {
        console.log("update:", error.issues.length);
    }

    const escaped: readonly ValidationIssue[] = collectIssues();
    console.log("escaped:", escaped.length);

    try {
        const profile: Profile = {
            address: {
                zipCode: readUsername(),
            },
        };
        console.log(profile.address.zipCode);
    } catch (error: ValidationError) {
        console.log("nested:", error.issues[0].target);
    }

    try {
        const team: Team = {
            members: [{
                address: {
                    zipCode: readUsername(),
                },
            }],
        };
        console.log(team.members.length);
    } catch (error: ValidationError) {
        console.log("array:", error.issues[0].target);
    }
};
"#,
    )
    .unwrap();

    let build = Command::new(env!("CARGO_BIN_EXE_tyrus"))
        .arg("build")
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let output = Command::new(root.join("target/debug/validation-aggregate-test"))
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "parameters: 3\nobject: 3\nusername\nusername\nemail\nupdate: 3\nescaped: 3\nnested: address.zipCode\narray: members[0].address.zipCode\n"
    );
    assert!(output.stderr.is_empty());

    fs::remove_dir_all(root).unwrap();
}

/// A value nested under a generic type argument is still validated.
///
/// Accepting `Boxed<Inner> = { label, value: { n } }`, which the checker used
/// to reject, must not accept it *unvalidated*: the literal filling `value: T`
/// is an `Inner`, and `Inner` declares `@min(10)`. The guarantee is that the
/// substitution changes what the checker allows, never what the program checks.
#[test]
fn a_value_nested_under_a_generic_type_argument_is_validated() {
    let suffix = format!(
        "{}-{}-{}",
        std::process::id(),
        PROJECT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let root = std::env::temp_dir().join(format!("tyrus-validation-generic-{suffix}"));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("tyrus.json"),
        r#"{
  "project": {
    "name": "validation-generic-test",
    "version": "0.1.0",
    "authors": [],
    "description": "Validation through a generic type argument"
  },
  "dependencies": {}
}
"#,
    )
    .unwrap();
    fs::write(
        root.join("src/main.tyr"),
        r#"interface Inner {
    @min(10)
    n: i32;
}

interface Boxed<T> {
    label: string;
    value: T;
}

const small = (): i32 => 1;

const main = (): void => {
    try {
        const boxed: Boxed<Inner> = { label: "okay", value: { n: small() } };
        console.log(boxed.value.n);
    } catch (error: ValidationError) {
        console.log("issues:", error.issues.length);
        console.log(error.issues[0].target);
    }
};
"#,
    )
    .unwrap();

    let build = Command::new(env!("CARGO_BIN_EXE_tyrus"))
        .arg("build")
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let output = Command::new(root.join("target/debug/validation-generic-test"))
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "issues: 1\nvalue.n\n"
    );
    assert!(output.stderr.is_empty());

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn custom_interface_validators_run_in_order_and_collect_issues() {
    let suffix = format!(
        "{}-{}-{}",
        std::process::id(),
        PROJECT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let root = std::env::temp_dir().join(format!("tyrus-custom-validator-{suffix}"));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("tyrus.json"),
        r#"{
  "project": {
    "name": "custom-validator-test",
    "version": "0.1.0",
    "authors": [],
    "description": "Custom interface field validators"
  },
  "dependencies": {}
}
"#,
    )
    .unwrap();
    fs::write(
        root.join("src/validators.tyr"),
        r#"type Validator = { isValid: true } | { isValid: false; message: string };

export validator function startsWithUser(value: string, constraint: string): Validator {
    if (!value.startsWith(constraint)) {
        return { isValid: false, message: "Username must start with the configured prefix" };
    }
    return { isValid: true };
}

export validator const hasNoSpaces = (value: string): Validator => {
    if (value.includes(" ")) {
        return { isValid: false, message: "Username cannot contain spaces" };
    }
    return { isValid: true };
};

export validator function atLeast(value: u32, constraint: u32): Validator {
    if (value < constraint) {
        return { isValid: false, message: `Value must be at least ${constraint}` };
    }
    return { isValid: true };
}
"#,
    )
    .unwrap();
    fs::write(
        root.join("src/main.tyr"),
        r#"import {
    hasNoSpaces as noSpaces,
    startsWithUser as hasUserPrefix,
    atLeast,
} from "./validators";

function checkNamedParameter(@noSpaces() value: string): string {
    return value;
}

const checkArrowParameter = (@hasUserPrefix("user_") value: string): string => value;

interface Account {
    @hasUserPrefix("user_")
    @noSpaces()
    username: string;
    @noSpaces()
    displayName?: string;
    @atLeast(5)
    age: u32;
}

const readValidUsername = (): string => "user_ada";
const readInvalidUsername = (): string => "guest user";

const main = (): void => {
    try {
        const validName: string = readValidUsername();
        const valid: Account = { username: validName, displayName: "Ada", age: 8 };
        print(valid.username);
    } catch (_error: ValidationError) {
        printError("unexpected validation failure");
    }

    try {
        checkNamedParameter("bad name");
    } catch (error: ValidationError) {
        print("named parameter:", error.issues[0].validator);
        print(error.issues[0].message);
    }

    try {
        checkArrowParameter("guest");
    } catch (error: ValidationError) {
        print("arrow parameter:", error.issues[0].validator);
        print(error.issues[0].message);
    }

    try {
        const invalidName: string = readInvalidUsername();
        const invalid: Account = { username: invalidName, age: 4 };
        print(invalid.username);
    } catch (error: ValidationError) {
        print(error.issues.length);
        print(error.issues[0].target);
        print(error.issues[0].validator);
        print(error.issues[0].message);
        print(error.issues[1].validator);
        print(error.issues[1].message);
        print(error.issues[2].target);
        print(error.issues[2].validator);
        print(error.issues[2].message);
    }
};
"#,
    )
    .unwrap();

    let build = Command::new(env!("CARGO_BIN_EXE_tyrus"))
        .arg("build")
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let output = Command::new(root.join("target/debug/custom-validator-test"))
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "user_ada\nnamed parameter: @noSpaces()\nUsername cannot contain spaces\narrow parameter: @hasUserPrefix(\"user_\")\nUsername must start with the configured prefix\n3\nusername\n@hasUserPrefix(\"user_\")\nUsername must start with the configured prefix\n@noSpaces()\nUsername cannot contain spaces\nage\n@atLeast(5)\nValue must be at least 5\n"
    );
    assert!(output.stderr.is_empty());

    fs::remove_dir_all(root).unwrap();
}
