---
title: Registro automático de rotas
description: Descubra e registre plugins de rota em `src/routes/`.
---

# Registro automático de rotas

Em um projeto de API, o entrypoint do Tyrus descobre recursivamente os módulos de rota dentro de `src/routes/`. Cada módulo pode exportar uma ou mais funções com a assinatura `(router: HttpRouter) => void`; o Tyrus chama cada função com o router da aplicação quando `createRouter()` é executado, antes de o servidor iniciar a escuta. O export default continua compatível. Não é necessário criar `index.tyr` nem importar cada rota.

<!-- @include: ../../../.fragments/modules/http/automatic-routes.md -->

## Discovery rules

Um arquivo `.tyr` em `src/routes/` sem uma função exportada que corresponda à assinatura de rota é tratado como módulo auxiliar e não é chamado. Um export default nessa pasta precisa corresponder à assinatura de plugin de rota ou o projeto falha ao carregar. Arquivos terminados em `.test.tyr` ficam fora da descoberta.

Exports fora de `src/routes/` nunca são registrados automaticamente. Importe e chame esses módulos explicitamente quando necessário.
