---
title: clipboardRead
description: Leia o texto atual da área de transferência do sistema através do módulo nativo tyrus:os.
---

# clipboardRead

`clipboardRead` lê o texto atual da área de transferência do sistema e retorna um erro estruturado se o serviço estiver indisponível ou não puder ser lido.

<SignatureBlock label="Assinatura">clipboardRead(): Result&lt;string?, OsError&gt;</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-clipboard-read.md -->
Retorna `Ok(ausente)` se a área de transferência estiver vazia ou contiver conteúdo que não seja texto (uma imagem ou uma lista de arquivos). Retorna `Err` com `ERR_OS_CLIPBOARD_UNAVAILABLE` se nenhum serviço de área de transferência estiver acessível, como em uma máquina headless, ou `ERR_OS_CLIPBOARD_READ_FAILED` se o texto não puder ser lido. Veja [`clipboardWrite`](./clipboard-write.md) para uma ressalva relacionada de timing específica do X11 ao ler de volta um valor que este mesmo programa acabou de escrever.
