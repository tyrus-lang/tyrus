---
title: CsrfOptions
description: Configure a proteção CSRF double-submit opt-in.
---

# CsrfOptions

`CsrfOptions` habilita uma verificação CSRF double-submit opt-in para um `HttpRouter`. `app.csrf(...)` compara um cookie nomeado a um header de requisição nomeado antes de ler o corpo ou chamar um handler para uma requisição insegura. `GET`, `HEAD`, `OPTIONS` e `TRACE` continuam legíveis sem token; todo outro método exige que os dois valores coincidam. Uma requisição recusada recebe `403` com o código estável `csrf_invalid` no corpo.

<SignatureBlock label="Assinatura">interface CsrfOptions { cookie: string; header: string; }</SignatureBlock>

<!-- @include: ../../../.fragments/modules/http/csrf-options/csrfoptions.md -->

A aplicação é dona da emissão do token, vínculo com a sessão, renovação e autorização. Gere um token imprevisível na fronteira de autenticação, coloque-o em um cookie seguro host-only e envie o mesmo token no header configurado a partir do código do navegador. O prefixo de cookie `__Host-` é um bom padrão porque navegadores exigem que ele seja `Secure`, tenha `Path=/` e omita `Domain`. Esta política de router deliberadamente não protege APIs bearer-token: não a habilite para rotas cuja autenticação não use cookies.

::: danger error
<!-- @include: ../../../.fragments/modules/http/csrf-options/csrfoptions-2.md -->
:::

Nomes de header devem ser nomes HTTP válidos, e nomes de cookie devem ser nomes de cookie válidos. Uma política malformada é recusada durante a configuração do servidor, antes que ele possa servir sem a proteção declarada no código.
