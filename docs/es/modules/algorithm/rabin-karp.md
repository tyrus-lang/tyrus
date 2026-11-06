---
title: rabinKarp
description: "Encuentra coincidencias de subcadenas con rolling hash."
---

# rabinKarp

Devuelve todos los offsets de byte zero-based donde `pattern` aparece en `text`. Las coincidencias de hash se verifican contra los bytes originales, por lo que las colisiones no producen resultados falsos.

<!-- @include: ../../../.fragments/modules/algorithm/rabin-karp/rabinkarp.md -->

## Resultado para estudio

<!-- @include: ../../../.fragments/modules/algorithm/rabin-karp/study-result.md -->

La complejidad esperada es O(n + m), con espacio auxiliar O(1) aparte del resultado.
