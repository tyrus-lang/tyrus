---
title: heavyLightDecomposition
description: "Decomposição heavy-light de árvores de tyrus:algorithm."
---

# heavyLightDecomposition

`heavyLightDecomposition(adj: usize[][], root: usize): Hld` constrói uma decomposição heavy-light sobre a árvore não direcionada descrita pela lista de adjacência `adj`, onde `adj[v]` guarda os vizinhos do vértice `v` e `root` ancora cada consulta. A `Hld` retornada é um handle de propriedade do chamador de `tyrus:algorithm`; ela expõe `queryPath(u, v)`, que devolve os intervalos contíguos `[l, r]` (na ordem de `position`) cuja união cobre o caminho `u → v` na árvore em O(log n), prontos para alimentar um segment tree para consultas de caminho.

## Construção

| Sintaxe | Descrição |
|---------|-----------|
| `heavyLightDecomposition(adj: usize[][], root: usize)` | Decompor a árvore `adj` em cadeias pesadas, enraizada em `root` |

## Métodos

| Método | Assinatura | Retorna | Descrição |
|--------|------------|---------|-----------|
| `queryPath` | `(u: usize, v: usize)` | `[usize, usize][]` | Intervalos de cadeia cuja união cobre o caminho `u → v` |
| `position` | `(v: usize)` | `usize` | Posição linear por cadeia do vértice `v` |
| `head` | `(v: usize)` | `usize` | Vértice no topo da cadeia pesada que contém `v` |

## Complexidade

| Operação | Complexidade Temporal |
|----------|-----------------------|
| construção | O(n), uma DFS mais a atribuição de cadeias |
| `queryPath` | O(log n), no máximo O(log n) intervalos de cadeia |
| `position`, `head` | O(1) |
| memória | O(n) |

## Exemplos

### árvore balanceada

<!-- @include: ../../../.fragments/modules/algorithm/hld/balanced-tree.md -->

### caminho profundo

<!-- @include: ../../../.fragments/modules/algorithm/hld/deep-path.md -->

### único vértice

<!-- @include: ../../../.fragments/modules/algorithm/hld/single-vertex.md -->

### importação por namespace

<!-- @include: ../../../.fragments/modules/algorithm/hld/namespace-import.md -->
