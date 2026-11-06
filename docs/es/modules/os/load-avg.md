---
title: loadAvg
description: Lee los promedios de carga del sistema mediante el módulo nativo tyrus:os.
---

# loadAvg

`loadAvg` devuelve los promedios de carga de 1/5/15 minutos como `[uno, cinco, quince]`.

<SignatureBlock label="Firma">loadAvg(): f64[]</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-load-avg.md -->
::: tip Siempre exactamente 3 valores
En Windows, que no tiene el concepto de promedio de carga, los tres son `0.0`.
:::
