---
title: lcaBinaryLifting
description: "Tabelas de menor ancestral comum para árvores de tyrus:algorithm."
---

# lcaBinaryLifting

`lcaBinaryLifting(adj: usize[][], root: usize): Lca` constrói tabelas de binary lifting sobre a árvore não direcionada descrita pela lista de adjacência `adj`, onde `adj[v]` guarda os vizinhos do vértice `v` e `root` ancora cada consulta. A `Lca` retornada é um handle de propriedade do chamador de `tyrus:algorithm`; ela expõe `query(u, v)`, que devolve o menor ancestral comum dos vértices `u` e `v` em O(log n) com binary lifting.

## Construção

| Sintaxe | Descrição |
|---------|-----------|
| `lcaBinaryLifting(adj: usize[][], root: usize)` | Construir tabelas de LCA sobre a árvore `adj`, enraizada em `root` |

## Métodos

| Método | Assinatura | Retorna | Descrição |
|--------|------------|---------|-----------|
| `query` | `(u: usize, v: usize)` | `usize` | Menor ancestral comum dos vértices `u` e `v` |

## Complexidade

| Operação | Complexidade Temporal |
|----------|-----------------------|
| construção | O(n log n), tabelas de binary lifting |
| `query` | O(log n) |
| memória | O(n log n) |

## Exemplos

### árvore balanceada

<!-- @include: ../../../.fragments/modules/algorithm/lca/balanced-tree.md -->

### caminho profundo

<!-- @include: ../../../.fragments/modules/algorithm/lca/deep-path.md -->

### único vértice

<!-- @include: ../../../.fragments/modules/algorithm/lca/single-vertex.md -->

### importação por namespace

<!-- @include: ../../../.fragments/modules/algorithm/lca/namespace-import.md -->
