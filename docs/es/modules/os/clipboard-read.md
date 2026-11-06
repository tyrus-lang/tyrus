---
title: clipboardRead
description: Lee el texto actual del portapapeles del sistema mediante el módulo nativo tyrus:os.
---

# clipboardRead

`clipboardRead` lee el texto actual del portapapeles del sistema y devuelve un error estructurado si el servicio no está disponible o no se puede leer.

<SignatureBlock label="Firma">clipboardRead(): Result&lt;string?, OsError&gt;</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-clipboard-read.md -->
Devuelve `Ok(ausente)` si el portapapeles está vacío o contiene contenido que no es texto (una imagen o una lista de archivos). Devuelve `Err` con `ERR_OS_CLIPBOARD_UNAVAILABLE` si no se puede acceder a ningún servicio de portapapeles, como en una máquina headless, o `ERR_OS_CLIPBOARD_READ_FAILED` si no se puede leer el texto. Consulta [`clipboardWrite`](./clipboard-write.md) para una advertencia relacionada de tiempo específica de X11 al leer de vuelta un valor que este mismo programa acaba de escribir.
