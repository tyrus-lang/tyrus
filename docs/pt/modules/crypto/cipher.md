---
title: cipher
description: Criptografia autenticada AES-256-GCM através de tyrus:crypto.
---

# cipher

O namespace `cipher.aes` fornece criptografia e descriptografia autenticada AES-256-GCM.

<!-- @include: ../../../.fragments/modules/crypto/cipher-usage.md -->

## Funções

| Função | Assinatura |
|--------|-----------|
| `cipher.aes.encrypt` | `(string \| u8[], u8[]) => Result<u8[], CryptoError>` |
| `cipher.aes.encryptWithAad` | `(string \| u8[], u8[], u8[]) => Result<u8[], CryptoError>` |
| `cipher.aes.decryptText` | `(u8[], u8[]) => Result<string, CryptoError>` |
| `cipher.aes.decryptBytes` | `(u8[], u8[]) => Result<u8[], CryptoError>` |
| `cipher.aes.decryptTextWithAad` | `(u8[], u8[], u8[]) => Result<string, CryptoError>` |
| `cipher.aes.decryptBytesWithAad` | `(u8[], u8[], u8[]) => Result<u8[], CryptoError>` |

`encrypt` cria um envelope AES-256-GCM versionado. `decryptText` autentica antes de decodificar UTF-8, enquanto `decryptBytes` autentica antes de retornar bytes. A chave deve conter exatamente 32 bytes.

`encryptWithAad`, `decryptTextWithAad` e `decryptBytesWithAad` vinculam dados autenticados adicionais ao envelope. A descriptografia deve receber AAD idêntico byte a byte. O envelope autentica magic, versão, identificador do algoritmo, nonce, ciphertext, tag e o AAD fornecido. As seis operações retornam `Result` com `CryptoError` para falhas recuperáveis; trate o valor com `match` ou propague-o com `?`. Códigos estáveis incluem `ERR_CRYPTO_INVALID_KEY_SIZE`, `ERR_CRYPTO_INVALID_ENVELOPE`, `ERR_CRYPTO_DECRYPTION_FAILED` e `ERR_CRYPTO_INVALID_UTF8`.
