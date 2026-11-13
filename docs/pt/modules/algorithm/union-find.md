---
title: UnionFind
description: "Estrutura de dados disjoint set (union-find) de tyrus:algorithm."
---

# UnionFind

`UnionFind<T>` implementa uma estrutura de dados disjoint set (union-find) com compressão de caminho e união por rank. Ele mantém uma coleção de conjuntos não sobrepostos e suporta operações eficientes de união e busca.

## Construção

| Sintaxe | Descrição |
|---------|-----------|
| `UnionFind<u32>(n)` | Cria um novo conjunto disjunto com `n` elementos indexados de `0` a `n - 1` |
| `UnionFind<u8>(n)` | Mesmo, com tipo de elemento `u8` |
| `UnionFind<u16>(n)` | Mesmo, com tipo de elemento `u16` |
| `UnionFind<u64>(n)` | Mesmo, com tipo de elemento `u64` |

Somente tipos de inteiro sem sinal são permitidos como tipo de elemento.

## Métodos

| Método | Assinatura | Retorna | Descrição |
|--------|------------|---------|-----------|
| `find` | `(T) => T` | `T` | Representante raiz do conjunto que contém `x` |
| `union` | `(T, T) => boolean` | `boolean` | Funde conjuntos; retorna `true` se houve fusão |
| `connected` | `(T, T) => boolean` | `boolean` | Se `x` e `y` pertencem ao mesmo conjunto |
| `setCount` | `() => T` | `T` | Número de conjuntos disjuntos |

Os elementos são inteiros indexados a partir de `0` até `n - 1`. O handle nativo é liberado automaticamente quando a variável sai de escopo.

## Complexidade

| Operação | Complexidade Temporal |
|----------|-----------------------|
| `find` | Amortizado O(α(n)), praticamente constante |
| `union` | Amortizado O(α(n)), praticamente constante |
| `connected` | Amortizado O(α(n)), praticamente constante |
| `setCount` | O(1) |

Onde α é a função inversa de Ackermann, que é efetivamente constante para todas as entradas práticas.

## Exemplos

### Construção

<!-- @include: ../../../.fragments/modules/algorithm/union-find/construction.md -->

### find

<!-- @include: ../../../.fragments/modules/algorithm/union-find/find.md -->

### union

<!-- @include: ../../../.fragments/modules/algorithm/union-find/union.md -->

### connected

<!-- @include: ../../../.fragments/modules/algorithm/union-find/connected.md -->

### setCount

<!-- @include: ../../../.fragments/modules/algorithm/union-find/setcount.md -->

