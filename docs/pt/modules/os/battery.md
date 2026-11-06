---
title: battery
description: Leia informações de bateria através do módulo nativo tyrus:os.
---

# battery

`battery` devolve as informações da primeira bateria como um `BatteryInfo?` nativo.

<SignatureBlock label="Assinatura">battery(): BatteryInfo?</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-battery.md -->
### Campos de BatteryInfo

| Campo | Tipo | Significado |
|-------|------|-------------|
| `level` | `f32` | Estado de carga, de `0.0` a `1.0`. |
| `charging` | `boolean` | Se a bateria está carregando no momento. |
| `timeToFull` | `u32?` | Segundos até completar a carga, ausente a menos que esteja carregando. |
| `timeToEmpty` | `u32?` | Segundos até esgotar, ausente a menos que esteja descarregando. |

::: tip Ausência é um resultado válido
Desktops e a maioria dos servidores não têm bateria nenhuma, isso é um resultado normal, não uma falha.
:::
