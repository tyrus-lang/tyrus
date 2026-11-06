---
title: Visibilidade
description: Os três níveis de visibilidade, sem export, export e export protect, além de como um re-export e um import por namespace se comportam.
---

# Visibilidade

Visibilidade responde a uma pergunta: quais arquivos podem importar um nome. O tyrus tem três níveis, e uma declaração escolhe um deles pelo que escreve na frente de si mesma.

| Forma | Quem pode importar |
|---|---|
| sem `export` | ninguém, o nome existe apenas dentro do próprio arquivo |
| `export` | todos os arquivos do projeto |
| `export protect` | a pasta que o declara e todas as subpastas dela |

Os três níveis formam uma escada: `export protect` fica entre um nome que ninguém alcança e um nome que todos alcançam, que é o nível de que um componente precisa para compartilhar um helper entre os próprios arquivos sem publicá-lo.

## Sem `export`: privado ao arquivo

Uma declaração sem `export` é invisível fora do seu arquivo. Não existe palavra-chave para isso, porque é o padrão.

<!-- @include: ../../.fragments/language/visibility/file-private.md -->

Importá-lo é erro: para qualquer outro arquivo, o nome não existe.

::: danger error
<!-- @include: ../../.fragments/language/visibility/file-private-error.md -->
:::

## Visibilidade controla o import, não a chamada

Visibilidade decide quais arquivos podem *nomear* uma declaração. Ela não diz nada sobre quais declarações podem *usá-la*. Uma função pública é livre para chamar um helper privado do próprio arquivo, e importar essa função funciona mesmo que o helper seja inalcançável para o importador.

<!-- @include: ../../.fragments/language/visibility/call-through.md -->

`sign` é público, então `src/http/handler.tyr` pode importá-lo. `salt` é privado a `src/auth/token.tyr`, então o mesmo arquivo não pode importá-lo, e nem precisa: a chamada a `salt` vive dentro de `token.tyr`, onde o nome está em escopo. O mesmo vale um nível acima, com `export protect salt` em vez de um `salt` privado: um `sign` público ainda pode chamá-lo, e um arquivo fora da pasta ainda pode importar `sign`.

É isso que torna os níveis úteis. Um componente publica a superfície pequena que quer que os chamadores vejam e mantém a maquinaria por trás dela no nível que couber, sem ter de alargar a maquinaria para acompanhar a superfície.

## `export`: público no projeto

`export` publica um nome para todos os arquivos do projeto, esteja ele em qualquer pasta.

<!-- @include: ../../.fragments/language/visibility/public.md -->

O alcance para no projeto. O compilador recusa um import que resolva fora da raiz do projeto, então `export` significa "público dentro deste projeto", nunca "público para o mundo". Pacotes externos ainda não são suportados.

## `export protect`: a pasta declarante e abaixo

`export protect` estreita o alcance até a pasta que declara o nome, mais todas as subpastas dela.

<!-- @include: ../../.fragments/language/visibility/protect.md -->

Os dois importadores acima estão dentro de `src/auth/`, então ambos resolvem. Um arquivo numa pasta irmã não está, e recebe `E3308` nomeando o símbolo, a pasta à qual ele está limitado e as duas saídas.

::: danger error
<!-- @include: ../../.fragments/language/visibility/protect-error.md -->
:::

O escopo depende apenas do arquivo declarante, nunca da profundidade de `src/`, então mover uma pasta inteira preserva todas as relações dentro dela.

### Uma pasta aninhada é um limite mais estreito

A relação corre só para baixo. Um nome protegido numa subpasta não é visível na pasta acima dela, porque o importador precisa ser descendente da pasta que declara.

<!-- @include: ../../.fragments/language/visibility/protect-nested.md -->

::: danger error
<!-- @include: ../../.fragments/language/visibility/protect-nested-error.md -->
:::

Cada pasta é um limite real, não um clube mútuo. Se um arquivo aninhado precisa ser alcançável de todo o componente, declare-o com um `export` simples e aceite o alcance maior.

### Numa cláusula separada

O modificador também vai numa cláusula de export, que é como um arquivo publica alguns nomes de forma estreita e outros de forma aberta.

<!-- @include: ../../.fragments/language/visibility/protect-clause.md -->

## Um re-export não consegue alargar o escopo

Um re-export encaminha um nome sem declará-lo, então carrega o escopo original sem alterá-lo. Dentro da pasta, passar por um arquivo barril funciona como sempre.

<!-- @include: ../../.fragments/language/visibility/protect-reexport.md -->

Fora da pasta não funciona, porque o barril encaminhou o escopo junto com o nome. Nem `export *` nem `export { nome } from` são rota de fuga.

::: danger error
<!-- @include: ../../.fragments/language/visibility/protect-reexport-error.md -->
:::

## Um import por namespace omite os nomes protegidos

Um import por namespace liga todos os exports de uma vez. Nomes fora de escopo simplesmente não estão no namespace, em vez de serem ligados e depois recusados.

<!-- @include: ../../.fragments/language/visibility/protect-namespace.md -->

Buscar `token.sign` de `src/main.tyr` falha do mesmo jeito que falha qualquer nome que o módulo não exporta.

## O que o `protect` recusa

`protect` descreve onde uma declaração é visível, então precisa de uma declaração à qual se ligar. Duas formas não têm uma.

| Forma | Código | Por quê |
|---|---|---|
| `export protect default name;` | `E3309` | um export default publica um espaço anônimo, então o escopo de pasta não tem a que se ligar |
| `export protect { name } from "./other";` | `E3310` | um re-export não declara o nome, ele encaminha um, e encaminhar preserva o escopo original |

Escrever o modificador primeiro, como `protect export`, é erro de sintaxe comum. A ordem é fixa em `export protect`, então existe apenas uma grafia.

## Testes e a raiz de código

Não há exceção para arquivos de teste. Um teste que precisa de um nome protegido pertence dentro da pasta que o possui, colocado junto do código que ele testa.

Uma declaração `protect` na raiz de código não estreita nada, pois a pasta dela já cobre todos os arquivos que o compilador aceitaria. Isso é válido mas inútil, e a regra de lint `export-protect-no-effect` avisa sobre isso.
