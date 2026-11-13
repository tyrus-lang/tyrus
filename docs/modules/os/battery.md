---
title: battery
description: Read battery information through the native tyrus:os module.
---

# battery

`battery` returns the first battery's information as a native `BatteryInfo?`.

<SignatureBlock label="Signature">battery(): BatteryInfo?</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-battery.md -->
### BatteryInfo Fields

| Field | Type | Meaning |
|-------|------|---------|
| `level` | `f32` | State of charge, from `0.0` to `1.0`. |
| `charging` | `boolean` | Whether the battery is currently charging. |
| `timeToFull` | `u32?` | Seconds until full, absent unless currently charging. |
| `timeToEmpty` | `u32?` | Seconds until empty, absent unless currently discharging. |

::: tip Absence is a valid outcome
Desktops and most servers have no battery at all, that is a normal result, not a failure.
:::
