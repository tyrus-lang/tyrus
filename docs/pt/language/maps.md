---
title: Map
description: Coleções Map e seus métodos nativos.
---

# Map




## Métodos e propriedades nativos

| Membro | Assinatura | Comportamento |
| --- | --- | --- |
| `size` | `usize` | Quantidade de pares chave-valor. |
| `set` | `set(key: K, value: V): Map<K, V>` | Insere ou substitui o valor de uma chave e retorna o map. |
| `get` | `get(key: K): V?` | Retorna o valor ou ausência se a chave não existir. |
| `getOrInsert` | `getOrInsert(key: K, fallback: V): V` | Retorna uma referência mutável ao valor armazenado e insere o fallback apenas se a chave não existir. |
| `has` | `has(key: K): boolean` | Verifica se uma chave está presente. |
| `delete` | `delete(key: K): boolean` | Remove uma chave e informa se ela estava presente. |
| `keys` | `keys(): K[]` | Retorna as chaves como um array. |
| `values` | `values(): V[]` | Retorna os valores como um array. |
| `entries` | `entries(): [K, V][]` | Retorna os pares chave-valor como um array; também pode ser percorrido com `for...of`. |

## Map

<!-- @include: ../../.fragments/language/collections/map.md -->

`getOrInsert(key, value)` retorna uma referência mutável ao valor de `key`. A expressão alternativa é lazy e só é executada quando a chave está ausente.
