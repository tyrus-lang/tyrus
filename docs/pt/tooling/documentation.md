---
title: Manutenção da Documentação
description: Checklist de release e política de manutenção para o site de documentação do tyrus.
---

# Manutenção da Documentação

A documentação do tyrus é versionada junto com o compilador e o runtime. A documentação atual fica em `docs/`, releases arquivados são gerados a partir de Git tags, e o site publicado deve expor a documentação estável mais recente em `/`.

## Ownership

Os mantenedores core são responsáveis por metadados de versão da documentação, atualizações do checklist de release e revisão de páginas que descrevem linguagem, compilador, runtime, CLI, formatter, linter, LSP ou comportamento de módulo nativo.

O ownership de tradução segue a mesma regra até existirem mantenedores dedicados de tradução. Inglês permanece a linguagem fonte para comportamento técnico, e páginas localizadas devem manter conteúdo localizado integral quando a rota existir.

## Checklist de Release

- Atualize a documentação para cada mudança visível ao usuário antes de criar a release tag.
- Mantenha a documentação atual em `docs/` e não crie pastas permanentes `docs/v0.1.0/` no branch principal.
- Confirme que `docs/.vitepress/versions.ts` tem a versão atual como `latestDocsVersion` e versões arquivadas em `previousDocsVersions`.
- Rode `npm run docs:build` antes de cada release tag.
- Rode `npm run docs:build:versions` antes de publicar o site de documentação.
- Verifique `/`, `/pt/`, `/es/`, links de versão, links de sidebar, assets de logo, assets de favicon e busca local no site buildado.
- Publique apenas `docs/.vitepress/dist` ou o diretório de saída estática específico do provider.

## Regras de Revisão

- Não documente comportamento incompleto ou experimental como estável.
- Marque recursos experimentais explicitamente e descreva o limite atualmente implementado.
- Mantenha páginas de API próximas da árvore pública de módulos, como `docs/modules/fs/read-file.md`, para que links de sidebar, resultados de busca e ownership de fonte permaneçam previsíveis.
- Não gere páginas de referência de API automaticamente até que o compilador ou LSP exponha metadata estável para assinaturas, diagnósticos, exports de módulo e exemplos.
- Prefira páginas pequenas com deep links estáveis em vez de uma grande página de API.
- Mantenha snippets executáveis ou diretamente alinhados a um exemplo executável quando o recurso tiver comportamento de runtime.

## Referência Gerada no Futuro

A documentação de referência gerada fica intencionalmente adiada até que o tyrus exponha metadata estável do compilador ou do LSP. O gerador futuro deve ler exports estruturados de módulos, assinaturas, códigos de diagnóstico e exemplos em vez de fazer scraping de Markdown ou comentários no código.
