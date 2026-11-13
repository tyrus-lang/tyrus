---
title: Interpolation Search
description: "Busca arrays de inteiros uniformemente distribuídos por interpolação."
---

# Interpolation Search

Busca um `i32[]` crescente estimando a posição do alvo a partir dos valores nos limites atuais. Retorna um `usize?` zero-based ou ausência quando o alvo não existe. O array deve estar ordenado em ordem crescente.

<!-- @include: ../../../.fragments/modules/algorithm/interpolation-search/interpolation-search.md -->

## Resultado de estudo

`interpolationSearchInfo` retorna um `InterpolationSearchInfo`, incluindo cada posição estimada usada pela busca.

<!-- @include: ../../../.fragments/modules/algorithm/interpolation-search/study-result.md -->

A complexidade média é O(log log n) para valores uniformemente distribuídos, com O(n) no pior caso e espaço auxiliar O(1).
