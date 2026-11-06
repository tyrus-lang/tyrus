---
title: BloomFilter
description: "Estructura probabilística de pertenencia a conjuntos con tasa de falso positivo configurable de tyrus:algorithm."
---

# BloomFilter

`BloomFilter` implementa una estructura probabilística de pertenencia a conjuntos para claves `string`. Un `BloomFilter(k, m)` almacena claves en `m` bits usando `k` funciones de hash independientes: `contains` nunca reporta un falso negativo, pero puede reportar un falso positivo con una probabilidad que disminuye a medida que crece el número de bits por clave. No es genérico: ambos argumentos del constructor son `usize`.

## Construcción

| Sintaxis | Descripción |
|----------|-------------|
| `BloomFilter(k, m)` | Crea un nuevo filtro vacío con `k` funciones de hash sobre `m` bits |

## Métodos

| Método | Firma | Devuelve | Descripción |
|--------|-------|----------|-------------|
| `insert` | `(key: string) => void` | `void` | Agrega `key` al filtro |
| `contains` | `(key: string) => boolean` | `boolean` | Si `key` puede haber sido insertada; nunca es un falso negativo |

## Complejidad

| Operación | Complejidad Temporal |
|-----------|----------------------|
| `insert` | O(k) |
| `contains` | O(k) |
| `BloomFilter(k, m)` | O(m / 64 + k) |

## Ejemplos

### insert

<!-- @include: ../../../.fragments/modules/algorithm/bloom-filter/insert.md -->

### contains

<!-- @include: ../../../.fragments/modules/algorithm/bloom-filter/contains.md -->

El hashing es determinista, así que las ausencias anteriores están garantizadas, no son probabilísticas. `contains` sigue permitiendo falsos positivos en general: cuando el filtro está denso, una clave ausente puede compartir todas las `k` posiciones de bits con claves insertadas.
