---
title: SortedMap
description: "Mapa ordenado de chave-valor de tyrus:algorithm."
---

# SortedMap

`SortedMap<K, V>` é um mapa ordenado de chave → valor implementado sobre uma árvore binária de busca balanceada. Ele mantém cada entrada ordenada pela chave. O handle nativo é liberado automaticamente quando a variável sai de escopo.

## Construção

| Sintaxe | Descrição |
|---------|-----------|
| `SortedMap<K, V>()` | Cria um novo mapa ordenado vazio |

`K` pode ser qualquer tipo inteiro, qualquer tipo de ponto flutuante, `boolean` ou `string`. `V` pode ser qualquer tipo.

## Métodos

| Método | Assinatura | Retorna | Descrição |
|--------|------------|---------|-----------|
| `set` | `(K, V) => void` | `void` | Insere ou substitui a entrada para `key` |
| `get` | `(K) => V?` | `V?` | Valor para `key`, ou nenhum valor quando ausente |
| `has` | `(K) => boolean` | `boolean` | Se `key` está presente |
| `delete` | `(K) => boolean` | `boolean` | Remove a entrada para `key`; retorna `true` quando ela estava presente |
| `min` | `() => K?` | `K?` | Menor chave, ou nenhum valor quando vazio |
| `max` | `() => K?` | `K?` | Maior chave, ou nenhum valor quando vazio |
| `len` | `() => usize` | `usize` | Número de entradas |
| `isEmpty` | `() => boolean` | `boolean` | Se o mapa está vazio |
| `clear` | `() => void` | `void` | Remove todas as entradas |

## Complexidade

| Operação | Complexidade Temporal |
|----------|-----------------------|
| `set` | O(log n) |
| `get` / `has` | O(log n) |
| `delete` | O(log n) |
| `min` / `max` | O(log n) |
| `len` / `isEmpty` | O(1) |
| `clear` | O(n) |

A árvore permanece balanceada, então toda busca, inserção e remoção é logarítmica independentemente da ordem de inserção.

## Exemplos

### Exemplo completo

<!-- @include: ../../../.fragments/modules/algorithm/sorted-map/complete-example.md -->

### Chaves de string

<!-- @include: ../../../.fragments/modules/algorithm/sorted-map/string-keys.md -->
