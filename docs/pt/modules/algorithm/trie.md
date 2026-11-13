---
title: Trie
description: "Estrutura de dados de árvore de prefixo para palavras de string de tyrus:algorithm."
---

# Trie

`Trie` implementa uma árvore de prefixo para palavras de string com busca, prefixo, inserção e consulta O(m), onde m é o comprimento da palavra. Ela não é genérica: `Trie()` cria uma árvore de prefixo vazia que armazena strings de comprimento arbitrário.

## Construção

| Sintaxe | Descrição |
|---------|-----------|
| `Trie()` | Cria uma nova árvore de prefixo vazia |

## Métodos

| Método | Assinatura | Retorna | Descrição |
|--------|------------|---------|-----------|
| `insert` | `(word: string) => void` | `void` | Insere uma cópia de `word`; inserções duplicadas são ignoradas |
| `search` | `(word: string) => boolean` | `boolean` | Se `word` está armazenada como uma palavra completa |
| `startsWith` | `(prefix: string) => boolean` | `boolean` | Se alguma palavra armazenada tem o prefixo informado |
| `len` | `() => usize` | `usize` | Número de palavras distintas armazenadas |
| `isEmpty` | `() => boolean` | `boolean` | Se a árvore não armazena palavras |
| `clear` | `() => void` | `void` | Remove todas as palavras |

## Complexidade

| Operação | Complexidade Temporal |
|----------|-----------------------|
| `insert` | O(m), comprimento da palavra |
| `search` | O(m), comprimento da palavra |
| `startsWith` | O(m), comprimento do prefixo |
| `len` | O(1) |
| `isEmpty` | O(1) |
| `clear` | O(n), número de nós armazenados |

## Exemplos

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
