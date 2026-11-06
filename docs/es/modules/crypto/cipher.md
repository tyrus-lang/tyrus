---
title: cipher
description: Cifrado autenticado AES-256-GCM a través de tyrus:crypto.
---

# cipher

El namespace `cipher.aes` proporciona cifrado y descifrado autenticado AES-256-GCM.

<!-- @include: ../../../.fragments/modules/crypto/cipher-usage.md -->

## Funciones

| Función | Firma |
|---------|-------|
| `cipher.aes.encrypt` | `(string \| u8[], u8[]) => Result<u8[], CryptoError>` |
| `cipher.aes.encryptWithAad` | `(string \| u8[], u8[], u8[]) => Result<u8[], CryptoError>` |
| `cipher.aes.decryptText` | `(u8[], u8[]) => Result<string, CryptoError>` |
| `cipher.aes.decryptBytes` | `(u8[], u8[]) => Result<u8[], CryptoError>` |
| `cipher.aes.decryptTextWithAad` | `(u8[], u8[], u8[]) => Result<string, CryptoError>` |
| `cipher.aes.decryptBytesWithAad` | `(u8[], u8[], u8[]) => Result<u8[], CryptoError>` |

`encrypt` crea un envelope AES-256-GCM versionado. `decryptText` autentica antes de decodificar UTF-8, mientras que `decryptBytes` autentica antes de devolver bytes. La clave debe contener exactamente 32 bytes.

`encryptWithAad`, `decryptTextWithAad` y `decryptBytesWithAad` vinculan datos autenticados adicionales al envelope. El descifrado debe recibir AAD idéntico byte por byte. El envelope autentica magic, versión, identificador del algoritmo, nonce, ciphertext, tag y el AAD proporcionado. Las seis operaciones devuelven `Result` con `CryptoError` para fallos recuperables; maneje el valor con `match` o propáguelo con `?`. Los códigos estables incluyen `ERR_CRYPTO_INVALID_KEY_SIZE`, `ERR_CRYPTO_INVALID_ENVELOPE`, `ERR_CRYPTO_DECRYPTION_FAILED` y `ERR_CRYPTO_INVALID_UTF8`.
