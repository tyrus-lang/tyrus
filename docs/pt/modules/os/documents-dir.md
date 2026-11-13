---
title: documentsDir
description: Leia o diretório de documentos do usuário através do módulo nativo tyrus:os.
---

# documentsDir

`documentsDir` devolve o diretório de documentos do usuário.

<SignatureBlock label="Assinatura">documentsDir(): string?</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-documents-dir.md -->
::: tip Ausência é um resultado válido
Um sistema headless (servidor, container, CI) comumente não tem diretório de documentos nenhum, isso é um resultado normal, não uma falha.
:::
