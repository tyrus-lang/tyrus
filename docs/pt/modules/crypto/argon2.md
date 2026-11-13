---
title: argon2
description: Hashing de senhas Argon2id através de tyrus:crypto.
---

# argon2

O namespace `argon2` fornece hashing de senhas Argon2id com parâmetros configuráveis.

<!-- @include: ../../../.fragments/modules/crypto/async-usage.md -->

## Funções

| Função | Assinatura |
|--------|-----------|
| `argon2.hash` | `(string \| u8[], options?) => Promise<Result<string, CryptoError>>` |
| `argon2.verify` | `(string, string \| u8[], options?) => Promise<Result<boolean, CryptoError>>` |
| `argon2.needsRehash` | `(string) => Result<boolean, CryptoError>` |

`argon2.hash` cumpre com `Ok(hash)` em caso de sucesso e `Err(CryptoError)` para entrada ou opções inválidas, concorrência esgotada, falha do gerador aleatório ou falha do worker. Usa Argon2id com salt aleatório de 16 bytes, saída de 32 bytes e o perfil padrão `m=65536`, `t=3`, `p=4`. As opções podem definir `type: "argon2id"`, `memoryCost` de 19.456 a 262.144 KiB, `timeCost` de 2 a 10, `parallelism` de 1 a 16 e um `secret` binário não vazio de até 1.024 bytes. Senhas textuais e binárias são limitadas a 1.024 bytes.

`argon2.verify` cumpre com `Ok(matches)` para um hash válido e `Err(CryptoError)` para strings PHC malformadas, parâmetros inválidos, pepper obrigatório ausente, concorrência esgotada ou falha do worker. Limita strings PHC a 4 KiB e valida parâmetros não confiáveis antes de alocar trabalho. Um hash com pepper contém um marcador autenticado não secreto; omitir `secret` retorna `ERR_CRYPTO_ARGON2_PEPPER_REQUIRED`, enquanto um secret incorreto retorna `Ok(false)`. Armazene e rotacione o pepper separadamente dos hashes de senha.

`argon2.needsRehash` retorna `Ok(true)` quando a string PHC é válida, mas seu algoritmo, versão, parâmetros, tamanho do salt ou tamanho da saída diferem do perfil padrão. Strings PHC inválidas ou grandes demais retornam `Err(CryptoError)` com o código `ERR_CRYPTO_ARGON2_INVALID_HASH`.

::: warning Limite de concorrência
No máximo quatro operações Argon2 executam simultaneamente. Chamadas adicionais cumprem com `Err(CryptoError)` e o código `ERR_CRYPTO_ARGON2_BUSY`.
:::
