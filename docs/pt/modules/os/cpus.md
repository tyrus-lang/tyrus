---
title: cpus
description: Liste os núcleos lógicos de CPU através do módulo nativo tyrus:os.
---

# cpus

`cpus` devolve um `CpuInfo` para cada núcleo lógico de CPU.

<SignatureBlock label="Assinatura">cpus(): CpuInfo[]</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-cpus.md -->
### Campos de CpuInfo

| Campo | Tipo | Significado |
|-------|------|-------------|
| `model` | `string` | O nome do modelo da CPU (ex.: `"Intel(R) Core(TM) i5-10400F CPU @ 2.90GHz"`). |
| `speed` | `u32` | A frequência do núcleo, em MHz. |
