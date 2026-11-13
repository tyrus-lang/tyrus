---
title: jwt
description: JSON Web Token signing, verification, and JWKS through tyrus:crypto.
---

# jwt

The `jwt` namespace provides JWT signing, verification, and JWKS key management.

<!-- @include: ../../.fragments/modules/crypto/jwt/jwt.md -->

## Functions

| Function | Signature |
|----------|-----------|
| `jwt.sign<T>` | `<T>(T, string, options?) => Result<string, CryptoError>` |
| `jwt.verify<T>` | `<T>(string, string, options?) => Result<T, CryptoError>` |
| `jwt.verifyWithJwks<T>` | `<T>(string, string, options?) => Result<T, CryptoError>` |
| `jwt.decodeUnverified<T>` | `<T>(string) => Result<T, CryptoError>` |
| `jwt.fetchJwks` | `(string, options?) => Promise<Result<string, CryptoError>>` |

### jwt.sign

Accepts a typed claims object and supports HS256/384/512 with a secret of 32 to 65,536 bytes and RS256/384/512, ES256/384, or EdDSA with a private PEM key. `T` is inferred from `claims`; supported fields are `string`, `boolean`, finite `f32`/`f64`, and `i32`/`i64`/`u32`/`u64`. Recoverable failures are returned as `Result<string, CryptoError>` and can be handled with `match` or propagated with `?`. Options select `algorithm`, `audience`, `expiresIn`, `issuer`, `jwtId`, `keyId`, `noTimestamp`, and `subject`. Signing adds `iat` unless disabled and adds `exp` only when `expiresIn` is present. Payloads and tokens are limited to 1 MiB.

### jwt.verify

Accepts a symmetric secret, a public PEM key, or an asymmetric public JWK encoded as JSON; symmetric `oct` JWK values are rejected so secrets cannot be smuggled through the public-key path. The required explicit type argument makes the expected claim layout unambiguous; the runtime parses the payload once, validates every declared field and numeric range, ignores additional registered claims, and returns `Err(CryptoError)` with `ERR_CRYPTO_JWT_INVALID_CLAIMS` for missing, mismatched, or out-of-range fields. Verification and decoding return `Result<T, CryptoError>` for recoverable failures; handle them with `match` or propagate them with `?`. The default allowlist contains only HS256; applications using another algorithm must pass `algorithms`, and the token algorithm, key family, JWK `alg`, `use`, and `key_ops` must agree. Verification enforces `exp` and `nbf` when present. Options also constrain `audience`, `issuer`, `subject`, `clockTolerance`, and `maxAge`. A JOSE `crit` header is rejected because the runtime does not implement critical header extensions.

### jwt.fetchJwks

Retrieves a bounded JWKS document over HTTPS port 443. It rejects credentials, fragments, IP-literal and non-public destinations, pins validated DNS results to prevent rebinding, disables proxies and redirects, applies connect/total timeouts, limits the body to 256 KiB and the set to 64 uniquely identified asymmetric public verification keys, and uses a ten-minute cache limited to 32 entries with single-flight request coalescing. Pass `{ forceRefresh: true }` after a controlled key-rotation signal to bypass a still-valid cached entry.

### jwt.verifyWithJwks

Requires exactly one compatible asymmetric public key matching the header `kid`.

### jwt.decodeUnverified

Parses and type-checks claims without authenticating them. Its result is attacker-controlled and must never authorize access, establish identity, or make a security decision.

::: danger Security warning
`jwt.decodeUnverified` does not authenticate the token. Never use its result for authorization or identity decisions.
:::
