---
title: race
description: Resuelve o rechaza con el JoinHandle de un array que se asienta primero, cancelando el resto.
---

# race

`race` resuelve o rechaza con el `JoinHandle<T>` de un array que se asienta primero.

<SignatureBlock label="Firma">race&lt;T&gt;(handles: JoinHandle&lt;T&gt;[]): Promise&lt;T&gt;</SignatureBlock>

En cuanto un handle se asienta, `race` cancela cooperativamente todos los demás a través del mismo mecanismo de `cancel`/`JoinHandle<T>.abort()`, un handle que ya se asentó para entonces (el propio ganador, o un perdedor que terminó justo entre que la carrera se asienta y esta limpieza corre) se deja tranquilo, ya que cancelar una Promise ya asentada es un no-op inofensivo. Todo handle pasado a `race` se consume igual que `.join()` consume uno, reutilizar un handle ya pasado a `race`, a `joinAll`, o a su propio `.join()`, se rechaza en tiempo de compilación (`E3033`).

<!-- @include: ../../../.fragments/modules/task/race.md -->
