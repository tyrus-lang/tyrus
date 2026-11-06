---
title: Map
description: Colecciones Map y sus métodos nativos.
---

# Map




## Métodos y propiedades nativos

| Miembro | Firma | Comportamiento |
| --- | --- | --- |
| `size` | `usize` | Cantidad de pares clave-valor. |
| `set` | `set(key: K, value: V): Map<K, V>` | Inserta o reemplaza el valor de una clave y devuelve el map. |
| `get` | `get(key: K): V?` | Devuelve el valor o ausencia si la clave no existe. |
| `getOrInsert` | `getOrInsert(key: K, fallback: V): V` | Devuelve una referencia mutable al valor almacenado e inserta el fallback solo si la clave no existe. |
| `has` | `has(key: K): boolean` | Comprueba si una clave está presente. |
| `delete` | `delete(key: K): boolean` | Elimina una clave e indica si estaba presente. |
| `keys` | `keys(): K[]` | Devuelve las claves como un array. |
| `values` | `values(): V[]` | Devuelve los valores como un array. |
| `entries` | `entries(): [K, V][]` | Devuelve los pares clave-valor como un array; también se puede recorrer con `for...of`. |

## Map

<!-- @include: ../../.fragments/language/collections/map.md -->

`getOrInsert(key, value)` devuelve una referencia mutable al valor de `key`. La expresión alternativa es lazy y solo se ejecuta cuando falta la clave.
