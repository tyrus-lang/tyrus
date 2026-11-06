---
title: loadAvg
description: Leia as médias de carga do sistema através do módulo nativo tyrus:os.
---

# loadAvg

`loadAvg` devolve as médias de carga de 1/5/15 minutos como `[um, cinco, quinze]`.

<SignatureBlock label="Assinatura">loadAvg(): f64[]</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-load-avg.md -->
::: tip Sempre exatamente 3 valores
No Windows, que não tem o conceito de média de carga, os três são `0.0`.
:::
