---
title: cipher
description: Authenticated AES-256-GCM encryption through tyrus:crypto.
---

# cipher

The `cipher.aes` namespace provides authenticated AES-256-GCM encryption and decryption.

<!-- @include: ../../.fragments/modules/crypto/cipher-usage.md -->

## Functions

| Function | Signature |
|----------|-----------|
| `cipher.aes.encrypt` | `(string \| u8[], u8[]) => Result<u8[], CryptoError>` |
| `cipher.aes.encryptWithAad` | `(string \| u8[], u8[], u8[]) => Result<u8[], CryptoError>` |
| `cipher.aes.decryptText` | `(u8[], u8[]) => Result<string, CryptoError>` |
| `cipher.aes.decryptBytes` | `(u8[], u8[]) => Result<u8[], CryptoError>` |
| `cipher.aes.decryptTextWithAad` | `(u8[], u8[], u8[]) => Result<string, CryptoError>` |
| `cipher.aes.decryptBytesWithAad` | `(u8[], u8[], u8[]) => Result<u8[], CryptoError>` |

`encrypt` creates a versioned AES-256-GCM envelope. `decryptText` authenticates before decoding UTF-8, while `decryptBytes` authenticates before returning bytes. The key must contain exactly 32 bytes.

`encryptWithAad`, `decryptTextWithAad`, and `decryptBytesWithAad` bind additional authenticated data to the envelope. Decryption must receive byte-for-byte identical AAD. The envelope authenticates its magic, version, algorithm identifier, nonce, ciphertext, tag, and supplied AAD. All six operations return `Result` with `CryptoError` for recoverable failures; inspect the value with `match` or propagate it with `?`. Stable codes include `ERR_CRYPTO_INVALID_KEY_SIZE`, `ERR_CRYPTO_INVALID_ENVELOPE`, `ERR_CRYPTO_DECRYPTION_FAILED`, and `ERR_CRYPTO_INVALID_UTF8`.
