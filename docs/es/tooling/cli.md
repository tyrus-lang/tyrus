---
title: CLI
description: Interfaz de línea de comandos de tyrus para scaffold, check, build, run, formatter, linter e integración con editor.
---

# CLI

La CLI Tyrus compila, ejecuta, formatea, analiza y crea scaffolds de proyectos Tyrus.

## Crear un proyecto

<!-- @include: ../../.fragments/tooling/cli-create-project.md -->

Ejecuta `tyrus new` sin argumentos. La CLI pregunta el nombre del proyecto y después el target.

<!-- @include: ../../.fragments/tooling/cli-target-flags.md -->

Para el target API, la CLI pregunta si se debe configurar ninguna base de datos, SQLite, PostgreSQL, MySQL o MongoDB. Al elegir SQLite, escribe las opciones recomendadas en `database.sqlite` y crea el directorio `data/`. Los backends servidor usan `DATABASE_URL` en `connectionEnv`; el scaffold existente de `.env` y `.env.example` incluye esa variable.

`tyrus new` también escribe `__tests__/` con una prueba que pasa, así que `tyrus test` informa un resultado desde la primera ejecución.

::: tip Proyectos API
Al elegir el target API en las preguntas de `tyrus new`, la CLI crea una estructura orientada a API y defaults de lint para las capas route, controller, service y repository. La CLI también pregunta qué SGBD configurar; elegir ninguna base de datos deja esa configuración fuera de `tyrus.json`.
:::

## Comandos comunes

<!-- @include: ../../.fragments/tooling/cli-common-commands.md -->

## Comandos de proyecto

| Comando | Finalidad |
|---------|-----------|
| `tyrus new` | Crea un proyecto mediante preguntas sobre el nombre y el target. |
| `tyrus --init` | Agrega claves default ausentes en `tyrus.json` preservando valores existentes. |
| `tyrus --default` | Restablece `tyrus.json` con los valores predeterminados generados para el proyecto y elimina scripts y ajustes personalizados. |
| `tyrus info` | Imprime metadatos del proyecto. |
| `tyrus clean [--cache] [--dry-run] [--force]` | Elimina el directorio `target/` del proyecto y, de forma opcional, la caché global de Tyrus. |
| `tyrus check` | Hace parse, type-check y borrow-check sin generar binario nativo. |
| `tyrus build [--release] [--rotate-seal-key] [--pgo-generate\|--pgo-use]` | Compila el proyecto. |
| `tyrus pgo-merge` | Fusiona los perfiles que una ejecución instrumentada escribió en un archivo. |
| `tyrus run [<path>\|<script>] [--release] [--rotate-seal-key] [-- <args>...]` | Ejecuta por nombre un script de `tyrus.json`, o compila y ejecuta `src/main.tyr` o un archivo fuente independiente; los argumentos después de `--` se pasan al script o al programa. |
| `tyrus test [<path>]` | Ejecuta todos los archivos `*.test.tyr` e informa los resultados. |
| `tyrus watch [--run\|--test\|--command <cmd>] [--debounce <ms>] [--clear]` | Compila de nuevo cada vez que cambia un archivo `.tyr` o el `tyrus.json`; `--run` inicia el programa tras cada build exitoso, `--test` ejecuta las pruebas y `--command` ejecuta un comando de shell en su lugar. |
| `tyrus dev [--debounce <ms>] [--clear]` | Vuelve a compilar y reinicia el proyecto después de cada cambio de fuente usando el mismo watcher y la misma sustitución ordenada de proceso que `tyrus watch --run`. |
| `tyrus db introspect` | Lee la base SQLite, PostgreSQL o MySQL configurada en modo de solo lectura y escribe `tyrus-db-schema.json` de forma atómica. |
| `tyrus db check` | Compara el schema SQLite, PostgreSQL o MySQL actual con el snapshot sin modificar la base. |
| `tyrus db diff` | Muestra diferencias entre el snapshot del schema y la base SQL activa sin escribir en ningún schema; también compara los Models del proyecto con el snapshot en los tres backends SQL. |
| `tyrus db explain-sql [SQL]` | Valida una sentencia SQL con el snapshot local y muestra el SQL normalizado, los tipos de parámetros inferidos y los campos de resultado con los valores ocultos. Pasa el SQL entre comillas como argumento o mediante stdin. |
| `tyrus db migrate create <name>` | Crea el siguiente archivo SQL numerado con un comentario inicial específico para SQLite, PostgreSQL o MySQL. |
| `tyrus db migrate generate <name>` | Genera migrations SQLite a partir de declaraciones `@model` o la migration inicial PostgreSQL/MySQL a partir de campos Model admitidos. Revisa el SQL antes de aplicarla. |
| `tyrus db migrate status` | Enumera las migrations aplicadas y pendientes en SQLite, PostgreSQL o MySQL sin modificar la base. |
| `tyrus db migrate apply` | Aplica migrations pendientes en una transacción SQLite/PostgreSQL o con un marcador de recuperación en MySQL, y registra sus checksums. |
| `tyrus db seed status [name]` | Enumera los programas de seed Tyrus aplicados y pendientes sin modificar la base. |
| `tyrus db apply <name.tyr>` | Ejecuta un programa de seed de `db/seeds/` y registra su checksum. |

`tyrus clean` elimina únicamente el directorio `target/` del proyecto. El comando pide confirmación en una terminal interactiva; usa `--force` para omitir la pregunta, y las ejecuciones sin terminal requieren esa opción antes de borrar archivos. Añade `--dry-run` para estimar el espacio que se liberaría sin cambiar archivos. Añade `--cache` para incluir `~/.tyrus/cache/`; Tyrus avisa que esta caché se comparte entre proyectos y pide confirmación por separado.

Pasa una ruta de fuente a `tyrus run` para ejecutar un archivo separado sin cambiar `src/main.tyr`. Esto es útil para experimentos rápidos y ejemplos que contienen statements de nivel superior:

<!-- @include: ../../.fragments/tooling/cli/project-commands.md -->

Sin una ruta, `tyrus run` sigue compilando y ejecutando `src/main.tyr`.

Los proyectos generados incluyen las entradas `fmt`, `lint` y `test` bajo `scripts` en `tyrus.json`. Declara scripts adicionales del proyecto como valores string y ejecuta uno por su nombre con `tyrus run <name>`, por ejemplo `tyrus run example`; los argumentos para el comando configurado van después de `--`. `tyrus --init` conserva los scripts personalizados al agregar los valores predeterminados que faltan, y `tyrus --default` reemplaza el manifiesto por los valores predeterminados generados para el proyecto.

`tyrus watch` observa `src/` (y `tests/` con `--test`) y espera `--debounce` milisegundos, 300 por defecto, para que un guardado que toca varios archivos inicie un solo ciclo. Un build que falla imprime sus diagnósticos y sigue observando. Con `--run`, el programa del ciclo anterior se detiene antes del siguiente build, así que un servidor libera su puerto. `target/` y `.git` siempre se ignoran; lista otras rutas en un archivo `.tyrusignore` en la raíz del proyecto, una por línea, donde `src/generated` ignora ese directorio y un nombre suelto como `fixtures` ignora cualquier directorio con ese nombre. Las líneas de estado van a stderr, así que la salida del programa o del comando permanece en stdout.

`tyrus dev` es el atajo enfocado en desarrollo para `tyrus watch --run`: acepta solo `--debounce` y `--clear`, recompila después de cambios en `src/` o en `tyrus.json`, detiene el programa anterior antes de iniciar el reemplazo y mantiene el estado del watcher en stderr.

## Comandos del schema de la base

Configura en `tyrus.json` una ruta SQLite relativa al proyecto. La configuración no tiene campos de credenciales y las rutas de la base y del snapshot deben permanecer dentro del directorio del proyecto.

<!-- @include: ../../.fragments/tooling/cli/db-config.md -->

En SQLite, `database.sqlite.journalMode` acepta `wal` o `delete`, `synchronous` acepta `full` o `normal`, y `busyTimeoutMs` acepta valores de 1 a 120000. Los valores predeterminados son WAL, FULL y 5000 ms; `:memory:` siempre usa el journal de memoria de SQLite. `database.sqlite` se rechaza en los demás backends. WAL requiere compatibilidad con memoria compartida y bloqueos, y no funciona en sistemas de archivos de red; Tyrus confirma que SQLite aplicó el modo solicitado, pero no puede verificar los bloqueos del sistema de archivos ni las garantías de sincronización del almacenamiento. En modo WAL, FULL sincroniza cada confirmación de transacción, mientras que NORMAL sincroniza con menos frecuencia y puede perder confirmaciones recientes tras un corte de energía, aunque conserva la coherencia de la base de datos. SQLite realiza checkpoints automáticamente durante el funcionamiento normal y cuando se cierra la última conexión. No copies solo el archivo principal de la base de datos mientras esté abierto; usa una operación de copia de seguridad coherente de SQLite. Consulta la [documentación WAL de SQLite](https://www.sqlite.org/wal.html) y la [documentación de la pragma synchronous](https://www.sqlite.org/pragma.html#pragma_synchronous) para conocer las garantías y compensaciones de almacenamiento.

Ejecuta `tyrus db introspect` cuando cambie el schema para crear o actualizar el snapshot versionado. El comando admite SQLite, PostgreSQL y MySQL y abre la base configurada en modo de solo lectura. Los builds y el Language Server permanecen offline y no se conectan a la base. Ejecuta `tyrus db check` en CI o antes de un build para detectar un snapshot ausente o desactualizado. `tyrus db diff` compara la base activa con el snapshot y compara los Models del proyecto con el snapshot para SQLite, PostgreSQL y MySQL.

Usa `tyrus db diff` para revisar las diferencias entre los Models del proyecto, el snapshot local y la base actual antes de cambiar cualquiera de ellos. Muestra cambios en las tablas y columnas de los Models, en la versión del schema SQLite, en los tipos lógico y nativo, la nulabilidad, la expresión default y el motor de la tabla, además de cambios en claves primarias, constraints unique, foreign keys e índices. La comparación con Models es de solo lectura; la generación de migraciones destructivas o in-place sigue limitada a SQLite. `tyrus db check` sigue siendo el comando de CI que falla cuando hay drift en la base.

Pasa una sentencia SQL entre comillas a `tyrus db explain-sql` o envíala mediante stdin para inspeccionar el SQL normalizado, los tipos de parámetros inferidos y los campos de resultado sin abrir una conexión a la base. Para `SELECT`, cada campo de resultado incluye su tipo y opcionalidad; las mutaciones muestran `void`. Los literales de cadena y numéricos se ocultan en el SQL mostrado, y los valores vinculados nunca se muestran.

<!-- @include: ../../.fragments/tooling/cli/db-explain.md -->

## Migrations SQLite

Crea una migration, edita su SQL, consulta la lista de pendientes y aplícala explícitamente. Usa `tyrus db migrate generate <name>` para crear instrucciones iniciales `CREATE TABLE` cuando la base configurada aún no existe. Para una base existente, puede crear tablas para Models nuevos, instrucciones aditivas seguras `ALTER TABLE ... ADD COLUMN` para campos opcionales o con valores predeterminados literales y reconstruir una tabla cuando un Model elimina columnas o cambia la clave primaria o constraints `@unique`. La reconstrucción copia las columnas conservadas y restaura las definiciones de índices personalizados desde el catálogo de schema de SQLite. Se rechaza si una columna eliminada se usa en un índice o si claves foráneas, vistas, triggers o adiciones simultáneas impiden conservar los datos de forma segura. La aplicación puede fallar si los registros existentes infringen una nueva constraint de clave; la transacción SQLite se revierte. La generación incremental requiere un snapshot actualizado del schema. Rechaza diferencias en el schema y cambios en definiciones de columnas existentes o constraints de claves foráneas; escribe esas migrations manualmente. Se rechazan los tipos sin un mapeo sin pérdida para SQLite. La generación crea un archivo para revisar y nunca modifica la base de datos. Actualiza el snapshot del schema después de aplicar una migration de schema.

<!-- @include: ../../.fragments/tooling/cli/db-migrate.md -->

Para PostgreSQL y MySQL, `tyrus db migrate generate <name>` escribe schemas iniciales sin conectarse al servidor. Después, se conecta al servidor configurado, verifica el schema activo frente al snapshot y genera nuevas tablas de Model o columnas aditivas seguras; las nuevas columnas únicas deben ser opcionales y no tener valor predeterminado, y PostgreSQL y MySQL pueden agregar columnas opcionales con clave foránea cuando la tabla de destino, la clave única y el tipo compatible consten en el snapshot; MySQL también exige InnoDB en ambas tablas y tipos enteros con signo coincidentes registrados por la introspección. Otros cambios de claves, los cambios en definiciones existentes y las operaciones destructivas requieren SQL escrito manualmente. Las columnas agregadas deben ser opcionales o tener un valor predeterminado literal, y los Models con valores predeterminados existentes o metadatos de claves foráneas se rechazan preventivamente para la generación incremental. El comando rechaza `@timestamps` en Models existentes y tipos de campo sin soporte. `status` y `apply` también admiten ambos backends de servidor; PostgreSQL usa una transacción y un lock advisory, mientras que MySQL registra un marcador de recuperación porque DDL puede confirmarse implícitamente.

Los archivos usan nombres como `0001_add_users.sql`. Cada archivo puede contener varias sentencias SQLite. La CLI registra un checksum SHA-256 en `_tyrus_migrations`, rechaza archivos aplicados modificados o ausentes y versiones pendientes anteriores a una versión aplicada, además de serializar aplicaciones concurrentes con un bloqueo de escritura de SQLite. Un fallo revierte todo el lote pendiente. `status` abre una base existente en modo de solo lectura; ni `status` ni un build aplican migrations. Los archivos deben ser archivos regulares UTF-8 de hasta 4 MiB. Las migrations administradas no aceptan comandos de transacción, `PRAGMA`, `ATTACH`, `DETACH` ni `VACUUM`; el control de transacción pertenece a la CLI. Las migrations requieren una ruta SQLite persistente y no tienen downgrade automático.

Crea manualmente programas de seed como archivos `.tyr` en `db/seeds/`. El historial de seeds es compatible con SQLite, PostgreSQL y MySQL; MongoDB todavía no ofrece historial de seeds administrado por la CLI. Una seed puede usar la API de Model o `db.query`; ejecuta un archivo con `tyrus db apply default_roles.tyr` después de aplicar las migrations. `tyrus db seed status [name]` enumera los estados de las seeds. La CLI registra un checksum SHA-256 en `_tyrus_seeds` y rechaza volver a ejecutar un archivo aplicado, aceptar cambios o archivos aplicados ausentes. El programa de seed se ejecuta en un proceso Tyrus separado, por lo que la CLI no puede envolver operaciones arbitrarias en una única transacción de la base; usa la API de transacciones cuando varios cambios deban ser atómicos. Un programa que falle o se interrumpa permanece como `started` y bloquea nuevos intentos. Inspecciona la base, corrige o revierte los cambios parciales y luego elimina la fila correspondiente de `_tyrus_seeds` antes de reintentarlo; crea un archivo de seed nuevo para cambios posteriores en los datos.

## Comandos de tooling

| Comando | Finalidad |
|---------|-----------|
| `tyrus fmt [--check] [--stdout] [--init] [--default] [<path>...]` | Formatea archivos fuente. |
| `tyrus lint [--fix] [--format json] [--max-warnings <n>] [--explain <rule>] [--init] [--default] [<path>...]` | Reporta diagnósticos de lint. |
| `tyrus lsp` | Inicia el Language Server por stdio. |
| `tyrus mcp install` | Descarga la documentación de Tyrus y configura automáticamente los clientes MCP detectados. |
| `tyrus mcp update` | Actualiza la caché de documentación MCP y las entradas administradas en los clientes. |
| `tyrus mcp status` | Muestra el estado de la caché MCP y de la configuración de los clientes. |
| `tyrus mcp uninstall` | Elimina las entradas Tyrus MCP sin cambios y la caché local de documentación. |
| `tyrus mcp serve` | Inicia el servidor MCP stdio; los clientes configurados ejecutan este comando. |
| `tyrus repl` | Inicia la consola interactiva REPL. |

## Convenciones de salida

La salida normal del programa pertenece a stdout. La salida de compilador, linter, fallo de runtime, prompt y progreso pertenece a stderr para que los programas generados sigan siendo amigables con pipes.

::: details Ejemplo de CI
<!-- @include: ../../.fragments/tooling/cli-ci-example.md -->
:::

### Optimización guiada por perfil

El `-O3` adivina qué ramas se toman; un perfil lo sabe. Compilar contra uno deja que el optimizador junte los bloques calientes y aparte los fríos, un camino de error, una rama rara, que es lo único que el código fuente por sí solo no dice.

Son tres pasos, porque un perfil tiene que venir de una ejecución real:

<!-- @include: ../../.fragments/tooling/cli/pgo-workflow.md -->

La ganancia es de layout, así que aparece donde hay ramas que predecir y no donde hay cuentas que hacer: medido en **1,20x** en un bucle de validación con cinco ramas desiguales y caminos de error fríos, y **1,03x** en un bucle aritmético simple. Ambas flags exigen `--release`; un build instrumentado en `-O0` mide código que el optimizador nunca ve.

`llvm-profdata` tiene que venir del mismo toolchain que el driver del compilador, porque el perfil crudo lleva una versión de formato ligada a él. El correcto se encuentra automáticamente; `TYRUS_LLVM_PROFDATA` sobrescribe la elección.
