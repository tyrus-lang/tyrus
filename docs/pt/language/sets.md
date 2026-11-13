---
title: Set
description: Coleções Set e suas operações nativas.
---

# Set


## Métodos e propriedades nativos

| Membro | Assinatura | Comportamento |
| --- | --- | --- |
| `size` | `usize` | Quantidade de valores distintos. |
| `add` | `add(value: T): Set<T>` | Insere um valor se ele ainda não existir e retorna o set. |
| `has` | `has(value: T): boolean` | Verifica se um valor está presente. |
| `delete` | `delete(value: T): boolean` | Remove um valor e informa se ele estava presente. |
| `values` | `values(): T[]` | Retorna os valores atuais como um array. |

Um set também pode ser percorrido diretamente com `for...of`. Adicionar um valor existente não cria uma duplicata.

<!-- @include: ../../.fragments/language/collections/set.md -->
