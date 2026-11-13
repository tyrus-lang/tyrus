---
title: Custom Validator
description: Validators definidos por el usuario para campos de interface y parámetros de función decorados.
---

# Custom Validator

Los validators personalizados permiten definir reglas de validación de la aplicación sin agregar nuevos validators integrados. Se pueden usar en campos de interface y parámetros de función, y se ejecutan de forma síncrona cuando se comprueba el valor decorado.

## Declaración

Declara un validator con `validator function` o `validator const`. El primer parámetro debe tener un tipo explícito y llamarse `value`; los parámetros opcionales de configuración van después y reciben los argumentos posicionales escritos en el decorator. La función devuelve la unión `Validator`. Un resultado válido es `{ isValid: true }`; un fallo devuelve `{ isValid: false, message: "..." }`.

El compilador proporciona el valor del campo o parámetro decorado como `value`. Por ejemplo, `@hasPrefix("user_")` llama al validator con el valor primero y `"user_"` como argumento de configuración. Usa paréntesis vacíos para validators sin configuración, como `@hasNoSpaces()`.

## Ejemplo

<!-- @include: ../../.fragments/language/validators/custom-example.md -->

## Mensajes devueltos

Un validator personalizado elige su mensaje de fallo al devolver `{ isValid: false, message }`. Ese mensaje se incluye en el `ValidationIssue` resultante; el valor comprobado no se inserta automáticamente. Un fallo lanza `ValidationError`, por lo que la función que lo contiene debe capturarlo o declarar `throws ValidationError`.
