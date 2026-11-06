---
title: BloomFilter
description: "Estrutura probabilística de pertinência em conjuntos com taxa de falso positivo configurável de tyrus:algorithm."
---

# BloomFilter

`BloomFilter` implementa uma estrutura probabilística de pertinência em conjuntos para chaves `string`. Um `BloomFilter(k, m)` armazena chaves em `m` bits usando `k` funções de hash independentes: `contains` nunca reporta falso negativo, mas pode reportar um falso positivo com probabilidade que diminui à medida que o número de bits por chave cresce. Ela não é genérica: ambos os argumentos do construtor são `usize`.

## Construção

| Sintaxe | Descrição |
|---------|-----------|
| `BloomFilter(k, m)` | Cria um novo filtro vazio com `k` funções de hash sobre `m` bits |

## Métodos

| Método | Assinatura | Retorna | Descrição |
|--------|------------|---------|-----------|
| `insert` | `(key: string) => void` | `void` | Adiciona `key` ao filtro |
| `contains` | `(key: string) => boolean` | `boolean` | Se `key` pode ter sido inserida; nunca é falso negativo |

## Complexidade

| Operação | Complexidade Temporal |
|----------|-----------------------|
| `insert` | O(k) |
| `contains` | O(k) |
| `BloomFilter(k, m)` | O(m / 64 + k) |

## Exemplos

### insert

<!-- @include: ../../../.fragments/modules/algorithm/bloom-filter/insert.md -->

### contains

<!-- @include: ../../../.fragments/modules/algorithm/bloom-filter/contains.md -->

O hashing é determinístico, então as falhas acima são garantidas, não probabilísticas. `contains` ainda permite falsos positivos em geral: quando o filtro está denso, uma chave ausente pode compartilhar todas as `k` posições de bits com chaves inseridas.
