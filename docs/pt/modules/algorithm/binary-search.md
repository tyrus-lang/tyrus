---
title: Binary Search
description: "Busca um array crescente de inteiros com busca binária."
---

# Binary Search

Busca um `i32[]` crescente e retorna o índice zero-based como `usize?`. O resultado fica ausente quando o valor não ocorre. O array já deve estar ordenado em ordem crescente.

<!-- @include: ../../../.fragments/modules/algorithm/binary-search/binary-search.md -->

## Resultado de estudo

`binarySearchInfo` retorna um `SearchInfo` com snapshot independente da entrada e comparações e probes determinísticos.

<!-- @include: ../../../.fragments/modules/algorithm/binary-search/study-result.md -->

A busca executa em O(log n) e usa espaço auxiliar O(1).
