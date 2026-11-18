# Lista de Mods

Os seus mods instalados: quais estão ativados e a ordem em que o jogo os carrega. Cada mudança feita aqui é salva na hora no perfil ativo e em `Zomboid/mods/default.txt`, a lista que o jogo lê.

## O que você vê

- O cabeçalho conta itens da Oficina, mods, mods **ativos**, e mods de mapa, biblioteca e script. Contadores vermelhos e âmbar (**Dependências faltando**, **Dependências desativadas**, **Incompatibilidade**, **Ordem da lista**) aparecem quando há um problema; clique em um para pular entre esses mods.
- A barra de ferramentas: busca, filtros, grupos e os botões **Ativar todos** / **Desativar todos**.
- A lista. A barra verde no topo (**CARREGA PRIMEIRO**) é onde ficam as bibliotecas; a barra âmbar embaixo (**CARREGA POR ÚLTIMO**) é onde um mod vence qualquer arquivo que dois mods compartilhem.
- À direita, o painel do mod que você escolheu (vazio até você clicar em um). Mais abaixo, em **O painel da direita**.

## O painel da direita

No topo, sempre à vista: a imagem do mod (clique para ver no tamanho real), se ele é um mod de mapa, biblioteca ou script, de onde ele vem (Steam, cópia compatível, local...), o nome, ID, autor e versão, e os botões **Apoie o autor** com os links que o autor colocou no changelog ou na Oficina (Ko-fi, Patreon...), abertos no seu navegador. Abaixo, um botão que o ativa ou desativa e diz onde ele carrega, e **Ver em Explorar Mods**.

Depois, três abas (a que você escolher fica quando você troca de mod):

- **Studio**: o que o app sabe sobre este mod na sua lista.
  - **Notas**: a sua nota inteira (**Editar nota**, ou **Adicionar uma nota**), os seus grupos e os perfis em que ele está, com o ativo destacado.
  - **O que fazer**: cada aviso da checagem com tudo o que ele diz, se você precisa agir, se vale a pena ou se é só bom saber, e o botão de correção. Sem nada a fazer, ele avisa em verde.
  - **Última sessão**: os erros ou avisos que o jogo registrou (com a última linha), quanto tempo levou para carregar e a parte dele no tempo do jogo.
  - **Dependências**: do que ele precisa e quais mods precisam dele; clique em um para ir até a linha dele.
  - **Detalhes dos metadados do mod** (o que o `mod.info` dele diz, com **O mais recente no changelog**) e **Classes que o Java dele altera**, recolhidos no fim.
  - **O que fazer** e **Última sessão** se recolhem com um clique no título e ficam como você os deixou.
- **Página da Oficina**: a página do mod sem sair do app: imagens e vídeos, inscritos, favoritos, tamanho, datas, autor, categorias, os itens que ele requer (verde quando você tem: clique para ir até a linha dele; âmbar quando não tem: abre em Explorar Mods) e a descrição inteira com **Traduzir**. Um mod que não está na Oficina mostra aqui a própria descrição.
- **Comentários**: os comentários da página dele na Oficina, dos mais novos para os mais antigos, com **Carregar mais**. Quando o perfil do autor é privado, a Steam não os mostra e o painel avisa.

## A coluna #

O número em cada linha é o lugar do mod entre os mods **ativados**: a ordem de carregamento que o jogo lê, e o mesmo número que o jogo mostra. Um mod desativado mostra **—** e fica cinza, porque não tem lugar na ordem de carregamento. Os mods desativados mantêm o lugar na lista, então ativar um geralmente o coloca de volta onde estava.

## Ativando e desativando mods

- Clique no quadradinho de uma linha, ou clique com o botão direito e escolha **Ativar** / **Desativar**.
- Quando um mod precisa de outros mods, uma janela lista o que é ativado junto com ele (e o que é desativado junto). Escolha **Ativar todos os N**, ou **Só ...** para aquele mod sozinho.
- **Ativar todos** e **Desativar todos** só agem nas linhas que aparecem agora (depois da busca e dos filtros), e perguntam antes.
- **Desfazer** / **Refazer** (as setas ao lado deles, ou Ctrl+Z / Ctrl+Y) andam para trás e para a frente nas mudanças feitas aqui: movimentos, arrastos, ativações, ordenações. Cada perfil tem os seus, mesmo depois de fechar o app.
- Ctrl+clique escolhe vários mods, Shift+clique escolhe um intervalo, Esc os solta. Clique com o botão direito nos escolhidos: ative ou desative, tire do perfil ou cancele a inscrição nos itens da Oficina deles.
- Um pacote rascunho aberto do Mesclador de Mods não pode ser ativado até ser empacotado.

## Mudando a ordem de carregamento

- Arraste uma linha pelo número e solte acima ou abaixo de outra linha. As linhas escondidas no meio são respeitadas.
- Os botões de seta à direita: mover para o topo, subir um, descer um, mover para o fim. Subir e descer trocam com o mod seguinte da lista completa, que pode ser um mod desativado.
- **Pos. #**: digite um número e clique em **Ir** ou aperte Enter. O mod vai para aquele lugar entre os mods ativados.
- **Ordenar dependências**: coloca cada mod depois do que ele requer e dos mods `loadModAfter` dele, antes dos mods `loadModBefore` dele, e depois dos mods que as suas próprias regras dizem. Todo o resto mantém a sua ordem; os pacotes do PZ Mod Studio vão por último. Uma janela diz quantos mods foram movidos, com **Desfazer**.

## Encontrando mods

- A caixa de busca procura no nome, no ID do mod, no ID da Oficina e nas suas notas.
- Os botões de filtro: **Todos**, **Ativados**, **Desativados**, **Favoritos**, **Atualizados**, **Problemas**, **Mapas**, **Bibliotecas**, **Não carregam**. Cada um mostra a contagem.
- Abaixo deles: **Ordenar: ordem de carregamento**, **Ordenar: nome**, **Ordenar: instalados mais recentes** ou **Ordenar: instalados mais antigos** (por quando cada mod foi instalado: quando a pasta dele foi criada; para um da Oficina, o primeiro download, que as atualizações não mudam; cada linha mostra a data e os mods não instalados vão por último), o menu **Grupos**, um dos seus grupos, a origem (**Oficina Steam**, **Seus envios para a Oficina**, **Instalados à mão**, **Cópias compatíveis**...), uma tag (erros ou avisos na última sessão, lento para carregar, pesado para o jogo, um problema do Diagnóstico) e **Mods do perfil** / **Todos os instalados**.
- A tela de Mods do jogo (mod Studio Link) tem os mesmos filtros na mesma ordem. Atualizados, Bibliotecas, o grupo, a origem e a tag precisam do PZ Mod Studio; sem ele, o jogo avisa onde eles estariam.
- Arrastar também funciona com a lista ordenada por nome: o mod vai para o lugar daquele em que você o solta.
- **limpar filtros** mostra todos os mods de novo.

## Grupos e notas

- O menu **Grupos** organiza a lista em seções que se recolhem: **Grupos: nenhum**, **Grupos: autor**, **Grupos: categoria** ou **Grupos: os meus**. Cada título mostra quantos estão ativados, com **Todos ativados** / **Todos desativados**. Em **Grupos: os meus**, um botão de nota edita a nota do grupo.
- Clique com o botão direito num mod e escolha **Grupos…** para marcar os grupos em que ele está, criar um novo (**Adicionar a ele**), renomear ou apagar um. Um grupo que fica vazio some. Os grupos nunca mudam a ordem de carregamento.
- Escolher um dos seus grupos mostra **Ativar o grupo** (com o que os mods dele requerem) e **Desativar o grupo**.
- Clique com o botão direito e escolha **Adicionar uma nota…** para escrever uma nota num mod (o painel da direita também faz isso). A linha dele mostra o começo da nota ao lado do nome: passe o mouse para ler tudo, ou clique para editar.
- Cada perfil tem os próprios grupos: um grupo pertence ao perfil em que você o criou. As notas são sobre um mod e são as mesmas em todos os perfis.
- Notas e grupos são só seus. O jogo os mostra (os grupos do perfil ativo) na tela de Mods do mod Studio Link.

## Os mods do perfil

Com um perfil ativo, a lista mostra os mods daquele perfil. O botão **Mods do perfil** muda para **Todos os instalados**, que lista todos os mods instalados e marca cada um com os perfis em que ele está. Os mods instalados que não estão em nenhum perfil (inscritos pelo site da Steam, por exemplo) ganham uma barra azul com **Adicionar ao perfil** (eles entram desativados), **Mostrar** ou o X para escondê-la.

## O menu do botão direito

**Ativar/Desativar**, **Adicionar ao perfil** / **Tirar do perfil** (tirar também o desativa), **Mover para o topo**, **Mover para o fim**, **Abrir em Explorar Mods** (**Procurar em Explorar Mods** para um mod que não está instalado), **Abrir a pasta dele**, **Adicionar uma nota…** (**Editar a nota…** quando ele já tem uma), **Grupos…**, **Regras de ordem…**, **Guardar esta versão**, **Versões…**, **Favoritar**, **Adicionar a uma lista…**, **Adicionar a um perfil…**, **Copiar o id do mod** e **Cancelar inscrição no item da Oficina** (a Steam apaga os arquivos dele; outros mods do mesmo item vão junto).

- **Regras de ordem…**: diga que um mod deve carregar depois de outro (coisas que você descobriu jogando). O Ordenar automático segue a regra e o Diagnóstico avisa quando a lista a quebra. Para passar a sua ordem para outra pessoa, exporte uma predefinição: ela leva a lista na ordem que as suas regras deram.
## Voltando para uma versão anterior

A Steam não guarda versões antigas de um mod da Oficina: uma atualização substitui os arquivos dele, e não há como baixar a anterior. Por isso o Studio guarda sozinho uma cópia da versão instalada de cada mod da Oficina que um perfil usa, antes que a Steam a substitua. Quando uma atualização quebra algo, a versão de antes já está neste PC.

- **Versões…** (botão direito; no Simples também) lista as versões guardadas do mod: quando cada uma foi guardada, o modversion dela, quando o autor a publicou, e tags: **instalada agora** (os mesmos arquivos que a Steam tem), **em uso**, **guardada pelo Studio** ou **guardada por você**.
- **Usar esta versão** a coloca em `Zomboid/mods` como uma cópia local que o jogo lê no lugar da da Oficina (quando iniciado pelo Studio). A pasta da Oficina não é mexida, então a Steam não desfaz isso. Recusado com o jogo rodando.
- **Voltar à versão da Oficina** (na mesma janela, ou a correção no Diagnóstico, onde o mod mostra **Versão guardada em uso**) move a cópia para os backups; o jogo volta a ler a da Oficina.
- O painel do mod avisa quando há uma versão anterior guardada ou uma em uso, com um botão **Versões…**.
- **Guardar esta versão** (Completo) guarda à mão o mod como ele está hoje: o limite de tamanho nunca a descarta.

As cópias ficam em `Zomboid/PZModStudio_Versions`, ou na pasta escolhida em Configurações, O app (**Escolher pasta…**, outro disco por exemplo). Cada arquivo é guardado uma vez só, então uma versão depois de uma atualização custa só o que a atualização mudou. O limite (10 GB no começo; o disco sempre fica com 15% do tamanho livre) e a chave ficam em Configurações, O app; passando do limite, as versões anteriores mais antigas vão embora primeiro, depois as cópias de mods que não estão no perfil ativo. As cópias feitas antes (`Zomboid/PZModStudio_Snapshots`) continuam listadas.

## Tags das linhas

Uma linha só mostra o que pede alguma coisa, para a lista continuar legível:

- Os avisos do Diagnóstico em que você precisa agir ou que valem a pena (como **Precisa de um mod desativado** ou **Carrega cedo demais**), com o botão de chave inglesa ao lado que aplica a correção. Clique na tag para ir até o mod relacionado ou ao Diagnóstico.
- Os que são só bom saber se juntam numa tag pequena com um **i** e quantos são; o painel da direita mostra todos.
- Erros ou avisos que o jogo registrou para o mod da última vez, um carregamento que levou mais de 1 segundo, e a parte dele no tempo do jogo a partir de 5 % (medida em Diagnóstico > Mods > Tempo por mod).
- De onde ele vem, só quando não é da Steam (**Local**, **Cópia compat.**...).
- Com **Todos os instalados**, um ícone de perfis com um número: **✓** quando ele está no perfil ativo (e **+N** quando está em outros também), ou em quantos perfis ele está. Passe o mouse para ver os nomes.

Os tempos de carregamento menores, a origem Steam e todo o resto ficam no painel da direita.

## Listas anteriores e predefinições

- **Listas anteriores** é o desfazer da lista. Cada vez que o Studio a gravou, a anterior foi guardada. Escolha uma e clique em **Voltar para esta**; a sua lista atual também fica guardada.
- O botão **Predefinições** salva a lista como uma predefinição com nome, carrega uma, ou exporta e importa arquivos `.pzpack`. Um pacote importado lista os mods que faltam para você, com **Inscrever-se em todos**.

## Bom saber

- **Atualizar** procura nas pastas de mods de novo depois que você se inscreve ou cancela uma inscrição.
- O mod PZ Mod Studio Link fica ativado em todos os perfis e escondido da lista enquanto um perfil está ativo.
- Com o jogo rodando, as mudanças feitas aqui chegam a ele no menu principal (pelo mod Studio Link), ou com **Sincronizar com o PZ Mod Studio** na tela de Mods dele.
