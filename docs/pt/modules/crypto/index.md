---
title: Crypto
description: Primitivas criptográficas nativas com limites explícitos e criptografia autenticada.
---

# Crypto

Importe utilitários criptográficos de `tyrus:crypto`. Falhas recuperáveis das APIs de crypto retornam valores `Result` tipados; operações assíncronas cumprem `Promise<Result<T, CryptoError>>` em vez de rejeitar por essas falhas. Funções `throws` definidas pelo usuário continuam disponíveis de forma independente. Um throw do usuário não capturado é escrito em stderr e encerra o processo com status diferente de zero.

## Bytes e limites

Use `tyrus:encoding` para conversão UTF-8 e UTF-16. Entradas gerais de crypto são limitadas a 64 MiB, e `rand.randomBytes` é limitado a 16 MiB por chamada.

<!-- @include: ../../../.fragments/modules/crypto/basic-usage.md -->

## Digest e HMAC

`digest.sha256`, `digest.sha384` e `digest.sha512` aceitam `string | u8[]` e retornam `Result<u8[], CryptoError>`; `digest.md5` tem o mesmo retorno e existe apenas para checksums de compatibilidade. MD5 possui colisões conhecidas e não deve proteger senhas, assinaturas, certificados ou integridade sensível à segurança.

`hmac.sha256`, `hmac.sha384` e `hmac.sha512` aceitam `string | u8[]` para dados e chave e retornam `Result<u8[], CryptoError>`. Cópias temporárias de chave feitas pelo runtime são zeroizadas após o uso.

`digest.create("sha256" | "sha384" | "sha512")` retorna `Result<DigestStream, CryptoError`. Os métodos `update(data)`, `digest()` e `reset()` também retornam valores `Result`. Trate-os com `match` ou propague-os com `?`. O handle nativo é liberado automaticamente ao final do escopo léxico; `dispose()` permite liberação antecipada determinística e deve ser chamado no máximo uma vez. Algoritmos desconhecidos retornam `Err` com `ERR_CRYPTO_DIGEST_ALGORITHM`.

## Comparação em tempo constante

`timingSafeEqual(a: string | u8[], b: string | u8[]): boolean` compara dados de mesmo tamanho em tempo constante. Tamanhos diferentes retornam `false` imediatamente; a função não oculta o tamanho dos dados.

## Criptografia autenticada

`cipher.aes.encrypt(data: string | u8[], key: u8[]): Result<u8[], CryptoError>` cria um envelope AES-256-GCM versionado; os métodos de descriptografia também retornam `Result` com `CryptoError` para falhas recuperáveis. `decryptText` autentica antes de decodificar UTF-8, enquanto `decryptBytes` autentica antes de retornar bytes. A chave deve conter exatamente 32 bytes.

`encryptWithAad`, `decryptTextWithAad` e `decryptBytesWithAad` vinculam dados autenticados adicionais ao envelope. A descriptografia deve receber AAD idêntico byte a byte. O envelope autentica magic, versão, identificador do algoritmo, nonce, ciphertext, tag e o AAD fornecido.

<!-- @include: ../../../.fragments/modules/crypto/cipher-usage.md -->

## Hashing de senhas

`argon2.hash(password: string | u8[], options?): Promise<Result<string, CryptoError>>` usa Argon2id com salt aleatório de 16 bytes, saída de 32 bytes e o perfil padrão `m=65536`, `t=3`, `p=4`. As opções podem definir `type: "argon2id"`, `memoryCost` de 19.456 a 262.144 KiB, `timeCost` de 2 a 10, `parallelism` de 1 a 16 e um `secret` binário não vazio de até 1.024 bytes. Senhas textuais e binárias são limitadas a 1.024 bytes. `argon2.verify(hash, password: string | u8[], options?): Promise<Result<boolean, CryptoError>>` retorna `Err(CryptoError)` para strings PHC malformadas, parâmetros inválidos, ausência do pepper exigido, concorrência esgotada e falhas do worker; limita strings PHC a 4 KiB e valida parâmetros não confiáveis antes de alocar trabalho. Um hash com pepper contém um marcador autenticado não secreto; omitir `secret` ao verificá-lo retorna `ERR_CRYPTO_ARGON2_PEPPER_REQUIRED`, enquanto um secret incorreto retorna `Ok(false)`. Armazene e rotacione o pepper separadamente dos hashes de senha. `argon2.needsRehash(hash: string): Result<boolean, CryptoError>` informa se uma string PHC válida difere do perfil padrão; strings malformadas ou grandes demais retornam `ERR_CRYPTO_ARGON2_INVALID_HASH`. 

No máximo quatro operações Argon2 executam simultaneamente. Chamadas adicionais são rejeitadas com `ERR_CRYPTO_ARGON2_BUSY` em vez de formar uma fila ilimitada de CPU ou memória.

O perfil padrão mediu 127,93–145,24 ms em uma baseline Criterion otimizada em um Intel Core i5-10400F de seis cores. Esta é uma referência reproduzível para esse host, não uma promessa universal de latência; faça benchmark no hardware de produção e reduza a concorrência quando o orçamento de memória ou latência exigir.

## JWT

`jwt.sign<T>(claims: T, key: string, options?): Result<string, CryptoError>` aceita um objeto tipado de claims e suporta HS256/384/512 com um segredo de 32 a 65.536 bytes e RS256/384/512, ES256/384 ou EdDSA com uma chave privada PEM. `T` é inferido de `claims`; os campos suportados são `string`, `boolean`, `f32`/`f64` finitos e `i32`/`i64`/`u32`/`u64`. As opções selecionam `algorithm`, `audience`, `expiresIn`, `issuer`, `jwtId`, `keyId`, `noTimestamp` e `subject`. A assinatura adiciona `iat` salvo quando desabilitado e adiciona `exp` somente quando `expiresIn` está presente. Payloads e tokens são limitados a 1 MiB.

`jwt.verify<T>(token, key, options?): Result<T, CryptoError>` aceita um segredo simétrico, uma chave pública PEM ou uma JWK pública assimétrica codificada como JSON; valores JWK simétricos `oct` são rejeitados para impedir que secrets sejam introduzidos pelo caminho de chave pública. O argumento de tipo explícito obrigatório torna inequívoco o layout esperado dos claims; o runtime analisa o payload uma única vez, valida cada campo declarado e intervalo numérico, ignora claims registrados adicionais e retorna `Err(CryptoError)` com `ERR_CRYPTO_JWT_INVALID_CLAIMS` para campos ausentes, incompatíveis ou fora do intervalo. A allowlist padrão contém somente HS256; aplicações que usam outro algoritmo devem passar `algorithms`, e o algoritmo do token, a família da chave, `alg`, `use` e `key_ops` da JWK devem concordar. A verificação aplica `exp` e `nbf` quando presentes. As opções também restringem `audience`, `issuer`, `subject`, `clockTolerance` e `maxAge`; audience, issuer ou subject configurado exige que o claim correspondente esteja presente e corresponda, e `clockTolerance` se aplica aos claims temporais registrados. Um header JOSE `crit` é rejeitado porque o runtime não implementa extensões críticas de header.

`jwt.fetchJwks(url, options?): Promise<Result<string, CryptoError>>` obtém um documento JWKS limitado por HTTPS na porta 443. Ele cumpre com `Err(CryptoError)` para falhas recuperáveis de URL, DNS, rede, HTTP, tamanho, documento e cache; não aceita credenciais, fragments, destinos por IP literal e não públicos, fixa resultados DNS validados para impedir rebinding, desabilita proxies e redirects, aplica timeouts de conexão e total, limita o corpo a 256 KiB e o conjunto a 64 chaves públicas assimétricas de verificação com identificadores únicos e usa cache de dez minutos limitado a 32 entradas com coalescência single-flight. Passe `{ forceRefresh: true }` após um sinal controlado de rotação de chave para ignorar uma entrada de cache ainda válida. `jwt.verifyWithJwks<T>(token, jwksJson, options?): Result<T, CryptoError>` exige exatamente uma chave pública assimétrica compatível correspondente ao `kid` do header.

`jwt.decodeUnverified<T>(token): Result<T, CryptoError>` analisa e verifica os tipos dos claims sem autenticá-los. Trate o resultado com `match` ou propague-o com `?`; o valor de sucesso é controlado pelo atacante e nunca deve autorizar acesso, estabelecer identidade ou tomar uma decisão de segurança.

<!-- @include: ../../../.fragments/modules/crypto/async-usage.md -->

## Aleatoriedade segura

`rand.randomBytes(length)`, `rand.randomInt(min, max)` e `rand.randomFloat()` usam aleatoriedade criptograficamente segura. `randomBytes`, `randomInt` e `ulid` retornam `Result<..., CryptoError>` em falhas recuperáveis; trate com `match` ou propague com `?`. `randomInt` é inclusivo e retorna `ERR_CRYPTO_INVALID_RANGE` quando `min > max`. `rand.uuidv4()` cria UUIDs aleatórios, `rand.uuidv7()` cria UUIDs ordenados por tempo e `rand.ulid()` usa um gerador sincronizado global ao processo para permanecer estritamente monotônico em chamadas consecutivas.
