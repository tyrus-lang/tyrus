---
title: Documentação em Português Brasileiro
description: Documentação da linguagem Tyrus em português brasileiro.
layout: home
hero:
  text: '<span class="hero-line">Sintaxe TypeScript</span><span class="hero-line">Semântica Rust</span><span class="hero-subtitle">Uma linguagem de programação feita para devs</span>'
  image:
    src: /tyrus-logo.png
    alt: Logotipo do Tyrus
  actions:
    - theme: brand
      text: Getting Started
      link: /pt/docs/getting-started
    - theme: alt
      text: Playground
      link: https://playground.tyruslang.org
    - theme: alt
      text: Explore Modules
      link: /pt/modules/
features:
  - title: Binários nativos
    details: Compilação nativa de arquivos .tyr com LLVM e runtime integrado ao executável.
  - title: Segurança por default
    details: Ownership, borrowing, Non-Lexical Lifetimes, limites numéricos, validadores e exaustividade antes de produção.
  - title: Tooling enterprise para APIs
    details: Scaffolding, formatter, linter, LSP, REPL, web playground, ambientes selados e 20 módulos nativos juntos.
---

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">Linguagem backend compilada</p>
<h2>Escreva código familiar em .tyr</h2>
<p>Tyrus usa sintaxe concisa em arquivos <code>.tyr</code> e adiciona execução nativa, semântica de memória determinística com NLL, diagnósticos precisos e tooling focado em backend.</p>

</div>
<div>

<!-- @include: ../.fragments/home/http-controller.md -->

</div>
</section>

<section class="tyrus-home-section">
<p class="tyrus-home-eyebrow">Superpoderes da linguagem</p>
<div class="tyrus-home-glow-grid">
<GlowCard size="md">
<template #icon><svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><g fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"/><path d="m9 12l2 2l4-4"/></g></svg></template>
<template #title>Validator decorations</template>
<template #description>Regras embutidas como @email, @minLength e @min, além de validações customizadas com validator function. Validação declarativa sem boilerplate em runtime.</template>
</GlowCard>
<GlowCard size="md">
<template #icon><svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 2L3 14h9l-1 8l10-12h-9l1-8z"/></svg></template>
<template #title>Comptime em compilação</template>
<template #description>Execute lógica pura durante a compilação com comptime(...). Pré-compute tabelas de lookup, constantes e configurações com zero custo em runtime.</template>
</GlowCard>
<GlowCard size="md">
<template #icon><svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><g fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><rect width="18" height="11" x="3" y="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/></g></svg></template>
<template #title>Env selado e espelho</template>
<template #description>Criptografe o .env em target/&lt;profile&gt;/.env.sealed com impressões digitais binárias. O espelho .env.example é sincronizado automaticamente a cada mudança.</template>
</GlowCard>
<GlowCard size="md">
<template #icon><svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><g fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><path d="M9.671 4.136a2.34 2.34 0 0 1 4.659 0a2.34 2.34 0 0 0 3.319 1.915a2.34 2.34 0 0 1 2.33 4.033a2.34 2.34 0 0 0 0 3.831a2.34 2.34 0 0 1-2.33 4.033a2.34 2.34 0 0 0-3.319 1.915a2.34 2.34 0 0 1-4.659 0a2.34 2.34 0 0 0-3.32-1.915a2.34 2.34 0 0 1-2.33-4.033a2.34 2.34 0 0 0 0-3.831A2.34 2.34 0 0 1 6.35 6.051a2.34 2.34 0 0 0 3.319-1.915"/><circle cx="12" cy="12" r="3"/></g></svg></template>
<template #title>Tipos</template>
<template #description>Inteiros e floats u8..u128, i8..i128, f32..f128, decimal, boolean, string, char e tipos Result&lt;T, E&gt; nativos.</template>
</GlowCard>
</div>
</section>

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">Segurança determinística de memória</p>
<h2>Ownership, borrowing e NLL.</h2>
<p>Segurança de memória sem garbage collector. Tipos que não são Copy movem por atribuição, enquanto empréstimos são validados em compilação. Com Non-Lexical Lifetimes (NLL), os empréstimos expiram após o último uso real, liberando variáveis para mutação imediata sem esperar o término do bloco.</p>
</div>
<div>

<!-- @include: ../.fragments/home/ownership-nll.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split tyrus-home-split--reversed">
<div>
<p class="tyrus-home-eyebrow">Limpeza por escopo</p>
<h2>Execute a limpeza sempre ou somente em erros.</h2>
<p><code>defer</code> executa quando o escopo termina. <code>errdefer</code> executa apenas quando um retorno opcional ou <code>Result</code> sai com erro, deixando o rollback explícito.</p>
<p><a href="/pt/language/control-flow#defer-and-errdefer">Saiba mais sobre limpeza adiada</a></p>
</div>
<div>

<!-- @include: ../.fragments/home/defer-errdefer.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split tyrus-home-split--reversed">
<div>
<p class="tyrus-home-eyebrow">Pattern matching exaustivo</p>
<h2>Combine qualquer forma. Estreite cada branch.</h2>
<p>Unions marcadas, variantes de enums algébricos e <code>match</code> com guard clauses dão controle de fluxo exaustivo. O compilador garante que toda variante é tratada e estreita os tipos em cada ramo.</p>
</div>
<div>

<!-- @include: ../.fragments/home/pattern-matching.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">Sem null. Sem undefined. Só <code>T?</code></p>
<h2>Ausência é um estado, não um valor.</h2>
<p>Um <code>return;</code> vazio produz ausência. <code>?.</code> e <code>??</code> propagam de forma segura. O compilador força você a tratar o caso ausente antes que o valor chegue em produção, sem surpresas em runtime.</p>
</div>
<div>

<!-- @include: ../.fragments/home/optionals.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split tyrus-home-split--reversed">
<div>
<p class="tyrus-home-eyebrow">Sucesso e falha explícitos</p>
<h2>Represente erros recuperáveis com <code>Result</code>.</h2>
<p><code>Result&lt;T, E&gt;</code> contém um valor <code>Ok</code> ou um valor <code>Err</code>. Combine as duas variantes ou propague o erro com <code>?</code>.</p>
<p><a href="/pt/language/types#result-values">Saiba mais sobre Result</a></p>
</div>
<div>

<!-- @include: ../.fragments/home/result.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">Tratamento de erros no ponto de chamada</p>
<h2>Mantenha o caminho de sucesso claro com <code>onfail</code>.</h2>
<p>Anexe um handler de erro ao inicializador de um <code>Result</code>. O handler recebe o erro, enquanto a variável contém o valor de sucesso quando a execução continua.</p>
<p><a href="/pt/language/types#inline-failure-handling">Saiba mais sobre onfail</a></p>
</div>
<div>

<!-- @include: ../.fragments/home/onfail.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split tyrus-home-split--reversed">
<div>
<p class="tyrus-home-eyebrow">Operador pipe</p>
<h2>Dados fluem de cima para baixo.</h2>
<p>O operador <code>|&gt;</code> encadeia valores em cadeias de função. O token <code>%</code> permite posicionar o valor pipeado exatamente onde ele deve ficar, sem variáveis intermediárias, sem aninhamento.</p>
</div>
<div>

<!-- @include: ../.fragments/home/pipe-operator.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">Código assíncrono nativo</p>
<h2>Escreva funções async e aguarde seus resultados.</h2>
<p>Funções <code>async</code> retornam <code>Promise&lt;T&gt;</code>. Use <code>await</code> para ler o valor quando a operação assíncrona terminar.</p>
<p><a href="/pt/language/async-await">Saiba mais sobre async e await</a></p>
</div>
<div>

<!-- @include: ../.fragments/home/async-await.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">Kernels numéricos de alta performance</p>
<h2>Métodos algébricos de ponto flutuante.</h2>
<p>Os primitivos <code>f32</code> e <code>f64</code> fornecem <code>algebraicAdd</code>, <code>algebraicSub</code>, <code>algebraicMul</code>, <code>algebraicDiv</code> e <code>algebraicRem</code>. Eles concedem permissão explícita ao compilador para reassociar cálculos de ponto flutuante para vetorização SIMD e aceleração matemática em hardware.</p>
</div>
<div>

<!-- @include: ../.fragments/home/algebraic-float.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split tyrus-home-split--reversed">
<div>
<p class="tyrus-home-eyebrow">Proteção de segredos em produção</p>
<h2>Env selado e .env.example sincronizado.</h2>
<p>A configuração <code>security.envSealed</code> criptografa o <code>.env</code> em <code>target/&lt;profile&gt;/.env.sealed</code> com chaves assimétricas e insere uma impressão digital no binário para que contêineres nunca exponham segredos em texto puro. Em desenvolvimento, o compilador espelha chaves e comentários em <code>.env.example</code> com valores limpos automaticamente.</p>
</div>
<div>

<!-- @include: ../.fragments/home/sealed-env.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">ORM e ODM tipados</p>
<h2>Use uma API de Model com SQL e MongoDB.</h2>
<p>Defina Models como interfaces tipadas e consulte-os de forma assíncrona. A mesma API de Model funciona em bancos relacionais e MongoDB, com falhas recuperáveis retornadas em <code>Result</code>.</p>
<p><a href="/pt/modules/db/">Explore o módulo de banco de dados</a></p>
</div>
<div>

<!-- @include: ../.fragments/home/orm-odm.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">Avaliação em tempo de compilação</p>
<h2>Execute código durante a compilação. Custo zero em runtime.</h2>
<p>Funções declaradas com <code>comptime</code> executam durante a compilação e incorporam seu resultado como escalares ou literais estáticos diretamente no binário. Calcule tabelas de lookup, constantes matemáticas e configurações com tipagem completa.</p>
</div>
<div>

<!-- @include: ../.fragments/home/comptime.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split tyrus-home-split--reversed">
<div>
<p class="tyrus-home-eyebrow">Exploração interativa</p>
<h2>Experimente no terminal. Execute no navegador.</h2>
<p>Explore o Tyrus instantaneamente sem configurar projetos. O <code>tyrus repl</code> fornece um console interativo com avaliação JIT via LLVM em processo, edição multilinha, persistência de sessões e inspeção com <code>:type</code>. Prefere o navegador? Abra o <strong>Web Playground</strong> em <a href="https://playground.tyruslang.org" target="_blank" rel="noreferrer">playground.tyruslang.org</a> para escrever, compilar para WebAssembly e rodar código direto no navegador com links compartilháveis.</p>
</div>
<div>

<!-- @include: ../.fragments/home/repl-session.md -->

</div>
</section>

<section class="tyrus-home-section">
<p class="tyrus-home-eyebrow">Uma toolchain de linguagem, não só um parser</p>
<div class="tyrus-home-glow-grid">
<GlowCard size="md">
<template #icon><svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="m4 7l8-4l8 4v10l-8 4l-8-4zm8 14V11m8-4l-8 4l-8-4"/></svg></template>
<template #title>REPL interativo e CLI</template>
<template #description>tyrus repl para avaliação JIT via LLVM em processo, tyrus dev para live reload e tyrus fmt para formatação sem configuração.</template>
</GlowCard>
<GlowCard size="md">
<template #icon><svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="m21.73 18l-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3M12 9v4m0 4h.01"/></svg></template>
<template #title>Diagnósticos e tyrus explain</template>
<template #description>Códigos de erro estáveis com tyrus explain, ranges precisos, mensagens de ajuda, exemplos de falha e Quick Fixes no editor.</template>
</GlowCard>
<GlowCard size="md">
<template #icon><svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497zM15 5l4 4"/></svg></template>
<template #title>Editores com suporte a LSP</template>
<template #description>Completion, hover, signature help, inlay hints, semantic tokens e code actions para VS Code, Neovim, Zed, IntelliJ e Kate.</template>
</GlowCard>
<GlowCard size="md">
<template #icon><svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14 2v6a2 2 0 0 0 .245.96l5.51 10.08A2 2 0 0 1 18 22H6a2 2 0 0 1-1.755-2.96l5.51-10.08A2 2 0 0 0 10 8V2M6.453 15h11.094M8.5 2h7"/></svg></template>
<template #title>Web Playground e testes</template>
<template #description>Execute no navegador via WebAssembly em playground.tyruslang.org e rode suítes locais com tyrus test para arquivos *.test.tyr.</template>
</GlowCard>
</div>
</section>

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">Formatação de código</p>
<h2>Tyrus Formatter</h2>
<p><code>tyrus fmt</code> formata arquivos `.tyr` de maneira consistente e aplica as opções do <code>tyrus-fmt.json</code> do projeto.</p>
<p><a href="/pt/tooling/formatter">Leia o guia do Formatter</a></p>
</div>
<div>

<!-- @include: ../.fragments/home/formatter-config.md -->

</div>
</section>

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">Análise de código</p>
<h2>Tyrus Lint</h2>
<p><code>tyrus lint</code> aponta problemas de estilo, correção, segurança, desempenho e arquitetura. Defina a severidade de cada regra em <code>tyrus-lint.json</code> no projeto.</p>
<p><a href="/pt/tooling/linter">Leia o guia do Lint</a></p>
</div>
<div>

<!-- @include: ../.fragments/home/lint-config.md -->

</div>
</section>

<section class="tyrus-home-section">
<p class="tyrus-home-eyebrow">Baterias incluídas</p>
<h2>Módulos nativos integrados.</h2>
<p>Tudo o que é necessário para construir serviços backend resilientes, ferramentas de CLI e software de sistemas já faz parte do runtime do Tyrus.</p>
<div class="tyrus-home-grid">
<a class="tyrus-home-card" href="/modules/algorithm/">
<strong>tyrus:algorithm</strong>
<span>Mais de 50 algoritmos e estruturas de dados integrados: Dijkstra, A*, Trie e árvore rubro-negra.</span>
</a>
<a class="tyrus-home-card" href="/modules/archive/">
<strong>tyrus:archive</strong>
<span>Criação, inspeção e extração de arquivos Tar e Zip.</span>
</a>
<a class="tyrus-home-card" href="/modules/cli/">
<strong>tyrus:cli</strong>
<span>Prompts interativos no terminal: select, multiselect, password, confirm e input.</span>
</a>
<a class="tyrus-home-card" href="/modules/compress/">
<strong>tyrus:compress</strong>
<span>Compressão e descompressão em Zstandard, Brotli, Gzip e Deflate.</span>
</a>
<a class="tyrus-home-card" href="/modules/crypto/">
<strong>tyrus:crypto</strong>
<span>Argon2, SHA-256, SHA-512, AES-GCM, HMAC, JWT, UUID v7 e CSPRNG.</span>
</a>
<a class="tyrus-home-card" href="/modules/db/">
<strong>tyrus:db</strong>
<span>Cliente de banco de dados e ORM integrados para PostgreSQL, MySQL, SQLite e MongoDB.</span>
</a>
<a class="tyrus-home-card" href="/modules/encoding/">
<strong>tyrus:encoding</strong>
<span>Codificadores nativos para Hex, Base64, URL, Varint, CSV e JSON.</span>
</a>
<a class="tyrus-home-card" href="/modules/fs/">
<strong>tyrus:fs</strong>
<span>Sistema de arquivos síncrono e assíncrono, gravação atômica, streaming, glob e monitoramento de arquivos.</span>
</a>
<a class="tyrus-home-card" href="/modules/http/">
<strong>tyrus:http</strong>
<span>Servidor HTTP/1.1 e HTTP/2, rotas automáticas, Swagger e WebSocket.</span>
</a>
<a class="tyrus-home-card" href="/modules/math/">
<strong>tyrus:math</strong>
<span>Constantes, trigonometria, teoria dos números, geometria e combinatória.</span>
</a>
<a class="tyrus-home-card" href="/modules/memory/">
<strong>tyrus:memory</strong>
<span>Operações determinísticas de memória: sizeOf, alignOf, zeroize, swap e replace.</span>
</a>
<a class="tyrus-home-card" href="/modules/net/">
<strong>tyrus:net</strong>
<span>Sockets TCP e UDP, resolução DNS, busca reversa, ping e traceroute.</span>
</a>
<a class="tyrus-home-card" href="/modules/os/">
<strong>tyrus:os</strong>
<span>Metadados do sistema, núcleos de CPU, estatísticas de memória, temperaturas e notificações nativas.</span>
</a>
<a class="tyrus-home-card" href="/modules/process/">
<strong>tyrus:process</strong>
<span>Processos filhos, execução de comandos, sinais e monitoramento de recursos.</span>
</a>
<a class="tyrus-home-card" href="/modules/simd/">
<strong>tyrus:simd</strong>
<span>Intrínsecos portáveis de matemática vetorial para operações numéricas de alto desempenho.</span>
</a>
<a class="tyrus-home-card" href="/modules/stream/">
<strong>tyrus:stream</strong>
<span>Primitivas componíveis de streaming binário e de texto para I/O de alto desempenho.</span>
</a>
<a class="tyrus-home-card" href="/modules/task/">
<strong>tyrus:task</strong>
<span>Runtime assíncrono, criação de tarefas em múltiplas threads, canais, mutex, rwlock e cancelamento.</span>
</a>
<a class="tyrus-home-card" href="/modules/temporal/">
<strong>tyrus:temporal</strong>
<span>Gerenciamento moderno de datas, horários, durações e fusos horários com sintaxe ergonômica.</span>
</a>
<a class="tyrus-home-card" href="/modules/test/">
<strong>tyrus:test</strong>
<span>Executor de testes integrado com describe, it e expect para arquivos *.test.tyr.</span>
</a>
<a class="tyrus-home-card" href="/modules/utils/">
<strong>tyrus:utils</strong>
<span>Estilização de texto no terminal com styleText, cores ANSI e banners ASCII FIGlet.</span>
</a>
</div>
</section>

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">Feedback de segurança enquanto você digita</p>
<h2>Erros fazem parte da experiência do produto.</h2>
<p>Diagnósticos do compilador e do linter passam pela mesma análise usada pela CLI e pelo Language Server, então terminal e editor concordam sobre o que está errado e como corrigir.</p>
</div>
<div>

::: danger error
<!-- @include: ../.fragments/home/type-error.md -->
:::

</div>
</section>

<section class="tyrus-home-section tyrus-home-split">
<div>
<p class="tyrus-home-eyebrow">Comece com um comando</p>

<!-- @include: ../.fragments/home/new-command.md -->

</div>
<div>
<p class="tyrus-home-eyebrow">Execute com um comando</p>

<!-- @include: ../.fragments/home/run-command.md -->

</div>
</section>

<section class="tyrus-home-section">
<p class="tyrus-home-eyebrow">Por que esse nome</p>
<h2>Uma rocha e um deus.</h2>
<p>Tyrus é a grafia em latim de Tiro, a cidade fenícia cujo nome significa "rocha". Tyr é o deus da guerra, da justiça e da coragem na mitologia nórdica.</p>
<p>Mas o nome é a junção das iniciais de <span class="tyrus-home-strong">Ty</span>peScript e <span class="tyrus-home-strong">Rus</span>t.</p>
</section>
