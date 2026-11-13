---
title: Operador pipe
description: O operador pipe, o token de topic e a composição de transformações em pipeline.
---

# Operador pipe

O operador `|>` passa um valor para uma chamada de função. O operador é sintaxe de tempo de compilação e não cria um objeto de pipeline em runtime.

<!-- @include: ../../.fragments/language/operators/pipeline.md -->

Sem `%`, o valor à esquerda é inserido como primeiro argumento da chamada à direita. Com `%`, o valor à esquerda é inserido exatamente na posição do topic.

Cadeias de pipeline se compõem com closures e métodos funcionais, fazendo as transformações de dados serem lidas de cima para baixo:

<!-- @include: ../../.fragments/language/operators/pipeline-closures.md -->

O token de topic `%` é útil quando uma função precisa do valor pipeado em uma posição diferente do primeiro argumento:

<!-- @include: ../../.fragments/language/operators/pipeline-topic.md -->
