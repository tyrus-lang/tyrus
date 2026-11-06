---
title: trash
description: Mueve una ruta a la papelera de la plataforma mediante el módulo nativo tyrus:os.
---

# trash

`trash` mueve `path` a la papelera de la plataforma, recuperable, a diferencia de eliminarlo directamente.

<SignatureBlock label="Firma">trash(path: string): Result&lt;void, OsError&gt;</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-trash.md -->
Devuelve `Err(OsError)` (`code: "ERR_OS_TRASH_FAILED"`) si `path` no existe o no se puede mover a la papelera.

### Campos de OsError

| Campo | Tipo | Significado |
|-------|------|-------------|
| `code` | `string` | Un identificador estable y legible por máquina (ej.: `"ERR_OS_TRASH_FAILED"`). |
| `message` | `string` | Una descripción legible de lo que salió mal. |
