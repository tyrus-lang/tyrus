---
title: cancel
description: Pide a una tarea que pare, y espera a que termine de parar.
---

# cancel

`cancel` pide a una Promise pendiente que pare. El trabajo se desenrolla por su limpieza y la Promise se resuelve como cancelada.

<SignatureBlock label="Firma">cancel&lt;T&gt;(promise: Promise&lt;T&gt;, reason: string, graceMillis?: u64): boolean</SignatureBlock>

Devuelve `true` solo para la llamada que inicia la cancelación, y `false` para una que llega después de que la Promise ya se resolvió o ya recibió la orden de parar.

La cancelación es cooperativa: surte efecto en un punto de suspensión, nunca en mitad de una sentencia. Una tarea en mitad de un cálculo largo no alcanza ningún punto de suspensión y por eso no se interrumpe; [`isCancelled`](./is-cancelled.md) es lo que un bucle así usa para pararse solo.

## Qué se ejecuta, y qué no

Una tarea cancelada ejecuta sus bloques `finally` y luego se va. Todo lo que retiene a través de un `await` se libera, y las sentencias posteriores al bloque no se ejecutan.

Un `catch` no ve la cancelación. Un `catch` se escribe para los fallos que un programa espera de su propio trabajo, y tratar "te dijeron que pararas" como uno de ellos significa que un manejador que registra y continúa, o que reintenta, empieza trabajo nuevo en una tarea que ya está terminando.

Ser cancelado y esperar algo cancelado son cosas distintas, y solo lo primero salta el `catch`. Una tarea que espera una Promise que otra persona canceló no ha recibido ninguna orden, y su `catch` es el único sitio donde puede enterarse de lo ocurrido.

<!-- @include: ../../../.fragments/modules/task/cancel.md -->

## Cuándo se resuelve la Promise

Cuando el trabajo termina de desenrollarse, no cuando se hace la petición. Quien para un worker y acto seguido reutiliza su puerto, su fichero o su conexión estaría compitiendo con la liberación, lo que aparece como un fallo raro en vez de como una respuesta incorrecta.

La razón que pasó quien llamó es la que lleva la Promise, sea lo que sea que la tarea alcanzara al salir: un valor que logró devolver es producto de trabajo al que ya se le había dicho que parara.

La cancelación alcanza el trabajo que el runtime hace en nombre de la tarea. Una tarea que espera una petición ve esa petición detenida, y la conexión se cierra.

## El plazo

`graceMillis` limita cuánto puede tardar la limpieza. Pasado ese tiempo el trabajo se corta, y la Promise lleva `cancellation cleanup exceeded its deadline` en lugar de la razón original, porque de otro modo quien llamó no podría distinguir una parada limpia de una que abandonó el trabajo a medio liberar.

Omitirlo significa que la limpieza nunca se corta. Una limpieza que sigue avanzando no es algo que interrumpir por suposición, así que quien tiene prisa es quien dice cuánto va a esperar.
