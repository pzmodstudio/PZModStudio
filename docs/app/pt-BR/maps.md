# Mapas

Mapas mostra o mundo do jogo e todos os mods de mapa instalados numa só imagem, para você ver que partes de Knox Country um mapa substitui e onde dois mods de mapa brigam pelo mesmo terreno. Também lista os mods de mapa que não adicionam terreno próprio e avisa sobre conflitos de tiles.

## O que você vê

- O mapa real do mundo do jogo, com as células de cada mod de mapa desenhadas por cima, cada um na sua cor.
- À direita, a lista de mods de mapa que adicionam células.
- Acima do mapa, um aviso para cada par de mods de mapa que compartilham células ou números de tile.
- Embaixo, **Outros mods de mapa**: pontos de spawn, listas de mapas e pacotes de tiles.

O mundo é dividido em células; uma célula tem 256 x 256 tiles.

## Navegando pelo mapa

- Clique no mapa primeiro, depois gire a **roda** do mouse para aproximar e afastar. O zoom é feito no ponto que está embaixo do ponteiro. Até você clicar, a roda rola a página (**Clique no mapa para usar o zoom com a roda** lembra você disso), e volta a rolar quando o ponteiro sai do mapa.
- **Segure o botão esquerdo e arraste** para mover o mapa.
- Dê um **clique duplo** no mapa, ou clique no botão do canto superior direito dele (**Ver o mapa inteiro**), para ver tudo de novo.
- Deixe o ponteiro sobre uma célula para ver o número dela, se ela faz parte do mundo do jogo e quais mapas a cobrem (**Mais de um mapa aqui** quando dois a compartilham).

A imagem é montada com as imagens de mapa que o jogo e os mods trazem. Enquanto isso acontece, você vê **Montando o mapa…**. Um mod de mapa que não traz imagem ainda mostra as células dele como quadrados coloridos.

## O que as cores querem dizer

- **O mundo do jogo**: o mapa base, com uma grade clarinha por cima.
- **A cor de cada mod de mapa**: as células dele. O quadradinho ao lado do nome dele na lista mostra a cor. As células de um mod de mapa por cima do mundo do jogo substituem o que havia ali.
- **Vermelho**: células que dois mods de mapa compartilham. Ali, um deles substitui o outro no seu jogo, a não ser que um seja construído em cima do outro.

## A lista de mods de mapa

Cada linha mostra o nome do mapa, o mod de onde ele vem, quantas células ele tem, quantas estão por cima do mundo do jogo (ou **fora do mundo do jogo**), em quais mapas ele **se baseia**, e **desativado** se o mod estiver desativado.

- Desmarque a caixa de uma linha para esconder aquele mapa da imagem. Isso só muda a visualização, não a sua lista de mods.
- Clique no ícone de lista de uma linha (**Ver na Lista de Mods**) para ir até aquele mod na Lista de Mods, destacado.
- Marque **Só mods de mapa ativados na minha lista** para mostrar só os mapas que você ativou. Isso também esconde os avisos e os outros mods de mapa que só dizem respeito a mods desativados. A marcação fica lembrada.
- O botão de atualizar no topo lê os mapas de novo. Eles também são lidos de novo quando a sua lista de mods muda.

## Conflitos

- Um aviso **vermelho** quer dizer que dois mods de mapa compartilham células e os dois estão ativados: no seu próximo jogo, um substitui o outro ali. Abaixo dele, cada mod tem um botão para vê-lo na Lista de Mods e um botão **Desativar** que o desativa (depois de perguntar).
- O que só entraria em conflito se você ativasse mods que hoje estão desativados fica numa linha cinza só (**N possíveis conflitos se você ativar mods hoje desativados: nada a fazer agora**). Abra para ver cada um.
- Um aviso de definição de tiles quer dizer que dois mods usam o mesmo número de definição de tiles. Se os dois estiverem ativados, os tiles de um sobrescrevem os do outro e você ganha paredes, pisos ou móveis errados. Deixe só um ativado. Abaixo dele, cada mod tem um botão para vê-lo na Lista de Mods, e **Desativar** quando mais de um está ativado.

## Outros mods de mapa

Estes mods são sobre mapas mas não adicionam células: pontos de spawn, uma lista de mapas para outro mapa, ou os tiles com que os mapas são construídos. Cada um tem a etiqueta **pontos de spawn / lista de mapas** ou **tiles**. Eles precisam que o mapa ou mod para o qual foram feitos esteja ativado. Clique em um para vê-lo na Lista de Mods.

## Bom saber

- Dois mapas que compartilham células não entram em conflito quando um cita o outro no `map.info` dele (`lots=`); a lista mostra isso como **se baseia em**.
- As imagens dos mapas são feitas uma vez e guardadas em `Zomboid/PZModStudio_Cache/maps/`, então a tela abre mais rápido da próxima vez.
- Nada nesta tela muda o seu jogo, exceto **Desativar**.
