---
title: revealInFolder
description: Abre el explorador de archivos de la plataforma en el directorio de una ruta mediante el módulo nativo tyrus:os.
---

# revealInFolder

`revealInFolder` abre el explorador de archivos de la plataforma en el directorio que contiene `path` (o en el propio `path`, si ya es un directorio).

<SignatureBlock label="Firma">revealInFolder(path: string): Result&lt;void, OsError&gt;</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-reveal-in-folder.md -->
Devuelve `Err(OsError)` con el código `ERR_OS_INVALID_ARGUMENT` cuando `path` es nulo en el límite nativo, o `ERR_OS_REVEAL_FAILED` cuando ningún explorador de archivos puede abrir el directorio que contiene la ruta. El mensaje describe el fallo.

::: tip Mejor esfuerzo, no una selección garantizada
A diferencia de una integración nativa con Explorer/Finder, esto no garantiza que `path` termine preseleccionado, ningún crate cross-platform expone eso, y el mecanismo de "seleccionar en el explorador de archivos" de cada escritorio es una opción de línea de comandos diferente y no documentada. Una máquina headless o sin explorador de archivos devuelve `ERR_OS_REVEAL_FAILED`.
:::
