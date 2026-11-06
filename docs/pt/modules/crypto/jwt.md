---
title: jwt
description: Assinatura, verificação e JWKS de JSON Web Token através de tyrus:crypto.
---

# jwt

O namespace `jwt` fornece assinatura, verificação e gerenciamento de chaves JWKS de JWT.

<!-- @include: ../../../.fragments/modules/crypto/jwt/jwt.md -->

## Funções

| Função | Assinatura |
|--------|-----------|
| `jwt.sign<T>` | `<T>(T, string, options?) => Result<string, CryptoError>` |
| `jwt.verify<T>` | `<T>(string, string, options?) => Result<T, CryptoError>` |
| `jwt.verifyWithJwks<T>` | `<T>(string, string, options?) => Result<T, CryptoError>` |
| `jwt.decodeUnverified<T>` | `<T>(string) => Result<T, CryptoError>` |
| `jwt.fetchJwks` | `(string, options?) => Promise<Result<string, CryptoError>>` |

### jwt.sign

Aceita um objeto tipado de claims e suporta HS256/384/512 com um segredo de 32 a 65.536 bytes e RS256/384/512, ES256/384 ou EdDSA com uma chave privada PEM. `T` é inferido de `claims`; os campos suportados são `string`, `boolean`, `f32`/`f64` finitos e `i32`/`i64`/`u32`/`u64`. Falhas recuperáveis são retornadas como `Result<string, CryptoError>` e podem ser tratadas com `match` ou propagadas com `?`. As opções selecionam `algorithm`, `audience`, `expiresIn`, `issuer`, `jwtId`, `keyId`, `noTimestamp` e `subject`. A assinatura adiciona `iat` salvo quando desabilitado e adiciona `exp` somente quando `expiresIn` está presente. Payloads e tokens são limitados a 1 MiB.

### jwt.verify

Aceita um segredo simétrico, uma chave pública PEM ou uma JWK pública assimétrica codificada como JSON; valores JWK simétricos `oct` são rejeitados para impedir que secrets sejam introduzidos pelo caminho de chave pública. O argumento de tipo explícito obrigatório torna inequívoco o layout esperado dos claims; o runtime analisa o payload uma única vez, valida cada campo declarado e intervalo numérico, ignora claims registrados adicionais e retorna `Err(CryptoError)` com `ERR_CRYPTO_JWT_INVALID_CLAIMS` para campos ausentes, incompatíveis ou fora do intervalo. A allowlist padrão contém somente HS256; aplicações que usam outro algoritmo devem passar `algorithms`, e o algoritmo do token, a família da chave, `alg`, `use` e `key_ops` da JWK devem concordar. A verificação aplica `exp` e `nbf` quando presentes. As opções também restringem `audience`, `issuer`, `subject`, `clockTolerance` e `maxAge`. A verificação e a decodificação retornam `Result<T, CryptoError>` para falhas recuperáveis; trate-as com `match` ou propague-as com `?`. Um header JOSE `crit` é rejeitado porque o runtime não implementa extensões críticas de header.

### jwt.fetchJwks

Obtém um documento JWKS limitado por HTTPS na porta 443. Ele rejeita credenciais, fragments, destinos por IP literal e não públicos, fixa resultados DNS validados para impedir rebinding, desabilita proxies e redirects, aplica timeouts de conexão e total, limita o corpo a 256 KiB e o conjunto a 64 chaves públicas assimétricas de verificação com identificadores únicos e usa cache de dez minutos limitado a 32 entradas com coalescência single-flight. Passe `{ forceRefresh: true }` após um sinal controlado de rotação de chave para ignorar uma entrada de cache ainda válida.

### jwt.verifyWithJwks

Exige exatamente uma chave pública assimétrica compatível correspondente ao `kid` do header.

### jwt.decodeUnverified

Analisa e verifica os tipos dos claims sem autenticá-los. Seu resultado é controlado pelo atacante e nunca deve autorizar acesso, estabelecer identidade ou tomar uma decisão de segurança.

::: danger Aviso de segurança
`jwt.decodeUnverified` não autentica o token. Nunca use seu resultado para decisões de autorização ou identidade.
:::
