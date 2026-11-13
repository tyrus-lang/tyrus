---
title: isCancelled
description: Pregunta, desde dentro de una tarea, si le dijeron que parara.
---

# isCancelled

`isCancelled` responde si a la tarea en ejecución le han pedido que pare.

<SignatureBlock label="Firma">isCancelled(): boolean</SignatureBlock>

Solo puede usarse dentro de una función `async`, porque la respuesta es sobre la tarea que se está ejecutando.

La cancelación es cooperativa: surte efecto en un punto de suspensión. Una tarea que pasa mucho tiempo calculando no alcanza ninguno, y por eso nunca se interrumpe. Eso es consecuencia del diseño y no un agujero en él, y es lo que un bucle así usa para pararse solo.

<!-- @include: ../../../.fragments/modules/task/is-cancelled.md -->

Parar así sigue siendo una cancelación. Un valor que la tarea devuelva tras ver la petición no se convierte en el valor de la Promise: el trabajo al que ya se le dijo que parara no llega a informar de un resultado.
