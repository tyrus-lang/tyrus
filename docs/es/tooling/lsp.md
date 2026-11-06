---
title: Language Server Protocol
description: Vista general del soporte de editor de Tyrus mediante el Language Server Protocol.
---

# Language Server Protocol

El Language Server de Tyrus ofrece inteligencia compartida de editor para integraciones con VS Code, Neovim, Zed, Kate e IntelliJ/WebStorm.

## Recursos

- Diagnósticos
- Hover
- Completion
- Signature help
- Code actions
- Semantic tokens

Dentro de un archivo de prueba, la completion y el hover cubren la superficie de `tyrus:test`: los matchers después de `expect(...)`, los modificadores `.not`, `.rejects` y `.resolves`, y los modificadores después de `test.`, `it.` o `describe.`.

Dentro de un literal de cadena SQL de `db.query`, la completion sugiere tablas y columnas del snapshot local del schema. El hover sobre una tabla o columna muestra la información del schema, incluida la opcionalidad producida por un join externo. El compilador y el Language Server usan el mismo snapshot para los diagnósticos SQL.

## Inicia el Server

<!-- @include: ../../.fragments/tooling/lsp-start-server.md -->

El server se comunica por stdio. Las integraciones de editor deben iniciar `tyrus lsp` y depender de los diagnósticos compartidos del compilador en vez de duplicar reglas del lenguaje en cada cliente de editor.

## Diagnósticos y Fixes

Los diagnósticos del compilador y del linter deben preservar códigos estables, rangos precisos, información relacionada y ayuda concreta. Cuando el compilador o linter tenga una edición determinística, el Language Server debe exponerla como code action.

## Navegación y Quick Fixes

Usa el comando Go to Definition del editor sobre un identificador para navegar a declaraciones locales o importadas. Para aplicar una corrección determinista, invoca Quick Fix sobre un diagnóstico mediante el comando de code action o el icono de bombilla del editor. El server publica las ediciones mediante code actions de LSP, mientras que los atajos dependen del editor.

## Integraciones de Editores

### VS Code

Instala la [extensión de VS Code](./vscode.md). Inicia el servidor compartido `tyrus lsp` para archivos `.tyr`.

### Neovim

Configura el cliente LSP de Neovim para ejecutar `tyrus lsp` con el filetype `tyrus`. La integración de Neovim del repositorio proporciona detección de archivos y syntax highlighting.

### Zed

Instala la extensión de desarrollo de Tyrus y configúrala para iniciar `tyrus lsp`. La integración actual de Zed es un scaffold de desarrollo.

### IntelliJ y WebStorm

Instala LSP4IJ, registra `tyrus lsp` como language server y asigna los archivos `*.tyr`. La grammar TextMate de VS Code puede proporcionar syntax highlighting.

### Kate

Habilita el LSP Client integrado de Kate y registra `tyrus lsp`. La configuración detallada de Kate aparece abajo.

## Kate Configuration

Kate soporta el Language Server Protocol a través del plugin LSP Client integrado. Tyrus se integra con Kate mediante el mismo server `tyrus lsp` que usan todos los demás editores.

### Prerrequisitos

- Tyrus instalado y `tyrus` disponible en el `PATH`.
- Kate con el plugin LSP Client (incluido por defecto en la mayoría de las distribuciones).

### Configuración del LSP

Abre Kate y habilita **Settings > Configure Kate > Plugins > LSP Client**. Después ve a **Settings > Configure Kate > LSP Client > User Server Settings** y establece **Settings File** con el archivo de configuración. Crear solamente el archivo JSON no hace que Kate lo cargue.

Usa la ruta correspondiente a tu instalación:

| Instalación | Archivo de configuración |
|---|---|
| Linux (nativo) | `$HOME/.config/kate/lspclient/settings.json` |
| Snap (`classic`) | `$HOME/.config/kate/lspclient/settings.json` |
| Snap (confinamiento estricto) | `$HOME/snap/kate/current/.config/kate/lspclient/settings.json` |

Ejecuta `snap list kate` y revisa la columna **Notes** para distinguir los dos modos de Snap. El Snap oficial de Kate usa actualmente `classic`, por lo que lee los mismos directorios del usuario que una instalación nativa.

Agrega una entrada para Tyrus:

<!-- @include: ../../.fragments/tooling/lsp-kate-settings.md -->

| Campo | Propósito |
|---|---|
| `command` | Binario y subcomando que inician el LSP. |
| `path` | Rutas adicionales para buscar ejecutables. Esto permite que Kate iniciado desde la interfaz encuentre el binario de Tyrus aunque el `PATH` heredado no incluya el directorio de instalación. |
| `highlightingModeRegex` | Debe coincidir con el nombre de la definición de syntax highlighting de Tyrus instalada en Kate. |
| `rootIndicationFileNames` | Archivos que señalan la raíz del proyecto. Kate recorre el árbol de directorios buscando uno de ellos. |

### Syntax Highlighting

Kate usa definiciones de syntax basadas en XML. La definición de Tyrus es generada por el crate xtask a partir de la misma fuente de keywords que los otros editores. En una instalación nativa de Linux, ejecuta el helper del repositorio para copiar el archivo generado:

<!-- @include: ../../.fragments/tooling/kate-install-syntax.md -->

Define `KATE_SYNTAX_DIR` al usar otro directorio de datos de Kate. Las rutas compatibles son:

| Plataforma | Ruta |
|---|---|
| Linux (nativo) | `$HOME/.local/share/org.kde.syntax-highlighting/syntax/` |
| Flatpak | `$HOME/.var/app/*package-name*/data/org.kde.syntax-highlighting/syntax/` |
| Snap (`classic`) | `$HOME/.local/share/org.kde.syntax-highlighting/syntax/` |
| Snap (confinamiento estricto) | `$HOME/snap/kate/current/.local/share/org.kde.syntax-highlighting/syntax/` |
| Windows | `%USERPROFILE%\AppData\Local\org.kde.syntax-highlighting\syntax\` |
| macOS | `$HOME/Library/Application Support/org.kde.syntax-highlighting/syntax/` |

Reinicia Kate después de instalar o actualizar el archivo. Si un archivo `.tyr` ya está abierto, ve a **Tools > Editing Tools > Refresh Syntax Highlighting** y confirma que el modo de highlighting sea **Tyrus**. La entrada `highlightingModeRegex` activa el server solamente para documentos que usan exactamente ese modo.

### Verificación

Abre un archivo `.tyr` dentro de un directorio que contenga `tyrus.json`. El LSP se inicia automáticamente y ofrece diagnósticos, completion, hover y otras funcionalidades. Kate no muestra un indicador persistente de conexión, y un archivo válido puede dejar vacío el panel de diagnósticos. Prueba la conexión con hover o completion y usa **View > Tool Views > Diagnostics** y **Symbol Outline** para obtener resultados visibles. Después de cambiar el JSON, usa **Tools > LSP Client > Restart LSP Server** o reinicia Kate.
