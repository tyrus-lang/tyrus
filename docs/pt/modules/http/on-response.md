---
title: HttpRouter.onResponse
description: Observe respostas concluídas de tyrus:http na configuração do servidor.
---

# HttpRouter.onResponse

`app.onResponse(hook)` registra um callback síncrono nomeado para cada resposta servida por esse roteador. Registre-o na configuração do servidor antes de `app.listen(...)`, e não dentro de uma rota.

## Assinatura

<!-- @include: ../../../.fragments/modules/http/on-response/signature.md -->

O callback roda uma vez quando o servidor termina de produzir ou descarta o corpo da resposta. Isso inclui uma resposta `StreamBody`. Ele observa o status final e a duração, mas não pode alterar a resposta e não é um `preHandler` de rota.

<!-- @include: ../../../.fragments/modules/http/on-response/signature-2.md -->

É permitido registrar mais de um hook. Os hooks rodam na ordem de registro. O callback pode realizar qualquer trabalho síncrono permitido pela aplicação, como atualizar uma métrica, gravar um registro de auditoria ou encaminhar dados para um adaptador de tracing.

::: danger error
<!-- @include: ../../../.fragments/modules/http/on-response/signature-3.md -->
:::

O hook precisa ser uma função nomeada. Um callback nomeado oferece ao servidor um ponto de entrada nativo estável e impede que uma closure sobreviva ao estado de rota que ela capturou.
