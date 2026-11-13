---
title: loadAvg
description: Read the system load averages through the native tyrus:os module.
---

# loadAvg

`loadAvg` returns the 1/5/15-minute load averages as `[one, five, fifteen]`.

<SignatureBlock label="Signature">loadAvg(): f64[]</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-load-avg.md -->
::: tip Always exactly 3 values
On Windows, which has no concept of a load average, all three are `0.0`.
:::
