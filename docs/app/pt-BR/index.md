# PZ Mod Studio

O PZ Mod Studio é um gerenciador de mods gratuito para Project Zomboid no Windows. Ele guarda os seus mods em perfis, faz backup dos seus saves, verifica a sua lista de mods do jeito que o jogo a lê e ajuda você a consertar o que quebra. Clique em "?" em qualquer tela para ver a página de ajuda dela.

Na primeira vez que você abre o Studio aparece um **tour de boas-vindas**: um cartão no canto inferior direito que passa por todas as telas, com a tela de verdade atrás e o botão dela aceso na barra lateral, junto com o que o Studio já sabe da sua instalação (as pastas que encontrou, os seus perfis, os problemas da sua lista). Avance com **Próximo** ou com as setas do teclado; o X fecha. Para ver de novo, abra a ajuda ("?") e clique em **Mostrar o tour de boas-vindas de novo**.

## As telas

A barra lateral lista as telas em **Módulos do Studio**. As telas com cadeado precisam de um perfil ativo antes: crie ou ative um em Perfis.

- **Perfis**: listas de mods com nome; crie, troque entre elas com um clique, compartilhe como arquivos `.pzpack`.
- **Saves**: os seus saves e os backups deles; faça um, guarde um ou restaure um.
- **Central de Monitoramento**: inicie o jogo e leia o log dele, com um cartão por erro e o mod de onde ele veio.
- **Lista de Mods**: ative e desative mods e defina a ordem de carregamento, com filtros, grupos, notas e desfazer.
- **Novidades**: os seus mods da Oficina atualizados desde a última vez que você jogou, com as notas de alteração e um aviso quando uma atualização pode quebrar saves.
- **Diagnóstico**: o que o app sabe sobre os seus mods (verificados do jeito que o jogo os lê: requisitos faltando ou desativados, ordem errada, mods incompatíveis, mods que a sua build não carrega, com correções), sobre o jogo (memória, CPU, inicialização, quadros, disco) e sobre o servidor rodando no seu PC, com o Doutor na página inicial.
- **Explorar Mods**: busque na Oficina Steam, inscreva-se, mantenha as suas próprias listas e instale mods baixados de outros lugares.
- **Mapas**: as células de cada mod de mapa por cima do mundo do jogo, e onde dois mods de mapa brigam pelas mesmas células.
- **Mesclador de Mods**: quando dois mods mudam o mesmo arquivo, combine-os num pacote de patch que carrega por último.
- **Desenvolvedor**: para quem faz mods: verifique os arquivos Lua e de script de um mod antes do jogo, e inicie o jogo só com esse mod ativado. Só aparece com **Recursos de desenvolvedor** ligado nas Configurações do App.
- **Servidores**: hospede e gerencie um servidor dedicado e mantenha os mods dele em dia com os seus.
- **Configurações do App**: onde ficam o seu jogo e os seus mods, quais avisos você vê e outras opções.

## Iniciando o jogo

O botão verde no canto superior direito inicia o Project Zomboid com a sua lista de mods. A seta ao lado abre o menu:

- **Modos de início**: com depuração (`-debug`, a Central de Monitoramento acompanha o log ao vivo) ou o jogo normal, cada um em tela cheia ou em janela.
- **Para onde**: **Menu principal**, como sempre, ou **Personalizado…**, que abre **Início personalizado** para ir direto a algum lugar:
  - **Um save**: os seus saves de um jogador, os do perfil ativo primeiro. O jogo abre e, no menu principal, o mod Studio Link carrega aquele save, com os mesmos avisos e backup de **Continuar**. Depois vem a tela preta "Clique para Iniciar" do jogo.
  - **Um servidor**: os seus servidores guardados. Entra como **Entrar** em Servidores, com a senha guardada e o seu nome de usuário; o jogo carrega os mods do servidor.
  - **Como** escolhe o modo de início. **Escolher** o define como o destino do botão; **Escolher e iniciar** também inicia o jogo. **Voltar ao menu principal** o limpa.

Com um destino escolhido, o botão diz qual é (**Jogar: (save)** ou **Entrar: (servidor)**) e o lembra, como o modo, enquanto **Lembrar como o jogo inicia** estiver ligado nas Configurações.

## O mod Studio Link no jogo

O PZ Mod Studio Link (`Z_StudioLink`) é um pequeno mod para a Build 42 que o Studio instala e mantém ativado em todos os perfis. Ele é o lado do app dentro do jogo. Uma parte funciona sozinha; o resto precisa do app.

### A própria tela de Mods dele

O botão **MODS** do menu principal abre a tela de Mods do Link em vez da do jogo. Sozinha, ela oferece:

- todos os mods instalados com uma caixa de busca (nome, ID do mod, ID da Oficina, autor) e filtros (**Todos**, **Ativados**, **Desativados**, **Favoritos**, **Atualizados**, **Problemas**, **Mapas**);
- os detalhes de cada mod: o que ele requer e quem precisa dele, as builds dele, a página dele na Oficina;
- ativar e desativar mods junto com o que eles requerem, vários de uma vez com Ctrl ou Shift;
- a ordem de carregamento: ordenar por ela, subir e descer, ou arrastar, nunca acima do que um mod requer;
- grupos por autor ou categoria, as opções de cada mod (**Opções do mod**), a **Ordem dos mapas**, e **Desfazer** / **Refazer**;
- **Arquivos compartilhados**: quando dois mods ativados trazem o mesmo arquivo Lua, o jogo roda só uma cópia, a do mod que carrega depois. O mod cuja cópia é usada fica marcado como **Sobrescreve**, e aquele cuja cópia não é usada como **Sobrescrito**; o botão de resumo **N compartilham arquivos** os lista (a dica dele explica as marcas) e os detalhes de cada mod citam os arquivos e o outro mod. Verificado com os mods que o jogo carregou, alguns segundos depois que o menu principal aparece; só os arquivos Lua são olhados;
- **Recarregar mods** faz o jogo ler as pastas de mods de novo, para que um item da Oficina em que você se inscreveu (ou que atualizou) com o jogo rodando apareça sem reiniciar; o jogo recarrega o Lua, como depois de **Aceitar**;
- **Linhas do servidor** copia os mods ativados como as linhas `Mods=`, `WorkshopItems=` e `Map=` de um servidor dedicado, na ordem de carregamento (cada mod depois do que ele requer, os mapas na ordem dos mapas), para colar por cima dessas linhas do `.ini` do servidor em `Zomboid/Server`; os mods sem item na Oficina são citados numa linha de comentário, para copiar para a própria pasta de mods do servidor;
- **Aceitar** salva na lista que o jogo lê, como faz a própria tela do jogo. **Tela do jogo** abre a original.

Com o app, a mesma tela também mostra só os mods do perfil (**Mods do perfil** / **Todos os instalados**), os avisos do Diagnóstico em cada mod, os mods atualizados desde a sua última sessão (nos detalhes deles, as notas de alteração mais novas do autor e se a atualização pode quebrar um save), as suas notas e grupos, e um botão para adicionar mods novos que não estão em nenhum perfil. **Sincronizar com o PZ Mod Studio** traz o que o app mudou, e **Abrir o PZ Mod Studio** traz o app para a frente.

Se outro gerenciador de mods que toma conta da tela de Mods estiver ativado, o Link pergunta qual tela você quer e a deixa para o outro se você disser.

### O cartão no menu principal

Quando o app já rodou neste PC, um cartão no menu principal mostra se o app está aberto ou fechado, o perfil ativo e o número de mods dele, os erros e avisos do Diagnóstico, e **Continuar: (save)** com o que mudou nos mods dele desde a última sessão. Se o app nunca foi instalado, o cartão convida você a baixá-lo, com **Baixar** (copia o endereço da página de download) e **Ocultar**.

### O aviso de erro

Quando o jogo registra um erro de Lua, um aviso diz de qual mod ele veio (e qual mod o chamou), com a última mensagem. **Copiar relatório** coloca o erro na área de transferência para mandar ao autor do mod, **Mostrar em Mods** abre a tela de Mods naquele mod, e **Ver no Studio** abre a Central de Monitoramento no cartão dele. O aviso fica quieto quando outro mod que já mostra esses erros está ativado. **Todos (N)**, e **Erros de Lua (N)** no menu de pausa, abrem todos os erros da sessão, cada tipo uma vez com quantas vezes apareceu, o mod e arquivo:linha: um clique copia um, **Copiar todos** copia todos com a build do jogo, o modo e quantos mods estão ativados. Um relatório cita cada mod com o ID, a versão e o item da Oficina.

Todo erro também vai para `Zomboid/Lua/PZModStudio_LinkErrorLog.txt` em até um segundo depois de acontecer, então um crash, uma desconexão ou voltar ao menu não faz você perdê-lo: cada tipo uma vez por inteiro (mod, arquivo:linha, mensagem, stack) e as repetições contadas, sob um cabeçalho com a data, a build e se foi no menu ou em qual save, com uma linha onde o jogo voltou ao menu principal ou o servidor o derrubou. O arquivo guarda mais ou menos as últimas 3000 linhas. **Log de erros** na lista abre a pasta dele e copia o caminho.

Para autores de mods: `PZModStudioLink.registerReport("YourModId", fn)` (ou `PZModStudioLinkReports["YourModId"] = fn` a partir de um Lua que roda antes do do Link) adiciona as suas próprias informações aos relatórios. `fn(err)` devolve uma string, uma lista de linhas ou uma tabela de pares nome = valor; só roda quando um relatório é feito, dentro de `pcall`. O texto dele vai para **Copiar todos**, e para **Copiar relatório** e o log de erros num erro do seu mod. Os relatórios de depuração de outros mods, os que os mods já registram para ferramentas de erro, vão para os mesmos lugares com o nome que cada mod deu a eles.

O contador vermelho **ERROR** do próprio jogo, no canto inferior direito, é desenhado pelo Java do jogo, que o Lua não alcança: o Link não consegue escondê-lo.

### Entrando num servidor

Quando o jogo derruba a conexão porque um mod da Oficina é diferente do do servidor, o Link diz qual é o mod (o jogo só dá um número), diz o que fazer em cada caso (o servidor atualiza quando reiniciar, ou a Steam precisa baixar a versão nova) e oferece **Copiar links da Oficina**. Quando a build do jogo é diferente da do servidor, ele avisa com as duas builds.

Ao hospedar (**HOSPEDAR**, **Iniciar**), o jogo carrega a própria lista de mods daquelas configurações de servidor, não a da tela de Mods. Se essa lista não tem mods que você tem ativados, ou cita mods que este PC não tem, o Link pergunta antes: **Sim** hospeda mesmo assim, **Não** volta (**Gerenciar configurações** muda os mods dela).

### Funções que dois mods mudam

Quando o Diagnóstico vê dois mods redefinindo a mesma função, o Link pergunta ao jogo, alguns segundos depois do menu principal e depois que um save carrega, de qual arquivo vem a função que ficou. O Diagnóstico mostra isso como **Visto no jogo rodando**.

### Com e sem o app

- Sem o app: a tela de Mods do Link (com **Linhas do servidor**), o aviso de erro e o log de erros dele (sem **Ver no Studio**), e o cartão de convite.
- Com um perfil ativo, mesmo com o app fechado: a tela de Mods mostra o perfil como o app o deixou da última vez; na tela de carregar, cada save é marcado conforme os mods dele batem ou não com o perfil; um save que usa mods fora do perfil não carrega; um save com a lista de mods vazia pergunta antes.
- Só com o app aberto: backups de um save antes de carregar com mods alterados e quando você sai dele, jogar um save numa cópia, adicionar mods novos ao perfil, **Sincronizar**, **Abrir o PZ Mod Studio** e **Ver no Studio**.
- Notas, grupos, perfis e o Diagnóstico são feitos no app; o jogo só os mostra.
