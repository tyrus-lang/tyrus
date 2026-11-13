---
title: kmpSearch
description: "Encontra todas as ocorrências exatas com pré-processamento de prefixos."
---

# kmpSearch

Retorna todos os offsets byte zero-based onde `pattern` ocorre em `text`. Um padrão vazio ocorre em cada limite, então o resultado contém offsets de `0` até `text.length`.

<!-- @include: ../../../.fragments/modules/algorithm/kmp-search/kmpsearch.md -->

## Resultado para estudo

<!-- @include: ../../../.fragments/modules/algorithm/kmp-search/study-result.md -->

O algoritmo executa em O(n + m) e usa espaço auxiliar O(m).
