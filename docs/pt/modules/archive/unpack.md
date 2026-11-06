---
title: unpack
description: Extrai com segurança um arquivo ZIP, TAR ou TAR.GZ para um diretório.
---

# unpack

`unpack` extrai um arquivo ZIP, TAR ou TAR.GZ para um diretório de destino, criando-o quando não existe. O formato é identificado pelo conteúdo, nunca pelo nome do arquivo.

<SignatureBlock label="Assinatura">unpack(archive: string, destination: string, options?: UnpackOptions): Promise&lt;Result&lt;ArchiveSummary, ArchiveError&gt;&gt;</SignatureBlock>

## Exemplo mínimo

<!-- @include: ../../../.fragments/modules/archive/unpack/minimal.md -->

## Todas as opções

<!-- @include: ../../../.fragments/modules/archive/unpack/maximal.md -->

| Opção | Tipo | Padrão | Efeito |
|--------|------|---------|--------|
| `overwrite` | `boolean` | `false` | Substitui arquivos que já existem no destino. Diretórios e links nunca são substituídos. |
| `stripComponents` | `u32` | `0` | Remove essa quantidade de componentes iniciais do caminho de cada entrada; entradas que ficam vazias são ignoradas. |
| `maxEntries` | `u64` | `100000` | Recusa arquivos com mais entradas. |
| `maxBytes` | `u64` | `4 GiB` | Recusa a extração que gravaria mais bytes, contados à medida que são gravados. |
| `maxRatio` | `u32` | `200` | Recusa uma entrada ZIP que expande mais que essa quantidade de vezes o tamanho comprimido, e um arquivo TAR.GZ cujos bytes gravados passam dessa quantidade de vezes os bytes comprimidos lidos até ali, depois de gravar mais de 1 MiB. |
| `timeoutMs` | `u64` | `600000` (10 minutos) | Desiste com `ERR_ARCHIVE_TIMEOUT` quando a operação demora mais, removendo o que criou. |

Os limites podem ser aumentados, mas nunca desativados: `0` falha com `ERR_ARCHIVE_INVALID_OPTION`.

## Segurança

- Todo nome de entrada é validado antes de qualquer coisa ser criada: caminhos absolutos, componentes `..`, prefixos de unidade, barras invertidas e nomes que alguma plataforma não consegue criar, como `CON` ou nomes terminados em ponto, são recusados.
- As gravações ficam dentro do destino mesmo quando ele já contém um link simbólico: a extração nunca atravessa um link.
- Links simbólicos, links físicos, dispositivos e outras entradas especiais são recusados em vez de recriados.
- Duas entradas que resultam no mesmo caminho, inclusive nomes que diferem só na caixa ou na normalização Unicode, são recusadas.
- Cada arquivo é gravado com um nome temporário e renomeado ao terminar, e em qualquer falha os arquivos e diretórios criados por esta chamada são removidos. O conteúdo que já existia no destino fica intocado.
- Arquivos extraídos nunca recebem bits de permissão especiais; só a permissão de execução do dono é mantida, para que programas extraídos continuem executáveis.
- Antes de extrair um arquivo ZIP, os tamanhos que ele declara, limitados a `maxBytes`, são comparados com o espaço livre no destino, e um arquivo que não cabe é recusado com `ERR_ARCHIVE_NO_SPACE` antes de qualquer arquivo ser criado. Além disso os tamanhos nunca são confiáveis: os limites contam os bytes gravados de fato, e um disco que enche durante a extração também rejeita com `ERR_ARCHIVE_NO_SPACE`.
- Arquivos compactados dentro do arquivo nunca são extraídos: uma entrada `.zip` ou `.tar.gz` é gravada como arquivo comum.
- `unpack` nunca executa, abre nem interpreta nada do que extrai.

## Tratamento de erro

::: danger error
<!-- @include: ../../../.fragments/modules/archive/unpack/error-handling.md -->
:::
