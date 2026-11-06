---
title: uptime
description: Read the system's uptime through the native tyrus:os module.
---

# uptime

`uptime` returns how many seconds have elapsed since the system itself last booted.

<SignatureBlock label="Signature">uptime(): u64</SignatureBlock>

<!-- @include: ../../.fragments/modules/os/usage-uptime.md -->

::: tip Not the current process's own uptime
This is the system's uptime, not the running program's. For how long the current process has been running, use [`process.uptime()`](../../language/environment.md#process-information).
:::
