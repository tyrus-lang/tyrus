---
title: Exponential Search
description: "Busca um array crescente de inteiros expandindo o intervalo exponencialmente."
---

# Exponential Search

Encontra um valor em um `i32[]` crescente expandindo o intervalo exponencialmente e depois aplicando busca binária. Retorna o índice zero-based como `usize?` ou ausência quando o alvo não existe.

<!-- @include: ../../../.fragments/modules/algorithm/exponential-search/exponential-search.md -->

## Resultado de estudo

`exponentialSearchInfo` retorna um `ExponentialSearchInfo` com as expansões bem-sucedidas e o intervalo final semiaberto pesquisado pela busca binária.

<!-- @include: ../../../.fragments/modules/algorithm/exponential-search/study-result.md -->

A busca executa em O(log n) e usa espaço auxiliar O(1).
