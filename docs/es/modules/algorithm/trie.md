---
title: Trie
description: "Estructura de datos de árbol de prefijo para palabras de string de tyrus:algorithm."
---

# Trie

`Trie` implementa un árbol de prefijo para palabras de string con búsqueda, prefijo, inserción y consulta O(m), donde m es la longitud de la palabra. No es genérico: `Trie()` crea un árbol de prefijo vacío que almacena strings de longitud arbitraria.

## Construcción

| Sintaxis | Descripción |
|----------|-------------|
| `Trie()` | Crea un nuevo árbol de prefijo vacío |

## Métodos

| Método | Firma | Devuelve | Descripción |
|--------|-------|----------|-------------|
| `insert` | `(word: string) => void` | `void` | Inserta una copia de `word`; las inserciones duplicadas se ignoran |
| `search` | `(word: string) => boolean` | `boolean` | Si `word` está almacenada como una palabra completa |
| `startsWith` | `(prefix: string) => boolean` | `boolean` | Si alguna palabra almacenada tiene el prefijo dado |
| `len` | `() => usize` | `usize` | Número de palabras distintas almacenadas |
| `isEmpty` | `() => boolean` | `boolean` | Si el árbol no almacena palabras |
| `clear` | `() => void` | `void` | Elimina todas las palabras |

## Complejidad

| Operación | Complejidad Temporal |
|-----------|----------------------|
| `insert` | O(m), longitud de la palabra |
| `search` | O(m), longitud de la palabra |
| `startsWith` | O(m), longitud del prefijo |
| `len` | O(1) |
| `isEmpty` | O(1) |
| `clear` | O(n), número de nodos almacenados |

## Ejemplos

### insert

<!-- @include: ../../../.fragments/modules/algorithm/trie/insert.md -->

### search

<!-- @include: ../../../.fragments/modules/algorithm/trie/search.md -->

### startsWith

<!-- @include: ../../../.fragments/modules/algorithm/trie/startswith.md -->

### len e isEmpty

<!-- @include: ../../../.fragments/modules/algorithm/trie/len-and-isempty.md -->

### clear

<!-- @include: ../../../.fragments/modules/algorithm/trie/clear.md -->
