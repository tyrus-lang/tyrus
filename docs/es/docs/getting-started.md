---
title: Primeros Pasos
description: Instala Tyrus y crea un proyecto con la CLI interactiva.
---

# Primeros Pasos

Instala Tyrus con el comando correspondiente a tu sistema operativo y crea un proyecto con la CLI interactiva.

## Instala Tyrus

### macOS, Linux y WSL

```sh
curl -fsSL https://tyruslang.org/install.sh | sh
```

### Windows PowerShell

```powershell
irm https://tyruslang.org/install.ps1 | iex
```

## Crea un Proyecto

Ejecuta:

```sh
tyrus new
```

La CLI pregunta el nombre y el target del proyecto. Para un proyecto de API, también pregunta si debe configurar una base de datos. Responde a las preguntas para generar el proyecto.

Desde el directorio del proyecto generado, ejecútalo:

```sh
tyrus run
```

## Siguientes Pasos

Lee la [**guía de la CLI**](../tooling/cli.md) para conocer los comandos disponibles o continúa con [**Configuración del Proyecto**](./configuration.md).
