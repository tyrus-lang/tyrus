---
title: Objetos e Interfaces
description: Interfaces, type aliases, campos opcionales y readonly, updates de objeto, unions y type utilities.
---

# Objetos e Interfaces

Tyrus usa sintaxis concisa para objetos, y interfaces nombradas y aliases de objeto hacen lowering a layouts de struct nativos.

## Interfaces

<!-- @include: ../../.fragments/language/objects/interfaces.md -->

`readonly` impide reasignar el campo. Los campos opcionales se leen como `T?` y deben tratarse con narrowing o `??`. Los literales de objeto se comprueban para campos ausentes, extra y con tipo incorrecto. Object spread crea updates inmutables con el mismo layout de interface del objeto fuente. Cuando el valor de un campo es una variable local con el mismo nombre, escribe el nombre una sola vez, `{ active }` es azúcar sintáctico para `{ active: active }`, y se combina libremente con campos explícitos `key: value` y con un spread update.

## Métodos automáticos de datos

Cada interface de datos nombrada recibe métodos automáticos de datos sin anotaciones ni declaraciones. Los primeros cuatro son llamadas de instancia ordinarias y `fromJson` es una llamada estática en el nombre concreto de la interface. Una interface de comportamiento puede definir un método de instancia con el mismo nombre y tiene precedencia sobre el método automático.

### `copy()`

Crea una copia estructural independiente.

`copy()` no es solo para objetos: existe en todo valor que posee un buffer, es decir, todo tipo que no es `Copy`. Eso cubre `string`, arrays, tuplas y los opcionales, results y uniones construidos a partir de ellos. En un escalar `Copy` como `i32` el método no existe, porque `let a = b` ya duplica y un método sugeriría que ocurre otra cosa.

El resultado es un valor corriente de su propio tipo. Una decoración pertenece al campo del que se leyó el valor, no al valor, así que nada de la declaración de origen acompaña a la copia.

Leer un campo en un binding del mismo tipo lo mueve: `const items = order.items;` entrega el campo a `items`, `order` sigue siendo utilizable por sus otros campos, y leer `order.items` u `order` entero después se rechaza (`E0382`) hasta que el campo se asigna de nuevo. Donde el movimiento no puede ocurrir, el compilador sugiere `copy()`: cuando los campos del dueño tienen validators, cuyas reglas describen el valor entero, cuando el campo se alcanza a través de una referencia, o cuando el binding se declara con un tipo distinto del campo, como un `string` leído de un `string?` estrechado (`E3081`). `order.items.copy()` dice lo que hace y mantiene el coste visible, cosa que una copia insertada en silencio no haría.

<!-- @include: ../../.fragments/language/objects/automatic-data-copy.md -->

### `equals(other: Type)`

Compara campos elegibles estructuralmente, incluidos los campos opcionales ausentes y presentes. Los arrays no son elegibles porque su igualdad es por dirección. El argumento debe tener la misma interface nominal.

<!-- @include: ../../.fragments/language/objects/automatic-data-equals.md -->

Una interface con campos coincidentes todavía no puede pasarse a `equals()` cuando tiene otro nombre:

::: danger error
<!-- @include: ../../.fragments/language/objects/automatic-data-methods-equals-error.md -->
:::

### `toString()`

Produce una representación legible en el orden declarado de los campos, entrecomilla y escapa strings, muestra campos opcionales ausentes como `None` y renderiza valores anidados recursivamente.

<!-- @include: ../../.fragments/language/objects/automatic-data-to-string.md -->

### `toJson(options?: { pretty: boolean })`

Emite JSON canónico con un espacio después de cada dos puntos. Pasa `{ pretty: true }` para JSON de varias líneas con sangría de cuatro espacios. Las claves salen en orden alfabético, no en el orden de declaración, para que el mismo dato produzca siempre el mismo texto: añadir un campo después no mueve los que ya estaban. Un `decimal` se escribe en notación común, el mismo texto con el que se imprime: `12.50d` sale como `12.5`, con todos los dígitos preservados y sin redondeo.

<!-- @include: ../../.fragments/language/objects/automatic-data-to-json.md -->

Un campo que lleva una unión discriminada se escribe como el objeto que su variante viva ya es: no hay envoltorio, y el campo discriminante es lo que le dice al lector qué variante recibió. `fromJson` la lee de vuelta del mismo modo, eligiendo la variante por su discriminante; un discriminante que no nombra ninguna variante declarada se rechaza con `ERR_JSON_UNKNOWN_VARIANT` en lugar de adivinarse, que es justamente para lo que se escribe.

<!-- @include: ../../.fragments/language/objects/automatic-data-to-json-union.md -->

### `Type.fromJson(text: string)`

Valida sintaxis, campos, tipos y rangos numéricos, y devuelve `JsonError` con `code` estable, `message` legible y `path` de campo en lugar de lanzar.

<!-- @include: ../../.fragments/language/objects/json-error.md -->

<!-- @include: ../../.fragments/language/objects/automatic-data-from-json.md -->

Usa `onfail` para imprimir un `JsonError` y terminar la función `void` actual cuando falle la decodificación:

::: danger error
<!-- @include: ../../.fragments/language/objects/automatic-data-from-json-onfail.md -->
:::


## Interfaces de comportamiento con `at`

Las interfaces de comportamiento adjuntan métodos ejecutables a uno o más tipos receptores mediante `at`. El receptor es el primer parámetro explícito, así que el método sigue siendo ordinario y la llamada usa la sintaxis `value.method()`. Cuando se omite su anotación para múltiples objetivos, el compilador especializa y comprueba la implementación por separado para cada objetivo. Una interface de comportamiento contiene métodos o shorthand bindings, no campos de datos.

<!-- @include: ../../.fragments/language/objects/behavior-interfaces.md -->

Los shorthand bindings nombran funciones declaradas en el mismo módulo o importadas desde otro módulo. Sus firmas se comprueban cuando se declara el comportamiento, y la firma importada se conserva entre módulos para la comprobación de tipos y la generación de código nativo. Las interfaces de comportamiento también pueden usar `extend` para heredar métodos de otras interfaces de comportamiento.

El receptor de `at` puede ser una interface nombrada o un tipo primitivo. La misma sintaxis de llamada por miembro funciona para valores numéricos, strings, booleanos y chars.

## Acceso a propiedad por nombre

Un campo se lee y se escribe con la forma de punto y con un literal de cadena entre corchetes, los dos son el mismo acceso: la clave está escrita en el código, así que nombra un campo que el compilador conoce, y ambos compilan al mismo offset fijo.

<!-- @include: ../../.fragments/language/objects/literal-property-access.md -->

Solo un literal califica. `person[clave]`, con `clave` siendo una variable, es error de compilación (`E0608`): una interfaz tiene un conjunto fijo de campos y ninguna búsqueda en tiempo de ejecución a la que recurrir. Un literal que no nombra ningún campo es `E0609`, reportado con el nombre. Para claves decididas en tiempo de ejecución, usa `Record<string, T>`, que es un hash map de verdad, `lookup["a"]` allí es una búsqueda que puede fallar, así que devuelve `T?`.

## Asignación de campo

Un campo puede asignarse en el lugar con `object.field = value`, incluidas las formas compuestas como `+=`, a través de receptores anidados y en elementos indexados.

<!-- @include: ../../.fragments/language/objects/field-assignment.md -->

La asignación en el lugar exige un binding mutable: un binding `const` es profundamente inmutable, y un campo `readonly` o un valor de un tipo `readonly` rechaza la escritura. La asignación en el lugar hoy funciona para campos de número, booleano y char; para cambiar un campo `string`, un array o un objeto anidado, construye un nuevo valor con object spread.

## Herencia de Interface

Una interface puede `extend` una o más interfaces para heredar sus campos. La interface hija exige todos los campos heredados más los propios, así que un valor del tipo hijo debe proporcionarlos todos. La herencia solo hace merge de campos, no hace que la hija sea asignable a la padre (ver el layout nominal abajo).

<!-- @include: ../../.fragments/language/objects/interface-inheritance.md -->

## Intersección de Tipos

Un alias `type` combina tipos de objeto con `&`. El alias exige todos los campos de todos los operandos, exactamente como lo hace la herencia, y cada operando es una interface nombrada o un objeto escrito inline. El `&` es la forma del alias de lo que `extend` hace en una interface: `interface` hereda con `extend`, `type` combina con `&`.

<!-- @include: ../../.fragments/language/objects/type-intersection.md -->

Cuando dos operandos declaran el mismo campo con tipos distintos, el alias se rechaza donde está escrito (`E3214`) en lugar de producir un campo que ningún valor puede satisfacer.

::: danger error
<!-- @include: ../../.fragments/language/objects/type-intersection-conflict.md -->
:::

El `&` pertenece a un alias `type` y a ningún otro lugar. No se acepta en una anotación de tipo (`E3215`): nombra la intersección primero y después usa el nombre.

::: danger error
<!-- @include: ../../.fragments/language/objects/type-intersection-annotation.md -->
:::

Todo operando debe ser un tipo de objeto. Un operando que sea un primitivo, un array, una unión o una función se rechaza con `E3213`, y un alias que se refiere a sí mismo por la cadena se rechaza con `E3216`. Mezclar `&` y `|` en el mismo alias se rechaza con `E3217`, un diagnóstico propio porque el motivo es la precedencia: el `|` liga más fuerte, así que `type A = B & C | D;` se leería como `B & (C | D)`. Los paréntesis por sí solos no ayudan, ya que una unión nunca es un operando válido. Nombra la intersección y usa ese nombre en la unión: `type BC = B & C;` y después `type A = BC | D;`.


## Layout Nativo Nominal

En tyrus no trata toda forma de objeto compatible como intercambiable. Las interfaces son tipos producto nativos nombrados, así que pasar un `CompleteUser` donde se espera `SummaryUser` exige un objeto de conversión explícito.

<!-- @include: ../../.fragments/language/objects/nominal-native-layout.md -->

Para guardar valores de varias formas en una misma colección, usa una tagged union con un campo discriminante literal y `match` en lugar de una interface base compartida, conserva los campos de cada variante y mantiene la exhaustividad:

<!-- @include: ../../.fragments/language/objects/tagged-union-narrowing.md -->

### Orden de Almacenamiento de los Campos

Los campos se almacenan en el orden que desperdicia menos espacio, que no siempre es el orden en que fueron escritos: un campo ancho se coloca delante de uno estrecho para que el hueco que el estrecho dejaría antes de él desaparezca. La interface de abajo ocupa 40 bytes escrita así y 24 bytes almacenada, y guardar un millón de ellas es la diferencia entre 40 MB y 24 MB.

<!-- @include: ../../.fragments/language/objects/field-storage-order.md -->

Nada cambia en el uso de la interface: los campos se leen, escriben, esparcen y desestructuran por nombre, un literal de objeto sigue evaluando sus campos en el orden en que los lista, e imprimir un valor sigue mostrando los campos en el orden en que la interface los declara. El orden es una decisión de almacenamiento y nunca es observable desde el programa.

## Type Aliases

<!-- @include: ../../.fragments/language/objects/type-aliases.md -->

Para formas de objeto, `type` e `interface` son equivalentes en layout nativo. Un alias `type` también puede nombrar unions, tuplas, tipos de función y utility types.

## Utilidades de Tipo

| Categoría | Utilities |
| --- | --- |
| Transformación de objeto | [`Partial<T>`](#partial-t), [`Optional<T, K>`](#optional-t-k), [`Required<T>`](#required-t), [`Readonly<T>`](#readonly-t), [`DeepPartial<T>`](#deeppartial-t) |
| Selección de claves | [`Pick<T, K>`](#pick-t-k), [`Omit<T, K>`](#omit-t-k) |
| Selección por tipo | [`PickByType<T, Type>`](#pickbytype-t-type), [`OmitByType<T, Type>`](#omitbytype-t-type) |
| Renombrado de claves | [`CamelCaseKeys<T>`](#camelcasekeys-t), [`SnakeCaseKeys<T>`](#snakecasekeys-t) |
| Control de campos | [`Require<T, K>`](#require-t-k), [`AtLeastOne<T, K>`](#atleastone-t-k) |
| Extracción de datos | [`DataOnly<T>`](#dataonly-t) |
| Mapeo clave/valor | [`keyof`](#keyof), [`Record<K, T>`](#record-k-t) |
| Tipos de función | [`ReturnType<T>`](#returntype-t), [`Parameters<T>`](#parameters-t) |
| Manipulación de uniones | [`Exclude<T, U>`](#exclude-t-u), [`Extract<T, U>`](#extract-t-u) |
| Tipos condicionales | [`T extends U ? X : Y`](#t-extends-u-x-y), [`infer R`](#infer-r) |

### Transformación de objeto

#### `Partial<T>`

Vuelve todo campo opcional, quien llama puede proporcionar cualquier subconjunto.

<SignatureBlock label="Firma">Partial&lt;T&gt; = { [K in keyof T]?: T[K] }</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/partial.md -->

::: danger error
<!-- @include: ../../.fragments/language/objects/type-utilities/partial-error.md -->
:::

#### `Optional<T, K>`

Mantiene todos los campos y vuelve opcionales solo las claves seleccionadas. La forma existente de un argumento, `Optional<T>`, sigue siendo la sintaxis de `T?`.

<SignatureBlock label="Firma">Optional&lt;T, K extends keyof T&gt; = { [P in keyof T]: P extends K ? T[P]? : T[P] }</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/optional.md -->

#### `Required<T>`

Vuelve todo campo opcional en obligatorio, todos los campos deben ser proporcionados.

<SignatureBlock label="Firma">Required&lt;T&gt; = { [K in keyof T]-?: T[K] }</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/required.md -->

#### `Readonly<T>`

Impide la reasignación de campos, el objeto entero se vuelve profundamente inmutable. Es uno de los seis lugares donde aparece la palabra clave; la página [Readonly](/es/language/readonly) los cubre todos.

<SignatureBlock label="Firma">Readonly&lt;T&gt; = { readonly [K in keyof T]: Readonly&lt;T[K]&gt; }</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/readonly.md -->

#### `DeepPartial<T>`

Vuelve todo campo opcional recursivamente, hasta 5 niveles de profundidad. Ideal para payloads de actualización parcial (PATCH).

<SignatureBlock label="Firma">DeepPartial&lt;T&gt; = { [K in keyof T]?: DeepPartial&lt;T[K]&gt; }</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/deep-partial.md -->

::: danger error
<!-- @include: ../../.fragments/language/objects/type-utilities/deep-partial-error.md -->
:::

### Selección de claves

#### `Pick<T, K>`

Selecciona un subconjunto de campos por clave, el resultado solo tiene las claves listadas.

<SignatureBlock label="Firma">Pick&lt;T, K extends keyof T&gt; = { [P in K]: T[P] }</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/pick.md -->

#### `Omit<T, K>`

Elimina un subconjunto de campos por clave, el resultado tiene todo excepto las claves listadas.

<SignatureBlock label="Firma">Omit&lt;T, K extends keyof T&gt; = { [P in Exclude&lt;keyof T, K&gt;]: T[P] }</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/omit.md -->

### Proyecciones de valor

`value.pick(keys)` y `value.omit(keys)` crean un objeto nuevo con los campos seleccionados o restantes. Las claves deben conocerse durante la compilación, en un array literal o en una variable con tipo finito de claves literales. El receptor no se muta, el orden de los campos sigue el tipo de origen y no hay búsqueda de claves en runtime.

<!-- @include: ../../.fragments/language/objects/type-utilities/optional.md -->

### Selección por tipo

#### `PickByType<T, Type>`

Selecciona campos por su tipo de valor, mantiene solo los campos cuyo tipo corresponde a `Type`.

<SignatureBlock label="Firma">PickByType&lt;T, Type&gt; = { [K in keyof T as T[K] extends Type ? K : never]: T[K] }</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/pick-by-type.md -->

::: danger error
<!-- @include: ../../.fragments/language/objects/type-utilities/pick-by-type-error.md -->
:::

#### `OmitByType<T, Type>`

Elimina campos por su tipo de valor, excluye todos los campos cuyo tipo corresponde a `Type`.

<SignatureBlock label="Firma">OmitByType&lt;T, Type&gt; = { [K in keyof T as T[K] extends Type ? never : K]: T[K] }</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/omit-by-type.md -->

::: danger error
<!-- @include: ../../.fragments/language/objects/type-utilities/omit-by-type-error.md -->
:::

### Mapeo clave/valor

#### `keyof`

Extrae las claves de un tipo como una unión literal de string.

<SignatureBlock label="Firma">keyof T = string literal union of T's keys</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/keyof.md -->

#### `Record<K, T>`

`Record<K, T>` es un tipo de utilidad integrado, como `Pick<T, K>` y `Omit<T, K>`. El compilador lo resuelve y no crea una clase `Record` en runtime. Cuando `K` es una unión finita de claves string literales, Tyrus produce un tipo objeto con un campo obligatorio por clave. Cuando `K` es abierto, como `string`, Tyrus lo resuelve como `Map<K, T>`, adecuado para claves dinámicas.

<SignatureBlock label="Firma">Record&lt;K extends string, T&gt; = campos obligatorios de objeto para K literal finito; Map&lt;K, T&gt; para K abierto</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/record.md -->

### Renombrado de claves

#### `CamelCaseKeys<T>`

Transforma todos los nombres de campo de `snake_case` a `camelCase`. Esencial para integraciones con APIs externas.

<SignatureBlock label="Firma">CamelCaseKeys&lt;T&gt; = { [K in camelCase&lt;keyof T&gt;]: T[snake_case&lt;K&gt;] }</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/camel-case-keys.md -->

::: danger error
<!-- @include: ../../.fragments/language/objects/type-utilities/camel-case-keys-error.md -->
:::

#### `SnakeCaseKeys<T>`

Transforma todos los nombres de campo de `camelCase` a `snake_case`. Útil para payloads de APIs legadas y bases de datos.

<SignatureBlock label="Firma">SnakeCaseKeys&lt;T&gt; = { [K in snake_case&lt;keyof T&gt;]: T[camelCase&lt;K&gt;] }</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/snake-case-keys.md -->

::: danger error
<!-- @include: ../../.fragments/language/objects/type-utilities/snake-case-keys-error.md -->
:::

### Tipos de función

#### `ReturnType<T>`

Infiera el tipo de retorno de un tipo función, extrae lo que la función produce.

<SignatureBlock label="Firma">ReturnType&lt;T extends (...args: any) =&gt; any&gt; = T's return type</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/returntype.md -->

#### `Parameters<T>`

Infiera los tipos de los parámetros de un tipo función como tupla, extrae lo que la función espera.

<SignatureBlock label="Firma">Parameters&lt;T extends (...args: any) =&gt; any&gt; = T's parameter tuple</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/parameters.md -->

### Control de campos

#### `Require<T, K>`

Vuelve campos específicos obligatorios. Es un noop si el campo ya es obligatorio.

<SignatureBlock label="Firma">Require&lt;T, K extends keyof T&gt; = T & { [P in K]-?: T[P] }</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/require.md -->

::: danger error
<!-- @include: ../../.fragments/language/objects/type-utilities/require-error.md -->
:::

#### `AtLeastOne<T, K>`

Vuelve todos los campos especificados opcionales pero requiere que al menos uno de ellos esté presente. Cada clave de `K` debe ser una propiedad de `T`.

<SignatureBlock label="Firma">AtLeastOne&lt;T, K extends keyof T&gt; = { [P in keyof T]: P extends K ? T[P]? : T[P] }</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/at-least-one.md -->

::: danger error
<!-- @include: ../../.fragments/language/objects/type-utilities/at-least-one-error.md -->
:::

### Extracción de datos

#### `DataOnly<T>`

Elimina todos los campos con tipo función, mantiene solo propiedades de datos puros. Ideal para payloads de API.

<SignatureBlock label="Firma">DataOnly&lt;T&gt; = { [K in keyof T as T[K] extends Function ? never : K]: T[K] }</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/data-only.md -->

::: danger error
<!-- @include: ../../.fragments/language/objects/type-utilities/data-only-error.md -->
:::

### Manipulación de uniones

#### `Exclude<T, U>`

Elimina miembros de `U` de `T`, mantiene todo en `T` que no está en `U`.

<SignatureBlock label="Firma">Exclude&lt;T, U&gt; = T extends U ? never : T</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/exclude.md -->

#### `Extract<T, U>`

Conserva solo miembros de `T` que también pertenecen a `U`, calcula la intersección.

<SignatureBlock label="Firma">Extract&lt;T, U&gt; = T extends U ? T : never</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/extract.md -->

### Tipos condicionales

Los tipos condicionales eligen entre dos tipos en tiempo de compilación. La restricción `extends` selecciona la rama, y `infer` extrae un tipo de dentro de la estructura coincidente.

#### `T extends U ? X : Y`

Un tipo condicional se resuelve a `X` cuando `T` satisface la restricción `U`, y a `Y` en caso contrario.

<SignatureBlock label="Firma">T extends U ? Then : Else</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/conditional.md -->

#### `infer R`

Vincula una variable de tipo dentro de la restricción de un condicional, cuando la restricción se cumple, el tipo inferido se convierte en la rama verdadera.

<SignatureBlock label="Firma">T extends (arg: i32) =&gt; infer R ? R : never</SignatureBlock>

<!-- @include: ../../.fragments/language/objects/type-utilities/infer.md -->

## Union Types

<!-- @include: ../../.fragments/language/objects/union-types.md -->

Tagged unions hacen narrowing mediante comprobaciones de discriminante literal. El compilador rastrea branches positivos y negativos para que cada branch exponga los campos disponibles del conjunto de variantes narrowed.

Las unions también funcionan con enums para payloads de eventos tipados:

<!-- @include: ../../.fragments/language/objects/union-with-enums.md -->

## Enums algebraicos

Los enums algebraicos modelan un conjunto cerrado de variantes, donde cada variante puede no tener payload, tener campos posicionales o campos con nombre. Un `match` sobre un enum algebraico debe ser exhaustivo salvo que incluya `_`; los patrones de tupla y struct vinculan directamente el payload seleccionado.

<!-- @include: ../../.fragments/language/objects/algebraic-enums.md -->
