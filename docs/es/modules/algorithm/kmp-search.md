---
title: kmpSearch
description: "Encuentra todas las coincidencias exactas con preprocesamiento de prefijos."
---

# kmpSearch

Devuelve todos los offsets de byte zero-based donde `pattern` aparece en `text`. Un patrón vacío coincide en cada límite, por lo que el resultado contiene offsets de `0` a `text.length`.

<!-- @include: ../../../.fragments/modules/algorithm/kmp-search/kmpsearch.md -->

## Resultado para estudio

<!-- @include: ../../../.fragments/modules/algorithm/kmp-search/study-result.md -->

El algoritmo se ejecuta en O(n + m) y usa espacio auxiliar O(m).
