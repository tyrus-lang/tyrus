---
title: Environment Variables
description: Typed .env files, validation, diagnostics, sealed env artifacts, and process.env access.
---

# Environment Variables

When `tyrus.json` sets `security.envFile`, the compiler reads that `.env` file and exposes variables through `process.env` with static types. The binary does not embed secrets. The compiler and Language Server share the same analysis.

## Configuration

<!-- @include: ../.fragments/docs/config.md -->

## Typed .env

<!-- @include: ../.fragments/docs/typed-env.md -->

Validator Decorations in `.env` files may be written raw or with a leading `#`. Tyrus parses both forms. Keep the `#` when another `.env` loader, such as Docker, also reads the file; that loader sees a comment while Tyrus still applies the validators.

```dotenv
# @type(u16)
# @port()
PORT=8080
```

The parser splits at the first `=`, requires a value, strips optional quotes, supports `${VAR}` interpolation, ignores comments and blank lines, accepts `export KEY=value`, and treats inline `#` as part of the value. Every variable must have `@type`, either raw or commented as `# @type(...)` for compatibility with other `.env` loaders.

## Access

<!-- @include: ../.fragments/docs/access.md -->

Member access and destructuring are typed from the `.env` declaration. Indexed access such as `process.env["PORT"]` is rejected so the compiler can keep static guarantees.

## Diagnostics

Common diagnostics include `E3147`, `E3148`, `E3149`, `E3150`, `E3151`, `E3152`, `E3144`, `E3146`, `E3142`, `E3143`, `E3141`, `E3140`, `E3145`, and `E3083`.

A `.env` diagnostic never prints the value it rejects. It names the variable and the rule that failed, and its code frame shows the value as `<redacted>`, not a mask of the same width, which would still publish how many characters the secret has. A failing build writes its diagnostics straight into a CI log, and the code frame prints the line it points at, so a careful message alone would not be enough.

## Runtime Resolution

Runtime values resolve in this order: the real process environment wins, and the `.env` file is the fallback. Startup validates presence, `@type`, and compatible Validator Decorations before user code runs. Failures print to stderr and exit non-zero.

## Reserved Variables

`TYRUS_WORKER_THREADS` is read by the runtime itself, not by your program. It sets how many worker threads the asynchronous runtime may use, and overrides `runtime.workerThreads` from `tyrus.json`, a deployment can shrink a program to the machine it landed on without rebuilding it. When neither is set, the runtime uses one worker per logical CPU.

A value that is not a positive integer is ignored with a warning on stderr rather than being fatal, and a count well above the detected number of logical CPUs is honoured but also reported, since oversubscribing the machine is occasionally deliberate.

`TYRUS_REPORT_PENDING_TASKS=1` asks the runtime to report, on stderr, how many asynchronous tasks were still pending when `main` finished. It is off by default because a correct program can end with pending tasks by design; turn it on when a task seems not to have run at all.

## Sealed Env

Set `security.envSealed` to `true` to encrypt the env source into `target/<profile>/.env.sealed` with `age`. The generated binary then requires `.env.sealed` next to the executable and rejects mismatched fingerprints or keys. Use `tyrus build --rotate-seal-key` to generate a new key pair and reseal.

## .env.example

Tyrus automatically maintains a safe `<envFile>.example` mirror next to the env file. Whenever you add, remove, or rename a variable in `.env`, Tyrus mirrors the change in `.env.example`, keeping order and decorators while clearing values after `=`. This file is intended to be committed so the team knows which variables are required without exposing their values.
