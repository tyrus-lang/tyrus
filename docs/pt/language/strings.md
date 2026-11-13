---
title: Strings
description: Literais de string, template literals, interpolação, texto multiline e tipos string literais em tyrus.
---

# Strings

`string` é o tipo nativo de texto UTF-8 no tyrus. A sintaxe fonte é concisa, enquanto a implementação de runtime é nativa.

## Strings com aspas

<!-- @include: ../../.fragments/language/strings/quoted-strings.md -->

O formatter normaliza aspas conforme `quote_style`, exceto quando configurado como `"off"`, que preserva as aspas e escapes originais de cada literal de string.

## Strings multilinha

Uma string delimitada por `"""` pode ocupar várias linhas mantendo o alinhamento com o código ao redor. O compilador remove o recuo em tempo de compilação: de cada linha, ele retira tantos espaços à esquerda quanto o `"""` de fechamento estiver recuado, e descarta uma primeira linha deixada em branco logo após o `"""` de abertura. O resultado é um literal `string` comum, então não há custo em runtime para escrever uma query ou um template alinhado em vez de rente à margem esquerda.

<!-- @include: ../../.fragments/language/strings/multiline-strings.md -->

A margem de dedent conta apenas espaços; um `"""` fechado numa linha recuada com tabs mede margem zero, então use espaços para o recuo que você quer remover.

## Template literals

Use template literals para interpolação e texto multiline.

<!-- @include: ../../.fragments/language/strings/template-literals.md -->

Uma interpolação pode carregar um especificador de formato depois de `:`, `.Nf` fixa um float em exatamente `N` casas decimais, e `0Nd` preenche um inteiro com zeros à esquerda até uma largura total de `N` caracteres. `.Nf` exige um valor float e `0Nd` exige um valor inteiro; uma incompatibilidade é um erro de compilação (`E0717`) que nomeia o tipo exigido.

Um valor de tipo união também pode ser interpolado. Qual variante ele guarda é um fato de tempo de execução, então a renderização é escolhida em tempo de execução e coincide com o que o `print` imprimiria para aquela variante.

## Tipos string literais

String literals podem participar de unions literais e tagged unions.

Uma união de literais é um valor comum: compare-a com qualquer uma de suas variantes (`status === "active"`), ramifique com um `match` que não precisa de `_` quando cobre toda variante, construa uma com um ternário, junte várias num array e chame nela qualquer coisa que uma `string` aceite, toda variante de uma união de literais de string é uma `string`. A comparação é uma checagem de um byte em vez de comparação de texto, porque uma união dessas é representada por qual variante ela guarda e nada mais.

<!-- @include: ../../.fragments/language/strings/literal-string-types.md -->

::: warning Valores literais não são membros de enum
Use strings literais para pequenos domínios textuais fechados. Use `enum` quando o domínio precisa de membros nomeados e semântica nativa de enum.
:::

## Grupos de grafemas

O `chars()` devolve valores escalares Unicode; o `graphemes()` devolve o que um leitor chama de caracteres. Os dois diferem sempre que um caractere visível é escrito com vários valores escalares, um emoji montado com sequência de zero-width joiner, ou uma letra seguida de acento combinante.

<!-- @include: ../../.fragments/language/strings/graphemes.md -->

Indexação, fatiamento e `length` contam valores escalares, não grupos: essa é a unidade em que toda posição da API de string é medida, e é o que faz `text.charAt(text.length - 1)` ser o último. O `graphemes()` responde à pergunta de exibição, quantos caracteres um leitor vê, e onde um cursor pararia, e devolve `string[]`, porque um grupo pode conter mais de um valor escalar.

## Propriedades de string

### length

<!-- @include: ../../.fragments/language/strings/length.md -->

`length` é lido do valor, não contado: uma `string` carrega a contagem de caracteres junto com o texto, então perguntá-la custa o mesmo com três caracteres ou com três milhões. U+0000 é um caractere comum numa `string`, o texto é delimitado pelo comprimento registrado, não por um terminador, então `"a\u{0}b".length` é `3` e ele sobrevive inteiro a comparação, interpolação e impressão.

`length` conta caracteres, valores escalares Unicode, nunca os bytes UTF-8 subjacentes, então concorda com `chars()` em qualquer texto. Toda posição na API de string usa essa mesma unidade: `charAt()`, `charCodeAt()`, `slice()`, `substring()`, `indexOf()`, `lastIndexOf()`, `padStart()` e `padEnd()` contam caracteres, o que é o que faz `text.charAt(text.length - 1)` ser o último caractere de qualquer string, acentuada ou não.

## Métodos

| Categoria | Métodos |
| --- | --- |
| Conversão de caixa | [`toUpperCase()`](#touppercase), [`toLowerCase()`](#tolowercase) |
| Transformação de caixa | [`toSlugCase()`](#toslugcase), [`toDashCase()`](#todashcase), [`toKebabCase()`](#tokebabcase), [`toPascalCase()`](#topascalcase), [`toCamelCase()`](#tocamelcase), [`toSnakeCase()`](#tosnakecase) |
| Remoção de espaços | [`trim()`](#trim), [`trimStart()`](#trimstart), [`trimEnd()`](#trimend) |
| Busca | [`includes()`](#includes), [`startsWith()`](#startswith), [`endsWith()`](#endswith), [`indexOf()`](#indexof), [`lastIndexOf()`](#lastindexof), [`search()`](#search) |
| Fatias e extração | [`slice()`](#slice), [`substring()`](#substring), [`charAt()`](#charat), [`charCodeAt()`](#charcodeat), [`truncate()`](#truncate), [`truncateEllipsis()`](#truncateellipsis) |
| Transformação | [`replace()`](#replace), [`repeat()`](#repeat), [`padStart()`](#padstart), [`padEnd()`](#padend), [`concat()`](#concat), [`split()`](#split) |
| HTML e codificação | [`stripHtml()`](#striphtml), [`escapeHtml()`](#escapehtml), [`urlEncode()`](#urlencode), [`urlDecode()`](#urldecode) |
| Correspondência de padrão | [`match()`](#match), [`matchAll()`](#matchall) |
| Iteração | [`chars()`](#chars), [`graphemes()`](#graphemes) |

### Conversão de caixa

---

#### `toUpperCase()`

Retorna a string com todos os caracteres em maiúsculo.

<SignatureBlock label="Assinatura">toUpperCase(): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/toUpperCase.md -->

---

#### `toLowerCase()`

Retorna a string com todos os caracteres em minúsculo.

<SignatureBlock label="Assinatura">toLowerCase(): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/toLowerCase.md -->

### Transformação de caixa

---

#### `toSlugCase()`

Normaliza a string para um slug amigável a URLs. Remove acentos via decomposição NFKD, remove caracteres não alfanuméricos e junta palavras com hífens.

<SignatureBlock label="Assinatura">toSlugCase(): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/toSlugCase.md -->

---

#### `toDashCase()`

Converte a string para dash-case (palavras minúsculas unidas por hífens). Tokeniza por espaços, underscores, hífens e limites de camelCase. Diferente de `toSlugCase()`, acentos são preservados.

<SignatureBlock label="Assinatura">toDashCase(): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/toDashCase.md -->

---

#### `toKebabCase()`

Alias de `toDashCase()`.

<SignatureBlock label="Assinatura">toKebabCase(): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/toKebabCase.md -->

---

#### `toPascalCase()`

Converte a string para PascalCase. Cada palavra é capitalizada e unida sem separadores.

<SignatureBlock label="Assinatura">toPascalCase(): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/toPascalCase.md -->

---

#### `toCamelCase()`

Converte a string para camelCase. A primeira palavra é minúscula, as demais são capitalizadas.

<SignatureBlock label="Assinatura">toCamelCase(): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/toCamelCase.md -->

---

#### `toSnakeCase()`

Converte a string para snake_case (palavras minúsculas unidas por underscores).

<SignatureBlock label="Assinatura">toSnakeCase(): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/toSnakeCase.md -->

### Remoção de espaços

---

#### `trim()`

Remove espaços em branco de ambas as extremidades da string.

<SignatureBlock label="Assinatura">trim(): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/trim.md -->

---

#### `trimStart()`

Remove espaços em branco do início da string.

<SignatureBlock label="Assinatura">trimStart(): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/trimStart.md -->

---

#### `trimEnd()`

Remove espaços em branco do final da string.

<SignatureBlock label="Assinatura">trimEnd(): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/trimEnd.md -->

### Busca

---

#### `includes()`

Verifica se a string contém uma substring.

<SignatureBlock label="Assinatura">includes(search: string): boolean</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/includes.md -->

---

#### `startsWith()`

Verifica se a string começa com uma substring.

<SignatureBlock label="Assinatura">startsWith(search: string): boolean</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/startsWith.md -->

---

#### `endsWith()`

Verifica se a string termina com uma substring.

<SignatureBlock label="Assinatura">endsWith(search: string): boolean</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/endsWith.md -->

---

#### `indexOf()`

Retorna o índice da primeira ocorrência de uma substring, ou `-1`.

<SignatureBlock label="Assinatura">indexOf(search: string, start?: i64): i64</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/indexOf.md -->

---

#### `lastIndexOf()`

Retorna o índice da última ocorrência de uma substring, ou `-1`.

<SignatureBlock label="Assinatura">lastIndexOf(search: string, start?: i64): i64</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/lastIndexOf.md -->

---

#### `search()`

Busca um padrão usando expressão regular e retorna o índice da correspondência.

<SignatureBlock label="Assinatura">search(pattern: Regex): i64</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/search.md -->

### Fatias e extração

---

#### `slice()`

Extrai uma seção da string e a retorna como uma nova string.

<SignatureBlock label="Assinatura">slice(start?: i64, end?: i64): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/slice.md -->

---

#### `substring()`

Retorna uma substring a partir de um índice dado.

<SignatureBlock label="Assinatura">substring(start: i64, end?: i64): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/substring.md -->

---

#### `charAt()`

Retorna o caractere em um índice especificado.

<SignatureBlock label="Assinatura">charAt(index: i64): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/charAt.md -->

---

#### `charCodeAt()`

Retorna o código Unicode do caractere em um índice especificado.

<SignatureBlock label="Assinatura">charCodeAt(index: i64): u32</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/charCodeAt.md -->

---

#### `truncate()`

Trunca a string para um número máximo de code points.

<SignatureBlock label="Assinatura">truncate(maxLength: i64): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/truncate.md -->

---

#### `truncateEllipsis()`

Trunca a string para um número máximo de code points e anexa `...` quando o texto excede o limite.

<SignatureBlock label="Assinatura">truncateEllipsis(maxLength: i64): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/truncateEllipsis.md -->

### Transformação

---

#### `replace()`

Substitui uma substring ou padrão por uma string de substituição.

<SignatureBlock label="Assinatura">replace(search: string | Regex, replacement: string): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/replace.md -->

---

#### `repeat()`

Retorna uma nova string com o original repetido um número dado de vezes.

<SignatureBlock label="Assinatura">repeat(count: i64): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/repeat.md -->

---

#### `padStart()`

Preenche o início da string com um caractere de preenchimento até atingir um comprimento alvo.

<SignatureBlock label="Assinatura">padStart(targetLength: i64, fill?: string): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/padStart.md -->

---

#### `padEnd()`

Preenche o final da string com um caractere de preenchimento até atingir um comprimento alvo.

<SignatureBlock label="Assinatura">padEnd(targetLength: i64, fill?: string): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/padEnd.md -->

---

#### `concat()`

Concatena a string com uma ou mais strings.

Guardar uma `string` além do valor de onde ela veio, colocá-la num array, capturá-la numa closure, armazená-la num campo, não copia mais o texto. As duas compartilham um buffer e cada uma segura uma referência a ele, o que é seguro justamente porque uma `string` é imutável: dois leitores de um buffer não têm como discordar.

Uma cadeia de `+` custa uma alocação, não uma por operador: `a + b + c + d` mede todos os operandos primeiro, cada `string` carrega o próprio comprimento, então medir não lê nada, e só então preenche um único buffer.

<SignatureBlock label="Assinatura">concat(...strings: string[]): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/concat.md -->

---

#### `split()`

Divide a string em um array de substrings usando um separador.

<SignatureBlock label="Assinatura">split(separator: string | Regex): string[]</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/split.md -->

### HTML e codificação

---

#### `stripHtml()`

Remove todas as tags HTML da string, deixando apenas o conteúdo textual. Decodifica entidades HTML comuns (`&lt;`, `&gt;`, `&amp;`, `&quot;`, `&#39;`).

<SignatureBlock label="Assinatura">stripHtml(): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/stripHtml.md -->

---

#### `escapeHtml()`

Escapa caracteres especiais para entidades HTML (`&` `<` `>` `"` `'`), prevenindo injeção XSS.

<SignatureBlock label="Assinatura">escapeHtml(): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/escapeHtml.md -->

---

#### `urlEncode()`

Codifica a string com percent-encoding para uso como parâmetro de query URL. Codifica todos os caracteres exceto caracteres não reservados (letras, dígitos, `-`, `_`, `.`, `~`).

<SignatureBlock label="Assinatura">urlEncode(): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/urlEncode.md -->

---

#### `urlDecode()`

Decodifica uma string codificada com percent-encoding de volta à sua forma original.

<SignatureBlock label="Assinatura">urlDecode(): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/urlDecode.md -->

### Correspondência de padrão

---

#### `match()`

Compara a string com uma expressão regular e retorna grupos capturados.

<SignatureBlock label="Assinatura">match(pattern: Regex): string[]?</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/match.md -->

---

#### `matchAll()`

Encontra todas as correspondências de uma expressão regular na string.

<SignatureBlock label="Assinatura">matchAll(pattern: Regex): string[][]?</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/matchAll.md -->

### Iteração

---

#### `chars()`

Itera sobre os caracteres da string.

<SignatureBlock label="Assinatura">chars(): char[]</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/chars.md -->

---

#### `graphemes()`

Itera sobre os grupos de grafemas da string, os caracteres que um leitor percebe, onde um grupo pode conter vários valores escalares.

<SignatureBlock label="Assinatura">graphemes(): string[]</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/methods/graphemes.md -->


## Trabalhando com números

### parseInt

Analisa uma string e retorna um inteiro opcional. Retorna vazio quando a entrada não é uma representação válida de inteiro.

<SignatureBlock label="Assinatura">parseInt(value: string): i64?</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/parseInt.md -->

### parseFloat

Analisa uma string e retorna um número de ponto flutuante opcional. Retorna vazio quando a entrada não é uma representação válida de float.

<SignatureBlock label="Assinatura">parseFloat(value: string): f64?</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/parseFloat.md -->

### toString

Converte um valor para sua representação em string.

<SignatureBlock label="Assinatura">toString(): string</SignatureBlock>

<!-- @include: ../../.fragments/language/strings/toString.md -->

## Integração com Regex

Strings trabalham nativamente com o tipo `Regex`:

<!-- @include: ../../.fragments/language/strings/regexp-integration.md -->

Literais de string não são `char`
Um literal de string com múltiplos caracteres não pode ser atribuído a `char`:

::: danger error
<!-- @include: ../../.fragments/language/strings/error-char.md -->
:::

::: danger error
<!-- @include: ../../.fragments/language/strings/error-char-output.md -->
:::
