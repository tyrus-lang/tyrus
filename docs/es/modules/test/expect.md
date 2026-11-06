---
title: expect
description: Comprueba un valor con un matcher.
---

# expect

<SignatureBlock label="Firma">expect&lt;T&gt;(value: T): Assertion</SignatureBlock>

`expect(value)` evalúa `value` exactamente una vez y devuelve una aserción. Un matcher llamado sobre la aserción compara el valor y reprueba la prueba cuando la comparación no se cumple. `.not` antes del matcher lo invierte.

| Matcher | Pasa cuando |
| --- | --- |
| `toBe(expected)`, `toEqual(expected)`, `toStrictEqual(expected)` | los valores son iguales: un escalar, un objeto campo a campo, o un array elemento a elemento, hasta el final |
| `toBeTruthy()`, `toBeFalsy()` | el valor es truthy o falsy |
| `toBeNull()`, `toBeUndefined()`, `toBeDefined()` | un valor opcional está ausente o presente |
| `toBeGreaterThan(n)`, `toBeGreaterThanOrEqual(n)` | el número es mayor que `n` (o igual a él) |
| `toBeLessThan(n)`, `toBeLessThanOrEqual(n)` | el número es menor que `n` (o igual a él) |
| `toBeCloseTo(n)` | el float está a lo sumo a `0.005` de `n` |
| `toHaveLength(n)` | la string o el array tiene longitud `n` |
| `toContain(value)` | la string contiene la substring, o el array contiene el elemento |
| `toMatch(text)` | la string contiene `text` |
| `toThrow()`, `toThrow(message)` | llamar a la función lanza (y la `string` lanzada es igual a `message`); la función debe declarar `throws` y no recibir argumentos |

Un matcher que falla nombra los valores comparados, con los strings entre comillas: `expected "abc" to equal "abd"`.

Dos matchers que otros frameworks tienen quedan fuera a propósito, y escribir uno de ellos es un error de compilación que indica qué usar en su lugar. `toHaveProperty` pregunta algo que el compilador ya decidió: un campo se resuelve en tiempo de compilación, así que la aserción solo puede ser siempre verdadera; para saber si un campo opcional tiene valor, escribe `expect(user.email).toBeDefined()`. `toBeInstanceOf` no tiene qué consultar, porque no existen instancias de clase; estrecha el valor con `match` o una comprobación de opcional y afirma sobre lo que el valor estrechado guarda.

`.rejects` antes de `toThrow` espera una promise y afirma que falló: `expect(load()).rejects.toThrow("not found")`, con el mensaje opcional y `.not` disponible. La espera ocurre por medio del runtime en lugar de suspender, así que también funciona en una prueba síncrona. La promise debe ser una cuya falla sea un rechazo, como `Promise.reject(...)` o una función `async` que devuelva una; una función que declara `throws` tiene ese efecto tratado donde se la llama, antes de que el matcher la vea. `.resolves` resuelve una promise de la misma manera y entrega al matcher siguiente el valor obtenido, así que todos los matchers funcionan después de él: `expect(load()).resolves.toBe(42)`, `expect(load()).resolves.toEqual({ id: 1 })`. Una promise que falló reprueba la aserción, indicando con qué fue rechazada. Los dos son exclusivos, porque una promise no puede afirmarse como falla y éxito a la vez, y `toThrow` pertenece a `.rejects`: después de `.resolves` el matcher ya tiene el valor, así que no queda falla alguna que afirmar. `toBe`, `toEqual` y `toStrictEqual` comparan valores escalares, objetos campo a campo (la misma comparación que hace el `equals` automático) y arrays elemento a elemento. Un array de escalares usa la misma igualdad de elemento que usan `includes` e `indexOf`; un array de objetos, o de arrays, se compara hasta el final, deteniéndose en la primera diferencia.

<!-- @include: ../../../.fragments/modules/test/expect.md -->
