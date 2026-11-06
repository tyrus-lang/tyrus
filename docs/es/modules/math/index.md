---
title: Math
description: Constantes y funciones matemáticas de tyrus:math.
---

# Math

`tyrus:math` proporciona constantes y funciones matemáticas. Todas las constantes son valores `f64` disponibles en tiempo de compilación sin costo de runtime.

## Imports

<SignatureBlock label="Importaciones">import { PI, E, TAU } from "tyrus:math";</SignatureBlock>

## Elementos Exportados

| Elemento | Tipo | Finalidad |
|----------|------|-----------|
| [`Constants`](./constants.md) | constantes `f64` | Constantes matemáticas fundamentales |
| [`Functions`](./functions.md) | funciones nativas | Helpers básicos, trigonométricos, hiperbólicos, exponenciales, logaritmos y selección |

## Quick Start

Un ejemplo mínimo usando constantes matemáticas:

<!-- @include: ../../../.fragments/modules/math/index/quick-start.md -->

::: tip Constantes sin costo
Estas constantes se emiten como globales inicializadas en el binario generado. Importarlas no añade overhead de runtime más allá de leer un valor global.
:::
