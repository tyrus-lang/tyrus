---
title: Operador pipe
description: El operador pipe, el token topic y la composición de transformaciones en pipeline.
---

# Operador pipe

El operador `|>` pasa un valor a una llamada de función. El operador es sintaxis de tiempo de compilación y no crea un objeto de pipeline en runtime.

<!-- @include: ../../.fragments/language/operators/pipeline.md -->

Sin `%`, el valor izquierdo se inserta como primer argumento de la llamada derecha. Con `%`, el valor izquierdo se inserta exactamente en la posición del topic.

Las cadenas de pipeline se componen con closures y métodos funcionales, haciendo que las transformaciones de datos se lean de arriba a abajo:

<!-- @include: ../../.fragments/language/operators/pipeline-closures.md -->

El token topic `%` es útil cuando una función necesita el valor pipeado en una posición distinta al primer argumento:

<!-- @include: ../../.fragments/language/operators/pipeline-topic.md -->
