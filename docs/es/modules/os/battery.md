---
title: battery
description: Lee información de batería mediante el módulo nativo tyrus:os.
---

# battery

`battery` devuelve la información de la primera batería como un `BatteryInfo?` nativo.

<SignatureBlock label="Firma">battery(): BatteryInfo?</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-battery.md -->
### Campos de BatteryInfo

| Campo | Tipo | Significado |
|-------|------|-------------|
| `level` | `f32` | Estado de carga, de `0.0` a `1.0`. |
| `charging` | `boolean` | Si la batería está cargando en este momento. |
| `timeToFull` | `u32?` | Segundos hasta completar la carga, ausente a menos que esté cargando. |
| `timeToEmpty` | `u32?` | Segundos hasta agotarse, ausente a menos que esté descargando. |

::: tip La ausencia es un resultado válido
Los escritorios y la mayoría de los servidores no tienen batería alguna, eso es un resultado normal, no un fallo.
:::
