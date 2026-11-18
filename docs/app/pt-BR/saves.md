# Saves

Os seus saves do Project Zomboid e os backups deles. O Studio faz backup de um save sozinho nos momentos em que ele corre mais risco; aqui você vê esses backups, faz um você mesmo, guarda um para sempre ou coloca um de volta.

## Os saves de cada perfil

Cada perfil tem os próprios saves. Um save pertence ao perfil que estava ativo da última vez que você o jogou com o Studio aberto; um save jogado sem o Studio só pertence a um perfil se os mods desse perfil forem exatamente os do save. Um save nunca é mudado para outro perfil.

- Esta tela mostra os saves do perfil ativo. **Mostrar todos (mais N)** adiciona os saves dos outros perfis e os sem perfil, cada um com uma etiqueta **Perfil: <nome>** ou **Sem perfil**. **Só os de <nome>** volta para os saves do perfil ativo.
- O save de um servidor aparece com o endereço dele (`ip:port`) e uma etiqueta **Online**, e pertence ao perfil com que você entrou nesse servidor pela tela **Servidores** (o Link não roda num servidor, então o Studio se guia pela própria lista de servidores).
- No jogo, com o mod Studio Link: a tela **Carregar** lista os saves do perfil ativo, com **Mostrar todos** ao lado da caixa de busca, e **Continuar** carrega o save mais novo do perfil ativo (fica escondido quando o perfil não tem nenhum). Sem o Studio, o jogo lista todos os saves como sempre.

## O que você vê

- No topo, **Abrir pasta de backups** abre a pasta para onde vão os backups (`Zomboid/PZModStudio_SaveBackups`), e a seta redonda atualiza a tela.
- Uma barra com o espaço no disco: quanto está livre, o tamanho do disco e quanto os backups usam. Ela fica âmbar com **Pouco espaço livre** quando o seu maior save pode não caber mais duas vezes.
- Uma linha por save: o nome do mundo, depois o modo de jogo, o tamanho, o número de arquivos, o número de mods e quando mudou pela última vez. Uma etiqueta **feito na <build>** diz qual build do jogo gravou o save; uma build mais nova o atualiza quando você o carrega.
- Quando os mods do save mudaram desde a última sessão dele, uma linha âmbar avisa: **Desde a última sessão (...)**, com quantos mods foram adicionados, tirados, atualizados, ou se a ordem mudou. Passe o mouse por cima para ver os nomes dos mods.
- Quando a lista de mods do save cita mods que não estão instalados, uma linha vermelha avisa: **Não vai carregar: N mods da lista dele não estão instalados**, com um botão por mod. Clique em um para abri-lo em **Explorar Mods** (o item da Oficina quando a lista o cita, senão uma busca pelo ID dele). Os saves de servidores não guardam lista própria: o jogo baixa os mods do servidor quando você entra.
- Quando os próprios mods do save trazem mapas que o save não carrega, uma linha âmbar avisa: **Não carrega N mapas dos mods dele**, com uma etiqueta por pasta de mapa. Um save guarda a lista de mapas com que foi feito e o jogo nunca adiciona nada a ela, então um mod de mapa ativado depois carrega sem o mapa (terreno vazio onde deveriam estar as células dele). Com o jogo fechado, **Adicionar os mapas ao save** faz backup do save, coloca esses mapas na lista dele e move o terreno vazio já gerado nas células deles para a pasta de backups, depois de uma confirmação.
- **Personagem e veículos** abre o que o save tem, lido dos próprios arquivos dele: cada personagem com nome, posição e se está morto, os itens que carrega, e cada veículo com a posição, a distância até o personagem e quantos itens tem dentro. As contagens de itens são aproximadas.
- À direita de cada linha, o estado dela:
  - **Backup em dia**: o backup mais novo é igual ao save.
  - **Mudou desde o backup de ...**: o save mudou depois do último backup.
  - **Sem backup**: ainda não há nenhum.
  - **Fazendo backup...** enquanto um backup está rodando.

## Quando o Studio faz um backup

- **Quando você saiu**: quando você volta ao menu principal, ou o jogo fecha enquanto você está no save.
- **Antes de carregar com mods alterados**: o mod Studio Link pede o backup quando você carrega um save cujos mods não são os da última sessão dele, ou foram atualizados desde então.
- **Antes de mudar a lista de mods**: quando a lista de mods do save está para ser mudada no jogo, ou antes de **Adicionar os mapas ao save**.
- **Antes de uma restauração**: o save como estava, logo antes de você restaurar um backup mais antigo.

Um save que não mudou desde o backup mais novo não é compactado de novo.

## Fazendo um backup você mesmo

Clique em **Fazer backup agora** na linha. O backup é um zip que é lido de novo depois de gravado; backups conferidos mostram **conferido**. Se o save não mudou, nenhum zip novo é feito: em vez disso, o backup mais novo é marcado como guardado. O botão fica cinza quando o disco não tem espaço para o save mais uma folga.

## Os backups de um save

Clique na seta à esquerda de uma linha para abrir os backups dela. Cada um mostra quando foi feito, por quê (**feito por você**, **quando você saiu**, ...), o tamanho do zip, o tamanho do save, arquivos e mods. Para cada backup:

- O alfinete o guarda: um backup guardado nunca é apagado para liberar espaço. Clique de novo para ele voltar a ser um backup automático.
- **Restaurar** coloca aquele backup de volta como o save, depois de uma confirmação.
- O botão da lixeira (**Apagar este backup**) o apaga, depois de uma confirmação. Isso não pode ser desfeito.

## Restaurando um save

1. Feche o Project Zomboid. A restauração é recusada enquanto o jogo está rodando.
2. Abra os backups do save e clique em **Restaurar** no que você quer.
3. Confirme. O Studio confere o backup, faz backup do save como está agora (para você poder desfazer), descompacta o backup ao lado do save e só então troca os dois.

## Bom saber

- Os 5 backups automáticos mais novos de cada save são mantidos; os mais antigos vão embora quando um novo é feito. Os backups que você faz ou fixa com o alfinete ficam até você apagá-los.
- Os backups automáticos precisam do PZ Mod Studio aberto enquanto você joga, e do mod Studio Link ativado no jogo (ele avisa o app quando você entra e sai de um save).
- O save de um servidor ganha backup quando o jogo fecha com o Studio aberto, se ele mudou durante aquela sessão.
- Com o app fechado, o mod Link avisa antes de carregar que não dá para fazer backup do save, e deixa você continuar sem backup.
- O mod Link também pode jogar um save numa cópia (`<world>_copy`), que o app faz; o original fica como está.
