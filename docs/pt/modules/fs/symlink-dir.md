---
title: symlinkDir
description: Cria um link simbólico para um diretório.
---

# symlinkDir

Cria um link simbólico em `link` que aponta para o diretório em `original`. Esta operação identifica o link como um link de diretório mesmo quando o destino ainda não existe. No Windows, criar um link simbólico pode exigir o Modo de Desenvolvedor, o privilégio `SeCreateSymbolicLinkPrivilege` ou um processo elevado; consulte a [documentação da Microsoft para `CreateSymbolicLinkW`](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-createsymboliclinkw). As falhas são retornadas como `Err(FsError)`.

<SignatureBlock label="Assinatura">symlinkDir(original: string, link: string): Result&lt;void, FsError&gt;</SignatureBlock>

## Exemplo

<!-- @include: ../../../.fragments/modules/fs/symlink-dir/example.md -->
