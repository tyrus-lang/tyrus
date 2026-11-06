---
title: ServerOptions
description: Configure associação, limites, encerramento, logging e TLS do servidor HTTP.
---

# ServerOptions

`ServerOptions` configura o listener passado para `app.listen(...)`. Passe as opções como objeto, diretamente na chamada ou por meio de um valor tipado como `ServerOptions`.

<SignatureBlock label="Assinatura">interface ServerOptions</SignatureBlock>

## Configurações

| Configuração | Finalidade |
|---------|---------|
| `host` | Endereço IP para associação. O padrão é todas as interfaces. |
| `requestTimeout` | Tempo máximo de uma requisição, incluindo corpo e handler. Requisições que excedem o limite recebem `503`. |
| `maxConcurrentRequests` | Número máximo de requisições em execução ao mesmo tempo. As demais aguardam uma vaga e continuam limitadas por `requestTimeout`. |
| `maxConnections` | Número máximo de conexões abertas. Vem sem limite por padrão; ao atingir o valor, o servidor para de aceitar conexões. |
| `shutdownTimeout` | Tempo máximo para aguardar requisições ativas durante o encerramento. |
| `logger` | Com `true`, escreve um evento estruturado de acesso por resposta concluída em stderr; o padrão é `false`. |
| `headerTimeout` | Tempo máximo para receber os cabeçalhos da requisição. O padrão é 10 segundos e não pode ser desativado. |
| `maxHeaderSize` | Tamanho máximo combinado da linha da requisição e dos cabeçalhos. O padrão é 64 KiB e o mínimo é 8 KiB. |
| `keepAlive` | Permite que uma conexão carregue várias requisições. Vem ativado por padrão. |
| `tls` | Serve HTTPS com o certificado e a chave privada configurados. Ambos são obrigatórios. |

`rateLimit` é configurado no roteador com `app.rateLimit(...)`, e não em `ServerOptions`. Os limites de cabeçalhos se aplicam antes da execução de uma rota, enquanto `requestTimeout` cobre a requisição depois da chegada dos cabeçalhos.

## Exemplo

<!-- @include: ../../../.fragments/modules/http/server-options-example.md -->
