# Servidores

A tela Servidores tem três abas no topo: **Servidores públicos** e **Meus servidores**, para achar um servidor multijogador e entrar nele, e **Hospedar um servidor**, para rodar o seu próprio servidor dedicado neste PC. Em **Hospedar um servidor**, uma barra diz se o seu servidor está ligado, com **Ver como está rodando** (abre Diagnóstico > Servidor: memória, CPU, se ele já iniciou e o que o log dele diz), **Compartilhar convite** e o botão que lê os arquivos do servidor de novo.

## Entrar num servidor: achando um

Em **Servidores públicos**, clique em **Procurar servidores**. O Studio pede à Steam todos os servidores de Project Zomboid, do jeito que o navegador de servidores do próprio jogo faz, pelo cliente da Steam aberto neste PC (não precisa de chave; a Steam mostra você no Project Zomboid enquanto pergunta). Os servidores aparecem conforme respondem; todos levam mais ou menos um minuto e meio. **Parar** termina a busca, **Procurar de novo** recomeça.

Filtre a lista com a caixa (**Nome ou mapa**) e os filtros: **Só a minha build**, **Com jogadores**, **Não lotado**, **Sem senha**, **Com mods e vanilla** / **Com mods** / **Vanilla**, e ordene por **Mais jogadores**, **Menor ping** ou **Nome**. Clique com o botão direito num servidor para ver um menu: **Ver o que ele diz**, **Entrar**, **Marcar com estrela**, **Copiar o endereço**, criar um perfil com os mods dele e, para um servidor em **Meus servidores**, **Esquecer este servidor**.

**Meus servidores** guarda os servidores que você marca com estrela (**Com estrela**) e aqueles em que você entrou por aqui (**Entrei por aqui**). Para adicionar um servidor privado ou de um amigo, digite o endereço dele em **Adicionar um servidor pelo endereço** (`ip:port`; porta 16261 se ficar de fora), um nome opcional e a senha, e clique em **Adicionar**. A senha fica guardada no repositório de credenciais do Windows, nunca num arquivo.

## Entrar num servidor: entrando

1. Clique num servidor. O painel da direita mostra a build dele, se ele é aberto ou só para a whitelist, o último wipe e os mods. Cada mod aparece colorido: **Instalado e ativado**, **Instalado** ou **O jogo baixa**.
2. Se o servidor roda outra build que não a sua, um aviso aparece: o jogo não deixa você entrar até as duas baterem (ramo beta da Steam).
3. Digite a senha do servidor, se ele tiver uma, e marque **Lembrar a senha do servidor** para guardá-la. Uma senha guardada aparece como uma linha verde (**Senha do servidor guardada no Windows**) no lugar do campo: o Studio a preenche quando você entra. **Mudar** abre o campo para uma nova e **Esquecer** a apaga. A senha da sua conta vai na janela de conexão do jogo.
4. Feche o jogo se ele estiver aberto, e clique em **Entrar**.

O jogo inicia na janela de conexão deste servidor com a senha do servidor preenchida, e o seu nome de usuário também quando você o digitou em **Seu nome de usuário neste servidor** (o Studio o guarda; o mod Studio Link o preenche). Lá você digita a senha da sua conta e clica em **Conectar**, que fica desligado sem nome de usuário. Um nome de usuário novo cria uma conta nova naquele servidor. O jogo baixa os mods do servidor e só os usa lá: a sua própria lista de mods fica como está; o perfil com que você entra os recebe (abaixo).

Com o mod Studio Link, o servidor também vai para os **Favoritos** do jogo (a aba Favoritos da tela Multijogador dele), com o seu nome de usuário embaixo, como se você o tivesse adicionado com **Adicionar servidor** e **Adicionar nova conta**. Assim, se você desconectar, ele está lá para entrar de novo sem fechar o jogo. Ele nunca é adicionado duas vezes, e a senha da sua conta não é guardada; a senha do servidor é, do jeito que o jogo a guarda.

## Criando um perfil para um servidor

Um servidor só anuncia os primeiros mods dele (a Steam corta a lista). Depois que você entra nele uma vez por aqui, o Studio conhece a lista completa, e o botão vira **Criar um perfil com estes N mods**. Antes disso, **Criar um perfil com os N mods que ele anuncia** usa a lista parcial. O perfil se chama "Server: (name)" e mantém a ordem do servidor. Os mods que não estão instalados agora são apontados antes de o perfil ser criado.

Para jogar com ele, ative-o em Perfis antes de entrar. Cada vez que você entra por aqui, assim que o jogo carregou os mods do servidor, eles vão para o perfil que estava ativo quando você entrou: ativados, na ordem do servidor, com os que o jogo fez você baixar. Os outros mods dele ficam nele, desativados, menos os que rodam só do seu lado (o ZombieBuddy e os mods Java só de cliente dele, como o Viewpoint): eles ficam como você os tinha. O painel do servidor diz qual é esse perfil.

## Hospedar um servidor: a lista de servidores

À esquerda, **Configurações de servidor (.ini)** lista as configurações de servidor em `Zomboid/Server`, com a contagem de mods e de itens da Oficina e **Rodando** para a que está ligada. Digite um nome e clique em **Criar** para fazer uma nova, ou **⚡ Criar configuração padrão (servertest.ini)** quando não houver nenhuma. A lixeira apaga uma configuração do disco (você é consultado antes). Escolha uma para ver as abas dela. A faixa do topo mostra **ONLINE** ou **OFFLINE**, **Compartilhar convite** (copia um texto de convite com o nome, a porta, o máximo de jogadores, a senha e a contagem de mods; você preenche o seu IP público e libera a porta) e um botão para recarregar os arquivos do servidor.

## Hospedar e iniciar

- **Memória (-Xmx)**: 4, 8, 12 ou 16 GB para o servidor.
- **Senha de admin**: define a senha da conta `admin` quando o servidor inicia. O Studio a guarda no Gerenciador de Credenciais do Windows (nunca num arquivo), para que um assistente conectado ao Studio possa iniciar este servidor de novo sem receber a senha.
- **Autenticação e rede**: **Steam ativada** ou **NoSteam (LAN)**.
- **Iniciar servidor dedicado (nome)** o inicia numa janela de console própria. Enquanto ele roda, **Parar servidor dedicado** para o servidor que o Studio iniciou (nunca outros programas Java), e **Reiniciar** o para e o inicia de novo na hora (para avisar os jogadores antes, use a aba **Reinícios**).

**Programa do servidor.** Um servidor roda a partir dos próprios arquivos do jogo, a não ser que você escolha outra coisa. A caixa **Programa do servidor** oferece, em vez disso, o **Project Zomboid Dedicated Server** oficial: um programa separado que a Steam dá de graça (app 380870), que pode estar num ramo diferente do seu jogo e é atualizado à parte.

1. Escolha uma **Pasta do servidor dedicado**: uma pasta vazia ou nova num disco com espaço (não a pasta do jogo, não `Zomboid`). Um servidor dedicado que já esteja neste PC (da Biblioteca > Ferramentas da Steam, ou de um SteamCMD anterior) é encontrado e oferecido.
2. Escolha o **Ramo**: **Estável (public)** ou **Instável (beta)**. Ele precisa bater com o jogo de quem entra; a caixa diz em qual ramo o seu jogo está e começa nesse. Um ramo que a Steam guarda atrás de uma senha não pode ser instalado: a caixa avisa.
3. Clique em **Instalar**. O Studio usa o SteamCMD, a Steam de linha de comando da Valve. Se ele não estiver neste PC, o Studio pergunta antes, depois o baixa do próprio endereço da Valve (uns 3 MB), confere se a Valve o assinou e deixa ele se atualizar sozinho. Então o SteamCMD baixa o servidor da Steam (vários GB): a caixa mostra cada passo com a porcentagem e os gigabytes (uns 7 GB para a 42.22), e o tamanho da pasta quando termina. **Parar** interrompe; clicar em Instalar de novo continua de onde parou.
4. Depois de instalado, escolha **O servidor dedicado**. Os servidores passam a iniciar daquela pasta, com a mesma pasta Zomboid (as suas configurações de servidor, os saves e `Zomboid/mods`). **Os arquivos do jogo** volta para o jeito de sempre. Mude isso com o servidor parado.

**Atualizar** roda o SteamCMD de novo naquela pasta e ramo (trocando de ramo quando você o mudou). Numa instalação completa, ele só baixa o que mudou e termina em segundos ("Already up to date" quando nada mudou); depois de uma instalação interrompida, ou de uma verificação dos arquivos que foi parada, ele verifica todos os arquivos, o que leva muitos minutos. A caixa também diz a versão do jogo com que o servidor inicia e se é a do seu jogo: um jogo só entra num servidor da mesma versão.

Mods Java com o ZombieBuddy precisam do `zbNative.dll` e do `ZombieBuddy.jar` do ZombieBuddy nessa pasta também: copie-os da pasta do jogo (o botão **Copiar os arquivos do ZombieBuddy para o servidor dedicado** em **Mods e sincronização** faz isso) (o ZombieBuddy os coloca lá quando o jogo inicia com ele). Até eles estarem lá, o servidor inicia sem o agente dele e **Mods e sincronização** avisa.

**Mods Java (ZombieBuddy).** Os mods da Build 42 com código Java precisam do ZombieBuddy, e o ZombieBuddy precisa do agente dele na linha de comando do servidor. Quando o `Mods=` do servidor tem o ZombieBuddy (escrito `\ZombieBuddy` também), o Studio o inicia com `-agentlib:zbNative=policy=deny-new`, e uma nota nesta aba avisa. *deny-new* quer dizer que o servidor só carrega os jars que este PC já aprovou na janela do ZombieBuddy (eles ficam guardados em `.zombie_buddy/mod_approvals.json` na sua pasta de usuário) e recusa o resto sem perguntar: um servidor não tem janela, e a pergunta o travaria para sempre. **Mods e sincronização** diz quais jars ainda não foram aprovados.

## Reinícios

O PZ Mod Studio pode reiniciar o servidor para você, avisando os jogadores no jogo antes. Um reinício manda os avisos por RCON (`servermsg`), depois `save`, depois `quit` (o servidor salva o mundo ao sair; ele só é fechado à força se não sair em 90 segundos), e o inicia de novo do mesmo jeito que foi iniciado (mesma memória, Steam ou NoSteam, programa).

- **Reinícios programados**: em horários do dia (o relógio deste PC) e/ou **A cada** 2 a 24 horas desde que o servidor iniciou. Um horário a menos de 30 minutos do início do servidor é pulado.
- **Avisar os jogadores antes**: 60, 30, 15, 10, 5 e 1 minutos, os que você marcar (30, 15, 5 e 1 no começo). As mensagens ficam no idioma do app.
- **Reiniciar quando um mod da Oficina deste servidor for atualizado**: a cada 15 minutos, o Studio pergunta à Steam (pela Web API pública dela) quando cada item do `WorkshopItems=` do servidor foi atualizado pela última vez. Um atualizado depois que o servidor iniciou ganha um reinício com os mesmos avisos, e o servidor baixa a atualização ao iniciar.
- **Iniciar de novo se ele parar sozinho** (ligado no começo): depois de um crash, se ele tinha subido (um servidor que falha enquanto inicia falharia de novo, então não é iniciado de novo). No máximo 3 vezes em 10 minutos; depois disso ele fica desligado e a aba avisa em vermelho. Iniciar à mão zera a contagem. Uma parada que você pediu (Parar, Reiniciar, um assistente, Discord) ou um `quit` de um admin no jogo nunca é confundido com um crash.

O topo da aba diz o que vem por aí: o próximo reinício programado, ou o que está em contagem regressiva, com quando e por quê. **Reiniciar agora (avisar jogadores)** começa uma contagem regressiva dos minutos que você escolher (**sem aviso (agora)** reinicia na hora); **Cancelar** a interrompe e avisa os jogadores. **Últimos reinícios** lista o que aconteceu e por quê: Programado, Atualização de mod, Pedido, Depois de um crash, Deixado desligado: crashes demais, Parou sozinho, Parado com o app fechado.

**Só com o app aberto.** Os reinícios são feitos pelo PZ Mod Studio. O servidor continua rodando se você fechar o app, mas aí nada o reinicia, avisa ninguém ou o inicia depois de um crash. A programação fica salva (`Zomboid/PZModStudio_ServerSchedule.json`) e continua quando você abre o app de novo; um servidor que parou com o app fechado é anotado, não iniciado de novo.

## Mods e sincronização

Isto compara os seus mods ativos com o que o `.ini` do servidor tem. Um servidor não lê a sua lista de mods: ele precisa de três linhas no `.ini` dele, `Mods=`, `WorkshopItems=` e `Map=`, e as três precisam bater, senão o servidor roda em silêncio sem os mods. O Studio as verifica e lista o que está errado (um mod nunca baixado, um item nunca carregado, um mapa ignorado).

**Gravar a minha lista neste servidor (Mods, WorkshopItems e Map)** grava as três linhas a partir dos seus mods ativos, na ordem de carregamento, e mantém todas as outras configurações como estavam. O arquivo anterior fica salvo em `Zomboid/PZModStudio_Backups`.

Um pacote do PZ Mod Studio (`Z_PZModStudio_*`) ativado também vai para o `Mods=`, mas ele é um mod local, não um item da Oficina: o servidor neste PC o tem, os jogadores não o recebem baixando, e o jogo não os deixa entrar sem ele. Compartilhe o pacote com eles, ou desative-o antes de gravar a lista.

A mesma checagem lê o que o servidor vai fazer com os **mods Java**: um jar que o ZombieBuddy ainda não aprovou neste PC (novo, ou mudado desde então: uma atualização do mod ou uma recompilação é um jar novo) é recusado pelo servidor. Inicie o jogo uma vez com esse mod ativado, permita-o na janela do ZombieBuddy e então inicie o servidor. Um jar que você mandou o ZombieBuddy recusar, e mods Java numa lista sem o ZombieBuddy (só o Lua deles carrega), também são apontados.

Com **NoSteam (LAN)** escolhido em **Hospedar e iniciar**, a checagem também lista os mods que só estão na pasta da Oficina Steam. Um servidor iniciado sem Steam só lê `Zomboid/mods`: ele dá esses como "not found" e descarta também todos os mods que os requerem. **Copiar N mod(s) da Oficina para Zomboid/mods** copia a pasta da Oficina de cada um para lá. Uma cópia, não um link: com alguns mods da Oficina linkados em `Zomboid/mods` (uma junção de diretório), um servidor NoSteam fecha com erro enquanto cria o mundo, então um link para uma pasta da Oficina também é apontado e o mesmo botão o troca por uma cópia. Uma cópia também vence o item da Oficina no seu próprio jogo. Quando o item da Oficina é atualizado depois que o Studio o copiou, a cópia é marcada como desatualizada e o botão a copia de novo. Uma pasta de mesmo nome que o Studio não colocou lá (a sua própria cópia) nunca é substituída.

**Esta lista funciona no multijogador?** (**Verificar**) dá um veredito para a lista e um por mod. Ele lê o que a descrição de cada mod diz sobre multijogador, e lista chamadas que o código de cliente de um mod faz diretamente onde o jogo as passa pelo servidor; no multijogador elas só mudam o mundo daquele jogador, ou são recusadas.

## Jogadores

**Jogadores conectados** lista quem está online, lido do próprio log do servidor, com cargo, ping, vida e posição. Enquanto o servidor roda, o Studio fala com ele por RCON (o `RCONPassword` e o `RCONPort` do `.ini` dele; um servidor que o Studio inicia sem senha ganha uma):

- **Expulsar** na linha de um jogador o desconecta; ele pode entrar de novo. Você pode dar um motivo, que ele vê.
- **Banir** o bane deste servidor, com um motivo opcional. Para desfazer um banimento, use o console do servidor: `unbanuser "name"`.
- A caixa no topo manda uma mensagem para todos no jogo (**Enviar para todos**, ou Enter); ela aparece na tela deles.

A resposta do servidor aparece abaixo da caixa.

## .ini rápido

Edite as configurações principais e clique em **Salvar configurações no .ini**: **Nome público do servidor**, **Senha do servidor (para entrar)** (em branco para um servidor aberto), **Senha de admin (-adminpassword)**, **Máximo de jogadores**, **Porta padrão**, **Descrição pública**, **Combate PVP** e **Pausar quando vazio**.

## Todas as opções

Todas as opções de servidor e de Sandbox, como o jogo as mostra: por página, com a ajuda, o padrão e as opções de Sandbox dos seus mods. **Servidor** e **Sandbox** trocam de lista, a busca acha uma opção pelo nome ou pela ajuda, e **Só as que não estão no padrão** mostra o que você mudou. A seta de uma opção a coloca de volta no padrão. **Salvar** grava só o que você mudou, com backup dos dois arquivos antes (em `PZModStudio_Backups/server_settings`), e recusa enquanto aquele servidor roda. O Studio Link anota a lista de opções na primeira vez que você abre o jogo até o menu principal (e de novo quando a build, o idioma ou os seus mods mudam).

## Discord

Um webhook do Discord deixa o servidor postar num canal: quando ele está no ar, parado, caiu ou voltou depois de um reinício, quem entra e sai, e quando os mods da Oficina dele foram atualizados e ele precisa de um reinício. Ninguém é marcado.

1. No Discord, abra as configurações do canal, Integrações, Webhooks, Novo webhook, e copie a URL dele.
2. Cole em **Endereço do webhook** e clique em **Salvar**. Ele fica guardado no Gerenciador de Credenciais do Windows e nunca mais aparece.
3. Clique em **Enviar um teste** e olhe o canal.
4. Marque **Postar as novidades deste servidor no canal** e escolha os eventos: **No ar, parado e caiu**, **Jogadores entrando e saindo**, **Reinícios**, **Mods da Oficina esperando um reinício**.

**Esquecer** tira o endereço deste PC. Cada servidor tem o próprio webhook.

### Chat com o jogo: o bot do próprio servidor

O servidor dedicado do Project Zomboid tem um bot de Discord próprio. O Studio o configura no `.ini` do servidor; ele vive dentro do servidor, então funciona sempre que o servidor roda, mesmo com o Studio fechado.

1. No Discord Developer Portal: **New Application**, depois **Bot**: **Reset Token** e copie, e ligue o **Message Content Intent**.
2. Em **Chat com o jogo**, cole o **Token do bot**, escreva o **Canal de chat** (o nome dele como está no Discord, sem `#`) e, se quiser, o **Canal de log**, marque **Ligar o bot** e clique em **Salvar no .ini**.
3. Clique em **Convidar o bot para o seu Discord**, escolha o seu servidor do Discord e inicie (ou reinicie) o servidor do jogo.

O chat geral do jogo passa então a ir para aquele canal, e o que é escrito lá vai para o jogo. O jogo lê o token do `.ini`, então ele fica gravado lá: quem tiver esse arquivo pode usar o bot. Se o `.ini` tiver um canal de comandos, a tela avisa você: qualquer coisa que alguém escreva lá roda como um comando de console de admin; **Limpar** a não ser que esse canal seja só seu.

Uma **mensagem de status que se edita sozinha** (ligada por padrão) mantém uma mensagem no canal dizendo se o servidor está no ar, desde quando e quem está dentro, em vez de postar uma nova a cada vez.

### Comandos pelo Discord (com o Studio aberto)

Desligados a não ser que você os ligue. Com o bot acima configurado, escreva um **Canal de comandos** e o **Cargo que pode usá-los** (os nomes como estão no Discord) e marque **Ligar os comandos**. O Studio lê aquele canal a cada 5 segundos e responde lá; membros sem o cargo ficam sabendo que não podem.

- `!status` e `!players`: se o servidor está no ar e quem está dentro.
- `!restart [minutes]` (1 a 60, 5 se não disser): os jogadores são avisados no jogo na hora, e 5 e 1 minuto antes; depois o servidor é parado do jeito de sempre (ele salva) e iniciado de novo. `!restart cancel` interrompe.
- `!save` salva o mundo; `!msg <text>` diz algo para todos no jogo.
- `!help` os lista. Eles também funcionam em espanhol (`!estado`, `!jugadores`, `!reiniciar`, `!guardar`, `!mensaje`).

Eles falam com o servidor por RCON: um servidor que o jogo criou não tem senha de RCON, então o Studio dá uma aleatória a ele da próxima vez que o iniciar (com backup do `.ini` antes); isso também deixa o **Parar** salvar antes de sair. Eles só funcionam com o PZ Mod Studio aberto, e nunca rodam um comando de console digitado à mão. Use um canal que só os admins veem: o canal de chat também vai para o jogo.

## Bom saber

- Postar no Discord só acontece com o PZ Mod Studio aberto e hospedando o servidor.
- Iniciar um servidor precisa da pasta do jogo definida em **Configurações do App**.
- Entrar num servidor inicia o jogo: feche-o antes.
- Para entrar no seu próprio servidor a partir deste PC, use o endereço de LAN dele (por exemplo `192.168.0.10:16261`): um cliente Steam não consegue entrar em `127.0.0.1` nem no IP público deste PC. Um servidor iniciado com **NoSteam (LAN)** precisa de um jogo iniciado com `-nosteam` (aí `127.0.0.1` funciona), e um jogo iniciado com `-debug` só pode entrar como admin.
- Assistentes conectados ao Studio (MCP) podem fazer o mesmo: `server_start`, `server_status`, `server_stop`, `server_join`, `server_schedule`, `server_restart` e `dedicated_server`.
