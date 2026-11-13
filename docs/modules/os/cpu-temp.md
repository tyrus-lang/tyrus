---
title: cpuTemp
description: Read the CPU temperature through the native tyrus:os module.
---

# cpuTemp

`cpuTemp` returns the highest reported sensor temperature, an approximation of the CPU package temperature, since sensors are not labeled consistently as "CPU" across platforms.

<SignatureBlock label="Signature">cpuTemp(unit?: "celsius" | "fahrenheit"): f32?</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-cpu-temp.md -->
`unit` defaults to `"celsius"` when omitted. Passing any value other than `"celsius"` or `"fahrenheit"` is a compile-time type error.

::: tip Absence is a valid outcome
Returns absent, not an aborted program, when no thermal sensor is available, common in VMs, containers, and on platforms without a sensor backend.
:::

::: warning A value from outside must be narrowed first
`unit` only accepts the literals `"celsius"` and `"fahrenheit"`, a plain `string` read from a config file, an environment variable, or an HTTP response does not satisfy that type and fails to compile if passed directly. Branch on the external value and call `cpuTemp` with a literal in each branch instead:
:::

<!-- @include: ../../.fragments/modules/os/external-unit-cpu-temp.md -->

Anything that is not `"fahrenheit"` falls into the `else` branch above and is treated as Celsius; reject it explicitly there instead if silently defaulting is not what you want.
