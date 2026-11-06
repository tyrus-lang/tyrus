---
title: Crypto
description: Native cryptographic primitives with explicit limits and authenticated encryption.
---

# Crypto

Import cryptographic utilities from `tyrus:crypto`. Recoverable failures from crypto APIs return typed `Result` values; asynchronous operations fulfill `Promise<Result<T, CryptoError>>` rather than rejecting for those failures. User-defined `throws` functions remain available independently. An uncaught user throw is written to stderr and terminates the process with a non-zero status.

## Bytes and limits

Use `tyrus:encoding` for UTF-8 and UTF-16 conversion. General crypto inputs are limited to 64 MiB, and `rand.randomBytes` is limited to 16 MiB per call.

<!-- @include: ../../.fragments/modules/crypto/basic-usage.md -->

## Digest and HMAC

`digest.sha256`, `digest.sha384`, and `digest.sha512` accept `string | u8[]` and return `Result<u8[], CryptoError>`; `digest.md5` has the same return type and exists only for compatibility checksums. MD5 is collision-broken and must not protect passwords, signatures, certificates, or security-sensitive integrity.

`hmac.sha256`, `hmac.sha384`, and `hmac.sha512` accept `string | u8[]` for data and key and return `Result<u8[], CryptoError>`. Temporary key copies made by the runtime are zeroized after use.

`digest.create("sha256" | "sha384" | "sha512")` returns `Result<DigestStream, CryptoError>`. The `update(data)`, `digest()`, and `reset()` methods also return `Result` values. Handle them with `match` or propagate them with `?`. The native handle is released automatically at the end of its lexical scope; `dispose()` permits deterministic early release and must be called at most once. Unknown algorithms return `Err` with `ERR_CRYPTO_DIGEST_ALGORITHM`.

## Constant-time comparison

`timingSafeEqual(a: string | u8[], b: string | u8[]): boolean` compares equal-length data in constant time. Different lengths return `false` immediately; the function does not conceal data length.

## Authenticated encryption

`cipher.aes.encrypt(data: string | u8[], key: u8[]): Result<u8[], CryptoError>` creates a versioned AES-256-GCM envelope; decryption methods also return `Result` with `CryptoError` for recoverable failures. `decryptText` authenticates before decoding UTF-8, while `decryptBytes` authenticates before returning bytes. The key must contain exactly 32 bytes.

`encryptWithAad`, `decryptTextWithAad`, and `decryptBytesWithAad` bind additional authenticated data to the envelope. Decryption must receive byte-for-byte identical AAD. The envelope authenticates its magic, version, algorithm identifier, nonce, ciphertext, tag, and supplied AAD.

<!-- @include: ../../.fragments/modules/crypto/cipher-usage.md -->

## Password hashing

`argon2.hash(password: string | u8[], options?): Promise<Result<string, CryptoError>>` uses Argon2id with a random 16-byte salt, a 32-byte output, and the default profile `m=65536`, `t=3`, `p=4`. Options may set `type: "argon2id"`, `memoryCost` from 19,456 to 262,144 KiB, `timeCost` from 2 to 10, `parallelism` from 1 to 16, and a non-empty binary `secret` up to 1,024 bytes. Text and binary passwords are limited to 1,024 bytes. `argon2.verify(hash, password: string | u8[], options?): Promise<Result<boolean, CryptoError>>` returns `Err(CryptoError)` for malformed PHC strings, invalid parameters, required pepper omissions, worker saturation, and worker failures; it limits PHC strings to 4 KiB and validates untrusted parameters before allocating work. A peppered hash contains a non-secret authenticated marker; omitting `secret` when verifying it returns `ERR_CRYPTO_ARGON2_PEPPER_REQUIRED`, while a supplied but incorrect secret returns `Ok(false)`. Store and rotate pepper separately from password hashes. `argon2.needsRehash(hash: string): Result<boolean, CryptoError>` reports whether a valid PHC string differs from the default profile; malformed or oversized strings return `ERR_CRYPTO_ARGON2_INVALID_HASH`. 

At most four Argon2 operations run concurrently. Additional calls are rejected with `ERR_CRYPTO_ARGON2_BUSY` instead of building an unbounded CPU or memory queue.

The default profile measured 127.93–145.24 ms in an optimized Criterion baseline on a six-core Intel Core i5-10400F. This is a reproducible reference for that host, not a universal latency promise; benchmark production hardware and lower concurrency when its memory or latency budget requires it.

## JWT

`jwt.sign<T>(claims: T, key: string, options?): Result<string, CryptoError>` accepts a typed claims object and supports HS256/384/512 with a secret of 32 to 65,536 bytes and RS256/384/512, ES256/384, or EdDSA with a private PEM key. `T` is inferred from `claims`; supported fields are `string`, `boolean`, finite `f32`/`f64`, and `i32`/`i64`/`u32`/`u64`. Options select `algorithm`, `audience`, `expiresIn`, `issuer`, `jwtId`, `keyId`, `noTimestamp`, and `subject`. Signing adds `iat` unless disabled and adds `exp` only when `expiresIn` is present. Payloads and tokens are limited to 1 MiB.

`jwt.verify<T>(token, key, options?): Result<T, CryptoError>` accepts a symmetric secret, a public PEM key, or an asymmetric public JWK encoded as JSON; symmetric `oct` JWK values are rejected so secrets cannot be smuggled through the public-key path. The required explicit type argument makes the expected claim layout unambiguous; the runtime parses the payload once, validates every declared field and numeric range, ignores additional registered claims, and returns `Err(CryptoError)` with `ERR_CRYPTO_JWT_INVALID_CLAIMS` for missing, mismatched, or out-of-range fields. The default allowlist contains only HS256; applications using another algorithm must pass `algorithms`, and the token algorithm, key family, JWK `alg`, `use`, and `key_ops` must agree. Verification enforces `exp` and `nbf` when present. Options also constrain `audience`, `issuer`, `subject`, `clockTolerance`, and `maxAge`; a configured audience, issuer, or subject requires the corresponding claim to be present and match, and `clockTolerance` applies to registered time claims. A JOSE `crit` header is rejected because the runtime does not implement critical header extensions.

`jwt.fetchJwks(url, options?): Promise<Result<string, CryptoError>>` retrieves a bounded JWKS document over HTTPS port 443. It fulfills with `Err(CryptoError)` for recoverable URL, DNS, network, HTTP, size, document, and cache failures; its URL policy disallows credentials, fragments, IP-literal and non-public destinations, pins validated DNS results to prevent rebinding, disables proxies and redirects, applies connect/total timeouts, limits the body to 256 KiB and the set to 64 uniquely identified asymmetric public verification keys, and uses a ten-minute cache limited to 32 entries with single-flight request coalescing. Pass `{ forceRefresh: true }` after a controlled key-rotation signal to bypass a still-valid cached entry. `jwt.verifyWithJwks<T>(token, jwksJson, options?): Result<T, CryptoError>` requires exactly one compatible asymmetric public key matching the header `kid`.

`jwt.decodeUnverified<T>(token): Result<T, CryptoError>` parses and type-checks claims without authenticating them. Handle the result with `match` or propagate it with `?`; its successful value is attacker-controlled and must never authorize access, establish identity, or make a security decision.

<!-- @include: ../../.fragments/modules/crypto/async-usage.md -->

## Secure random

`rand.randomBytes(length)`, `rand.randomInt(min, max)`, and `rand.randomFloat()` use cryptographically secure randomness. `randomBytes`, `randomInt`, and `ulid` return `Result<..., CryptoError>` for recoverable failures; inspect them with `match` or propagate with `?`. `randomInt` is inclusive and returns `ERR_CRYPTO_INVALID_RANGE` when `min > max`. `rand.uuidv4()` creates random UUIDs, `rand.uuidv7()` creates time-ordered UUIDs, and `rand.ulid()` uses a process-wide synchronized generator to remain strictly monotonic for consecutive calls.
