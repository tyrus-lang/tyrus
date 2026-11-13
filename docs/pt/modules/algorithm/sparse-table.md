---
title: SparseTable
description: "Tabela RMQ imutável de tyrus:algorithm."
---

# SparseTable

`SparseTable` pré-computa uma tabela imutável de mínimo por intervalo sobre um array de tamanho fixo de inteiros de 32 bits, respondendo consultas de mínimo em O(1) após um pré-processamento de O(n log n).

## Construção

| Sintaxe | Descrição |
|---------|-----------|
| `SparseTable(data)` | Constrói uma tabela RMQ sobre os valores em `data` |

A tabela é construída a partir de um array não genérico de valores `i32`. As posições são indexadas a partir de zero, de `0` a `len - 1`. O comprimento pode ser zero. Uma vez construída, a tabela é imutável: use `SegmentTree` quando precisar de atualizações pontuais.

## Métodos

| Método | Assinatura | Retorna | Descrição |
|--------|------------|---------|-----------|
| `query` | `(from: usize, to: usize) => i32` | `i32` | Retorna o mínimo do intervalo `[from, to)` |
| `len` | `() => usize` | `usize` | Retorna o número de posições |
| `isEmpty` | `() => boolean` | `boolean` | Se a tabela não tem posições |

Os intervalos usam limite superior exclusivo: `query(from, to)` retorna o mínimo dos elementos nas posições `from` até `to - 1`, então `query(0, len)` é o mínimo sobre todas as posições. `query` aborta com erro de runtime quando `from >= to` ou `to > len`.

## Complexidade

| Operação | Complexidade Temporal |
|----------|-----------------------|
| `query` | O(1) |
| `len` | O(1) |
| `isEmpty` | O(1) |

## Exemplos

### Construção

<!-- @include: ../../../.fragments/modules/algorithm/sparse-table/construction.md -->

### query

<!-- @include: ../../../.fragments/modules/algorithm/sparse-table/query.md -->

### len e isEmpty

<!-- @include: ../../../.fragments/modules/algorithm/sparse-table/len-and-isempty.md -->
