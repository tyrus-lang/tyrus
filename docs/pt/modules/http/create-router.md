---
title: createRouter
description: Crie um router HTTP por tyrus:http.
---

# createRouter

Para iniciar o servidor e configurar o listener, consulte [listen](./listen.md) e [ServerOptions](./server-options.md).

`createRouter` cria um router HTTP nativo. Rotas são declaradas com um objeto de rota: `response` mapeia cada status code para o tipo do payload, e `handler` recebe `(request, reply)`. O tipo da resposta pode ser uma interface declarada, um array como `User[]` ou um primitivo aceito, como `string`, `boolean` ou um tipo numérico. Uma rota com apenas a resposta padrão `200` pode usar o tipo diretamente; uma rota com vários status usa um mapa, como `{ 200: User[], 500: ApiError }`. Todo status que um handler envia precisa estar declarado em `response`, então `reply.status(...).send(...)` é validado contra o schema. Schemas de requisição e resposta aceitam campos escalares, interfaces aninhadas declaradas e arrays de ambos; campos de resposta podem ser opcionais e, nesse caso, um campo ausente é omitido do objeto JSON. Schemas de requisição continuam estritos e rejeitam propriedades que não são declaradas em qualquer nível do objeto.

Uma rota pode responder com texto em vez de um objeto JSON declarando `TextBody` como sua resposta, importado de `tyrus:http` como qualquer outro tipo que o módulo exporta. O `send` passa a receber uma `string`, e o corpo é essa string como foi escrita, com `content-type: text/plain; charset=utf-8` a menos que `reply.type(...)` diga outra coisa. Quem decide é a declaração, não o valor: uma rota `TextBody` recusa um objeto e uma rota de objeto recusa uma string, então o que a rota responde continua legível na rota e não na última linha do handler. `TextBody` não tem campos, porque não há o que descrever: quem dá forma ao texto é a aplicação.

Uma rota pode responder com bytes em vez de texto ou JSON declarando `BytesBody`, e pode lê-los do mesmo jeito com `body: BytesBody`. O `send` passa a receber um `u8[]`, o `request.body` é um `u8[]`, e os bytes cruzam o fio exatamente como são: sem codificação, sem caracteres de substituição para o que não é texto, e com `content-type: application/octet-stream` a menos que `reply.type(...)` nomeie o formato. O `body: TextBody` é o equivalente para texto, entregando ao handler o corpo como uma `string` em vez de um objeto. Os dois são declarados na rota e não inferidos de um content type que o remetente escolheu, e os dois são recusados em qualquer lugar que não seja `body` e `response`: eles descrevem um corpo, e `params`, `query`, `headers` e `cookies` são pares nome-valor sem uma única string de texto ou de bytes para ser. Uma rota que lê um corpo sem forma aceita qualquer content type, já que não há formato sobre o qual o remetente possa discordar.

Uma rota lê um corpo `multipart/form-data` declarando `body: MultipartBody`, e o `request.body` passa a ser um `MultipartPart[]`. Cada parte carrega `name`, os opcionais `fileName` e `contentType`, e `bytes`. Os opcionais são opcionais porque a ausência é um estado real: um campo de texto de um formulário não tem nome de arquivo, e o remetente pode não dizer o que a parte contém. O nome de arquivo é reduzido ao último segmento antes de o programa vê-lo, então um remetente não consegue fazê-lo escapar de um diretório que o programa escolheu. Um corpo que não é multipart, ou cujo content type está ausente, é recusado com `415`, já que o boundary vive nesse cabeçalho; um que não parseia é `400`; e um que traz mais partes, ou uma parte maior, do que a rota aceita é `413`. As partes ficam em memória e são limitadas pelo `bodyLimit` antes de um byte ser lido, então `MultipartBody` é um tipo só de requisição: nunca aparece num `response`.

<!-- @include: ../../../.fragments/modules/http/create-router/createrouter.md -->

<SignatureBlock label="Assinatura">createRouter(): HttpRouter</SignatureBlock>

## HttpRouter

`HttpRouter` é o valor nativo de router retornado por `createRouter()`. Ele registra rotas e expõe as funções de método HTTP (`get`, `post`, `put`, `patch`, `delete`, `head`, `options`, `connect`, `query` e `trace`), `fallback`, `cors` e `listen`. Uma requisição `HEAD` preserva o status e os headers da resposta, mas nunca transfere um corpo.

<!-- @include: ../../../.fragments/modules/http/create-router/httprouter.md -->
## Padrões de rota

O router suporta métodos HTTP padrão e parâmetros de path:

`router.query(path, options)` registra o método HTTP `QUERY`, que pode transportar critérios de busca estruturados no corpo da requisição. Os campos da query string da URL continuam disponíveis em `request.query`.

Para agrupar rotas em seções nomeadas no Swagger UI, passe `tags` no objeto de opções de cada rota, por exemplo `router.get("/users", { tags: ["user"], ... })`. Rotas sem tags ficam agrupadas em `default`. As mesmas tags são gravadas no documento OpenAPI. O documento gerado usa OpenAPI 3.2, então rotas `QUERY` aparecem como operações com o corpo de requisição declarado.

<!-- @include: ../../../.fragments/modules/http/create-router/route-patterns.md -->

## Canais da requisição

Um campo inesperado é recusado apenas no `body`, que é escrito por quem envia para esta rota. Os outros canais sempre carregam mais do que um schema nomeia: toda requisição tem `host`, `user-agent` e `accept`, e um navegador adiciona cookies próprios, então `params`, `query`, `headers` e `cookies` leem os campos que declaram e ignoram o resto.

Uma rota que declara `body` lê JSON, então uma requisição cujo `Content-Type` é outro media type é recusada com `415` e um header `Accept: application/json`. Um tipo de fornecedor que ainda carrega JSON, como `application/vnd.api+json`, é lido. Uma requisição que não declara content type é lida como antes, e uma rota sem schema de `body` não inspeciona o media type.

Uma requisição cujo corpo não pode ser delimitado é recusada antes que esse corpo seja lido. Carregar `content-length` e `transfer-encoding` ao mesmo tempo dá à mensagem dois fins possíveis, e cada hop entre o cliente e o servidor é livre para acreditar num deles, que é o que o request smuggling explora; uma requisição assim é respondida com `400` e `ambiguous_framing`, em vez de resolvida por adivinhação. Um corpo delimitado mas ilegível, como dados chunked que não são analisáveis, é `400` e `malformed_body`: só um corpo que passou do limite de tamanho é `413`.

Testar um campo prova que ele está presente no resto daquele ramo, então `if (request.body.nickname)` o lê como `string` onde o schema declara `string?`. Isso vale para qualquer caminho sobre um binding imutável: uma raiz mutável poderia ser reatribuída entre o teste e a leitura, então lá o campo continua opcional.

Um campo de schema de requisição pode ser opcional. `nickname?: string` é lido como três resultados em vez de dois: presente, ausente e errado. Um campo ausente produz o valor vazio, que `??` e `?.` leem normalmente, enquanto um campo de tipo errado continua sendo `400`. Campos opcionais de requisição hoje guardam escalares, porque a ausência é distinguida pelo leitor de valor e os leitores de objeto aninhado e de array ainda não têm equivalente; um schema de resposta pode omitir um campo de qualquer tipo suportado, já que escrever um só precisa deixá-lo de fora. Estreitar um campo opcional entrega o valor que ele guarda, com tudo o que esse valor tem: dentro de `if (request.query.tag)` o campo é uma `string`, então `request.query.tag.length` e `request.query.tag.toUpperCase()` são lidos como em qualquer outra string.

Um campo de schema pode ser um enum. O valor que atravessa a rede é o valor que o membro declara: `Role.Admin = "admin"` é a string JSON `"admin"`, e `Level.High = 9` é o número `9`. Nada mapeia um número para o nome do membro na saída, porque aí o valor no programa e o valor na rede seriam duas coisas diferentes e só uma delas seria a que um `switch` compara. A leitura é onde isso se paga: um valor do tipo certo que não é um membro declarado é recusado com `400`, exatamente como um valor de tipo errado. Sem essa checagem a rede poderia entregar ao programa um `Level` contendo `5`, que não é membro nenhum que o programa declarou, e todo `switch` sobre ele cairia fora. Um enum cujos membros carregam payload não é aceito, porque não tem um valor único para ser; a recusa nomeia o campo e diz o motivo.

Um campo de schema de resposta também pode ser uma união discriminada: toda variante um objeto, com um campo compartilhado carregando um literal distinto em cada uma. Ela sai como o objeto que a variante viva já é, então `{ kind: "done"; value: i32 } | { kind: "failed"; reason: string }` escreve `{"kind": "done","value": 42}` e mais nada. Não há envelope, porque o campo discriminante é o que diz ao leitor qual variante ele recebeu. Um schema de requisição a lê de volta do mesmo jeito, escolhendo a variante pelo discriminante. Um corpo cujo discriminante não nomeia nenhuma variante declarada é recusado com `400`, exatamente como um valor de tipo errado: o fio não pode entregar ao programa uma variante que ele nunca declarou.

Somente `body` é JSON escrito por quem envia. Path params, query, headers e cookies carregam texto: `?limit=25` são dois caracteres na rede, e `/users/123` não diz se `123` é um identificador ou uma contagem. O schema declarado decide como esse texto é lido, então `id: string` aceita `/users/123` e `limit: i32` lê `?limit=25` como número. Um valor que o schema não consegue ler é rejeitado com `400`, nomeando o canal que falhou, como `Invalid request query`. Um corpo JSON permanece estrito: `{"limit": "25"}` não satisfaz `limit: i32`.

<!-- @include: ../../../.fragments/modules/http/request-channels.md -->

## Requisições rejeitadas

Uma requisição que não se encaixa nos seus schemas é respondida com `400` e um corpo que carrega `statusCode`, `code`, `message` e `issues`. A mensagem nomeia o canal que falhou, e cada issue nomeia o `field`, o `validator` que a rejeitou e o motivo. Uma requisição rejeitada antes de qualquer decoration rodar, como uma cujo corpo não é JSON, tem `issues` vazio.

<!-- @include: ../../../.fragments/modules/http/validation-issues.md -->

## Handlers em outro módulo

Um controller importado pode deixar sem tipo os dois parâmetros do handler. A rota fornece o formato de `req` pelos schemas da requisição e o tipo de `res` pelos status de resposta declarados, então o controller não precisa importar esses tipos de schema nem repetir a união de respostas. O controller ainda pode importar valores que usa, como `HttpStatus`, e anotações explícitas de parâmetros continuam aceitas. Um controller inferido tem um único contrato de rota; use controllers separados ou uma assinatura compartilhada explícita quando as rotas tiverem schemas diferentes.

<!-- @include: ../../../.fragments/modules/http/named-handler.md -->

## Endereço do cliente e proxies confiáveis

O rate limit de uma rota é chaveado pelo endereço do cliente, e esse endereço é o peer da conexão. `app.trustedProxies([...], identityHeader?)` lista os peers cujos headers de encaminhamento o servidor acredita, como endereços (`10.0.0.1`) ou redes em formato CIDR (`10.0.0.0/8`).

<!-- @include: ../../../.fragments/modules/http/trusted-proxies.md -->

Atrás de um proxy listado o servidor lê os headers de encaminhamento no lugar: ele percorre cada cadeia da direita para a esquerda e pula hops que também são confiáveis, então o primeiro endereço que não é será o mais próximo pelo qual a cadeia confiável respondeu. `Forwarded` e `X-Forwarded-For` são lidos os dois, e os dois precisam nomear o mesmo cliente. Um proxy normalmente escreve um deles e não remove o outro, então deixar qualquer um decidir sozinho permitiria que um chamador enviasse o header sobrando e escolhesse o próprio endereço, derrotando o limite, o ban e o teto compartilhado de uma vez. Quando os dois divergem a requisição conta contra o peer, o único endereço que o servidor observou por conta própria. Passe `identityHeader` quando o proxy usa outro header de endereço, como `"x-client-ip"`; ele substitui os headers padrão, passa a ser o único lido e aceita a mesma cadeia de endereços separada por vírgulas. Ele ainda só é lido depois que o peer corresponde à lista confiável, e um endereço malformado volta para o peer em vez de virar uma identidade nova. Vindos de qualquer outro peer, esses headers são ignorados, porque um cliente pode escrevê-los, e acreditar neles deixaria um chamador apresentar um endereço novo por requisição e nunca alcançar um limite. Uma entrada malformada rejeita a lista inteira em vez de descartar uma rede em silêncio, e o servidor para em vez de subir com a política errada.

## Ciclo de vida

`app.listen(port, options?)` retorna `Result<HttpServer, HttpError>` depois de vincular a porta. Trate uma falha de inicialização com `onfail`; a variável do bloco contém o valor `HttpError`. Em uma função `void`, o bloco de falha retorna dessa função, enquanto uma chamada bem-sucedida fornece um handle `HttpServer`.

<!-- @include: ../../../.fragments/modules/http/listen-example.md -->

`server.port()` informa a porta realmente vinculada, inclusive quando a porta solicitada foi `0`. `server.close()` para de aceitar novas conexões e se resolve depois que as requisições já em andamento terminam. O runtime mantém um listener ativo depois que `main` retorna e aguarda SIGINT ou SIGTERM; o sinal fecha os servidores ativos e conclui as requisições, limitado por `shutdownTimeout`. Um segundo sinal encerra o processo imediatamente.

`shutdownTimeout` limita quanto tempo `close` espera. Passado o prazo, as conexões ainda abertas são encerradas. Sem `shutdownTimeout`, a espera não tem limite. Um fechamento que encerra requisições em andamento informa `ERR_HTTP_SHUTDOWN_TIMEOUT`; fechar um handle inválido informa `ERR_HTTP_SHUTDOWN`.

<!-- @include: ../../../.fragments/modules/http/create-router/error-handling.md -->

## Configuração de rota

As opções de rota podem incluir `config`, um objeto tipado de ajustes por rota. Tanto `config` quanto cada uma de suas chaves são opcionais, mas uma chave presente precisa estar completa.

<!-- @include: ../../../.fragments/modules/http/route-config.md -->

`bodyLimit` é um tamanho em bytes escrito como string que substitui o padrão do servidor, de 2 MB. Unidades decimais (`kb`, `mb`, `gb`) são potências de 1000 e unidades binárias (`kib`, `mib`, `gib`) são potências de 1024, e `"off"` desativa o limite por completo, o que aceita qualquer tamanho de corpo e é uma troca por negação de serviço a se fazer apenas numa rota interna confiável. Um corpo acima do limite efetivo é recusado com `413`.

`rateLimit` declara `max`, uma contagem inteira de requisições por identidade de cliente, e `timeWindow`, uma string de duração como `"1m"`, `"30s"`, `"500ms"` ou a forma verbosa `"10 seconds"`; as duas são obrigatórias quando `rateLimit` está presente. `globalMax` é um teto inteiro opcional compartilhado por todas as identidades que usam esse limite. `ban` é uma contagem opcional não negativa de requisições consecutivas que excedem o limite da própria identidade; depois dessa contagem, a identidade recebe `403` com `rate_limit_banned` até `banWindow` expirar. `ban: 0` bane na primeira requisição excedida, e `banWindow` assume `timeWindow` como padrão. Esgotar `globalMax` nunca adiciona uma ocorrência de banimento, então um servidor ocupado não pode banir um cliente que se comporta corretamente. O limite é um token bucket por identidade de cliente, aplicado antes da leitura do corpo. Ele começa com `max` tokens e recarrega continuamente ao longo de `timeWindow`, então uma rajada não pode usar duas janelas completas em sua fronteira. Quando `globalMax` está presente, um segundo token bucket começa com essa quantidade de tokens e é compartilhado por todas as identidades; uma requisição só prossegue quando os dois buckets têm crédito. Um balde esgotado responde `429` com `Retry-After`. A tabela de baldes de identidade é limitada, e uma tabela cheia descarta as identidades usadas há mais tempo em vez de recusar um cliente que nunca viu, então enchê-la a partir de muitos endereços não transforma o limitador na negação de serviço que ele existe para evitar. Uma identidade cumprindo um ban é mantida qualquer que seja sua idade: uma requisição banida é recusada antes de encostar no relógio, então ela é o registro mais parado da tabela e seria a primeira a sair, encerrando o ban exatamente quando chega a pressão que o motivou.

O `rateLimit` de uma rota substitui o `app.rateLimit(...)` para aquela rota, em vez de somar-se a ele, então uma rota pode ser tão mais frouxa quanto mais estrita que o padrão do servidor. Definir o limite no router é o lugar usual dele: um limite que precisa ser repetido em toda rota é um limite que alguma rota vai acabar sem.

Os formatos de `bodyLimit` e `timeWindow` são validados em tempo de compilação, então são escritos inline.

## CORS

`app.cors(options)` instala o middleware de CORS sobre o router inteiro e trata as requisições de preflight `OPTIONS` antes do despacho de rota. `origin`, `allowedHeaders` e `exposedHeaders` são arrays de string; `methods` é restrito aos literais de método HTTP em maiúsculas, então um erro de digitação é erro de tipo. `allowedHeaders` continua sendo um array de string comum de propósito, porque a aplicação pode autorizar um header que ela mesma inventou. `maxAge` é em segundos, e `0` deixa a opção sem valor.

## CSRF

`app.csrf({ cookie, header })` habilita uma verificação double-submit opt-in para rotas autenticadas por cookie. Antes de uma requisição insegura alcançar um handler, o runtime compara o cookie e o header configurados em tempo constante; um valor ausente ou diferente retorna `403` com `csrf_invalid`. Geração do token e vínculo com a sessão continuam sendo política da aplicação. Veja [CsrfOptions](./csrf-options.md).

`"*"` é aceito em `origin` e `allowedHeaders`, mas não ao lado de `credentials: true`. O navegador recusa esse par por inteiro em vez de estreitá-lo, então toda requisição com credencial falharia; o compilador aponta o `"*"` que causou isso.

::: danger error
<!-- @include: ../../../.fragments/modules/http/cors-credentials.md -->
:::

As origens são verificadas enquanto ainda são literais. O navegador envia `Origin: <esquema>://<host>[:<porta>]` e nada mais, então uma entrada escrita com caminho, query, barra final ou sem esquema é comparada com um valor que nunca pode conter isso, e silenciosamente não casa com nada. Essa falha é invisível em tempo de execução, e é por isso que aqui ela é erro de compilação. Uma entrada que carrega um caminho ganha um Quick Fix que o corta; a falta de esquema não ganha, porque `http` e `https` são origens diferentes e um chute produziria uma regra que parece certa e não casa com nada.

Uma lista `origin` vazia também é recusada: ela não casa com origem nenhuma, o que recusa toda requisição cross-origin, e um router que quisesse isso simplesmente não chamaria `cors`.

As mesmas regras são aplicadas de novo na subida, para valores que não são literais. Os métodos de registro de rotas `get`, `post`, `put`, `patch`, `delete`, `head`, `options`, `connect`, `query`, `trace`, `fallback`, `ws` e `docs`, além de `onResponse`, `cors`, `csrf`, `fetchPolicy`, `rateLimit` e `trustedProxies`, retornam `Result<void, HttpError>` quando o runtime recusa uma rota ou configuração. A inicialização pode propagar essas falhas com `?` ou tratá-las com `match` antes de começar a escutar.

## Requisições de saída

`app.fetchPolicy(options)` diz para onde o `fetch` pode ir. Nada é recusado enquanto o programa não escrever isso: falar com `10.0.0.5` ou com `localhost` é comum, e um runtime que recusasse por padrão ensinaria o autor a desligar a proteção inteira.

<!-- @include: ../../../.fragments/modules/http/fetch-policy.md -->

`denyPrivate` recusa destinos alcançáveis apenas de dentro: loopback, as faixas privadas, link-local e o resto do espaço em que nenhum host da internet pública responde. `allow` é uma lista de exceções, não uma lista de permissões. Ela nomeia hosts que podem ser alcançados mesmo que o endereço deles fosse recusado, então acrescentar uma entrada só amplia o que o programa alcança, nunca restringe.

A política existe para uma forma específica de programa: uma rota que entrega ao `fetch` uma URL escolhida pelo cliente. A requisição sai de dentro da rede e por isso alcança o que a rede estava protegendo. `169.254.169.254` é o serviço de metadados da nuvem, que responde credenciais a quem conseguir alcançá-lo e trata a alcançabilidade como autenticação; um cache ou um painel administrativo num endereço privado está protegido do mesmo jeito.

Um destino é julgado pelo endereço para o qual resolve, não pela grafia. Verificar o nome do host não seria verificação nenhuma: nada impede um nome público de resolver para um endereço privado, e uma política que confiasse na grafia aprovaria o serviço de metadados no instante em que alguém apontasse um domínio para ele. Pelo mesmo motivo, cada salto de um redirecionamento é verificado como o primeiro, porque um redirecionamento nomeia um destino que o programa nunca escreveu.

Um destino recusado rejeita a promise com `ERR_HTTP_FETCH_BLOCKED`, que é um código próprio e não o mesmo de um nome que não resolve: um chamador informado de que o nome do host está errado iria procurar no lugar errado por algo que a política dele mesmo decidiu.

## Headers de segurança

Headers de segurança são escritos como qualquer outro header de resposta, com `reply.header(name, value)`. Não existe um conjunto pronto que os adicione: o que o navegador recebe é o que o handler escreveu, e o editor oferece os quatro pelo nome, com seus valores, dentro de `reply.header("`.

<!-- @include: ../../../.fragments/modules/http/security-headers.md -->

<!-- @include: ../../../.fragments/modules/http/security-headers-table.md -->

Cada um é uma instrução estreita para o navegador, e um cliente que a ignora não é afetado.

O editor também oferece `content-security-policy` e `strict-transport-security`, e deixa o valor dos dois para a aplicação. Uma política genérica o bastante para toda aplicação ou é tão frouxa que não protege nada ou é tão estrita que quebra a página, então o que ela deve dizer é decisão da aplicação. `strict-transport-security` é uma promessa sobre TLS: envie-a apenas por trás de TLS, porque sobre HTTP puro ela pede ao navegador que recuse a conexão que funciona.

Nada disso torna uma aplicação segura contra cross-site scripting. Esses headers reduzem o que o navegador faz com uma resposta; não dizem nada sobre o que a aplicação coloca dentro dela.

Uma resposta que o próprio router produziu, como um `404` para um caminho sem rota, não passou por handler nenhum e por isso não carrega nenhum deles.

## Respostas que não batem com o schema

Quando um handler produz uma resposta que não satisfaz o schema declarado pela rota, o cliente recebe um `500` genérico com `code: "internal_server_error"` e mais nada. O cliente não causou a falha e não pode agir sobre ela, e os nomes de campo envolvidos descrevem o schema do próprio servidor, não algo que o cliente enviou.

O campo e a regra que falharam vão para o stderr, onde quem opera e pode corrigir vai ver. Isso é deliberadamente diferente do contrato do `400`: uma requisição recusada nomeia o campo e a regra no corpo, porque ali a falha é do cliente e é ele quem corrige.

## Pré-handlers

As opções de rota podem incluir `preHandler`, um array literal de funções síncronas nomeadas. Importe `HttpRequest` e `HttpResponse` ao declarar um pre-handler. Ele recebe o `HttpRequest` bruto e retorna `HttpResponse | true`: `true` continua para o próximo pre-handler ou controller, enquanto um `HttpResponse` interrompe a cadeia e é enviado imediatamente. A cadeia é fixa em tempo de compilação e está disponível tanto nas rotas quanto em `fallback`.

Para geração OpenAPI e configuração do Swagger UI, incluindo autenticação, estilos personalizados e proteção da documentação, consulte [OpenAPI / Swagger](./openapi-swagger.md).

## Tratamento de erros

Declare o corpo de erro na aplicação e mapeie um erro de domínio na fronteira do controller para o status HTTP escolhido. O status HTTP já está disponível ao cliente como status da resposta e não precisa ser duplicado no JSON.

Um handler não pode deixar um erro escapar: o servidor o chama sem nenhum `catch` em volta, então um handler que lança, inline ou referenciado pelo nome, é recusado com `E3298`. Capture o erro dentro do handler e responda com o status que ele representa.

<!-- @include: ../../../.fragments/modules/http/create-router/error-handling.md -->
