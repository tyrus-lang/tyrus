---
title: Validator Decorations
description: Anotaciones nativas de validación para interfaces, tipos de objeto, parámetros de función, fronteras HTTP y comprobaciones de runtime.
---

# Validator Decorations

Validator Decorations son anotaciones nativas de validación de tyrus. Pueden adjuntarse a campos de interface, campos de tipos de objeto y parámetros de función. El compilador verifica que cada validator exista, apunte a un tipo compatible y reciba argumentos literales del tipo esperado.

## Ejemplo

<!-- @include: ../../.fragments/language/validators/example.md -->

Los validators integrados verifican valores literales en tiempo de compilación y se ejecutan en runtime para valores dinámicos en parámetros decorados, literales de objeto y updates de objeto.

## Validator Decorations en archivos `.env`

Tyrus acepta Validator Decorations en archivos `.env` con o sin `#` al inicio. Usa la forma comentada cuando otro loader de `.env`, como Docker, también lea el archivo; Tyrus sigue interpretando la decoración, mientras los demás loaders ven un comentario.

```dotenv
# @type(u16)
# @port()
PORT=8080
```

## Mensajes de error

Los validators integrados permiten reemplazar el mensaje predeterminado con un literal string como último argumento aceptado.

<!-- @include: ../../.fragments/language/validators/error-messages.md -->

La misma regla funciona en parámetros de función decorados.

<!-- @include: ../../.fragments/language/validators/error-messages-parameter.md -->

::: tip Prefiere mensajes customizados en fronteras con usuario
Los mensajes default son útiles durante el desarrollo. Los mensajes customizados son mejores para fronteras de API, prompts de CLI y fallas de validación que pueden aparecer para usuarios.
:::

## Familias de Validators

| Familia | Ejemplos |
| --- | --- |
| Numéricos | [`@min`](#min), [`@max`](#max), [`@range`](#range), [`@positive`](#positive), [`@negative`](#negative), [`@nonNegative`](#nonnegative), [`@nonPositive`](#nonpositive), [`@multipleOf`](#multipleof), [`@powerOfTwo`](#poweroftwo), [`@integer`](#integer), [`@safeInteger`](#safeinteger), [`@port`](#port), [`@statusCode`](#statuscode) |
| String | [`@minLength`](#minlength), [`@maxLength`](#maxlength), [`@length`](#length), [`@notEmpty`](#notempty), [`@notBlank`](#notblank), [`@isTrimmed`](#istrimmed), [`@isLowercase`](#islowercase), [`@isUppercase`](#isuppercase), [`@startsWith`](#startswith), [`@endsWith`](#endswith), [`@includes`](#includes), [`@regex`](#regex) |
| Formato | [`@email`](#email), [`@url`](#url), [`@uuid`](#uuid), [`@ip`](#ip), [`@ipv4`](#ipv4), [`@ipv6`](#ipv6), [`@e164`](#e164), [`@cep`](#cep), [`@cpf`](#cpf), [`@cnpj`](#cnpj) |
| Arrays y valores | [`@minItems`](#minitems), [`@maxItems`](#maxitems), [`@lengthItems`](#lengthitems), [`@notEmptyItems`](#notemptyitems), [`@unique`](#unique), [`@in`](#in), [`@notIn`](#notin), [`@equal`](#equal), [`@notEqual`](#notequal), [`@notNull`](#notnull), [`@optional`](#optional), [`@required`](#required) |

## Objetivos de los Validators

Un validator numérico se comprueba en la aritmética del tipo que decora: un campo `i64` se compara como entero y un campo `f64` como float. Un número entero por encima de 2^53 conserva, por lo tanto, su valor exacto, y el veredicto en tiempo de compilación siempre coincide con la comprobación emitida para el mismo valor.

`@port`, `@powerOfTwo` y `@statusCode` apuntan solo a tipos enteros, porque sus reglas no tienen lectura sobre un valor fraccionario. Un puerto no es `8080.5`.

`@unique` compara elementos por valor, así que acepta arrays de números, strings, booleanos, enums, tuplas, tipos de objeto e interfaces nombradas cuyos campos también se comparen por valor. Un array de arrays se rechaza en tiempo de compilación: los arrays se comparan por dirección, así que `[[1, 2], [1, 2]]` sería reportado como único.

## Operación del Validator

Cada familia de validators impone restricciones específicas en los valores que decora. El compilador verifica valores literales en tiempo de compilación y emite comprobaciones de runtime para valores dinámicos. A continuación hay ejemplos para cada familia de validators con sus mensajes de error predeterminados.

### Numéricos

#### `@min`

Garantiza que el valor sea como mínimo el especificado.

<SignatureBlock label="Firma">@min(value: i8 | i16 | i32 | i64 | u8 | u16 | u32 | u64 | f32 | f64 | f128, message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/numeric/min.md -->

#### `@max`

Garantiza que el valor sea como máximo el especificado.

<SignatureBlock label="Firma">@max(value: i8 | i16 | i32 | i64 | u8 | u16 | u32 | u64 | f32 | f64 | f128, message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/numeric/max.md -->

#### `@range`

Garantiza que el valor esté dentro del rango inclusivo especificado.

<SignatureBlock label="Firma">@range(min: i8 | i16 | i32 | i64 | u8 | u16 | u32 | u64 | f32 | f64 | f128, max: i8 | i16 | i32 | i64 | u8 | u16 | u32 | u64 | f32 | f64 | f128, message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/numeric/range.md -->

#### `@positive`

Garantiza que el valor sea mayor que cero.

<SignatureBlock label="Firma">@positive(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/numeric/positive.md -->

#### `@negative`

Garantiza que el valor sea menor que cero.

<SignatureBlock label="Firma">@negative(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/numeric/negative.md -->

#### `@nonNegative`

Garantiza que el valor sea cero o mayor.

<SignatureBlock label="Firma">@nonNegative(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/numeric/nonNegative.md -->

#### `@nonPositive`

Garantiza que el valor sea cero o menor.

<SignatureBlock label="Firma">@nonPositive(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/numeric/nonPositive.md -->

#### `@multipleOf`

Garantiza que el valor sea múltiplo del divisor indicado.

<SignatureBlock label="Firma">@multipleOf(value: i8 | i16 | i32 | i64 | u8 | u16 | u32 | u64 | f32 | f64 | f128, message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/numeric/multipleOf.md -->

#### `@powerOfTwo`

Garantiza que el valor sea una potencia de dos.

<SignatureBlock label="Firma">@powerOfTwo(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/numeric/powerOfTwo.md -->

#### `@integer`

Garantiza que un valor de punto flotante no tenga parte fraccionaria.

<SignatureBlock label="Firma">@integer(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/numeric/integer.md -->

#### `@safeInteger`

Garantiza que el valor entero esté dentro del rango seguro de ±2⁵³.

<SignatureBlock label="Firma">@safeInteger(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/numeric/safeInteger.md -->

#### `@port`

Garantiza que el valor sea un puerto registrado entre 1024 y 49151.

<SignatureBlock label="Firma">@port(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/numeric/port.md -->

#### `@statusCode`

Garantiza que el valor sea un código HTTP válido entre 100 y 599.

<SignatureBlock label="Firma">@statusCode(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/numeric/statusCode.md -->

### String

#### `@minLength`

Garantiza que la cadena contenga al menos el número especificado de caracteres.

<SignatureBlock label="Firma">@minLength(value: i64, message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/string/minLength.md -->

#### `@maxLength`

Garantiza que la cadena contenga como máximo el número especificado de caracteres.

<SignatureBlock label="Firma">@maxLength(value: i64, message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/string/maxLength.md -->

#### `@length`

Garantiza que la cadena tenga exactamente la longitud especificada.

<SignatureBlock label="Firma">@length(value: i64, message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/string/length.md -->

#### `@notEmpty`

Garantiza que la cadena no esté vacía.

<SignatureBlock label="Firma">@notEmpty(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/string/notEmpty.md -->

#### `@notBlank`

Garantiza que la cadena contenga al menos un carácter no blanco.

<SignatureBlock label="Firma">@notBlank(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/string/notBlank.md -->

#### `@isTrimmed`

Garantiza que la cadena no tenga espacios al inicio o al final.

<SignatureBlock label="Firma">@isTrimmed(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/string/isTrimmed.md -->

#### `@isLowercase`

Garantiza que la cadena contenga solo caracteres en minúscula.

<SignatureBlock label="Firma">@isLowercase(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/string/isLowercase.md -->

#### `@isUppercase`

Garantiza que la cadena contenga solo caracteres en mayúscula.

<SignatureBlock label="Firma">@isUppercase(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/string/isUppercase.md -->

#### `@startsWith`

Garantiza que la cadena comience con el prefijo indicado.

<SignatureBlock label="Firma">@startsWith(value: string, message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/string/startsWith.md -->

#### `@endsWith`

Garantiza que la cadena termine con el sufijo indicado.

<SignatureBlock label="Firma">@endsWith(value: string, message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/string/endsWith.md -->

#### `@includes`

Garantiza que la cadena contenga la subcadena indicada.

<SignatureBlock label="Firma">@includes(value: string, message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/string/includes.md -->

#### `@regex`

Garantiza que la cadena coincida con la expresión regular indicada.

<SignatureBlock label="Firma">@regex(value: Regex, message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/string/regex.md -->

### Formato

#### `@email`

Garantiza que el valor sea una dirección de e-mail válida.

<SignatureBlock label="Firma">@email(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/format/email.md -->

#### `@url`

Garantiza que el valor sea una URL válida.

<SignatureBlock label="Firma">@url(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/format/url.md -->

#### `@uuid`

Garantiza que el valor sea un UUID válido.

<SignatureBlock label="Firma">@uuid(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/format/uuid.md -->

#### `@ip`

Garantiza que el valor sea una dirección IP válida (v4 o v6).

<SignatureBlock label="Firma">@ip(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/format/ip.md -->

#### `@ipv4`

Garantiza que el valor sea una dirección IPv4 válida.

<SignatureBlock label="Firma">@ipv4(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/format/ipv4.md -->

#### `@ipv6`

Garantiza que el valor sea una dirección IPv6 válida.

<SignatureBlock label="Firma">@ipv6(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/format/ipv6.md -->

#### `@e164`

Garantiza que el valor sea un número de teléfono E.164 válido.

<SignatureBlock label="Firma">@e164(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/format/e164.md -->

#### `@cep`

Garantiza que el valor sea un CEP brasileño válido.

<SignatureBlock label="Firma">@cep(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/format/cep.md -->

#### `@cpf`

Garantiza que el valor sea un CPF brasileño válido.

<SignatureBlock label="Firma">@cpf(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/format/cpf.md -->

#### `@cnpj`

Garantiza que el valor sea un CNPJ brasileño válido.

<SignatureBlock label="Firma">@cnpj(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/format/cnpj.md -->

### Array

#### `@minItems`

Garantiza que el array contenga al menos el número especificado de elementos.

<SignatureBlock label="Firma">@minItems(value: i64, message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/array/minItems.md -->

#### `@maxItems`

Garantiza que el array contenga como máximo el número especificado de elementos.

<SignatureBlock label="Firma">@maxItems(value: i64, message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/array/maxItems.md -->

#### `@lengthItems`

Garantiza que el array tenga exactamente la longitud especificada.

<SignatureBlock label="Firma">@lengthItems(value: i64, message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/array/lengthItems.md -->

#### `@notEmptyItems`

Garantiza que el array no esté vacío.

<SignatureBlock label="Firma">@notEmptyItems(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/array/notEmptyItems.md -->

#### `@unique`

Garantiza que todos los elementos del array sean distintos.

<SignatureBlock label="Firma">@unique(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/array/unique.md -->

### Igualdad y pertenencia

#### `@equal`

Garantiza que el valor sea igual al literal indicado.

<SignatureBlock label="Firma">@equal(value: T, message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/equality-membership/equal.md -->

#### `@notEqual`

Garantiza que el valor sea diferente del literal indicado.

<SignatureBlock label="Firma">@notEqual(value: T, message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/equality-membership/notEqual.md -->

#### `@in`

Garantiza que el valor sea una de las opciones especificadas.

<SignatureBlock label="Firma">@in(value: T[], message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/equality-membership/in.md -->

#### `@notIn`

Garantiza que el valor no sea ninguna de las opciones especificadas.

<SignatureBlock label="Firma">@notIn(value: T[], message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/equality-membership/notIn.md -->

### Anulabilidad y presencia

#### `@notNull`

Garantiza que un objetivo opcional tenga un valor.

<SignatureBlock label="Firma">@notNull(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/nullability-presence/notNull.md -->

#### `@optional`

Marca un campo o parámetro como explícitamente opcional.

<SignatureBlock label="Firma">@optional(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/nullability-presence/optional.md -->

#### `@required`

Garantiza que el objetivo tenga un valor.

<SignatureBlock label="Firma">@required(message?: string)</SignatureBlock>

<!-- @include: ../../.fragments/language/validators/nullability-presence/required.md -->

La validación se aplica automáticamente cuando se crean o actualizan objetos tipados y cuando se llaman funciones decoradas. El comportamiento de schemas de ruta pertenece al roadmap HTTP y no forma parte de este mecanismo a nivel de lenguaje.

## Objetivos Opcionales

Un validator colocado en un objetivo opcional se aplica al valor que el opcional lleva, así que solo se ejecuta cuando hay un valor presente. Un opcional ausente pasa todos los validators, excepto `@notNull` y `@required`, que comprueban la presencia misma y por eso se aplican al opcional y no al valor que lleva.

<!-- @include: ../../.fragments/language/validators/optional-targets.md -->

Ni el campo `email` omitido ni el argumento `age` omitido fallan la validación: no hay valor que comprobar. Suministrar un valor inválido hace que los mismos validators fallen exactamente como lo harían en un objetivo obligatorio.

## Validación Recuperable en Runtime

Las fallas de validación dinámica lanzan `ValidationError`; no terminan la aplicación cuando son tratadas. `ValidationError.issues` siempre está presente y contiene al menos un `ValidationIssue` de solo lectura, por lo que el mismo tratamiento funciona cuando falla una regla o varias reglas. Una función decorada declara este efecto implícitamente. Una función intermedia debe tratarlo o propagarlo con `throws ValidationError`, mientras que `main` debe tratar todos los errores alcanzables y no puede declarar `throws`.

El destino de cada issue es una ruta lógica determinista. Los campos anidados usan notación de punto, como `profile.address.zipCode`, y los elementos de array incluyen su índice basado en cero, como `members[0].address.zipCode`.

La ruta tiene como raíz el valor que la operación valida, y no un tipo más externo cualquiera. Construir un objeto anidado antes, en su propia variable, convierte esa construcción en una operación de validación por sí misma, así que sus issues tienen como raíz ese objeto: el destino es `zipCode`, y no `members[0].address.zipCode`.

Una operación de validación recopila todas las reglas que fallaron antes de lanzar: la creación o actualización de un objeto recopila issues de todos los campos decorados, y una llamada de función decorada recopila issues de todos los parámetros decorados. Las issues preservan el orden de declaración y evaluación. Operaciones separadas lanzan valores `ValidationError` separados y nunca se combinan.

<!-- @include: ../../.fragments/language/validators/recoverable-runtime.md -->

`ValidationIssue` normalmente se infiere cuando se lee un elemento de `error.issues`; el desarrollador no lo construye ni lo lanza. Pasar un issue a un helper de solo lectura no requiere escribir `&`.

Leer e iterar `error.issues` dentro del `catch` es el patrón habitual. Un move o retorno directo de `error.issues` también es seguro: la propiedad acompaña al array retornado en lugar de liberarse con el error capturado.

<!-- @include: ../../.fragments/language/validators/print-issue.md -->

La validación de `.env` tipado es la excepción: una configuración inicial inválida es fatal porque la aplicación no puede iniciar con un entorno inválido.

`@in` y `@notIn` aceptan un array homogéneo inline o un array `const` de nivel de módulo que pueda resolverse completamente en tiempo de compilación. El array puede contener miembros de enum. Variables runtime, bindings `let`, llamadas de función y valores de API son rechazados porque los argumentos de decorations son metadatos de compilación. Los schemas `.env` tipados continúan usando arrays inline porque los archivos `.env` no tienen scope de símbolos ni imports de tyrus.

<!-- @include: ../../.fragments/language/validators/in-not-in.md -->

## Validators con Regex

<!-- @include: ../../.fragments/language/validators/regex.md -->

`@regex` usa el tipo nativo `Regex`. Las flags soportadas son `i`, `m`, `s` y `x`; look-around y backreferences no son soportados intencionalmente.
