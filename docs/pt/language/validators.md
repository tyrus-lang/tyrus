---
title: Validator Decorations
description: Anotações nativas de validação para interfaces, tipos de objeto, parâmetros de função, fronteiras HTTP e checagens de runtime.
---

# Validator Decorations

Validator Decorations são anotações nativas de validação do tyrus. Elas podem ser anexadas a campos de interface, campos de tipos de objeto e parâmetros de função. O compilador verifica se cada validator existe, mira um tipo compatível e recebe argumentos literais do tipo esperado.

## Exemplo

<!-- @include: ../../.fragments/language/validators/example.md -->

Validators nativos verificam valores literais em tempo de compilação e executam em runtime para valores dinâmicos em parâmetros decorados, literais de objeto e updates de objeto.

## Validator Decorations em arquivos `.env`

O Tyrus aceita Validator Decorations em arquivos `.env` com ou sem `#` no início. Use a forma comentada quando outro loader de `.env`, como o Docker, também ler o arquivo; o Tyrus ainda interpreta a decoração, enquanto os outros loaders veem um comentário.

```dotenv
# @type(u16)
# @port()
PORT=8080
```

## Mensagens de erro

Validators nativos permitem sobrescrever a mensagem padrão com uma string literal no último argumento aceito.

<!-- @include: ../../.fragments/language/validators/error-messages.md -->

A mesma regra funciona em parâmetros de função decorados.

<!-- @include: ../../.fragments/language/validators/error-messages-parameter.md -->

::: tip Prefira mensagens customizadas em fronteiras com usuário
Mensagens padrão são úteis durante o desenvolvimento. Mensagens customizadas são melhores para fronteiras de API, prompts de CLI e falhas de validação que podem aparecer para usuários.
:::

## Famílias de Validators

| Família | Exemplos |
| --- | --- |
| Numéricos | [`@min`](#min), [`@max`](#max), [`@range`](#range), [`@positive`](#positive), [`@negative`](#negative), [`@nonNegative`](#nonnegative), [`@nonPositive`](#nonpositive), [`@multipleOf`](#multipleof), [`@powerOfTwo`](#poweroftwo), [`@integer`](#integer), [`@safeInteger`](#safeinteger), [`@port`](#port), [`@statusCode`](#statuscode) |
| String | [`@minLength`](#minlength), [`@maxLength`](#maxlength), [`@length`](#length), [`@notEmpty`](#notempty), [`@notBlank`](#notblank), [`@isTrimmed`](#istrimmed), [`@isLowercase`](#islowercase), [`@isUppercase`](#isuppercase), [`@startsWith`](#startswith), [`@endsWith`](#endswith), [`@includes`](#includes), [`@regex`](#regex) |
| Formato | [`@email`](#email), [`@url`](#url), [`@uuid`](#uuid), [`@ip`](#ip), [`@ipv4`](#ipv4), [`@ipv6`](#ipv6), [`@e164`](#e164), [`@cep`](#cep), [`@cpf`](#cpf), [`@cnpj`](#cnpj) |
| Arrays e valores | [`@minItems`](#minitems), [`@maxItems`](#maxitems), [`@lengthItems`](#lengthitems), [`@notEmptyItems`](#notemptyitems), [`@unique`](#unique), [`@in`](#in), [`@notIn`](#notin), [`@equal`](#equal), [`@notEqual`](#notequal), [`@notNull`](#notnull), [`@optional`](#optional), [`@required`](#required) |

## Alvos dos Validators

Um validator numérico é checado na aritmética do tipo que ele decora: um campo `i64` é comparado como inteiro e um campo `f64` como float. Um número inteiro acima de 2^53 mantém, portanto, seu valor exato, e o veredito de tempo de compilação sempre concorda com o check emitido para o mesmo valor.

`@port`, `@powerOfTwo` e `@statusCode` miram apenas tipos inteiros, porque suas regras não têm leitura sobre um valor fracionário. Uma porta não é `8080.5`.

`@unique` compara elementos por valor, então aceita arrays de números, strings, booleanos, enums, tuplas, tipos de objeto e interfaces nomeadas cujos campos também sejam comparados por valor. Um array de arrays é rejeitado em tempo de compilação: arrays são comparados por endereço, então `[[1, 2], [1, 2]]` seria reportado como único.

## Operação do Validator

Cada família de validators impõe restrições específicas nos valores que ela decora. O compilador verifica valores literais em tempo de compilação e emite checks de runtime para valores dinâmicos. Abaixo estão exemplos para cada família de validators com suas mensagens de erro padrão.

### Numéricos

#### `@min`

Garante que o valor seja no mínimo o especificado.

<SignatureBlock label="Assinatura">@min(value: i8 | i16 | i32 | i64 | u8 | u16 | u32 | u64 | f32 | f64 | f128, message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/numeric/min.md -->

#### `@max`

Garante que o valor seja no máximo o especificado.

<SignatureBlock label="Assinatura">@max(value: i8 | i16 | i32 | i64 | u8 | u16 | u32 | u64 | f32 | f64 | f128, message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/numeric/max.md -->

#### `@range`

Garante que o valor esteja dentro do intervalo inclusivo especificado.

<SignatureBlock label="Assinatura">@range(min: i8 | i16 | i32 | i64 | u8 | u16 | u32 | u64 | f32 | f64 | f128, max: i8 | i16 | i32 | i64 | u8 | u16 | u32 | u64 | f32 | f64 | f128, message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/numeric/range.md -->

#### `@positive`

Garante que o valor seja maior que zero.

<SignatureBlock label="Assinatura">@positive(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/numeric/positive.md -->

#### `@negative`

Garante que o valor seja menor que zero.

<SignatureBlock label="Assinatura">@negative(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/numeric/negative.md -->

#### `@nonNegative`

Garante que o valor seja zero ou maior.

<SignatureBlock label="Assinatura">@nonNegative(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/numeric/nonNegative.md -->

#### `@nonPositive`

Garante que o valor seja zero ou menor.

<SignatureBlock label="Assinatura">@nonPositive(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/numeric/nonPositive.md -->

#### `@multipleOf`

Garante que o valor seja múltiplo do divisor informado.

<SignatureBlock label="Assinatura">@multipleOf(value: i8 | i16 | i32 | i64 | u8 | u16 | u32 | u64 | f32 | f64 | f128, message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/numeric/multipleOf.md -->

#### `@powerOfTwo`

Garante que o valor seja uma potência de dois.

<SignatureBlock label="Assinatura">@powerOfTwo(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/numeric/powerOfTwo.md -->

#### `@integer`

Garante que um valor de ponto flutuante não tenha parte fracionária.

<SignatureBlock label="Assinatura">@integer(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/numeric/integer.md -->

#### `@safeInteger`

Garante que o valor inteiro esteja dentro do intervalo seguro de ±2⁵³.

<SignatureBlock label="Assinatura">@safeInteger(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/numeric/safeInteger.md -->

#### `@port`

Garante que o valor seja uma porta registrada entre 1024 e 49151.

<SignatureBlock label="Assinatura">@port(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/numeric/port.md -->

#### `@statusCode`

Garante que o valor seja um código HTTP válido entre 100 e 599.

<SignatureBlock label="Assinatura">@statusCode(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/numeric/statusCode.md -->

### String

#### `@minLength`

Garante que a string contenha ao menos o número especificado de caracteres.

<SignatureBlock label="Assinatura">@minLength(value: i64, message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/string/minLength.md -->

#### `@maxLength`

Garante que a string contenha no máximo o número especificado de caracteres.

<SignatureBlock label="Assinatura">@maxLength(value: i64, message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/string/maxLength.md -->

#### `@length`

Garante que a string tenha exatamente o comprimento especificado.

<SignatureBlock label="Assinatura">@length(value: i64, message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/string/length.md -->

#### `@notEmpty`

Garante que a string não esteja vazia.

<SignatureBlock label="Assinatura">@notEmpty(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/string/notEmpty.md -->

#### `@notBlank`

Garante que a string contenha ao menos um caractere não-branco.

<SignatureBlock label="Assinatura">@notBlank(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/string/notBlank.md -->

#### `@isTrimmed`

Garante que a string não tenha espaços no início ou no fim.

<SignatureBlock label="Assinatura">@isTrimmed(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/string/isTrimmed.md -->

#### `@isLowercase`

Garante que a string contenha apenas caracteres minúsculos.

<SignatureBlock label="Assinatura">@isLowercase(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/string/isLowercase.md -->

#### `@isUppercase`

Garante que a string contenha apenas caracteres maiúsculos.

<SignatureBlock label="Assinatura">@isUppercase(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/string/isUppercase.md -->

#### `@startsWith`

Garante que a string comece com o prefixo informado.

<SignatureBlock label="Assinatura">@startsWith(value: string, message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/string/startsWith.md -->

#### `@endsWith`

Garante que a string termine com o sufixo informado.

<SignatureBlock label="Assinatura">@endsWith(value: string, message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/string/endsWith.md -->

#### `@includes`

Garante que a string contenha a substring informada.

<SignatureBlock label="Assinatura">@includes(value: string, message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/string/includes.md -->

#### `@regex`

Garante que a string corresponda à expressão regular informada.

<SignatureBlock label="Assinatura">@regex(value: Regex, message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/string/regex.md -->

### Formato

#### `@email`

Garante que o valor seja um endereço de e-mail válido.

<SignatureBlock label="Assinatura">@email(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/format/email.md -->

#### `@url`

Garante que o valor seja uma URL válida.

<SignatureBlock label="Assinatura">@url(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/format/url.md -->

#### `@uuid`

Garante que o valor seja um UUID válido.

<SignatureBlock label="Assinatura">@uuid(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/format/uuid.md -->

#### `@ip`

Garante que o valor seja um endereço IP válido (v4 ou v6).

<SignatureBlock label="Assinatura">@ip(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/format/ip.md -->

#### `@ipv4`

Garante que o valor seja um endereço IPv4 válido.

<SignatureBlock label="Assinatura">@ipv4(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/format/ipv4.md -->

#### `@ipv6`

Garante que o valor seja um endereço IPv6 válido.

<SignatureBlock label="Assinatura">@ipv6(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/format/ipv6.md -->

#### `@e164`

Garante que o valor seja um número de telefone E.164 válido.

<SignatureBlock label="Assinatura">@e164(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/format/e164.md -->

#### `@cep`

Garante que o valor seja um CEP brasileiro válido.

<SignatureBlock label="Assinatura">@cep(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/format/cep.md -->

#### `@cpf`

Garante que o valor seja um CPF brasileiro válido.

<SignatureBlock label="Assinatura">@cpf(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/format/cpf.md -->

#### `@cnpj`

Garante que o valor seja um CNPJ brasileiro válido.

<SignatureBlock label="Assinatura">@cnpj(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/format/cnpj.md -->

### Array

#### `@minItems`

Garante que o array contenha ao menos o número especificado de itens.

<SignatureBlock label="Assinatura">@minItems(value: i64, message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/array/minItems.md -->

#### `@maxItems`

Garante que o array contenha no máximo o número especificado de itens.

<SignatureBlock label="Assinatura">@maxItems(value: i64, message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/array/maxItems.md -->

#### `@lengthItems`

Garante que o array tenha exatamente o comprimento especificado.

<SignatureBlock label="Assinatura">@lengthItems(value: i64, message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/array/lengthItems.md -->

#### `@notEmptyItems`

Garante que o array não esteja vazio.

<SignatureBlock label="Assinatura">@notEmptyItems(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/array/notEmptyItems.md -->

#### `@unique`

Garante que todos os itens do array sejam distintos.

<SignatureBlock label="Assinatura">@unique(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/array/unique.md -->

### Igualdade e pertinência

#### `@equal`

Garante que o valor seja igual ao literal informado.

<SignatureBlock label="Assinatura">@equal(value: T, message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/equality-membership/equal.md -->

#### `@notEqual`

Garante que o valor seja diferente do literal informado.

<SignatureBlock label="Assinatura">@notEqual(value: T, message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/equality-membership/notEqual.md -->

#### `@in`

Garante que o valor seja uma das opções especificadas.

<SignatureBlock label="Assinatura">@in(value: T[], message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/equality-membership/in.md -->

#### `@notIn`

Garante que o valor não seja nenhuma das opções especificadas.

<SignatureBlock label="Assinatura">@notIn(value: T[], message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/equality-membership/notIn.md -->

### Nulidade e presença

#### `@notNull`

Garante que um alvo opcional carregue um valor.

<SignatureBlock label="Assinatura">@notNull(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/nullability-presence/notNull.md -->

#### `@optional`

Marca um campo ou parâmetro como explicitamente opcional.

<SignatureBlock label="Assinatura">@optional(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/nullability-presence/optional.md -->

#### `@required`

Garante que o alvo tenha um valor.

<SignatureBlock label="Assinatura">@required(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/nullability-presence/required.md -->

A validação é aplicada automaticamente quando objetos tipados são criados ou atualizados e quando funções decoradas são chamadas. O comportamento de schemas de rota pertence ao roadmap HTTP e não faz parte deste mecanismo no nível da linguagem.

## Alvos Opcionais

Um validator colocado em um alvo opcional se aplica ao valor que o opcional carrega, então ele só roda quando há um valor presente. Um opcional ausente passa em todos os validators, exceto `@notNull` e `@required`, que testam a própria presença e por isso se aplicam ao opcional, não ao valor que ele carrega.

<!-- @include: ../../.fragments/language/validators/optional-targets.md -->

Nem o campo `email` omitido nem o argumento `age` omitido falham na validação: não há valor a verificar. Fornecer um valor inválido faz os mesmos validators falharem exatamente como fariam em um alvo obrigatório.

## Validação Recuperável em Runtime

Falhas de validação dinâmica lançam `ValidationError`; elas não encerram a aplicação quando são tratadas. `ValidationError.issues` está sempre presente e contém pelo menos um `ValidationIssue` somente de leitura, portanto o mesmo tratamento funciona quando uma regra ou várias regras falham. Uma função decorada declara esse efeito implicitamente. Uma função intermediária deve tratá-lo ou propagá-lo com `throws ValidationError`, enquanto `main` deve tratar todo erro alcançável e não pode declarar `throws`.

O alvo de cada issue é um caminho lógico determinístico. Campos aninhados usam notação de ponto, como `profile.address.zipCode`, e elementos de array incluem seu índice baseado em zero, como `members[0].address.zipCode`.

O caminho tem como raiz o valor que a operação valida, e não um tipo mais externo qualquer. Construir um objeto aninhado antes, numa variável própria, torna aquela construção uma operação de validação por si só, então as issues dela têm como raiz aquele objeto: o alvo é `zipCode`, e não `members[0].address.zipCode`.

Uma operação de validação coleta todas as regras que falharam antes de lançar: a criação ou atualização de um objeto coleta issues de todos os campos decorados, e uma chamada de função decorada coleta issues de todos os parâmetros decorados. As issues preservam a ordem de declaração e avaliação. Operações separadas lançam valores `ValidationError` separados e nunca são mescladas.

<!-- @include: ../../.fragments/language/validators/recoverable-runtime.md -->

`ValidationIssue` normalmente é inferido quando um item é lido de `error.issues`; o desenvolvedor não o constrói nem o lança. Passar um issue para um helper somente de leitura não exige escrever `&`.

Ler e iterar `error.issues` dentro do `catch` é o padrão usual. Um move ou retorno direto de `error.issues` também é seguro: a propriedade acompanha o array retornado em vez de ser liberada com o erro capturado.

<!-- @include: ../../.fragments/language/validators/print-issue.md -->

A validação de `.env` tipado é a exceção: uma configuração inicial inválida é fatal porque a aplicação não pode iniciar com um ambiente inválido.

`@in` e `@notIn` aceitam um array homogêneo inline ou um array `const` de nível de módulo que possa ser completamente resolvido em tempo de compilação. O array pode conter membros de enum. Variáveis runtime, bindings `let`, chamadas de função e valores de API são rejeitados porque os argumentos de decorations são metadados de compilação. Schemas `.env` tipados continuam usando arrays inline porque arquivos `.env` não possuem escopo de símbolos ou imports do tyrus.

<!-- @include: ../../.fragments/language/validators/in-not-in.md -->

## Validators com Regex

<!-- @include: ../../.fragments/language/validators/regex.md -->

`@regex` usa o tipo nativo `Regex`. As flags suportadas são `i`, `m`, `s` e `x`; look-around e backreferences não são suportados intencionalmente.
