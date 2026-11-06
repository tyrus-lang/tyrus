---
title: trash
description: Mova um caminho para a lixeira da plataforma através do módulo nativo tyrus:os.
---

# trash

`trash` move `path` para a lixeira da plataforma, recuperável, diferente de excluir diretamente.

<SignatureBlock label="Assinatura">trash(path: string): Result&lt;void, OsError&gt;</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-trash.md -->
Retorna `Err(OsError)` (`code: "ERR_OS_TRASH_FAILED"`) se `path` não existe ou não pode ser movido para a lixeira.

### Campos de OsError

| Campo | Tipo | Significado |
|-------|------|-------------|
| `code` | `string` | Um identificador estável e legível por máquina (ex.: `"ERR_OS_TRASH_FAILED"`). |
| `message` | `string` | Uma descrição legível do que deu errado. |
