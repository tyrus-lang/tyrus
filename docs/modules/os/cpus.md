---
title: cpus
description: List logical CPU cores through the native tyrus:os module.
---

# cpus

`cpus` returns one `CpuInfo` per logical CPU core.

<SignatureBlock label="Signature">cpus(): CpuInfo[]</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-cpus.md -->
### CpuInfo Fields

| Field | Type | Meaning |
|-------|------|---------|
| `model` | `string` | The CPU model name (e.g. `"Intel(R) Core(TM) i5-10400F CPU @ 2.90GHz"`). |
| `speed` | `u32` | The core's frequency, in MHz. |
