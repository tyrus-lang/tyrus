---
title: documentsDir
description: Lee el directorio de documentos del usuario mediante el módulo nativo tyrus:os.
---

# documentsDir

`documentsDir` devuelve el directorio de documentos del usuario.

<SignatureBlock label="Firma">documentsDir(): string?</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-documents-dir.md -->
::: tip La ausencia es un resultado válido
Un sistema headless (servidor, contenedor, CI) comúnmente no tiene ningún directorio de documentos, eso es un resultado normal, no un fallo.
:::
