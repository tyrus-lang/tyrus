---
title: Keywords
description: Palabras clave activas, palabras reservadas y built-ins ambientales en Tyrus.
---

# Keywords

Las palabras clave son tokens reservados en la gramática de Tyrus que tienen significados sintácticos específicos. No se pueden utilizar como identificadores de variables, constantes, funciones, interfaces, tipos, parámetros u otras declaraciones.

## Active Keywords

Las palabras clave activas son reconocidas y analizadas por el compilador y lexer de Tyrus. Están organizadas en cinco categorías funcionales principales.

### Declarations and Modifiers

Estas palabras clave introducen declaraciones, visibilidad de ámbito, inmutabilidad y evaluaciones en tiempo de compilación.

| Keyword | Description |
|---|---|
| `async` | Declara funciones asíncronas que devuelven promesas |
| `comptime` | Evalúa expresiones o bloques en tiempo de compilación |
| `const` | Declara bindings profundamente inmutables |
| `enum` | Define un tipo enumerado con variantes fijas |
| `export` | Publica un símbolo para uso en archivos de todo el proyecto |
| `from` | Especifica el módulo de origen en sentencias import |
| `function` | Declara una función nombrada o anónima |
| `import` | Carga declaraciones de otros archivos o módulos nativos |
| `interface` | Define una forma de objeto estructural o contrato |
| `let` | Declara un binding mutable |
| `protect` | Restringe el alcance de export a la carpeta declarante y subcarpetas |
| `readonly` | Impone inmutabilidad profunda en propiedades y tipos |
| `type` | Declara un alias de tipo |
| `validator` | Declara lógica de validación personalizada para decoradores |

### Control Flow and Error Handling

Estas palabras clave controlan bifurcaciones, bucles, retornos, limpiezas diferidas y manejo de excepciones.

| Keyword | Description |
|---|---|
| `break` | Termina la ejecución del bucle o switch más cercano |
| `case` | Define un caso de coincidencia en una sentencia switch |
| `catch` | Captura errores lanzados dentro de un bloque try |
| `continue` | Avanza la ejecución a la siguiente iteración del bucle |
| `default` | Rama de reserva predeterminada en una sentencia switch |
| `defer` | Programa limpieza incondicional al salir del ámbito actual |
| `do` | Inicia el bloque de un bucle do-while |
| `else` | Rama alternativa en estructuras condicionales |
| `errdefer` | Programa limpieza ejecutada solo si el ámbito sale con error |
| `finally` | Ejecuta limpieza independientemente de éxito o excepción |
| `for` | Itera sobre colecciones, rangos o secuencias numéricas |
| `if` | Rama de ejecución condicional |
| `loop` | Constructor de bucle infinito incondicional |
| `match` | Expresión de pattern matching con desestructuración |
| `onfail` | Asocia un manejador de recuperación en fallo de bloque |
| `return` | Devuelve un valor y termina la función actual |
| `switch` | Despacha control según valores literales o enums |
| `throw` | Lanza un error o excepción |
| `throws` | Declara contratos de errores recuperables en funciones |
| `try` | Ejecuta código con captura de excepciones |
| `while` | Ejecuta repetición mientras la condición sea verdadera |

### Expressions and Operators

Estas palabras clave actúan como operadores, conversiones de tipo o cláusulas en expresiones.

| Keyword | Description |
|---|---|
| `and` | Operador de conjunción lógica |
| `as` | Cláusula de aserción de tipo o alias en import |
| `at` | Cláusula de indexación posicional o destino de decorador |
| `await` | Pausa la ejecución esperando la resolución asíncrona |
| `extends` | Cláusula de restricción de tipo genérico |
| `in` | Prueba de pertenencia en bucles y expresiones |
| `is` | Operador de comprobación de tipos |
| `not` | Operador de negación lógica |
| `of` | Cláusula de iteración de valores en bucles for-of |
| `or` | Operador de disyunción lógica |
| `typeof` | Inspecciona metadatos de tipo en compilación o tiempo de ejecución |

### Primitive Types

Estas palabras clave representan los tipos escalares y numéricos primitivos integrados en el lenguaje.

| Keyword | Description |
|---|---|
| `boolean` | Tipo lógico verdadero o falso |
| `char` | Carácter escalar Unicode de 32 bits |
| `decimal` | Tipo aritmético decimal de precisión fija exacta |
| `f128` | Punto flotante IEEE 754 de 128 bits |
| `f32` | Punto flotante de precisión simple de 32 bits |
| `f64` | Punto flotante de precisión doble de 64 bits |
| `i128` | Entero con signo de 128 bits |
| `i16` | Entero con signo de 16 bits |
| `i32` | Entero con signo de 32 bits |
| `i64` | Entero con signo de 64 bits |
| `i8` | Entero con signo de 8 bits |
| `isize` | Entero con signo del tamaño de puntero de la arquitectura |
| `never` | Tipo inferior que representa código o valores inalcanzables |
| `string` | Tipo cadena inmutable codificada en UTF-8 |
| `u128` | Entero sin signo de 128 bits |
| `u16` | Entero sin signo de 16 bits |
| `u32` | Entero sin signo de 32 bits |
| `u64` | Entero sin signo de 64 bits |
| `u8` | Entero sin signo de 8 bits |
| `unknown` | Tipo superior seguro que requiere narrowing antes de su uso |
| `usize` | Entero sin signo del tamaño de puntero de la arquitectura |
| `void` | Tipo de retorno para funciones que no producen valor |

### Literal Values

Estas palabras clave denotan literales primitivos y constantes de ausencia.

| Keyword | Description |
|---|---|
| `false` | Literal booleano falso |
| `Infinity` | Literal de infinito positivo en punto flotante |
| `NaN` | Literal de valor no numérico (Not-a-Number) |
| `None` | Centinela de ausencia opcional en pattern matching |
| `null` | Literal de ausencia reservado (prohibido como valor en tiempo de ejecución) |
| `true` | Literal booleano verdadero |

## Reserved for Future Use

Estas palabras clave están reservadas para la evolución futura del lenguaje y no pueden utilizarse como identificadores.

| Category | Keywords | Description |
|---|---|---|
| Clases y POO | `class`, `constructor`, `implements`, `instanceof`, `new`, `override`, `super`, `this` | Reservadas para posibles constructos orientados a objetos y clases |
| Visibilidad y Modificadores | `abstract`, `private`, `protected`, `public`, `static` | Reservadas para visibilidad de miembros y calificadores de ámbito |
| Módulos y Ámbitos | `declare`, `module`, `namespace`, `package`, `require`, `var` | Reservadas para declaraciones ambientales y extensiones de módulos |
| Sistema de Tipos | `asserts`, `infer`, `intrinsic`, `keyof`, `satisfies`, `unique` | Reservadas para operadores avanzados de tipos y narrowing |
| Descriptores y Parámetros | `get`, `out`, `set` | Reservadas para descriptores de propiedades y modos de paso |
| Ejecución y Control | `debugger`, `delete`, `when`, `with`, `yield` | Reservadas para depuración, generadores y condiciones de patrones |

## Ambient Built-ins

Estos identificadores integrados están disponibles en el ámbito global sin necesidad de una sentencia import explícita.

| Identifier | Description |
|---|---|
| `assert` | Función de aserción en tiempo de ejecución que verifica una condición |
| `assertEqual` | Función de verificación de igualdad entre dos valores |
| `assertThrows` | Función que verifica si una clausura lanza el error esperado |
| `console` | Espacio de nombres estándar para registro en consola |
| `isNaN` | Comprueba si un valor numérico es NaN |
| `Map` | Colección de mapa asociativo clave-valor integrada |
| `panic` | Aborta la ejecución inmediatamente con un mensaje de error |
| `print` | Función de impresión en la salida estándar stdout |
| `printCR` | Función auxiliar de retorno de carro para progreso en terminal |
| `printError` | Función de impresión en el flujo de error estándar stderr |
| `printRaw` | Función de impresión sin formato adicional |
| `printWarn` | Función de advertencia en el flujo de error estándar stderr |
| `Set` | Colección de conjunto de valores únicos integrada |

