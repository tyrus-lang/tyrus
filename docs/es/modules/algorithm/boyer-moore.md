---
title: boyerMoore
description: "Encuentra coincidencias de subcadenas con la regla del mal carácter."
---

# boyerMoore

Devuelve todos los offsets de byte zero-based donde `pattern` aparece en `text`, omitiendo comparaciones mediante un desplazamiento basado en el mal carácter.

<!-- @include: ../../../.fragments/modules/algorithm/boyer-moore/boyermoore.md -->

## Resultado de estudio

<!-- @include: ../../../.fragments/modules/algorithm/boyer-moore/study-result.md -->

El resultado de estudio ejecuta la misma búsqueda por mal carácter mientras registra cada comparación y desplazamiento aplicado. Su almacenamiento adicional es O(m + s), donde `m` es el tamaño del patrón y `s` es el número de alineaciones inspeccionadas.

La búsqueda usa tiempo O(nm) en el peor caso y espacio auxiliar O(1) para su tabla de bytes.
