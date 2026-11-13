---
title: Tipos
description: Tipos primitivos, numéricos, opcionales, objeto, array, tupla, union y función en tyrus.
---

# Tipos

Tyrus usa anotaciones de tipo y semántica nativa aplicada por el compilador. Los valores normalmente pueden inferirse desde los inicializadores, pero las anotaciones explícitas son recomendables en fronteras de API y cuando el ancho numérico importa.

::: tip La inferencia es local y predecible
`const answer = 42;` se infiere desde el literal. Usa `const answer: i64 = 42;` cuando el ancho forma parte del contrato.
:::

::: warning Un literal de objeto nombra su interfaz
Las interfaces son nominales, así que un literal declara la interfaz que construye en vez de que se adivine por su forma: `const user: User = { id: 1 };`. Un literal de objeto sin anotación es un error, y el `help:` nombra cada interfaz con forma compatible, ofrecida en el editor como Quick Fix. Adivinar por la forma haría que el significado de un archivo dependiera de todas las interfaces del programa.
:::

## Tipos primitivos

| Tipo | Úsalo para | Ejemplo |
|------|------------|---------|
| `boolean` | Valores de verdad y condiciones de branch. | `const enabled: boolean = true;` |
| `i8` | Bytes y valores con signo pequeños, de un byte de ancho. | `const delta: i8 = -8;` |
| `i16` | Valores con signo pequeños, de dos bytes de ancho. | `const offset: i16 = -1024;` |
| `i32` | Entero con signo default para contadores, IDs y la mayoría de la aritmética. | `const count: i32 = 42;` |
| `i64` | Enteros con signo grandes, timestamps e identificadores persistidos. | `const micros: i64 = 1720000000000;` |
| `i128` | Cálculos enteros con signo muy grandes. | `const ledger: i128 = 9000000000000000000;` |
| `isize` | Índices con signo pointer-sized y valores de tamaño nativo. | `const offset: isize = -1;` |
| `u8` | Bytes, códigos de protocolo y canales de color. | `const red: u8 = 255;` |
| `u16` | Puertos, code points y campos de protocolo de dos bytes. | `const port: u16 = 8080;` |
| `u32` | Contadores y tamaños unsigned que caben en 32 bits. | `const size: u32 = 4096;` |
| `u64` | Contadores e identificadores unsigned grandes. | `const sequence: u64 = 10000000000;` |
| `u128` | Cálculos enteros unsigned muy grandes. | `const mask: u128 = 340282366920938463463374607431768211455;` |
| `usize` | Índices, longitudes y capacidades unsigned pointer-sized. | `const length: usize = names.length;` |
| `f32` | Valores de punto flotante compactos cuando el tradeoff de precisión es aceptable. | `const ratio: f32 = 0.75;` |
| `f64` | Cálculos de punto flotante default. | `const total: f64 = 19.99;` |
| `f128` | Cálculos de punto flotante extendidos y constantes nativas min/max. | `const largest: f128 = f128.MAX;` |
| `decimal` | Aritmética exacta de base diez con 34 dígitos significativos. | `const total: decimal = 0.1d + 0.2d;` |
| `char` | Un único carácter Unicode, para recorrer texto carácter a carácter. | `const letter: char = 'A';` |
| `string` | Texto UTF-8 con ownership. | `const label: string = "tyrus";` |
| `void` | Funciones que no devuelven valor. | `function log(): void { print("ok"); }` |
| `unknown` | Valores que necesitan narrowing antes del uso. | `const payload: unknown = readInput();` |
| `never` | Caminos de código que no retornan normalmente. | `function fail(message: string): never { throw message; }` |

## Tipos numéricos

Usa enteros con signo (`i*`) cuando los valores pueden ser negativos. Usa enteros unsigned (`u*`) para bytes, tamaños, campos de protocolo y valores que nunca pueden ser negativos. Usa `f32`/`f64` para valores fraccionarios, no para dinero ni contadores exactos.

<!-- @include: ../../.fragments/language/types/numeric-types.md -->

## Métodos algebraicos de punto flotante

`f32` y `f64` proporcionan `algebraicAdd`, `algebraicSub`, `algebraicMul`, `algebraicDiv` y `algebraicRem`. Corresponden respectivamente a suma, resta, multiplicación, división y resto, y permiten explícitamente que el compilador reasocie el cálculo de punto flotante seleccionado para optimización. Los resultados pueden variar entre builds y targets, incluso para `NaN`, infinito y cero con signo. Los operadores aritméticos normales permanecen estrictos.

<!-- @include: ../../.fragments/language/types/algebraic-float-methods.md -->

## Valores especiales de punto flotante

`Infinity` y `NaN` son literales reservados de punto flotante disponibles sin importación. `-Infinity` es el `-` unario aplicado a `Infinity`. Se infieren como `f64` y pueden recibir contexto como `f32`, `f64` o `f128`. La aritmética de punto flotante sigue IEEE 754, incluso para resultados no finitos; la división entera por cero sigue siendo un error de runtime.

<!-- @include: ../../.fragments/language/types/special-floating-point-values.md -->

`isNaN(value)` acepta un valor de punto flotante y devuelve `boolean`. La igualdad no identifica `NaN`: toda comparación de igualdad con él es falsa, mientras que toda comparación de desigualdad es verdadera.

::: warning El ancho numérico forma parte del tipo
Pasar un `i64` donde se espera un `i32` exige conversión explícita de narrowing. El compilador rechaza narrowing implícito porque puede perder datos.
:::

::: danger error
<!-- @include: ../../.fragments/language/types/narrowing-example.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/types/narrowing-error.md -->
:::

## Constantes numéricas

Los tipos numéricos exponen constantes `MIN` y `MAX` mediante sintaxis de propiedad en el nombre del tipo.

<!-- @include: ../../.fragments/language/types/numeric-constants.md -->

## Sintaxis de literales numéricos

Tyrus admite literales enteros hexadecimales, binarios, octales y decimales, con guiones bajos opcionales como separadores visuales. Los prefijos y dígitos hexadecimales no distinguen entre mayúsculas y minúsculas. Los separadores de enteros decimales deben agrupar los dígitos en conjuntos de tres. Los literales enteros positivos por encima de `i128.MAX` requieren un contexto `u128` explícito y no pueden superar `u128.MAX`; el límite negativo con signo es `i128.MIN`. La notación científica se admite en literales decimales de punto flotante, y las partes enteras grandes no se amplían implícitamente a `f128`.

<!-- @include: ../../.fragments/language/types/numeric-literals.md -->

## Análisis decimal

`parseDecimal(text)` analiza una cadena decimal completa de base diez y devuelve `decimal?`. Texto no válido, un sufijo `d` o `D`, valores fuera del rango decimal y separadores malformados producen ausencia en lugar de una aproximación de punto flotante binario. Los casts de entero a decimal son exactos; un cast de decimal a entero exige un valor integral y rango suficiente en el destino.

<!-- @include: ../../.fragments/language/types/decimal-parsing.md -->

`MIN` de entero unsigned siempre es cero. `isize` y `usize` tienen el ancho de un puntero en la máquina para la que se compila el programa, así que sus límites no los fija el lenguaje: no exponen `MIN` ni `MAX`, y conviene usar un tipo de ancho fijo cuando el límite en sí importa.


::: danger Un literal entero debe caber en su tipo
`const byte: u8 = 300;` es un error de compilación (`E0706`) anclado en el literal, nunca un valor que da la vuelta en silencio. El editor ofrece un quick fix que ensancha el tipo al más estrecho que sostiene el valor.
:::
Los literales numéricos pueden llevar un sufijo explícito de tipo entero o de punto flotante. El sufijo fija el tipo del literal, participa en la inferencia y es distinto de una conversión explícita con `as`.

<!-- @include: ../../.fragments/language/types/typed-numeric-literals.md -->
## Caracteres

Un `char` guarda exactamente un carácter Unicode y nunca se infiere: llega por anotación, y el valor se escribe como un literal de un carácter. Texto de cualquier otra longitud es `string`.

<!-- @include: ../../.fragments/language/types/char.md -->

Las comillas no tienen significado, lo que hace que un valor sea un `char` es la anotación, pero `tyrus fmt` escribe un `char` con comillas simples y un `string` con el estilo de comillas configurado, así que ambos se distinguen a simple vista.

"Un carácter Unicode" significa un único valor escalar Unicode, un único punto de código, no un grafema, que es lo que el lector percibe como un símbolo. Algunos emoji son un único valor escalar y caben en un `char`; muchos se escriben con varios puntos de código y deben ser `string`:

| Emoji | Puntos de código | Cantidad | `char` |
|-------|------------------|----------|--------|
| 😀 | `U+1F600` | 1 | ✅ |
| 🇧🇷 | dos indicadores regionales (`U+1F1E7` `U+1F1F7`) | 2 | ❌ |
| ❤️ | corazón más selector de variación (`U+2764` `U+FE0F`) | 2 | ❌ |
| 👍🏽 | pulgar más modificador de tono de piel | 2 | ❌ |
| 👨‍👩‍👧‍👦 | familia, una secuencia ZWJ | 7 | ❌ |

Así `const emoji: char = '😀';` se acepta, mientras que un emoji compuesto como `'🇧🇷'` falla en tiempo de compilación, `a char holds exactly one character, but 🇧🇷 has 2`, y el diagnóstico apunta a `string`.

`char` ofrece `toUpperCase`, `toLowerCase`, `isDigit`, `isAlphabetic`, `isAlphanumeric`, `isWhitespace` y `toString`. Mayúsculas y clasificación siguen las reglas de Unicode, no las de ASCII, así que `'ç'.toUpperCase()` es `'Ç'`. Convertir un entero en `char` está comprobado: un valor que no es un carácter Unicode aborta, y `as char?` devuelve ausencia.

## Valores unknown

`unknown` guarda un valor cuyo tipo solo se conoce en tiempo de ejecución. Acepta un `string`, un `boolean`, un `char`, cualquier tipo numérico o un `decimal`, incluida una union de ellos, y no se puede hacer nada con el valor hasta que una comprobación `typeof` demuestre lo que guarda. En la rama donde `typeof value === "string"` es verdadero, `value` es un `string`. La comparación puede escribirse en cualquier orden, y la rama `else` de `typeof value !== "string"` se estrecha de la misma forma.

<!-- @include: ../../.fragments/language/types/unknown-narrowing.md -->

Fuera de la rama comprobada el valor sigue siendo `unknown`. Una comprobación de `"object"` o `"function"`, o la rama donde la comprobación falló, no demuestra ningún tipo único y mantiene el valor como `unknown`. Comparar `typeof` con un nombre que nunca produce, como `"number"` o `"String"`, es un error de compilación (`E0795`) anclado en el literal, y el editor ofrece un quick fix cuando se pretende exactamente una categoría.

Los arrays, objetos, interfaces, opcionales y funciones no pueden guardarse en `unknown` (`E0794`). Dale al valor un tipo que lo describa.

::: danger error
<!-- @include: ../../.fragments/language/types/unknown-composite-error.md -->
:::

## Tipos opcionales

`T?` representa un valor que puede estar ausente. tyrus no tiene `undefined` ni valor `null`: la ausencia no es un valor, sino el estado de un `T?` que no tiene valor.

<!-- @include: ../../.fragments/language/types/optional-basic.md -->

Como la ausencia no es un valor, la produces de cuatro formas: omitir un campo o parámetro opcional, escribir un `return;` vacío en una función cuyo retorno es `T?`, declarar un binding opcional sin inicializador (`let x: T?;` empieza vacío) o llamar a un método nativo que no encuentra nada (`array.find`). No hay literal `null` para escribir.

<!-- @include: ../../.fragments/language/types/optional-absence.md -->

Lee miembros de un valor opcional con `?.`. El acceso hace short-circuit cuando el receiver está ausente, y su resultado sigue siendo opcional: `name?.length` es `usize?`, no `usize`.

<!-- @include: ../../.fragments/language/types/optional-chaining.md -->

::: danger Trata valores opcionales antes del uso
El unwrap forzado con `value!` es rechazado por el compilador (`E0716`) porque puede convertir ausencia en falla de runtime. Haz narrowing del valor con `if (value)` o proporciona un fallback con `??`.
:::

Los tipos opcionales componen con colecciones y retornos de función:

<!-- @include: ../../.fragments/language/types/optional-composition.md -->

## Valores Result

`Result<T, E>` representa un payload de éxito exactamente `T` o un payload de error exactamente `E`. Todo valor retornado por una función `Result` debe envolverse en `Ok(value)` o `Err(value)`, por lo que la variante elegida es explícita incluso cuando `T` y `E` difieren. El postfix `?` extrae el payload de éxito y retorna el error desde la función actual cuando es necesario.

<!-- @include: ../../.fragments/language/types/result-values.md -->

Los tipos de los payloads deben ser exactos, y los payloads opcionales como `Result<User?, string>` son rechazados. `Result<void, E>` retorna éxito con `Ok()`. Un retorno vacío se rechaza con un Quick Fix para usar `Ok()`; un `return value;` sin constructor se rechaza con un Quick Fix que envuelve el valor en el único constructor compatible, u ofrece ambos constructores cuando los dos tipos de payload coinciden. Un result se estrecha con patrones explícitos `Ok(binding)` y `Err(binding)` en `match`, y ambas variantes son obligatorias salvo que un brazo `_` explícito cubra el caso restante. `Ok()` está disponible cuando el payload de éxito es `void`. El operador `?` es la alternativa explícita cuando el error debe propagarse al llamador.

`?` es un operador posfijo, por lo que el valor que desenvuelve puede participar en cualquier expresión: `load()? + 1`, `load()? === 40`, o una rama de ternario como `wanted ? load()? : 0`. Comparte el token con el ternario, y los dos se distinguen por lo que sigue: un `?` cuya expresión siguiente va seguida de `:` abre un ternario, así que `flag ? -1 : 2` sigue siendo un ternario mientras `load()? - 1` propaga.

<!-- @include: ../../.fragments/language/types/try-in-expressions.md -->

En una función `async`, el `await` abarca su operando hasta el primer `?`, así que `await loadUser(id)?` espera la promesa y luego propaga el `Result`: `const user = await loadUser(id)?;` o `return Ok(await load()? + 1);`. Los paréntesis mantienen su significado, así que `await (load()?)` aplica el `?` a la propia promesa, lo que se rechaza con un `help` que apunta a `await load()?`. Un `await` también puede ser el sujeto de un `match` o de un `if`, o el argumento de una llamada, incluso dentro de un bucle.

<!-- @include: ../../.fragments/language/types/result-in-async.md -->

## Manejo de fallos en línea

`onfail` es una guarda terminal adjunta al inicializador de un `const` o `let` simple. Maneja `Result<T, E>`, una `Promise<Result<T, E>>` esperada, una llamada directa con un único efecto `throws E` declarado o un rechazo esperado. El nombre declarado es el error dentro del handler y el valor de éxito después de él. Un handler nunca continúa con un valor de reemplazo: sale con `return`, `throw`, `continue` o `break`, y llegar a su final sale por él, retornando de una función `void`, devolviendo `Err(name)` desde una función que retorna `Result<U, E>` con el mismo tipo de error o propagando la excepción permitida. En una función que retorna cualquier otro tipo, el handler debe salir explícitamente (`E0277`).

<!-- @include: ../../.fragments/language/types/inline-failure-handling.md -->

Sobre un `Result` esperado, el `Err` se separa después del `await`, así que la misma guarda maneja los fallos de los módulos nativos. El handler en sí se ejecuta sin suspender, y un `await` dentro de él se rechaza (`E3002`).

<!-- @include: ../../.fragments/language/types/inline-failure-handling-async.md -->

Usa `try`/`catch` para varias operaciones protegidas, recuperación con un valor de reemplazo o un bloque `finally`. Un `finally` externo todavía se ejecuta exactamente una vez cuando termina un handler de `onfail`.

## Tipos de objeto

<!-- @include: ../../.fragments/language/types/object-types.md -->

Los literales de objeto se comprueban estructuralmente contra declaraciones de interface. Los campos opcionales se acceden como valores opcionales.

## Arrays y tuplas

<!-- @include: ../../.fragments/language/types/arrays-tuples.md -->

Los arrays guardan un tipo de elemento repetido. Las tuplas guardan una forma fija con un tipo conocido por posición.

Usa `[valor; tamaño]` para crear un array donde todos los elementos son iguales al valor dado. El tamaño debe ser un entero constante conocido en tiempo de compilación. Se soportan tanto tipos Copy (enteros, booleanos) como tipos no-Copy (strings, arrays anidados); los elementos no-Copy se clonan profundamente para cada posición.

Usa `Array.withCapacity(n)` para crear un array vacío con búfer preasignado. El tipo del elemento se infiere de la anotación de la variable. Esto evita realocaciones repetidas cuando se sabe aproximadamente cuántos elementos se van a insertar.

## Constructores de tipos genéricos

Usa paréntesis angulares para proporcionar argumentos de tipo a constructores de tipo con nombre. `Array<T>` equivale a `T[]`, y `Tuple<T, ...>` equivale a `[T, ...]`; elige la forma que haga más fácil leer el tipo circundante.

| Forma | Argumentos | Significado |
| --- | --- | --- |
| `Array<T>` | uno | Una secuencia mutable o inmutable de `T`; equivale a `T[]`. |
| <code style="white-space: nowrap">Tuple&lt;T, ...&gt;</code> | <span style="white-space: nowrap">cero o más</span> | Una secuencia de longitud fija con un tipo por posición; equivale a `[T, ...]`. |
| `Optional<T>` | uno | Un valor que puede estar ausente; equivale a `T?`. |
| `Range<T>` | uno | Un range numérico producido por `a..b` o `a..=b`. |
| `Set<T>` | uno | Una colección de valores únicos de `T`. |
| `Map<K, V>` | dos | Una colección con claves de `K` a `V`. |
| `Record<K, V>` | dos | Un objeto con claves `K` y valores `V`; `Record<string, V>` es un map dinámico. |
| `Promise<T>` | uno | El resultado de trabajo asíncrono que se resuelve como `T`. |

Las utilidades de tipo de objetos también son genéricas: `Partial<T>`, `Required<T>`, `Readonly<T>`, `DeepPartial<T>`, `Pick<T, K>`, `Omit<T, K>`, `PickByType<T, Type>`, `OmitByType<T, Type>`, `CamelCaseKeys<T>`, `SnakeCaseKeys<T>`, `Require<T, K>`, `AtLeastOne<T, K>`, `DataOnly<T>`, `Exclude<T, U>`, `Extract<T, U>`, `ReturnType<T>` y `Parameters<T>`. Consulta [Colecciones](./collections), [Objetos e interfaces](./objects) y [Async y Await](./async-await) para sus operaciones y restricciones.

## Tipos de función

<!-- @include: ../../.fragments/language/types/function-types.md -->

Las firmas de función incluyen tipos de parámetros y retorno. El compilador usa esas firmas para comprobación de llamadas, completion, hover y signature help.

Los tipos de función componen con otros constructores de tipo:

<!-- @include: ../../.fragments/language/types/function-composition.md -->

Las funciones pueden almacenarse en arrays y maps, permitiendo registros de callbacks y sistemas de eventos:

<!-- @include: ../../.fragments/language/types/function-registries.md -->

Los paréntesis agrupan un tipo, así que un array de tipo función puede escribirse en forma inline en vez de solo a través de un alias:

<!-- @include: ../../.fragments/language/types/parenthesized-types.md -->

Lo que separa un grupo de una lista de parámetros es la flecha, y solo es legible después del paréntesis de cierre: `(n: i32) => i32` es un tipo función, `(i32)[]` es un array de `i32`. Un grupo contiene un solo tipo, sin nombre y sin `...`; cualquier otra cosa es lista de parámetros y sigue exigiendo la flecha.


Un tipo de función puede declarar lo que la función puede lanzar: `(string) => i32 throws string`. Una llamada a través de un valor de ese tipo tiene que capturar o propagar esos errores, como una llamada directa, y una función que lanza no puede usarse donde el tipo de función no los declara (`E3298`). Una función `async` informa sus errores como rechazo, así que encaja en un tipo de función sin `throws`.

<!-- @include: ../../.fragments/language/types/function-throws.md -->
