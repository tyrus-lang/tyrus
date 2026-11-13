---
title: Visibilidad
description: Los tres niveles de visibilidad, sin export, export y export protect, además de cómo se comportan una reexportación y una importación por namespace.
---

# Visibilidad

La visibilidad responde a una pregunta: qué archivos pueden importar un nombre. tyrus tiene tres niveles, y una declaración elige uno de ellos por lo que escribe delante de sí misma.

| Forma | Quién puede importarlo |
|---|---|
| sin `export` | nadie, el nombre existe solo dentro de su propio archivo |
| `export` | todos los archivos del proyecto |
| `export protect` | la carpeta que lo declara y todas sus subcarpetas |

Los tres niveles forman una escala: `export protect` queda entre un nombre que nadie alcanza y un nombre que todos alcanzan, que es el nivel que un componente necesita para compartir un helper entre sus propios archivos sin publicarlo.

## Sin `export`: privado al archivo

Una declaración sin `export` es invisible fuera de su archivo. No existe palabra clave para eso, porque es el valor predeterminado.

<!-- @include: ../../.fragments/language/visibility/file-private.md -->

Importarlo es un error: para cualquier otro archivo, el nombre no existe.

::: danger error
<!-- @include: ../../.fragments/language/visibility/file-private-error.md -->
:::

## La visibilidad controla la importación, no la llamada

La visibilidad decide qué archivos pueden *nombrar* una declaración. No dice nada sobre qué declaraciones pueden *usarla*. Una función pública es libre de llamar a un helper privado de su propio archivo, e importar esa función funciona aunque el helper sea inalcanzable para el importador.

<!-- @include: ../../.fragments/language/visibility/call-through.md -->

`sign` es público, por lo que `src/http/handler.tyr` puede importarlo. `salt` es privado de `src/auth/token.tyr`, por lo que el mismo archivo no puede importarlo, y no lo necesita: la llamada a `salt` vive dentro de `token.tyr`, donde el nombre está en ámbito. Lo mismo vale un nivel más arriba, con `export protect salt` en lugar de un `salt` privado: un `sign` público todavía puede llamarlo, y un archivo fuera de la carpeta todavía puede importar `sign`.

Esto es lo que hace útiles los niveles. Un componente publica la superficie pequeña que quiere que vean quienes lo llaman y mantiene la maquinaria detrás en el nivel que corresponda, sin tener que ampliar la maquinaria para igualar la superficie.

## `export`: público en el proyecto

`export` publica un nombre para todos los archivos del proyecto, esté en la carpeta que sea.

<!-- @include: ../../.fragments/language/visibility/public.md -->

El alcance termina en el proyecto. El compilador rechaza una importación que resuelva fuera de la raíz del proyecto, así que `export` significa "público dentro de este proyecto", nunca "público para el mundo". Los paquetes externos todavía no se admiten.

## `export protect`: la carpeta que declara y hacia abajo

`export protect` restringe el alcance a la carpeta que declara el nombre, más todas sus subcarpetas.

<!-- @include: ../../.fragments/language/visibility/protect.md -->

Los dos importadores anteriores están dentro de `src/auth/`, por lo que ambos resuelven. Un archivo en una carpeta hermana no lo está, y recibe `E3308` nombrando el símbolo, la carpeta a la que está limitado y las dos salidas.

::: danger error
<!-- @include: ../../.fragments/language/visibility/protect-error.md -->
:::

El ámbito depende solo del archivo que declara, nunca de la profundidad de `src/`, por lo que mover una carpeta completa preserva todas las relaciones dentro de ella.

### Una carpeta anidada es un límite más estrecho

La relación corre solo hacia abajo. Un nombre protegido en una subcarpeta no es visible en la carpeta superior, porque el importador debe ser descendiente de la carpeta que declara.

<!-- @include: ../../.fragments/language/visibility/protect-nested.md -->

::: danger error
<!-- @include: ../../.fragments/language/visibility/protect-nested-error.md -->
:::

Cada carpeta es un límite real, no un club mutuo. Si un archivo anidado debe ser alcanzable desde todo el componente, decláralo con un `export` simple y acepta el alcance mayor.

### En una cláusula separada

El modificador también va en una cláusula de exportación, que es como un archivo publica algunos nombres de forma restringida y otros de forma abierta.

<!-- @include: ../../.fragments/language/visibility/protect-clause.md -->

## Una reexportación no puede ampliar el ámbito

Una reexportación reenvía un nombre sin declararlo, por lo que lleva el ámbito original sin modificarlo. Dentro de la carpeta, pasar por un archivo barril funciona como siempre.

<!-- @include: ../../.fragments/language/visibility/protect-reexport.md -->

Fuera de la carpeta no funciona, porque el barril reenvió el ámbito junto con el nombre. Ni `export *` ni `export { name } from` son una vía de escape.

::: danger error
<!-- @include: ../../.fragments/language/visibility/protect-reexport-error.md -->
:::

## Una importación por namespace omite los nombres protegidos

Una importación por namespace vincula todas las exportaciones a la vez. Los nombres fuera de ámbito simplemente no están en el namespace, en lugar de vincularse y luego rechazarse.

<!-- @include: ../../.fragments/language/visibility/protect-namespace.md -->

Buscar `token.sign` desde `src/main.tyr` falla igual que falla cualquier nombre que el módulo no exporta.

## Qué rechaza `protect`

`protect` describe dónde es visible una declaración, por lo que necesita una declaración a la que unirse. Dos formas no tienen una.

| Forma | Código | Por qué |
|---|---|---|
| `export protect default name;` | `E3309` | una exportación default publica un espacio anónimo, así que el ámbito de carpeta no tiene a qué unirse |
| `export protect { name } from "./other";` | `E3310` | una reexportación no declara el nombre, reenvía uno, y reenviar preserva el ámbito original |

Escribir el modificador primero, como `protect export`, es un error de sintaxis común. El orden es fijo en `export protect`, por lo que existe una sola escritura.

## Pruebas y la raíz del código

No hay excepción para los archivos de prueba. Una prueba que necesita un nombre protegido pertenece dentro de la carpeta que lo posee, junto al código que prueba.

Una declaración `protect` en la raíz del código no restringe nada, pues su carpeta ya cubre todos los archivos que el compilador aceptaría. Eso es válido pero inútil, y la regla de lint `export-protect-no-effect` avisa sobre ello.
