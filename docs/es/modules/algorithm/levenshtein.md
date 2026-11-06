---
title: levenshtein
description: "Calcula la distancia de edición entre dos cadenas."
---

# levenshtein

Devuelve el número mínimo de inserciones, eliminaciones y sustituciones para transformar una cadena en otra. La distancia se mide sobre bytes UTF-8.

<!-- @include: ../../../.fragments/modules/algorithm/levenshtein/levenshtein.md -->

## Resultado de estudio

<!-- @include: ../../../.fragments/modules/algorithm/levenshtein/study-result.md -->

El resultado de estudio conserva la fila incremental utilizada por el cálculo e informa sus dimensiones y celdas evaluadas sin devolver una matriz cuadrática.

El algoritmo se ejecuta en O(nm) y usa espacio auxiliar O(m).
