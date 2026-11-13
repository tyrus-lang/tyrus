---
title: centroidDecomposition
description: "Decomposição centroid de árvores de tyrus:algorithm."
---

# centroidDecomposition

`centroidDecomposition(adj: usize[][]): Centroid` constrói a decomposição centroid sobre a árvore não direcionada descrita pela lista de adjacência `adj`, onde `adj[v]` guarda os vizinhos do vértice `v`. A `Centroid` retornada é um handle de propriedade do chamador de `tyrus:algorithm`; todo vértice da árvore de entrada vira um nó da árvore centroid, e remover um centroid divide cada componente em componentes filhos com profundidade de no máximo metade do seu tamanho. Ela expõe `parentOf(v)`, que devolve o pai na árvore centroid de `v` como `usize?` (a raiz centroid global não tem pai), e `depth(v)`, a profundidade de `v` na árvore centroid (raiz com profundidade 0).

## Construção

| Sintaxe | Descrição |
|---------|-----------|
| `centroidDecomposition(adj: usize[][])` | Decompor a árvore `adj` em uma árvore centroid |

## Métodos

| Método | Assinatura | Retorna | Descrição |
|--------|------------|---------|-----------|
| `parentOf` | `(v: usize)` | `usize?` | Pai na árvore centroid de `v`; ausente para a raiz centroid global |
| `depth` | `(v: usize)` | `usize` | Profundidade de `v` na árvore centroid (raiz com profundidade 0) |

## Complexidade

| Operação | Complexidade Temporal |
|----------|-----------------------|
| construção | O(n), cálculo iterativo de tamanhos por componente e busca do centroid |
| `parentOf`, `depth` | O(1) |
| memória | O(n) |

## Exemplos

### árvore balanceada

<!-- @include: ../../../.fragments/modules/algorithm/centroid/balanced-tree.md -->

### caminho profundo

<!-- @include: ../../../.fragments/modules/algorithm/centroid/deep-path.md -->

### único vértice

<!-- @include: ../../../.fragments/modules/algorithm/centroid/single-vertex.md -->

### importação por namespace

<!-- @include: ../../../.fragments/modules/algorithm/centroid/namespace-import.md -->
