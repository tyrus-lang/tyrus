---
title: Tyrus MCP
---

# Tyrus MCP

O servidor MCP do Tyrus permite que clientes de IA compatíveis pesquisem a documentação oficial e verifiquem código Tyrus independente com a análise compartilhada do compilador. A instalação é automática para os clientes compatíveis detectados na máquina.

## Instalação

Execute um comando após instalar o Tyrus:

```bash
tyrus mcp install
```

O comando baixa e verifica um cache limitado da documentação, testa o handshake do servidor MCP local e adiciona uma entrada `tyrus` aos arquivos de configuração dos clientes detectados. Há suporte à configuração de usuário do Claude Desktop, Cursor, Zed e VS Code Copilot. As configurações existentes são preservadas, entradas `tyrus` existentes não são sobrescritas e arquivos alterados recebem uma cópia de segurança antes da primeira edição.

Reinicie ou recarregue cada cliente configurado. Em seguida, pergunte sobre Tyrus ao assistente. O cliente inicia `tyrus mcp serve` em segundo plano; normalmente você não executa esse comando manualmente. Se nenhum cliente compatível for detectado, instale um e execute `tyrus mcp install` novamente ou siga a configuração manual abaixo.

## Comandos

| Comando | Finalidade |
|---------|------------|
| `tyrus mcp install` | Baixa o cache da documentação, configura clientes detectados e testa o handshake do servidor. |
| `tyrus mcp update` | Atualiza o cache e as entradas gerenciadas do Tyrus nos clientes. |
| `tyrus mcp status` | Mostra as versões do cache, as configurações dos clientes detectados e os conflitos. |
| `tyrus mcp uninstall` | Remove entradas gerenciadas inalteradas e o cache local da documentação. |
| `tyrus mcp serve` | Comando interno do servidor stdio iniciado por um cliente MCP configurado. |

## Recursos MCP disponíveis

| Recurso | Descrição |
|---------|-----------|
| `search_docs` | Pesquisa títulos indexados e retorna trechos classificados e links oficiais. |
| `get_page` | Lê uma página completa listada pelo índice da documentação. |
| `check_source` | Faz parsing e verificação de tipos e empréstimos de até 128 KiB de código independente em memória. |
| `tyrus://docs/index` | Lê o índice da documentação. |
| `tyrus://docs/{path}` | Lê uma página Markdown indexada. |
| `tyrus://llms.txt` | Lê o índice compacto da documentação. |
| `tyrus://llms-full.txt` | Solicita explicitamente o pacote completo da documentação. |
| `generate_tyrus_code` | Orienta o modelo a encontrar a documentação atual antes de gerar código. |
| `fix_tyrus_code` | Orienta o modelo a verificar e corrigir código independente. |

`search_docs` aceita uma consulta de até 512 bytes UTF-8 e retorna no máximo 20 resultados. `get_page` retorna até 1 MiB por página; o recurso da documentação completa é limitado a um pacote baixado de 32 MiB. `check_source` aceita até 128 KiB e interrompe a análise após cinco segundos.

`check_source` retorna códigos de diagnóstico estáveis, severidades, mensagens, intervalos em bytes e posições de linha e coluna iniciadas em um. Não executa código, cria arquivos, lê um projeto, carrega variáveis de ambiente nem resolve imports relativos ao projeto. Imports de módulos nativos estão disponíveis quando o compilador tem declarações independentes para eles. A análise não é um sandbox de segurança. Os clientes e modelos decidem quando chamar ferramentas MCP, então o servidor não pode garantir que o assistente verificará o código ou consultará a documentação.

## Configuração manual

Use o caminho absoluto do executável Tyrus no campo `command`. Preserve as demais configurações do arquivo. Cursor, Claude Desktop e Copilot usam um objeto `mcpServers`; Zed usa `context_servers`.

| Cliente | Local da configuração do usuário | Chave raiz |
|---------|---------------------------------|------------|
| Claude Desktop | macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`; Windows: `%APPDATA%/Claude/claude_desktop_config.json`; Linux: `${XDG_CONFIG_HOME:-~/.config}/Claude/claude_desktop_config.json` | `mcpServers` |
| Cursor | `~/.cursor/mcp.json` | `mcpServers` |
| Zed | macOS: `~/Library/Application Support/Zed/settings.json`; Windows: `%APPDATA%/Zed/settings.json`; Linux: `${XDG_CONFIG_HOME:-~/.config}/zed/settings.json` | `context_servers` |
| VS Code Copilot | `${COPILOT_HOME:-~/.copilot}/mcp-config.json` | `mcpServers` |

```json
{
  "mcpServers": {
    "tyrus": {
      "command": "/absolute/path/to/tyrus",
      "args": ["mcp", "serve"]
    }
  }
}
```

No Zed, use a mesma entrada `tyrus` sob `context_servers` no arquivo de configurações. Os locais dos arquivos podem variar conforme o sistema operacional, perfil ou versão do cliente. Prefira a interface de configurações MCP do cliente quando o caminho diferir do padrão. Consulte os guias oficiais de configuração do [Claude Desktop](https://modelcontextprotocol.io/quickstart/user), [Cursor](https://docs.cursor.com/context/model-context-protocol), [Zed](https://zed.dev/docs/ai/mcp) e [VS Code Copilot](https://code.visualstudio.com/docs/agent-customization/mcp-servers).

## Conflitos e solução de problemas

Se um cliente já tiver uma entrada `tyrus` que o instalador não gerencia, o instalador a preserva e informa o conflito. Revise essa entrada, altere-a ou remova-a manualmente e execute `tyrus mcp install` novamente. Se o arquivo de configuração for inválido ou não puder ser preservado com segurança, use a interface de configurações do cliente ou adicione a entrada manualmente.

Execute `tyrus mcp status` para inspecionar o cache e as entradas dos clientes. Execute `tyrus mcp update` para atualizar a documentação desatualizada. Execute `tyrus mcp uninstall` para remover entradas que ainda correspondam aos valores registrados pelo Tyrus; entradas editadas após a instalação são preservadas e devem ser removidas manualmente. Reinicie o cliente depois de alterar a configuração MCP.
