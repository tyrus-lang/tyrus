---
title: Tyrus MCP
---

# Tyrus MCP

El servidor MCP de Tyrus permite que clientes de IA compatibles busquen la documentación oficial y comprueben código Tyrus independiente con el análisis compartido del compilador. La instalación es automática para los clientes compatibles detectados en la máquina.

## Instalación

Ejecuta un comando después de instalar Tyrus:

```bash
tyrus mcp install
```

El comando descarga y verifica una caché limitada de la documentación, comprueba el handshake del servidor MCP local y añade una entrada `tyrus` a los archivos de configuración de los clientes detectados. Se admite la configuración de usuario de Claude Desktop, Cursor, Zed y VS Code Copilot. Se conservan los ajustes existentes, no se sobrescriben las entradas `tyrus` existentes y los archivos modificados reciben una copia de seguridad antes de la primera edición.

Reinicia o recarga cada cliente configurado. Después, pregunta al asistente sobre Tyrus. El cliente inicia `tyrus mcp serve` en segundo plano; normalmente no ejecutas ese comando manualmente. Si no se detecta ningún cliente compatible, instala uno y vuelve a ejecutar `tyrus mcp install` o sigue la configuración manual a continuación.

## Comandos

| Comando | Finalidad |
|---------|-----------|
| `tyrus mcp install` | Descarga la caché de documentación, configura los clientes detectados y comprueba el handshake del servidor. |
| `tyrus mcp update` | Actualiza la caché y las entradas administradas por Tyrus en los clientes. |
| `tyrus mcp status` | Muestra las versiones de la caché, la configuración de los clientes detectados y los conflictos. |
| `tyrus mcp uninstall` | Elimina las entradas administradas sin cambios y la caché local de documentación. |
| `tyrus mcp serve` | Comando interno del servidor stdio iniciado por un cliente MCP configurado. |

## Funciones MCP disponibles

| Función | Descripción |
|---------|-------------|
| `search_docs` | Busca títulos indexados y devuelve fragmentos clasificados y enlaces oficiales. |
| `get_page` | Lee una página completa incluida en el índice de documentación. |
| `check_source` | Analiza sintaxis, tipos y préstamos de hasta 128 KiB de código independiente en memoria. |
| `tyrus://docs/index` | Lee el índice de documentación. |
| `tyrus://docs/{path}` | Lee una página Markdown indexada. |
| `tyrus://llms.txt` | Lee el índice compacto de documentación. |
| `tyrus://llms-full.txt` | Solicita explícitamente el paquete completo de documentación. |
| `generate_tyrus_code` | Guía al modelo para encontrar la documentación actual antes de generar código. |
| `fix_tyrus_code` | Guía al modelo para comprobar y corregir código independiente. |

`search_docs` acepta una consulta de hasta 512 bytes UTF-8 y devuelve como máximo 20 resultados. `get_page` devuelve hasta 1 MiB por página; el recurso de documentación completa está limitado a un paquete descargado de 32 MiB. `check_source` acepta hasta 128 KiB e interrumpe el análisis después de cinco segundos.

`check_source` devuelve códigos de diagnóstico estables, niveles de severidad, mensajes, rangos en bytes y posiciones de línea y columna que empiezan en uno. No ejecuta código, crea archivos, lee un proyecto, carga variables de entorno ni resuelve imports relativos al proyecto. Los imports de módulos nativos están disponibles cuando el compilador tiene declaraciones independientes para ellos. El análisis no es un sandbox de seguridad. Los clientes y modelos deciden cuándo llamar a las herramientas MCP, por lo que el servidor no puede garantizar que el asistente compruebe el código o consulte la documentación.

## Configuración manual

Usa la ruta absoluta del ejecutable de Tyrus en el campo `command`. Conserva los demás ajustes del archivo. Cursor, Claude Desktop y Copilot usan un objeto `mcpServers`; Zed usa `context_servers`.

| Cliente | Ubicación de la configuración del usuario | Clave raíz |
|---------|------------------------------------------|------------|
| Claude Desktop | macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`; Windows: `%APPDATA%/Claude/claude_desktop_config.json`; Linux: `${XDG_CONFIG_HOME:-~/.config}/Claude/claude_desktop_config.json` | `mcpServers` |
| Cursor | `~/.cursor/mcp.json` | `mcpServers` |
| Zed | macOS: `~/Library/Application Support/Zed/settings.json`; Windows: `%APPDATA%/Zed/settings.json`; Linux: `${XDG_CONFIG_HOME:-~/.config}/zed/settings.json` | `context_servers` |
| VS Code Copilot | `${COPILOT_HOME:-~/.copilot}/mcp-config.json` | `mcpServers` |

```json
{
  "mcpServers": {
    "tyrus": {
      "command": "/absolute/path/to/tyrus",
      "args": ["mcp", "serve"]
    }
  }
}
```

En Zed, usa la misma entrada `tyrus` bajo `context_servers` en su archivo de ajustes. Las ubicaciones de los archivos pueden variar según el sistema operativo, el perfil o la versión del cliente. Prefiere la interfaz de ajustes MCP del cliente cuando la ruta difiera de la predeterminada. Consulta las guías oficiales de configuración de [Claude Desktop](https://modelcontextprotocol.io/quickstart/user), [Cursor](https://docs.cursor.com/context/model-context-protocol), [Zed](https://zed.dev/docs/ai/mcp) y [VS Code Copilot](https://code.visualstudio.com/docs/agent-customization/mcp-servers).

## Conflictos y solución de problemas

Si un cliente ya tiene una entrada `tyrus` que el instalador no administra, el instalador la conserva e informa del conflicto. Revisa esa entrada, cámbiala o elimínala manualmente y vuelve a ejecutar `tyrus mcp install`. Si el archivo de configuración no es válido o no se puede conservar de forma segura, usa la interfaz de ajustes del cliente o añade la entrada manualmente.

Ejecuta `tyrus mcp status` para inspeccionar la caché y las entradas de los clientes. Ejecuta `tyrus mcp update` para actualizar la documentación desactualizada. Ejecuta `tyrus mcp uninstall` para eliminar las entradas que todavía coincidan con los valores registrados por Tyrus; las entradas editadas después de la instalación se conservan y deben eliminarse manualmente. Reinicia el cliente después de cambiar la configuración MCP.
