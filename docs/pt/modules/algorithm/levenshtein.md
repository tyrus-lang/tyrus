---
title: levenshtein
description: "Calcula a distância de edição entre duas strings."
---

# levenshtein

Retorna o número mínimo de inserções, remoções e substituições para transformar uma string em outra. A distância é medida sobre bytes UTF-8.

<!-- @include: ../../../.fragments/modules/algorithm/levenshtein/levenshtein.md -->

## Resultado de estudo

<!-- @include: ../../../.fragments/modules/algorithm/levenshtein/study-result.md -->

O resultado de estudo mantém a linha incremental usada pelo cálculo e informa suas dimensões e células avaliadas sem retornar uma matriz quadrática.

O algoritmo executa em O(nm) e usa espaço auxiliar O(m).
