---
title: BitSet
description: "Estrutura de dados de conjunto de bits compacto com operações de conjunto do tyrus:algorithm."
---

# BitSet

`BitSet` implementa um conjunto de bits compacto sobre uma capacidade fixa, apoiado em um array de palavras de 64 bits, com operações de conjunto.

## Construção

| Sintaxe | Descrição |
|---------|-----------|
| `BitSet(n)` | Cria um conjunto de bits com capacidade para `n` bits, todos inicialmente zerados |
| `BitSet.fromArray(arr)` | Cria um conjunto de bits com os bits das posições em `arr` ligados |

`fromArray` exige um argumento `usize[]`. As posições de bits são indexadas a partir de `0` até `n - 1`.

## Métodos

| Método | Assinatura | Retorno | Descrição |
|--------|------------|---------|-----------|
| `set` | `(index: usize, value?: boolean) => void` | `void` | Liga, desliga ou define condicionalmente o bit em `index` (padrão: `true`) |
| `flip` | `(index: usize) => void` | `void` | Alterna o bit em `index` |
| `get` | `(index: usize) => boolean` | `boolean` | Se o bit em `index` está ligado |
| `count` | `() => usize` | `usize` | Número de bits ligados |
| `isEmpty` | `() => boolean` | `boolean` | Se nenhum bit está ligado |
| `isSubset` | `(other: BitSet) => boolean` | `boolean` | Se todo bit ligado também está ligado em `other` |
| `isSuperset` | `(other: BitSet) => boolean` | `boolean` | Se todo bit ligado em `other` também está ligado |
| `union` | `(other: BitSet) => BitSet` | `BitSet` | Conjunto de bits com todos os bits de ambos os conjuntos |
| `intersection` | `(other: BitSet) => BitSet` | `BitSet` | Conjunto de bits com os bits ligados em ambos |
| `difference` | `(other: BitSet) => BitSet` | `BitSet` | Conjunto de bits com os bits ligados aqui mas não em `other` |
| `complement` | `() => BitSet` | `BitSet` | Conjunto de bits com cada bit alternado sobre a capacidade |
| `clear` | `() => void` | `void` | Zera todos os bits |

`union`, `intersection`, `difference` e `complement` retornam um `BitSet` novo de posse do chamador; ele é liberado automaticamente quando a variável sai de escopo.

## Complexidade

| Operação | Complexidade de Tempo |
|----------|-----------------------|
| `set` | O(1) |
| `flip` | O(1) |
| `get` | O(1) |
| `count` | O(n), número de palavras de máquina |
| `isEmpty` | O(n), número de palavras de máquina |
| `isSubset` | O(n), número de palavras de máquina |
| `isSuperset` | O(n), número de palavras de máquina |
| `union` | O(n), número de palavras de máquina |
| `intersection` | O(n), número de palavras de máquina |
| `difference` | O(n), número de palavras de máquina |
| `complement` | O(n), número de palavras de máquina |
| `clear` | O(n), número de palavras de máquina |

## Exemplos

### Construção

<!-- @include: ../../../.fragments/modules/algorithm/bit-set/construction.md -->

### set e get

<!-- @include: ../../../.fragments/modules/algorithm/bit-set/set-and-get.md -->

### flip

<!-- @include: ../../../.fragments/modules/algorithm/bit-set/flip.md -->

### count e isEmpty

<!-- @include: ../../../.fragments/modules/algorithm/bit-set/count-and-isempty.md -->

### isSubset e isSuperset

<!-- @include: ../../../.fragments/modules/algorithm/bit-set/issubset-and-issuperset.md -->

### union, intersection e difference

<!-- @include: ../../../.fragments/modules/algorithm/bit-set/union-intersection-and-difference.md -->

### complement

<!-- @include: ../../../.fragments/modules/algorithm/bit-set/complement.md -->

### fromArray

<!-- @include: ../../../.fragments/modules/algorithm/bit-set/fromarray.md -->

### clear

<!-- @include: ../../../.fragments/modules/algorithm/bit-set/clear.md -->
