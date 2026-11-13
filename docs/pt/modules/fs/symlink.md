---
title: symlink
description: Cria um link simbólico.
---

# symlink

Cria um link simbólico em `link` que aponta para `original`. No Unix, `original` pode ser um arquivo ou diretório. No Windows, esta operação cria links simbólicos para arquivos; use [`symlinkDir`](./symlink-dir.md) para diretórios. A criação de um link simbólico no Windows pode exigir o Modo de Desenvolvedor, o privilégio `SeCreateSymbolicLinkPrivilege` ou um processo elevado; consulte a [documentação da Microsoft para `CreateSymbolicLinkW`](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-createsymboliclinkw). As falhas são retornadas como `Err(FsError)`.

<SignatureBlock label="Assinatura">symlink(original: string, link: string): Result&lt;void, FsError&gt;</SignatureBlock>

## Exemplo

<!-- @include: ../../../.fragments/modules/fs/symlink/example.md -->
