# Configurações do App

As Configurações do App são onde você diz ao Studio onde ficam o seu jogo e os seus mods, e escolhe como o Studio se comporta: idioma, janela, avisos, como ele inicia e alguns extras opcionais. Cada mudança é salva assim que você a faz.

## Simples e Completo

O cartão no topo, **Quanto do Studio mostrar**, escolhe uma de duas visões. A mesma chave está sempre no pé da barra da esquerda, em **Visão**. Mudar isso nunca apaga nem desativa nada: só mostra ou esconde telas, e você pode ir e voltar quando quiser.

- **Simples** mostra o que você precisa para jogar com mods:
  - **Início**: o perfil em uso, **Jogar**, **Continuar** (o seu último jogo com este perfil, quando você tiver um) e a checagem numa linha só: verde quando não há nada a fazer, vermelha com **Ver o que fazer** quando algo precisa ser consertado antes.
  - **Perfis**, **Lista de Mods**, **Explorar Mods**, **Saves** e **Diagnóstico**.
  - A **Lista de Mods** deixa você ativar e desativar mods, ordená-los (à mão ou com **Ordenar automático**), buscar e filtrar por ativados, desativados e com problema. Cada mod é marcado em vermelho, em palavras simples, só pelo que precisa de conserto, com o botão dele ao lado. Grupos, notas, predefinições, os outros filtros e os tempos de carregamento ficam no Completo.
  - O **Diagnóstico** abre direto no que você precisa resolver, cada um com um botão. As sugestões opcionais (o jogo funciona sem elas) e o que é só bom saber ou já está resolvido são contados abaixo da lista, sem aparecer; **Mostrar tudo (Completo)** os mostra. **Corrigir N problema(s) automaticamente** conta e corrige só as linhas da lista que você vê: as sugestões escondidas não são mexidas.
  - Aqui, as Configurações mantêm as abas **Caminhos das pastas**, **Alertas** e **O app**.
- **Completo** é o Studio inteiro: todas as telas na barra lateral (Central de Monitoramento, Novidades, Mapas, Mesclador de Mods, Servidores, Desenvolvedor), todas as abas do Diagnóstico e todas as abas das Configurações.

Uma instalação nova começa no **Simples**, com boas-vindas em três passos: onde o seu jogo foi encontrado, criar ou escolher um perfil, jogar. Se o Studio já foi usado neste PC, ele começa no **Completo**, como sempre foi, e na primeira vez uma linha no topo oferece experimentar o Simples: **Experimentar o Simples** ou **Não, obrigado**. Ela aparece uma vez só, e nunca se você já escolheu uma visão. A escolha fica lembrada neste PC.

Um link de outra tela ainda pode abrir uma tela que o Simples esconde (os passos do Doutor, por exemplo); ela aparece na barra lateral enquanto estiver aberta.

## O que você vê

A tela tem abas no topo, lado a lado:

- **Caminhos das pastas**: onde estão o jogo, a Oficina, a sua pasta Zomboid e a lista de mods.
- **Alertas**: quais avisos o Studio mostra para você.
- **O app**: como o Studio inicia, a janela dele e o botão de fechar, o idioma, lembrar como o jogo inicia, a tela de Mods do jogo, procurar versões novas e tradução.
- **Avançado**: o diagnóstico do app, os recursos de desenvolvedor e, com eles, os mods Java que você permitiu no ZombieBuddy.
- **Correções da Build 42**: o que o shim de compatibilidade da Build 42 cobre, com uma chave por mod que ele corrige.
- **Servidor de agentes MCP**: só aparece com **Recursos de desenvolvedor** ligado.

## Pastas

Clique em **Detectar pastas** para o Studio achar tudo sozinho. Para definir uma pasta à mão, clique em **Procurar...** ao lado dela. Uma nota **Salvo.** confirma a mudança.

- **Pasta de instalação do Project Zomboid**: a pasta com o `ProjectZomboid64.exe`. Mostra **Jogo encontrado** quando a pasta está certa, e **O Studio não encontra o jogo aqui** quando não está. Instalações da Steam, da GOG e manuais funcionam.
- **Pasta da Oficina Steam (opcional se não usa Steam)**: onde a Steam baixa os mods da Oficina (`workshop/content/108600`). Deixe vazio se você joga pela GOG ou coloca mods em `Zomboid/mods` à mão.
- **Pasta Zomboid do usuário (dados e mods locais)**: a sua pasta `C:\Users\<User>\Zomboid`, com logs, saves, perfis e mods locais. É obrigatória.
- **Arquivo da lista de mods (Zomboid/mods/default.txt)**: o arquivo que o jogo lê para saber quais mods estão ativados e em que ordem. É encontrado automaticamente; **Procurar...** aqui escolhe um arquivo, não uma pasta.

Sem uma pasta do jogo válida, telas como o Mesclador de Mods pedem para você vir aqui antes.

## Avisos

Em **Notificações e avisos do sistema** você pode silenciar ou trazer de volta três mensagens:

- **Aviso ao iniciar sobre conflitos de mods não mesclados**: o aviso antes de iniciar quando os seus mods ativos compartilham arquivos que nenhum pacote de mescla ativo mescla (ou mescla a partir de versões antigas dos mods).
- **Aviso ao iniciar sobre mods que definem as mesmas coisas**: o aviso antes de iniciar quando dois mods ativos definem o mesmo item, receita ou texto de jeitos diferentes e nenhum pacote decide qual fica. Nada quebra: vence o que carrega por último.
- **Mensagem explicativa de Ordenar dependências**: a explicação mostrada quando você ordena dependências na Lista de Mods.

Cada uma tem **Silenciar aviso** / **Restaurar aviso** (ou **Silenciar notificação** / **Restaurar notificação**).

## Como o Studio inicia e a janela dele

- **O que deve abrir quando o Studio inicia?**: **Tela de perfis (início)** (recomendado) mostra o gerenciador de perfis; **Abrir direto o último perfil** carrega o último perfil ativo e abre a Lista de Mods.
- **Abrir a janela como eu deixei**: mesmo tamanho, lugar e monitor, maximizada ou em tela cheia se estava assim. Se aquele monitor não existir mais, ela abre no principal.
- **O botão de fechar (X)**: **Fecha o Studio** o encerra (o jogo continua rodando se estiver aberto). **Deixa na bandeja, perto do relógio** esconde a janela e o Studio continua de olho no jogo (backups quando você sai, o perfil na tela de Mods do jogo). Clique no ícone da bandeja para abri-lo de novo; o menu dele tem **Sair**.

## Idioma

Escolha o idioma do app em **Idioma**: English, Español, Português (Brasil), Français, Deutsch, Русский ou 简体中文. Uma linha que falta num idioma aparece em inglês. As mensagens do próprio jogo (erros, avisos, as linhas de log do Monitor) ficam como o jogo as escreve.

## Início do jogo e a tela de Mods do jogo

- **Lembrar como o jogo inicia**: ligado, o botão Iniciar começa no modo que você usou da última vez (Debug, em janela, normal...). Desligado, ele sempre começa em Modo Debug (tela cheia), o recomendado.
- **A tela de Mods do jogo, com outro gerenciador de mods ativado**: só um gerenciador de mods pode desenhar a tela de Mods do jogo. Quando outro está ativado, escolha **Perguntar no jogo**, **A do PZ Mod Studio (pergunta de novo se o outro for ativado)** ou **A do outro gerenciador**. Os backups dos saves e a checagem antes de carregar um save continuam com o PZ Mod Studio de qualquer jeito.

## Atualizações do app

- **Procurar novas versões do PZ Mod Studio**: ao iniciar e a cada 6 horas enquanto está aberto, o Studio pergunta ao GitHub qual é a última versão. Quando há uma mais nova, uma barra abaixo do cabeçalho avisa. **Atualizar agora** baixa a versão nova, confere se o projeto a assinou (a assinatura do `SHA256SUMS.txt` e o SHA-256 do zip), a coloca no lugar da sua e reinicia o Studio; se algo não bater, nada muda. **Baixar** abre a página da versão no seu navegador, e **Agora não** esconde o aviso até a próxima versão. Quando o Studio está instalado numa pasta em que ele não pode gravar (Arquivos de Programas, para todos os usuários), atualize com o instalador da página. Nada é instalado sem você pedir, e nada é enviado. **Procurar agora** pergunta na hora. Quando o GitHub não responde, o Studio pergunta à página dele no itch.io; aí o aviso oferece só a página de download, não **Atualizar agora**. Desligado, o GitHub não é consultado.

## Versões anteriores de mods

- **Guardar versões anteriores dos mods** (ligado no começo): a Steam não guarda versões antigas de um mod da Oficina. O Studio guarda uma cópia da versão instalada de cada mod da Oficina que um perfil usa, em `Zomboid/PZModStudio_Versions`, antes que a Steam a substitua; **Versões…** da Lista de Mods coloca uma de volta. **Até** define o espaço que elas podem ocupar (10 GB no começo; um limite que você escolheu fica como você escolheu); cada arquivo é guardado uma vez só, então uma versão depois de uma atualização custa só o que mudou. Passando do limite, as versões anteriores mais antigas vão embora primeiro, depois as cópias de mods que não estão no perfil ativo; as versões guardadas à mão ficam. Seja qual for o limite, o Studio nunca enche o disco: uma rodada sempre deixa 15% do tamanho do disco livre, e quando é isso que segura o depósito o cartão diz **Limitado pelo espaço livre em D:** (com o limite que vale por enquanto). **Pasta** mostra onde ficam as cópias e quanto ocupam; **Escolher pasta…** as move, por exemplo para outro disco: cada arquivo é copiado e conferido com o original, e só então a pasta antiga é removida (no mesmo disco, ela só é renomeada). Uma pasta que já tem outros arquivos ganha uma pasta `PZModStudio_Versions` dentro dela. Mover é recusado com o jogo rodando ou durante uma rodada, e para a raiz de um disco, a pasta da Oficina da Steam, a pasta do jogo, `Zomboid/mods` e `Zomboid/Saves`. A linha abaixo diz quanto está sendo usado e quantos dos seus mods da Oficina estão cobertos; os mods que ficaram sem cópia por falta de espaço são citados. **Guardar agora** roda uma rodada na hora (ela também roda sozinha um pouco depois que o Studio inicia e sempre que a Steam atualiza algo). Nada é feito com o jogo rodando, e a pasta da Oficina só é lida.

## Diagnóstico

**Diagnóstico** junta o que um relatório de bug precisa: as versões do Studio e do Windows, as pastas, a build do jogo, o perfil ativo, o Studio Link, o Zombie Buddy, se a Steam está limitando pedidos e o fim do log do Studio. **Copiar diagnóstico** o copia; o seu nome de usuário do Windows e o nome do PC são trocados por `<user>` e `<pc>`. Dê uma olhada antes de colar em qualquer lugar. **Abrir a pasta de logs** abre a pasta com os logs do próprio Studio.

## Tradução

Explorar Mods tem um botão Traduzir na página de um mod e um para os resumos de uma página de resultados; os rascunhos de tradução da tela Desenvolvedor usam o mesmo serviço. Sem chave, é usado o serviço gratuito do Google Tradutor (ele não é oficial e pode parar um dia). Com uma chave da API do DeepL (gratuita até 500.000 caracteres por mês), o DeepL é usado primeiro: clique em **Obter uma chave do DeepL**, cole a chave e clique em **Verificar e salvar**. A chave fica guardada no Gerenciador de Credenciais do Windows, só neste PC; **Remover** a apaga. As traduções já feitas ficam guardadas neste PC, então um texto não é enviado de novo; **Apagar** as limpa.

## Recursos de desenvolvedor

**Recursos de desenvolvedor** mostra a tela Desenvolvedor na barra lateral, a aba **Servidor de agentes MCP** (que deixa um assistente de IA comandar o Studio) e o mod companheiro que roda comandos dentro do jogo. Um jogador não precisa disso; deixe desligado a não ser que esteja desenvolvendo com ele. A escolha fica lembrada neste PC.

## Correções da Build 42

Esta aba explica o shim de compatibilidade da Build 42: um arquivo que o Studio grava em todo pacote de mescla que ele monta, que corrige chamadas que os mods da Build 41 fazem a coisas que a Build 42 renomeou ou tirou. As partes 1 a 6 sempre valem enquanto um pacote de mescla está ativo. A parte 7 corrige mods específicos, cada um com uma chave **LIGADO** / **DESLIGADO**. Uma chave é gravada em todos os pacotes do Studio no disco na hora; não precisa publicar, e o jogo a lê na próxima vez que iniciar.

## Servidor de agentes MCP

Só com **Recursos de desenvolvedor** ligado. Escolha o seu cliente de IA, clique em **Copiar JSON do MCP** e cole no arquivo de configuração desse cliente; as instruções passo a passo para cada cliente aparecem abaixo. Não abra o `pz-mcp-server.exe` com clique duplo: uma janela preta vazia é normal, e o seu cliente de IA o inicia e o para sozinho.

## Bom saber

- As escolhas de pasta, idioma, janela e avisos valem na hora; nada precisa reiniciar, exceto as chaves do shim, que o jogo lê na próxima vez que iniciar.

## No Linux

A versão para Linux (AppImage ou .deb) encontra a Steam nas pastas de sempre (`~/.local/share/Steam`, `~/.steam`, a do Flatpak) e a pasta Zomboid na sua pasta pessoal. Algumas coisas só funcionam no Windows: atualizar o app no lugar (no Linux, **Baixar** abre a página da versão), mover e capturar a janela do jogo para um assistente de IA (estacionar, capturas de tela, cliques, teclas), o registro de falta de memória do Windows, inscrever-se pelo cliente da Steam e baixar o SteamCMD (instale pela sua distribuição). Chaves e senhas salvas ficam num arquivo privado, `~/.config/PZModStudio/secrets.json`, em vez de um cofre de credenciais. Para um cliente de IA, o servidor de console é `/usr/bin/pz-mcp-server` com o .deb; o AppImage serve MCP sozinho com `--mcp`.
