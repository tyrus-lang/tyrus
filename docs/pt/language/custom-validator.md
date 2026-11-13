---
title: Custom Validator
description: Validators definidos pelo usuário para campos de interface e parâmetros de função decorados.
---

# Custom Validator

Validators personalizados permitem definir regras de validação da aplicação sem adicionar novos validators nativos. Eles podem ser usados em campos de interface e parâmetros de função e executam de forma síncrona quando o valor decorado é verificado.

## Declaração

Declare um validator com `validator function` ou `validator const`. O primeiro parâmetro deve ter tipo explícito e se chamar `value`; parâmetros opcionais de configuração vêm depois e recebem os argumentos posicionais escritos no decorator. A função retorna a união `Validator`. Um resultado válido é `{ isValid: true }`; uma falha retorna `{ isValid: false, message: "..." }`.

O compilador fornece o valor do campo ou parâmetro decorado como `value`. Por exemplo, `@hasPrefix("user_")` chama o validator com o valor primeiro e `"user_"` como argumento de configuração. Use parênteses vazios para validators sem configuração, como `@hasNoSpaces()`.

## Exemplo

<!-- @include: ../../.fragments/language/validators/custom-example.md -->

## Mensagens retornadas

Um validator personalizado escolhe a mensagem de falha retornando `{ isValid: false, message }`. Essa mensagem é incluída no `ValidationIssue` resultante; o valor verificado não é inserido automaticamente. Uma falha lança `ValidationError`, então a função que o contém deve capturá-lo ou declarar `throws ValidationError`.
