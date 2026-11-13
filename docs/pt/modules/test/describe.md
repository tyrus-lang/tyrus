---
title: describe
description: Agrupa testes em uma suíte nomeada.
---

# describe

<SignatureBlock label="Assinatura">describe(name: string, callback: () =&gt; void): void</SignatureBlock>

`describe(name, callback)` abre uma suíte e executa `callback` imediatamente, então os testes e hooks registrados dentro dele pertencem a essa suíte. Nenhum teste executa durante o registro: os testes executam depois que o programa registrou tudo, na ordem em que foram escritos, e uma suíte aninhada executa onde aparece.

O nome completo de um teste é o seu caminho pelas suítes, unido por ` > `, como em `discount > from 100 on > is ten percent`.

Quando uma asserção falha durante o registro do callback de uma suíte, a suíte é reportada como falha e nenhum dos seus testes executa.

<!-- @include: ../../../.fragments/modules/test/describe.md -->
