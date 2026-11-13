---
title: Tuplas
description: Tipos de tupla de tamaño fijo, indexación y desestructuración.
---

# Tuplas


## Operaciones nativas de tupla

| Operación | Comportamiento |
| --- | --- |
| `length` | Longitud de la tupla, de solo lectura y conocida en tiempo de compilación. |
| `tuple[index]` | Lee o escribe un elemento con un índice literal entero; el tipo depende de la posición. |
| Desestructuración | Extrae elementos de la tupla en bindings locales; `_` descarta un elemento. |
| Métodos mutables de array | Los métodos que cambian elementos o longitud se rechazan porque la tupla tiene un layout fijo. |

Se pueden usar métodos que devuelven un array nuevo cuando sus firmas aceptan una secuencia; consulta [Array](./arrays).
<!-- @include: ../../.fragments/language/collections/tuples.md -->

Las tuplas se traducen a structs nativos. Los índices de tupla deben ser literales enteros, por lo que un acceso fuera de rango es un error de compilación.

Un wildcard `_` en una posición de desestructuración descarta ese elemento: igual se extrae y se libera como cualquier otra variable local, pero no declara ningún nombre que el resto del ámbito pueda volver a leer, así que un `_` posterior en el mismo patrón nunca choca con uno anterior. Leer `_` después es un error común de identificador no declarado, igual que cualquier otro nombre nunca declarado.
