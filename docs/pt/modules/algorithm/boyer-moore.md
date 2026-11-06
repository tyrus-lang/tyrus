---
title: boyerMoore
description: "Encontra ocorrências de substring com a regra do mau caractere."
---

# boyerMoore

Retorna todos os offsets byte zero-based onde `pattern` ocorre em `text`, pulando comparações por meio de um deslocamento baseado no mau caractere.

<!-- @include: ../../../.fragments/modules/algorithm/boyer-moore/boyermoore.md -->

## Resultado de estudo

<!-- @include: ../../../.fragments/modules/algorithm/boyer-moore/study-result.md -->

O resultado de estudo executa a mesma busca por mau caractere enquanto registra cada comparação e deslocamento aplicado. Seu armazenamento adicional é O(m + s), onde `m` é o tamanho do padrão e `s` é o número de alinhamentos inspecionados.

A busca usa tempo O(nm) no pior caso e espaço auxiliar O(1) para sua tabela de bytes.
