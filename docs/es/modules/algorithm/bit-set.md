---
title: BitSet
description: "Estructura de datos de conjunto de bits compacto con operaciones de conjunto de tyrus:algorithm."
---

# BitSet

`BitSet` implementa un conjunto de bits compacto sobre una capacidad fija, respaldado por un array de palabras de 64 bits, con operaciones de conjunto.

## Construcción

| Sintaxis | Descripción |
|--------|-------------|
| `BitSet(n)` | Crea un conjunto de bits con capacidad para `n` bits, todos inicialmente en cero |
| `BitSet.fromArray(arr)` | Crea un conjunto de bits con los bits de las posiciones en `arr` activados |

`fromArray` requiere un argumento `usize[]`. Las posiciones de bits están indexadas desde `0` hasta `n - 1`.

## Métodos

| Método | Firma | Retorno | Descripción |
|--------|-----------|---------|-------------|
| `set` | `(index: usize, value?: boolean) => void` | `void` | Activa, desactiva o define condicionalmente el bit en `index` (por defecto: `true`) |
| `flip` | `(index: usize) => void` | `void` | Alterna el bit en `index` |
| `get` | `(index: usize) => boolean` | `boolean` | Si el bit en `index` está activado |
| `count` | `() => usize` | `usize` | Número de bits activados |
| `isEmpty` | `() => boolean` | `boolean` | Si no hay bits activados |
| `isSubset` | `(other: BitSet) => boolean` | `boolean` | Si todo bit activado también está activado en `other` |
| `isSuperset` | `(other: BitSet) => boolean` | `boolean` | Si todo bit activado en `other` también está activado |
| `union` | `(other: BitSet) => BitSet` | `BitSet` | Conjunto de bits con todos los bits de ambos conjuntos |
| `intersection` | `(other: BitSet) => BitSet` | `BitSet` | Conjunto de bits con los bits activados en ambos |
| `difference` | `(other: BitSet) => BitSet` | `BitSet` | Conjunto de bits con bits activados aquí pero no en `other` |
| `complement` | `() => BitSet` | `BitSet` | Conjunto de bits con cada bit alternado sobre la capacidad |
| `clear` | `() => void` | `void` | Limpia todos los bits |

`union`, `intersection`, `difference` y `complement` devuelven un `BitSet` nuevo propiedad del llamador; se libera automáticamente cuando la variable sale del ámbito.

## Complejidad

| Operación | Complejidad de Tiempo |
|-----------|-----------------|
| `set` | O(1) |
| `flip` | O(1) |
| `get` | O(1) |
| `count` | O(n), número de palabras de máquina |
| `isEmpty` | O(n), número de palabras de máquina |
| `isSubset` | O(n), número de palabras de máquina |
| `isSuperset` | O(n), número de palabras de máquina |
| `union` | O(n), número de palabras de máquina |
| `intersection` | O(n), número de palabras de máquina |
| `difference` | O(n), número de palabras de máquina |
| `complement` | O(n), número de palabras de máquina |
| `clear` | O(n), número de palabras de máquina |

## Ejemplos

### Construcción

<!-- @include: ../../../.fragments/modules/algorithm/bit-set/construction.md -->

### set y get

<!-- @include: ../../../.fragments/modules/algorithm/bit-set/set-and-get.md -->

### flip

<!-- @include: ../../../.fragments/modules/algorithm/bit-set/flip.md -->

### count e isEmpty

<!-- @include: ../../../.fragments/modules/algorithm/bit-set/count-and-isempty.md -->

### isSubset y isSuperset

<!-- @include: ../../../.fragments/modules/algorithm/bit-set/issubset-and-issuperset.md -->

### union, intersection y difference

<!-- @include: ../../../.fragments/modules/algorithm/bit-set/union-intersection-and-difference.md -->

### complement

<!-- @include: ../../../.fragments/modules/algorithm/bit-set/complement.md -->

### fromArray

<!-- @include: ../../../.fragments/modules/algorithm/bit-set/fromarray.md -->

### clear

<!-- @include: ../../../.fragments/modules/algorithm/bit-set/clear.md -->
