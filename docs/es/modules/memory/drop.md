---
title: drop
description: Libera un valor con propiedad antes de salir del ámbito.
---

# drop

<SignatureBlock label="Firma">drop(value: string): void</SignatureBlock>

<SignatureBlock label="Firma">drop&lt;T&gt;(value: T[]): void</SignatureBlock>

`drop(value)` consume un `string`, array, colección o handle nativo local y libera sus recursos inmediatamente. Los valores estáticos no necesitan liberación. El binding no puede usarse después.

Cuando el flujo de control tiene más de una ruta posible, el binding solo se considera descartado después de la bifurcación cuando todas las rutas que continúan lo descartan. Una ruta que termina con `return` o `throw` no afecta la disponibilidad de una ruta hermana que continúa.

<!-- @include: ../../../.fragments/modules/memory/drop.md -->
