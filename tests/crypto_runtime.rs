//! Native end-to-end coverage for `tyrus:crypto` operations.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

/// Tells this binary's scratch projects apart. A name built only from a
/// timestamp collided whenever two tests reached it inside the same tick, and
/// since `Drop` removes the whole directory, the loser's binary was gone by the
/// time it tried to run: `NotFound` from a build that had just succeeded. The
/// process id and this counter make the name unique by construction, and the
/// timestamp stays to keep a directory left behind by a crashed run, under a
/// process id the system later reused, from being adopted as if it were ours.
static PROJECT_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

struct CryptoProject {
    root: PathBuf,
    name: String,
    release: std::sync::atomic::AtomicBool,
}

impl CryptoProject {
    fn new(source: &str) -> Self {
        let suffix = format!(
            "{}-{}-{}",
            std::process::id(),
            PROJECT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let name = format!("crypto-runtime-{suffix}");
        let root = std::env::temp_dir().join(&name);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("tyrus.json"),
            format!(
                r#"{{
  "project": {{
    "name": "{name}",
    "version": "0.1.0",
    "authors": [],
    "description": "Crypto runtime integration test"
  }},
  "security": {{
    "envFile": ".env",
    "envSealed": false
  }},
  "dependencies": {{}}
}}
"#
            ),
        )
        .unwrap();
        fs::write(root.join(".env"), "").unwrap();
        fs::write(root.join("src/main.tyr"), source).unwrap();
        Self {
            root,
            name,
            release: std::sync::atomic::AtomicBool::new(false),
        }
    }

    fn build(&self, release: bool) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_tyrus"));
        command.arg("build");
        if release {
            command.arg("--release");
        }
        self.release
            .store(release, std::sync::atomic::Ordering::Relaxed);
        command.current_dir(&self.root).output().unwrap()
    }

    fn run(&self) -> Output {
        let profile = if self.release.load(std::sync::atomic::Ordering::Relaxed) {
            "release"
        } else {
            "debug"
        };
        Command::new(self.root.join("target").join(profile).join(&self.name))
            .output()
            .unwrap()
    }
}

impl Drop for CryptoProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn synchronous_primitives_survive_native_codegen_and_runtime() {
    let project = CryptoProject::new(
        r#"import { cipher, CryptoError, digest, hmac, jwt, rand, timingSafeEqual } from "tyrus:crypto";



import { EncodingError, utf8 } from "tyrus:encoding";
function cryptoString(result: Result<string, CryptoError>): string {
    match (result) {
        Ok(value) => { return value; }
        Err(error) => { printError(error.code); return ""; }
    }
}

function cryptoBytes(result: Result<u8[], CryptoError>): u8[] {
    match (result) {
        Ok(value) => { return value; }
        Err(error) => { printError(error.code); return []; }
    }
}

function check(value: boolean): void {
    console.log(value);
    console.log("assert:", true);
}

function exerciseRepeatedCleanup(input: u8[]): void {
    for (let index: i32 = 0; index < 1024; index++) {
        const repeatedDigest: u8[] = cryptoBytes(digest.sha256(input));
        const repeatedMac: u8[] = cryptoBytes(hmac.sha256(input, input));
        timingSafeEqual(repeatedDigest, repeatedMac);
    }
}

function runChecks(): Result<void, CryptoError> {
    const text: string = "cryptographic ✓";
    const encoded: u8[] = utf8.encode(text);
    const decodeResult: Result<string, EncodingError> = utf8.decode(encoded);
    match (decodeResult) {
        Ok(decoded) => { check(timingSafeEqual(decoded, text)); }
        Err(error) => { console.error(error.code); }
    }

    const sha256: u8[] = cryptoBytes(digest.sha256(text));
    const sha384: u8[] = cryptoBytes(digest.sha384(encoded));
    const sha512: u8[] = cryptoBytes(digest.sha512(text));
    const md5: u8[] = cryptoBytes(digest.md5(text));
    check(sha256.length === 32);
    check(sha384.length === 48);
    check(sha512.length === 64);
    check(md5.length === 16);
    exerciseRepeatedCleanup(sha256);

    const key: u8[] = [
        0, 1, 2, 3, 4, 5, 6, 7,
        8, 9, 10, 11, 12, 13, 14, 15,
        16, 17, 18, 19, 20, 21, 22, 23,
        24, 25, 26, 27, 28, 29, 30, 31,
    ];
    const mac256: u8[] = cryptoBytes(hmac.sha256(text, key));
    check(mac256.length === 32);
    const mac384: u8[] = cryptoBytes(hmac.sha384(encoded, key));
    const mac512: u8[] = cryptoBytes(hmac.sha512(text, key));
    check(mac384.length === 48);
    check(mac512.length === 64);
    const sameMac: u8[] = cryptoBytes(hmac.sha256(text, key));
    const differentMac: u8[] = cryptoBytes(hmac.sha256("different", key));
    check(timingSafeEqual(mac256, sameMac));
    check(!timingSafeEqual(mac256, differentMac));
    const shortA: u8[] = [1];
    const shortB: u8[] = [1, 2];
    check(!timingSafeEqual(shortA, shortB));

    const stream: DigestStream = digest.create("sha256")?;
    stream.update("crypto")?;
    const streamSuffix: u8[] = utf8.encode(" stream");
    stream.update(streamSuffix)?;
    const streamed: u8[] = stream.digest()?;
    check(streamed.length === 32);
    stream.reset()?;
    stream.update("crypto stream")?;
    const streamedAgain: u8[] = stream.digest()?;
    check(timingSafeEqual(streamed, streamedAgain));
    stream.dispose();

    match (cipher.aes.encrypt(text, key)) {
        Ok(encrypted) => {
            match (cipher.aes.decryptText(encrypted, key)) {
                Ok(decrypted) => { check(timingSafeEqual(decrypted, text)); }
                Err(error) => { check(false); }
            }
        }
        Err(error) => { check(false); }
    }
    const binary: u8[] = [0, 127, 128, 255];
    match (cipher.aes.encrypt(binary, key)) {
        Ok(encryptedBinary) => {
            match (cipher.aes.decryptBytes(encryptedBinary, key)) {
                Ok(decryptedBinary) => { check(timingSafeEqual(decryptedBinary, binary)); }
                Err(error) => { check(false); }
            }
        }
        Err(error) => { check(false); }
    }
    const aad: u8[] = utf8.encode("record:42");
    match (cipher.aes.encryptWithAad(text, key, aad)) {
        Ok(protectedEnvelope) => {
            match (cipher.aes.decryptTextWithAad(protectedEnvelope, key, aad)) {
                Ok(decryptedWithAad) => { check(timingSafeEqual(decryptedWithAad, text)); }
                Err(error) => { check(false); }
            }
            const wrongAad: u8[] = utf8.encode("record:43");
            match (cipher.aes.decryptTextWithAad(protectedEnvelope, key, wrongAad)) {
                Ok(value) => { check(false); }
                Err(error) => { check(timingSafeEqual(error.code, "ERR_CRYPTO_DECRYPTION_FAILED")); }
            }
            match (cipher.aes.decryptBytesWithAad(protectedEnvelope, key, aad)) {
                Ok(value) => { check(value.length > 0); }
                Err(error) => { check(false); }
            }
        }
        Err(error) => { check(false); }
    }

    match (rand.randomBytes(32)) {
        Ok(randomBytes) => { check(randomBytes.length === 32); }
        Err(error) => { check(false); }
    }
    match (rand.randomInt(-5, 5)) {
        Ok(integer) => { check(integer >= -5 && integer <= 5); }
        Err(error) => { check(false); }
    }
    const fraction: f64 = rand.randomFloat();
    check(fraction >= 0.0 && fraction < 1.0);
    const uuidv4: string = rand.uuidv4();
    const uuidv7: string = rand.uuidv7();
    check(uuidv4.length === 36);
    check(uuidv7.length === 36);
    match (rand.ulid()) {
        Ok(firstUlid) => {
            check(firstUlid.length === 26);
            match (rand.ulid()) {
                Ok(secondUlid) => { check(!timingSafeEqual(firstUlid, secondUlid)); }
                Err(error) => { check(false); }
            }
        }
        Err(error) => { check(false); }
    }

    const jwtKey: string = "0123456789abcdef0123456789abcdef";
    const token: string = cryptoString(jwt.sign({ role: "reader" }, jwtKey, { noTimestamp: true }));
    const claimsResult: Result<RuntimeClaims, CryptoError> = jwt.verify<RuntimeClaims>(token, jwtKey);
    match (claimsResult) {
        Ok(claims) => { check(claims.role === "reader"); }
        Err(error) => { check(false); }
    }
    const wrongClaimsResult: Result<NumericRuntimeClaims, CryptoError> = jwt.verify<NumericRuntimeClaims>(token, jwtKey);
    match (wrongClaimsResult) {
        Ok(_) => { check(false); }
        Err(error) => { check(timingSafeEqual(error.code, "ERR_CRYPTO_JWT_INVALID_CLAIMS")); }
    }
    return Ok();
}

interface RuntimeClaims {
    role: string;
}

interface NumericRuntimeClaims {
    count: i32;
}

function main(): void {
    match (runChecks()) {
        Ok() => {}
        Err(error) => { printError(error.code); }
    }
}"#,
    );

    for release in [false, true] {
        let build = project.build(release);
        assert!(
            build.status.success(),
            "{} build failed:\nstdout:\n{}\nstderr:\n{}",
            if release { "release" } else { "debug" },
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr)
        );
        let output = project.run();
        assert!(
            output.status.success(),
            "{} program failed:\nstdout:\n{}\nstderr:\n{}",
            if release { "release" } else { "debug" },
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert_eq!(stdout.lines().count(), 54, "unexpected stdout:\n{stdout}");
        assert!(
            stdout
                .lines()
                .all(|line| line == "true" || line == "assert: true"),
            "unexpected stdout:\n{stdout}"
        );
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn argon2_options_and_pepper_survive_native_async_execution() {
    let project = CryptoProject::new(
        r#"import { argon2, CryptoError } from "tyrus:crypto";


async function main(): Promise<void> {
    const result: Result<void, CryptoError> = await exerciseArgon2();
    match (result) {
        Ok() => {}
        Err(error) => { console.error(error.code); }
    }
}

async function exerciseArgon2(): Promise<Result<void, CryptoError>> {
    const pepper: u8[] = [7, 11, 13, 17, 19, 23, 29, 31];
    const hash: string = await argon2.hash("correct horse battery staple", {
        memoryCost: 19456,
        parallelism: 1,
        secret: pepper,
        timeCost: 2,
        type: argon2.argon2id,
    })?;
    console.log(hash.startsWith("$argon2id$v=19$"));
    console.log("assert:", true);
    const valid: boolean = await argon2.verify(hash, "correct horse battery staple", { secret: pepper })?;
    console.log(valid);
    console.log("assert:", true);
    const wrong: boolean = await argon2.verify(hash, "wrong password", { secret: pepper })?;
    console.log(wrong);
    console.log("assert:", false);
    const migrate: boolean = argon2.needsRehash(hash)?;
    console.log(migrate);
    console.log("assert:", true);
    const missingPepper: Result<boolean, CryptoError> =
        await argon2.verify(hash, "correct horse battery staple");
    match (missingPepper) {
        Ok(_) => { console.log("unexpected success"); }
        Err(error) => {
            console.log(error.code);
            console.log("assert:", "ERR_CRYPTO_ARGON2_PEPPER_REQUIRED");
        }
    }
    return Ok();
}
"#,
    );

    let build = project.build(false);
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "true\nassert: true\ntrue\nassert: true\nfalse\nassert: false\ntrue\nassert: true\nERR_CRYPTO_ARGON2_PEPPER_REQUIRED\nassert: ERR_CRYPTO_ARGON2_PEPPER_REQUIRED\n"
    );
    assert!(output.stderr.is_empty());
}
