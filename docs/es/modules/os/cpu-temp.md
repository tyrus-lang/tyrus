---
title: cpuTemp
description: Lee la temperatura de la CPU mediante el módulo nativo tyrus:os.
---

# cpuTemp

`cpuTemp` devuelve la mayor temperatura de sensor reportada, una aproximación de la temperatura del paquete de la CPU, ya que los sensores no se etiquetan de forma consistente como "CPU" entre plataformas.

<SignatureBlock label="Firma">cpuTemp(unit?: "celsius" | "fahrenheit"): f32?</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-cpu-temp.md -->
`unit` es `"celsius"` por defecto cuando se omite. Pasar cualquier valor distinto de `"celsius"` o `"fahrenheit"` es un error de tipo en tiempo de compilación.

::: tip La ausencia es un resultado válido
Devuelve ausencia, no un programa abortado, cuando no hay ningún sensor térmico disponible, común en VMs, contenedores y plataformas sin backend de sensores.
:::

::: warning Un valor externo debe tratarse antes
`unit` solo acepta los literales `"celsius"` y `"fahrenheit"`, un `string` genérico leído de un archivo de configuración, una variable de entorno o una respuesta HTTP no satisface ese tipo y falla al compilar si se pasa directamente. Ramifica sobre el valor externo y llama a `cpuTemp` con un literal en cada rama:
:::

<!-- @include: ../../../.fragments/modules/os/external-unit-cpu-temp.md -->

Cualquier valor distinto de `"fahrenheit"` cae en la rama `else` de arriba y se trata como Celsius; recházalo explícitamente ahí si el valor por defecto silencioso no es lo que quieres.
