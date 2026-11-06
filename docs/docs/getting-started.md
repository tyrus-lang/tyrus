---
title: Getting Started
description: Install Tyrus and create a project with the interactive CLI.
---

# Getting Started

Install Tyrus with the command for your operating system, then create a project with the interactive CLI.

## Install Tyrus

### macOS, Linux, and WSL

```sh
curl -fsSL https://tyruslang.org/install.sh | sh
```

### Windows PowerShell

```powershell
irm https://tyruslang.org/install.ps1 | iex
```

## Create a Project

Run:

```sh
tyrus new
```

The CLI asks for the project name and target. For an API project, it also asks whether to configure a database. Answer the prompts to scaffold the project.

From the generated project directory, run:

```sh
tyrus run
```

## Next Steps

Read the [**CLI guide**](../tooling/cli.md) for available commands, or continue with [**Project Configuration**](./configuration.md).
