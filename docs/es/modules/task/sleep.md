---
title: sleep
description: Bloquea el hilo actual a través de tyrus:task.
---

# sleep

`sleep` bloquea el hilo actual durante la cantidad solicitada de milisegundos.

<SignatureBlock label="Firma">sleep(ms: u64): void</SignatureBlock>

Use `sleep` en programas de línea de comandos, scripts simples y pruebas donde bloquear el hilo actual sea intencional. En un programa asíncrono con un único hilo de trabajo, bloquea todo el runtime.

## Qué continúa durante la espera

El código debajo de `await sleepAsync(...)` en la misma función espera a que termine el retraso. Otras tasks listas pueden usar el hilo mientras la función actual está suspendida. En el primer programa, `sleep` bloquea el hilo, por lo que imprime `first` y después `second`. En el segundo programa, `sleepAsync` suspende la primera task, por lo que imprime `second` y después `first`.

<!-- @include: ../../../.fragments/modules/task/sleep-ordering.md -->

::: danger error
<!-- @include: ../../../.fragments/modules/task/sleep-error.md -->
:::
