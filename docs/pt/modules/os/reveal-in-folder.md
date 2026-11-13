---
title: revealInFolder
description: Abra o gerenciador de arquivos da plataforma no diretório de um caminho através do módulo nativo tyrus:os.
---

# revealInFolder

`revealInFolder` abre o gerenciador de arquivos da plataforma no diretório que contém `path` (ou no próprio `path`, se ele já for um diretório).

<SignatureBlock label="Assinatura">revealInFolder(path: string): Result&lt;void, OsError&gt;</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-reveal-in-folder.md -->
Retorna `Err(OsError)` com o código `ERR_OS_INVALID_ARGUMENT` quando `path` é nulo no limite nativo, ou `ERR_OS_REVEAL_FAILED` quando nenhum gerenciador de arquivos consegue abrir o diretório que contém o caminho. A mensagem descreve a falha.

::: tip Melhor esforço, não uma seleção garantida
Diferente de uma integração nativa com Explorer/Finder, isto não garante que `path` termine pré-selecionado, nenhuma crate cross-platform expõe isso, e o mecanismo de "selecionar no gerenciador de arquivos" de cada desktop é uma flag de linha de comando diferente e não documentada. Uma máquina headless ou sem gerenciador de arquivos retorna `ERR_OS_REVEAL_FAILED`.
:::
