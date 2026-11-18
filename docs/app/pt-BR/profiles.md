# Perfis

Um perfil é uma lista de mods com nome: quais mods fazem parte dela, quais deles estão ativados e a ordem de carregamento. Aqui você cria perfis, troca entre eles com um clique, compartilha e apaga. A maioria das outras telas fica bloqueada até haver um perfil ativo.

## O que você vê

- No topo, quantos perfis você tem salvos e o botão **Novo perfil**, que abre uma janela que guia você na criação em três passos.
- Um cartão por perfil salvo, o ativo primeiro. O ativo tem moldura verde, a etiqueta **Ativo** e o botão **Ver os mods**, que abre a Lista de Mods; os outros dizem **Inativo** e têm **Ativar (1 clique)**.
- Cada cartão mostra a descrição, quantos mods estão ativados (**Mods ativos**), o **Total** de mods na ordem dele e a build: **Build 41/42** quando você a definiu à mão, ou **Jogado na ...** com a build em que foi jogado pela última vez pelo Studio.

## Criando um perfil

Clique em **Novo perfil**: abre uma janela com três passos (**Nome**, **Quais mods**, **Conferir**), cada um com uma explicação no topo. **Próximo** e **Voltar** passam de um para o outro; Esc ou o X fecha sem criar nada.

1. Digite um **Nome do perfil** (obrigatório) e, se quiser, uma **Descrição (opcional)**.
2. Em **Quais mods**, escolha com o que o perfil começa:
   - **O que está ativado agora**: os mods ativados neste momento.
   - **Tudo o que está instalado**: todos os mods que o Studio encontrou.
   - **Escolher**: uma lista com caixa de busca e botões **Todos** / **Nenhum**; marque os mods que você quer.
   - **Nada ainda**: um perfil vazio. O Studio pede confirmação, porque trocar para ele desativa todos os mods. Com um perfil ativo, um botão **Encontrar mods em Explorar Mods** leva você até lá para se inscrever primeiro.
3. Em **Conferir** você vê o nome e quantos mods começam ativados. Clique em **Criar e ativar**.

O perfil novo é criado e ativado na hora, e mantém a ordem de carregamento que a sua lista já tem. Se o Studio ainda não encontrou nenhum mod (pasta do jogo ou do Zomboid não definida nas Configurações do App), ele recusa, para que um perfil não desative todos os seus mods por engano.

## Trocando de perfil

Clique em **Ativar (1 clique)** num cartão. O Studio então:

- grava os mods daquele perfil na lista que o jogo lê (`Zomboid/mods/default.txt`);
- marca o perfil como ativo e abre a Lista de Mods com a ordem dele;
- instala o mod PZ Mod Studio Link e o mantém ativado em todos os perfis, para que a tela de Mods do jogo mostre o perfil.

Se um mod do perfil precisa de um agente Java no arquivo de inicialização do jogo (o Zombie Buddy, por exemplo), uma janela mostra primeiro o que sai ou volta para esse arquivo. Clique em **Ativar ...** para seguir. Se o jogo estiver rodando, essa parte espera até o jogo fechar. Antes, uma cópia do arquivo de inicialização é guardada.

## Isolar este perfil

**Isolar este perfil** mantém separados, para aquele perfil:

- os **saves** de um jogador;
- as **opções do jogo**: gráficos, som e teclas;
- as **configurações dos mods**: o que cada mod salva na pasta Zomboid (as opções, as configurações, os dados dele).

Com o perfil ativado, o jogo usa só os dele, e com outro perfil, os daquele (ou os compartilhados, quando o outro não está isolado). Quando você o isola, escolhe quais saves compartilhados ele leva junto; o resto continua compartilhado. As opções e as configurações dos mods começam como uma **cópia** das que você tem agora, então o perfil começa igualzinho a antes e os outros perfis mantêm as deles.

Os **mods em si são os mesmos para todos os perfis**: eles não são copiados nem movidos, e não há nada para instalar de novo. Um mod (ou parte de um mod) que está ativado num perfil e desativado em outro funciona como sempre. Os saves de servidores (`Saves/Multiplayer`), os logs do jogo, os servidores e os arquivos do app e do Link também não são separados.

Dados grandes que não são configurações não são copiados quando você isola um perfil: um arquivo sozinho acima de 20 MB ou uma pasta acima de 50 MB (o cache de um mod, gravações ou dumps) fica **compartilhado**, igual para todos os perfis daí em diante, então isolar nunca enche o disco. As opções e teclas do próprio jogo são sempre copiadas, seja qual for o tamanho. O que ficou compartilhado é anotado em `Zomboid/PZModStudio_ProfileSavesShared.json`.

Ao trocar de perfil, o app coloca tudo no lugar renomeando, sem copiar nada. Ele só faz isso com o Project Zomboid fechado: o jogo, e também qualquer servidor do Project Zomboid neste PC (o servidor do Studio, da tela Servidores, ou um iniciado à mão). Se algum estiver rodando, o app diz qual e pede para você fechá-lo antes. O que pertence aos outros perfis espera em `Zomboid/PZModStudio_ProfileSaves`. Se o PC desligar no meio de uma troca, o app termina a troca da próxima vez que abrir.

Todo save e configuração que o app move, copia ou guarda nos backups é anotado no log do Studio (`Zomboid/PZModStudio_Logs/studio.log`, linhas com `[Profiles]`), de onde para onde, então se um save parecer sumido você pode ver para onde ele foi.

**Deixar de isolar** coloca os saves dele junto com os compartilhados e traz de volta as opções e configurações de mods compartilhadas; as próprias do perfil ficam guardadas em `Zomboid/PZModStudio_Backups/ProfileSettings`, nada sobrescrito. Apagar o perfil faz o mesmo, então nunca se perde um save ou uma configuração.

## Editando, compartilhando e apagando

- O botão do lápis (**Renomear este perfil**) abre **Editar perfil de mods**: mude o nome, a descrição e a **Build do jogo**. Deixe a build em **Como foi jogado por último** ou defina **Build 41** / **Build 42**. Com uma build definida, o Studio pergunta antes de iniciar este perfil em outra build, mesmo antes de ele ter sido jogado. Clique em **Salvar alterações**.
- O botão de compartilhar salva o perfil num arquivo `.pzpack` que você pode mandar para qualquer pessoa. Ele tem só os mods ativados, em ordem, com os IDs da Oficina. Quem abrir pode se inscrever no que estiver faltando.
- O botão da lixeira apaga o perfil depois de uma confirmação. Se alguns dos itens da Oficina dele não são usados por nenhum outro perfil, a confirmação é uma janela que os lista (**Só este perfil usa**), marcados, com **Marcar todos** / **Desmarcar todos**. **Apagar e cancelar inscrição** apaga o perfil e cancela a inscrição nos itens marcados (a Steam precisa estar aberta); **Excluir só o perfil** os mantém. Itens que outro perfil usa, e mods locais, nunca são mexidos. Se o perfil usa pacotes de patch do Mesclador de Mods (`Z_PZModStudio_...`), uma segunda pergunta quer saber se eles também devem ser apagados; **Cancelar** os mantém.

## Bom saber

- Um perfil apagado não se perde: o arquivo dele é movido para `Zomboid/PZModStudio_Backups/profiles`.
- Apagar o perfil ativo deixa nenhum perfil ativo, e a tela de Mods do jogo deixa de mostrar um perfil.
- As mudanças que você faz na Lista de Mods (ativar mods, reordenar) são salvas no perfil ativo na hora.
- As mudanças aceitas na própria tela de Mods do jogo também voltam para o perfil ativo.
- Um mod ativado passa a ser um dos mods do perfil. Os mods que não pertencem a nenhum perfil são oferecidos na Lista de Mods com **Adicionar ao perfil**.
- Os saves nunca passam de uma build principal para outra (41 e 42), e é por isso que um perfil lembra a build dele.
