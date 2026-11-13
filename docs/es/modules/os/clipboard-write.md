---
title: clipboardWrite
description: Escribe texto en el portapapeles del sistema mediante el módulo nativo tyrus:os.
---

# clipboardWrite

`clipboardWrite` solicita escribir `text` en el portapapeles del sistema, reemplazando el contenido anterior, y devuelve un error estructurado si el portapapeles no puede aceptarlo.

<SignatureBlock label="Firma">clipboardWrite(text: string): Result&lt;void, OsError&gt;</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-clipboard-write.md -->
Devuelve `ERR_OS_INVALID_ARGUMENT` para una cadena nula, `ERR_OS_CLIPBOARD_UNAVAILABLE` si no se puede abrir el portapapeles del sistema o `ERR_OS_CLIPBOARD_WRITE_FAILED` si el portapapeles rechaza el texto. Maneja el `Result` devuelto antes de asumir que el portapapeles se actualizó.

### Campos de OsError

| Campo | Tipo | Significado |
|-------|------|-------------|
| `code` | `string` | Un identificador de error estable y legible por máquina. |
| `message` | `string` | Una descripción legible del error. |

::: warning X11: la escritura no es necesariamente duradera todavía cuando esta llamada retorna
En Linux bajo X11 (a diferencia de Wayland, macOS o Windows), el portapapeles no lo almacena centralmente el sistema operativo, el propio programa que escribió sigue sirviendo el contenido bajo demanda, delegando esa responsabilidad a un proceso en segundo plano de vida corta en cuanto termina. Leerlo de vuelta inmediatamente después, en el mismo programa, puede transitoriamente verse como ausente antes de que esa transición termine. Un pegado real (un humano presionando una tecla un instante después) nunca es lo bastante rápido para toparse con esto; solo una escritura-luego-lectura automatizada en el mismo instante sí.
:::
