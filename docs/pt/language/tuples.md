---
title: Tuplas
description: Tipos de tupla de tamanho fixo, indexação e desestruturação.
---

# Tuplas


## Operações nativas de tupla

| Operação | Comportamento |
| --- | --- |
| `length` | Tamanho da tupla, somente leitura e conhecido em tempo de compilação. |
| `tuple[index]` | Lê ou escreve um elemento usando um índice literal inteiro; o tipo depende da posição. |
| Desestruturação | Extrai elementos da tupla para bindings locais; `_` descarta um elemento. |
| Métodos mutáveis de array | Métodos que alteram elementos ou tamanho são rejeitados porque a tupla tem layout fixo. |

Métodos que retornam um novo array podem ser usados quando suas assinaturas aceitam uma sequência; consulte [Array](./arrays).
<!-- @include: ../../.fragments/language/collections/tuples.md -->

Tuplas viram structs nativas. Índices de tupla precisam ser literais inteiros, então acesso fora do intervalo é erro de compilação.

Um wildcard `_` numa posição de desestruturação descarta aquele elemento: ele ainda é extraído e liberado como qualquer outro local, mas não declara nenhum nome que o resto do escopo possa ler de volta, então um `_` posterior no mesmo padrão nunca colide com um anterior. Ler `_` depois disso é um erro comum de identificador não declarado, igual a qualquer outro nome nunca declarado.
