---
title: Math
description: Constantes e funções matemáticas de tyrus:math.
---

# Math

`tyrus:math` fornece constantes e funções matemáticas. Todas as constantes são valores `f64` disponíveis em tempo de compilação sem custo de runtime.

## Imports

<SignatureBlock label="Importações">import { PI, E, TAU } from "tyrus:math";</SignatureBlock>

## Itens Exportados

| Item | Tipo | Finalidade |
|------|------|-----------|
| [`Constants`](./constants.md) | constantes `f64` | Constantes matemáticas fundamentais |
| [`Functions`](./functions.md) | funções nativas | Helpers básicos, trigonométricos, hiperbólicos, exponenciais, logaritmos e seleção |

## Quick Start

Um exemplo mínimo usando constantes matemáticas:

<!-- @include: ../../../.fragments/modules/math/index/quick-start.md -->

::: tip Constantes sem custo
Estas constantes são emitidas como globais inicializados no binário gerado. Importá-las não adiciona overhead de runtime além de ler um valor global.
:::
