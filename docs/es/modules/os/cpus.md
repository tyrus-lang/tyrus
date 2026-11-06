---
title: cpus
description: Lista los núcleos lógicos de CPU mediante el módulo nativo tyrus:os.
---

# cpus

`cpus` devuelve un `CpuInfo` por cada núcleo lógico de CPU.

<SignatureBlock label="Firma">cpus(): CpuInfo[]</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-cpus.md -->
### Campos de CpuInfo

| Campo | Tipo | Significado |
|-------|------|-------------|
| `model` | `string` | El nombre del modelo de la CPU (ej.: `"Intel(R) Core(TM) i5-10400F CPU @ 2.90GHz"`). |
| `speed` | `u32` | La frecuencia del núcleo, en MHz. |
