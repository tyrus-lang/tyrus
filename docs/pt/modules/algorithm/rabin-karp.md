---
title: rabinKarp
description: "Encontra ocorrências de substring com rolling hash."
---

# rabinKarp

Retorna todos os offsets byte zero-based onde `pattern` ocorre em `text`. Correspondências de hash são verificadas contra os bytes originais, portanto colisões não produzem resultados falsos.

<!-- @include: ../../../.fragments/modules/algorithm/rabin-karp/rabinkarp.md -->

## Resultado para estudo

<!-- @include: ../../../.fragments/modules/algorithm/rabin-karp/study-result.md -->

A complexidade esperada é O(n + m), com espaço auxiliar O(1) além do resultado.
