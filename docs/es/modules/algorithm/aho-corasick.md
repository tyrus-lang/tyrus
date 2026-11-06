---
title: ahoCorasick
description: "Encuentra múltiples patrones en un texto."
---

# ahoCorasick

Busca todos los patrones en un texto y devuelve pares `[endIndex, patternIndex]`. `endIndex` es el offset de byte inclusivo de la coincidencia; los pares se ordenan por offset final y después por índice del patrón.

<!-- @include: ../../../.fragments/modules/algorithm/aho-corasick/ahocorasick.md -->

## Resultado de estudio

<!-- @include: ../../../.fragments/modules/algorithm/aho-corasick/study-result.md -->

El resultado de estudio almacena un estado del autómata por cada byte de entrada. Su almacenamiento adicional es O(n + p), donde `n` es la longitud del texto y `p` es la longitud total de los patrones.

El autómata se ejecuta en O(longitud del texto + suma de longitudes de patrones + número de coincidencias) y usa espacio auxiliar O(suma de longitudes de patrones).
