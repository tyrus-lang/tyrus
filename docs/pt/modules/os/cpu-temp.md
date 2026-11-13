---
title: cpuTemp
description: Leia a temperatura da CPU através do módulo nativo tyrus:os.
---

# cpuTemp

`cpuTemp` devolve a maior temperatura de sensor reportada, uma aproximação da temperatura do pacote da CPU, já que os sensores não são rotulados de forma consistente como "CPU" entre plataformas.

<SignatureBlock label="Assinatura">cpuTemp(unit?: "celsius" | "fahrenheit"): f32?</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-cpu-temp.md -->
`unit` assume `"celsius"` por padrão quando omitido. Passar qualquer valor diferente de `"celsius"` ou `"fahrenheit"` é um erro de tipo em tempo de compilação.

::: tip Ausência é um resultado válido
Devolve ausência, não um programa abortado, quando nenhum sensor térmico está disponível, comum em VMs, containers e plataformas sem backend de sensores.
:::

::: warning Um valor vindo de fora precisa ser tratado antes
`unit` só aceita os literais `"celsius"` e `"fahrenheit"`, um `string` genérico lido de um arquivo de configuração, uma variável de ambiente ou uma resposta HTTP não satisfaz esse tipo e falha ao compilar se passado direto. Ramifique sobre o valor externo e chame `cpuTemp` com um literal em cada ramo:
:::

<!-- @include: ../../../.fragments/modules/os/external-unit-cpu-temp.md -->

Qualquer valor diferente de `"fahrenheit"` cai no ramo `else` acima e é tratado como Celsius; rejeite-o explicitamente ali caso o padrão silencioso não seja o que você quer.
