//! The two runtimes export one C ABI, and this test is what enforces it.
//!
//! Generated code declares a runtime function once and calls it on either
//! target. When the two crates disagree about a signature, `wasm-ld` does not
//! reject the program: it emits a *warning* and links a stub that traps if the
//! call is ever reached. Nothing failed at build time, so the divergence
//! surfaced as an "unreachable" in the browser, far from its cause. That is how
//! 122 of the 233 shared symbols came to disagree, a second hand-written copy
//! of a module drifts silently, and review does not catch it.
//!
//! The durable fix is that the runtime is one source: pure computation lives in
//! `tyrus-runtime` and is pulled into this crate with `#[path]`. This test is
//! the guard on that arrangement. It fails when
//!
//!   * a symbol both crates define disagrees on parameter or return types,
//!   * a symbol is defined twice in this crate (a copy reintroduced next to the
//!     shared original, a link error, but only after a full wasm build),
//!   * or [`KNOWN_DIVERGENCES`] names a symbol that no longer diverges.
//!
//! The last one matters most: the allowlist can only shrink. Every entry is a
//! stub whose signature was invented rather than ported, and each is a program
//! that traps instead of failing gracefully. Removing an entry, by giving the
//! stub the native signature, is progress the test then locks in.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const KNOWN_DIVERGENCES: &[&str] = &[];

/// One exported function, reduced to what the ABI actually cares about.
#[derive(Debug, PartialEq, Eq)]
struct Signature {
    file: String,
    parameters: Vec<String>,
    result: String,
}

impl Signature {
    fn render(&self) -> String {
        format!("({}) -> {}", self.parameters.join(", "), self.result)
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the crate sits two levels below the workspace root")
        .to_path_buf()
}

/// Strip the module path off a type so `crate::abi::TyrusArray` and
/// `TyrusArray` compare equal, the same type reached by two paths.
fn normalize(kind: &str) -> String {
    let collapsed = kind.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = String::with_capacity(collapsed.len());
    let mut token = String::new();
    for character in collapsed.chars() {
        if character.is_alphanumeric() || character == '_' || character == ':' {
            token.push(character);
        } else {
            out.push_str(token.rsplit("::").next().unwrap_or(&token));
            token.clear();
            out.push(character);
        }
    }
    out.push_str(token.rsplit("::").next().unwrap_or(&token));
    out
}

fn split_parameters(list: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut current = String::new();
    for character in list.chars() {
        match character {
            '<' | '(' | '[' => depth += 1,
            '>' | ')' | ']' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(std::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.push(character);
    }
    if !current.trim().is_empty() {
        parts.push(current);
    }
    parts
        .iter()
        .map(|part| normalize(part.split_once(':').map_or(part.as_str(), |(_, kind)| kind)))
        .collect()
}

/// Collect every `extern "C"` export declared in `source`.
///
/// Bodies are found by brace balance rather than by regex, so a `{` inside the
/// function does not end it early.
fn exports_in(source: &str, file: &str, into: &mut BTreeMap<String, Vec<Signature>>) {
    let bytes = source.as_bytes();
    let mut cursor = 0usize;
    while let Some(offset) = source[cursor..].find("extern \"C\" fn ") {
        let start = cursor + offset;
        cursor = start + "extern \"C\" fn ".len();
        // Only exported functions form the ABI; `extern "C" { fn .. }` blocks
        // and private helpers do not.
        let prefix = &source[start.saturating_sub(24)..start];
        if !prefix.contains("pub ") {
            continue;
        }
        let name_end = source[cursor..]
            .find(|c: char| !(c.is_alphanumeric() || c == '_'))
            .map_or(source.len(), |end| cursor + end);
        let name = source[cursor..name_end].to_string();
        // A `macro_rules!` template writes `extern "C" fn $name(..)`. The
        // expansions are shared source on both sides, so the template itself
        // carries no parity information.
        if name.is_empty() {
            continue;
        }
        let Some(open) = source[name_end..].find('(') else {
            continue;
        };
        let open = name_end + open;
        let mut depth = 0i32;
        let mut close = open;
        for (index, byte) in bytes[open..].iter().enumerate() {
            match byte {
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        close = open + index;
                        break;
                    }
                }
                _ => {}
            }
        }
        let parameters = split_parameters(&source[open + 1..close]);
        let tail = &source[close + 1..];
        let body = tail.find('{').unwrap_or(tail.len());
        let result = tail[..body]
            .trim()
            .strip_prefix("->")
            .map_or_else(|| "()".to_string(), |kind| normalize(kind.trim()));
        into.entry(name).or_default().push(Signature {
            file: file.to_string(),
            parameters,
            result,
        });
        cursor = close;
    }
}

fn exports_of(files: &[PathBuf]) -> BTreeMap<String, Vec<Signature>> {
    let mut map = BTreeMap::new();
    for path in files {
        let source = fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        exports_in(&source, name, &mut map);
    }
    map
}

fn rust_sources(directory: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = Vec::new();
    for entry in fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("cannot list {}: {error}", directory.display()))
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if path.is_dir() {
            files.extend(rust_sources(&path));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    files.sort();
    files
}

fn shared_modules(root: &Path) -> Vec<PathBuf> {
    let lib = fs::read_to_string(root.join("crates/tyrus-runtime-wasm/src/lib.rs"))
        .expect("the wasm crate has a lib.rs");
    let mut paths = Vec::new();
    for line in lib.lines() {
        let Some(rest) = line.trim().strip_prefix("#[path = \"") else {
            continue;
        };
        let Some(relative) = rest.split('"').next() else {
            continue;
        };
        paths.push(
            root.join("crates/tyrus-runtime-wasm/src")
                .join(relative)
                .canonicalize()
                .unwrap_or_else(|error| panic!("shared module {relative} is missing: {error}")),
        );
    }
    assert!(
        !paths.is_empty(),
        "no `#[path]` modules found, either the wasm crate stopped sharing native source, or this \
         test can no longer find them; both are worth failing on"
    );
    paths
}

fn wasm_exports(root: &Path) -> BTreeMap<String, Vec<Signature>> {
    let mut files = rust_sources(&root.join("crates/tyrus-runtime-wasm/src"));
    files.extend(shared_modules(root));
    exports_of(&files)
}

#[test]
fn shared_symbols_agree_on_their_signature() {
    let root = workspace_root();
    let native = exports_of(&rust_sources(&root.join("crates/tyrus-runtime/src")));
    let wasm = wasm_exports(&root);
    let allowed: BTreeSet<&str> = KNOWN_DIVERGENCES.iter().copied().collect();

    let mut problems = Vec::new();
    for (name, native_signatures) in &native {
        let Some(wasm_signatures) = wasm.get(name) else {
            continue;
        };
        if allowed.contains(name.as_str()) {
            continue;
        }
        let native_signature = &native_signatures[0];
        if wasm_signatures.iter().any(|candidate| {
            candidate.parameters != native_signature.parameters
                || candidate.result != native_signature.result
        }) {
            problems.push(format!(
                "  {name}\n    tyrus-runtime      [{}] {}\n    tyrus-runtime-wasm [{}] {}",
                native_signature.file,
                native_signature.render(),
                wasm_signatures[0].file,
                wasm_signatures[0].render(),
            ));
        }
    }

    assert!(
        problems.is_empty(),
        "{} runtime symbol(s) disagree across targets. Generated code declares each of these once \
         and calls it on both, so a mismatch links a trapping stub instead of the function. Share \
         the native module with `#[path]` rather than writing a second copy:\n{}",
        problems.len(),
        problems.join("\n")
    );
}

#[test]
fn no_symbol_is_defined_twice_in_the_wasm_runtime() {
    let wasm = wasm_exports(&workspace_root());
    let duplicates: Vec<String> = wasm
        .iter()
        .filter_map(|(name, signatures)| {
            let files: BTreeSet<&str> = signatures
                .iter()
                .map(|signature| signature.file.as_str())
                .collect();
            (files.len() > 1).then(|| {
                format!(
                    "  {name} is defined in {}",
                    files.into_iter().collect::<Vec<_>>().join(" and ")
                )
            })
        })
        .collect();

    assert!(
        duplicates.is_empty(),
        "{} symbol(s) have two definitions in the wasm runtime, which is a duplicate-symbol link \
         error that only shows up in a full wasm build:\n{}",
        duplicates.len(),
        duplicates.join("\n")
    );
}

#[test]
fn the_divergence_allowlist_has_no_stale_entries() {
    let root = workspace_root();
    let native = exports_of(&rust_sources(&root.join("crates/tyrus-runtime/src")));
    let wasm = wasm_exports(&root);

    let stale: Vec<&str> = KNOWN_DIVERGENCES
        .iter()
        .copied()
        .filter(|name| match (native.get(*name), wasm.get(*name)) {
            (Some(native_signatures), Some(wasm_signatures)) => {
                native_signatures[0].parameters == wasm_signatures[0].parameters
                    && native_signatures[0].result == wasm_signatures[0].result
            }
            _ => true,
        })
        .collect();

    assert!(
        stale.is_empty(),
        "{} allowlist entr(y/ies) no longer diverge. Delete them from KNOWN_DIVERGENCES so the \
         parity that was won stays won:\n  {}",
        stale.len(),
        stale.join("\n  ")
    );

    let mut sorted = KNOWN_DIVERGENCES.to_vec();
    sorted.sort_unstable();
    assert_eq!(
        KNOWN_DIVERGENCES,
        sorted.as_slice(),
        "keep KNOWN_DIVERGENCES sorted so entries are easy to find and remove"
    );
}
