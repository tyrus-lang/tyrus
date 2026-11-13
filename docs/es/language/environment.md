---
title: Variables de Entorno
description: Archivos .env tipados, validación, diagnósticos, artefactos sealed y acceso mediante process.env.
---

# Variables de Entorno

Cuando `tyrus.json` define `security.envFile`, el compilador lee ese archivo `.env` y expone variables mediante `process.env` con tipos estáticos. El binario no embebe secrets. El compilador y el Language Server comparten el mismo análisis.

## Configuración

<!-- @include: ../../.fragments/docs/config.md -->

## .env Tipado

<!-- @include: ../../.fragments/docs/typed-env.md -->

Validator Decorations en archivos `.env` pueden escribirse sin `#` o con `#` al inicio. Tyrus interpreta ambas formas. Mantén el `#` cuando otro loader de `.env`, como Docker, también lea el archivo; ese loader verá un comentario y Tyrus seguirá aplicando los validators.

```dotenv
# @type(u16)
# @port()
PORT=8080
```

El parser separa en el primer `=`, exige valor, elimina comillas opcionales, soporta interpolación `${VAR}`, ignora comentarios y líneas en blanco, acepta `export KEY=value` y trata `#` inline como parte del valor. Toda variable debe tener `@type`, raw o comentado como `# @type(...)` para compatibilidad con otros loaders de `.env`.

## Acceso

<!-- @include: ../../.fragments/docs/access.md -->

Member access y destructuring se tipan desde la declaración del `.env`. El acceso indexado como `process.env["PORT"]` se rechaza para que el compilador conserve garantías estáticas.

## Diagnósticos

Diagnósticos comunes incluyen `E3147`, `E3148`, `E3149`, `E3150`, `E3151`, `E3152`, `E3144`, `E3146`, `E3142`, `E3143`, `E3141`, `E3140`, `E3145` y `E3083`.

Un diagnóstico de `.env` nunca imprime el valor que rechaza. Nombra la variable y la regla que falló, y su code frame muestra el valor como `<redacted>`, y no una máscara del mismo ancho, que aún publicaría cuántos caracteres tiene el secreto. Un build que falla escribe sus diagnósticos directamente en un log de CI, y el code frame imprime la línea a la que apunta, así que un mensaje cuidadoso por sí solo no bastaría.

## Resolución en Runtime

Los valores en runtime se resuelven en este orden: el entorno real del proceso gana, y el archivo `.env` es fallback. La inicialización valida presencia, `@type` y Validator Decorations compatibles antes de ejecutar código de usuario. Los fallos imprimen en stderr y terminan con status distinto de cero.

## Variables Reservadas

`TYRUS_WORKER_THREADS` la lee el propio runtime, no tu programa. Define cuántos hilos de trabajo puede usar el runtime asíncrono y sobrescribe `runtime.workerThreads` de `tyrus.json`, un deploy puede encoger un programa para la máquina en la que aterrizó sin recompilarlo. Cuando ninguna de las dos está definida, el runtime usa un worker por CPU lógica.

Un valor que no sea un entero positivo se ignora con una advertencia en stderr en lugar de ser fatal, y una cuenta muy por encima del número detectado de CPUs lógicas se respeta, pero también se reporta, ya que sobrecargar la máquina es ocasionalmente deliberado.

`TYRUS_REPORT_PENDING_TASKS=1` le pide al runtime que reporte, en stderr, cuántas tareas asíncronas seguían pendientes cuando `main` terminó. Viene apagada porque un programa correcto puede terminar con tareas pendientes a propósito; actívala cuando una tarea parezca no haber corrido nada.

## Env Sealed

Define `security.envSealed` como `true` para cifrar la fuente del env en `target/<profile>/.env.sealed` con `age`. El binario generado pasa a exigir `.env.sealed` junto al ejecutable y rechaza fingerprints o claves incompatibles. Usa `tyrus build --rotate-seal-key` para generar un nuevo par de claves y reseal.

## .env.example

Tyrus mantiene automáticamente un espejo seguro `<envFile>.example` junto al archivo de env. Cada vez que agregas, eliminas o renombras una variable en `.env`, Tyrus refleja el cambio en `.env.example`, preservando orden y decorators mientras limpia valores después de `=`. Este archivo está pensado para commit para que el equipo sepa qué variables son necesarias sin exponer sus valores.
