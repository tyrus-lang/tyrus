---
title: ahoCorasick
description: "Encontra múltiplos padrões em um texto."
---

# ahoCorasick

Busca todos os padrões em um texto e retorna pares `[endIndex, patternIndex]`. `endIndex` é o offset byte inclusivo da correspondência; os pares são ordenados por offset final e depois por índice do padrão.

<!-- @include: ../../../.fragments/modules/algorithm/aho-corasick/ahocorasick.md -->

## Resultado de estudo

<!-- @include: ../../../.fragments/modules/algorithm/aho-corasick/study-result.md -->

O resultado de estudo armazena um estado do autômato para cada byte de entrada. Seu armazenamento adicional é O(n + p), onde `n` é o tamanho do texto e `p` é o tamanho total dos padrões.

O autômato executa em O(tamanho do texto + soma dos tamanhos dos padrões + número de correspondências) e usa espaço auxiliar O(soma dos tamanhos dos padrões).
