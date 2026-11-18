# Desenvolvedor

A tela Desenvolvedor é para quem faz mods. Você pode criar um mod novo, verificar todos os arquivos de um mod antes do jogo, iniciar o jogo só com esse mod ativado, recarregar o Lua dele com o jogo rodando, verificá-lo para multijogador e contra a build instalada, escrever rascunhos de tradução, publicá-lo na Oficina Steam e abrir os editores de mapa.

## O que você vê

A tela só aparece na barra lateral quando **Recursos de desenvolvedor** está ligado em **Configurações do App** e, como as outras telas, quando há um perfil ativo. No topo, **O mod em que você está trabalhando** guarda o mod com que todas as ferramentas trabalham, com **Abrir a pasta dele** e **Iniciar o jogo só com este mod**. Abaixo há uma aba por ferramenta: **Verificar arquivos**, **Recarregar ao vivo**, **Multijogador**, **Contra a build**, **Traduções**, **Oficina**, **Mapas** e **Mod novo**. Uma aba mostra um pequeno selo quando tem algo a dizer (problemas encontrados, observando, o veredito de multijogador). O Studio lembra a última aba que você abriu. Todas as abas, menos **Oficina**, **Mapas** e **Mod novo**, precisam de um mod escolhido antes.

## Criando um mod novo

1. Abra a aba **Mod novo**.
2. Preencha **Nome**, **ID** (a pasta e a entrada na lista: letras, números, `_` e `-`; ele acompanha o nome até você mudá-lo), **Autor** e **Descrição**.
3. Marque **Build 41 também** se o mod também deve funcionar na Build 41.
4. Clique em **Criar**.

O Studio cria o mod em `Zomboid/mods` do jeito que a Build 42 o lê: `42/mod.info`, as pastas `42/media/lua/client`, `server` e `shared`, `42/media/scripts`, `common/media/lua`, um pôster para substituir e um arquivo Lua que imprime o nome do mod quando o jogo o carrega. Ele recusa um ID que não seja um nome simples ou uma pasta que já existe. **Abrir a pasta** a abre no Explorador do Windows, e o mod novo é escolhido como aquele em que você está trabalhando. Ative-o na Lista de Mods quando estiver pronto.

## Escolhendo o mod em que você trabalha

Em **O mod em que você está trabalhando**, escolha um mod na lista. Os mods ficam agrupados pela origem, com os que você desenvolve primeiro: **Seus envios para a Oficina (Zomboid/Workshop)**, **Pastas em Zomboid/mods**, **Instalados do Nexus, de um arquivo ou de um link**, as cópias que o Studio adaptou à sua build, **Os mods do próprio Studio**, e **Inscrições na Oficina** por último, para dar uma olhada. Abaixo da lista você vê o ID do mod, a versão, o número da Oficina e a pasta. **Abrir a pasta dele** abre a pasta do mod. O Studio lembra a sua escolha.

## Verificando todos os arquivos

Na aba **Verificar arquivos**, clique em **Verificar todos os arquivos**. Cada arquivo `.lua` passa pelo mesmo analisador de Lua que o Mesclador de Mods usa, e cada `media/scripts/*.txt` por uma checagem de chaves (o jogo perde em silêncio um arquivo de script inteiro no primeiro bloco desbalanceado). O resultado aparece logo abaixo do botão, com quantos arquivos Lua e de script foram verificados. Ele diz **Todos os arquivos estão legíveis**, ou lista os problemas por arquivo com linha e coluna. Clique num problema para abrir o arquivo naquela linha no VS Code, quando ele está instalado, ou senão no programa que o Windows usa para aquele arquivo.

Isso só verifica a sintaxe. O que o jogo faz com os arquivos aparece no Monitor.

## Iniciando o jogo só com este mod

**Iniciar o jogo só com este mod**, no topo da tela, abre o jogo em Debug (em janela) só com este mod, o que ele requer e a ponte do Studio ativados. A sua lista de mods é deixada de lado, e uma faixa avisa disso. Ela volta quando o jogo fecha, ou na hora com **Devolver a minha lista agora**. A lista também fica guardada em **Listas anteriores** na Lista de Mods. O botão fica desligado enquanto um início desses está rodando.

## Recarregando no jogo com ele rodando

Isso deixa você mudar um arquivo Lua e ver o resultado no jogo sem reiniciar.

1. Inicie o jogo com o mod da ponte (`Z_PZModStudio_Bridge`) ativado.
2. Abra a aba **Recarregar ao vivo** e ligue o interruptor à direita. Ele diz **Observando (nome do mod)**.
3. Salve um arquivo Lua do mod. O Studio o vê no momento em que é salvo, o verifica com o analisador e o envia para o jogo, que o recarrega no menu principal, dentro do jogo ou com ele pausado.

**Recarregar todo o Lua deste mod agora** envia todos os arquivos Lua do mod de uma vez.

O log abaixo dos botões mostra cada passo:

- **alterado:** um arquivo foi enviado para o jogo.
- **não enviado, não passa na análise:** o arquivo tem um erro de sintaxe, com a linha, a coluna e a mensagem. Ele não é enviado, então o jogo nunca roda código quebrado.
- A resposta do jogo para cada arquivo, em verde quando funcionou e em vermelho quando falhou.
- Os tempos: detectado, enviado, aplicado no jogo e de volta no Studio, em milissegundos.
- **O jogo não está rodando** ou **Sem resposta do jogo em 6 s** quando não havia onde recarregar (confira se o mod da ponte está ativado).

Quando um arquivo é recarregado, o código de nível superior dele roda de novo, e a ponte preserva o que isso quebraria:

- **Handlers de eventos**: cada um que o arquivo adiciona roda uma vez, um handler que você adiciona pela primeira vez começa a funcionar, e um que você tira do arquivo para.
- **Tabelas**: quando o arquivo cria de novo uma tabela global (`MyMod = {}`, `MyPanel = ISPanel:derive(...)`), a tabela antiga é mantida e recebe as funções novas, então janelas abertas e referências guardadas rodam o código novo com o estado delas. Um valor que a tabela antiga já tinha mantém o valor antigo (a ponte não consegue distinguir o seu estado de uma constante que você mudou): para uma constante alterada, use um Reset Lua completo.
- **A sua própria preparação**: `if PZStudio then PZStudio.onReload(function(file) ... end) end` roda depois de cada recarga daquele arquivo, para o que o `OnGameStart` fazia e não faz de novo.
- **Arquivos que fazem `require` dele**: os outros arquivos do mod que carregam o arquivo alterado com `require` são recarregados logo depois dele, para pegarem o código novo.

O log diz o que foi mantido, por exemplo "1 handler(s), 1 table(s) kept".

## Verificação de multijogador

Na aba **Multijogador**, **Verificar este mod para multijogador** procura no Lua de cliente e compartilhado do mod chamadas que mudam o mundo, um veículo, um animal ou o mapa diretamente, onde o código do próprio jogo pede ao servidor. Cada achado mostra o arquivo, a linha e o handler do jogo que faz isso no servidor. Uma chamada dentro de um ramo `if isClient()` conta como tratada. Clique num achado para abri-lo no seu editor. Ele só verifica essas chamadas: um mod sem nada na lista ainda pode se comportar mal no multijogador de jeitos que isto não enxerga.

## Contra a build instalada

Na aba **Contra a build**, **Verificar os nomes que esta build tem** procura cada nome que o Lua do mod chama no jogo que você tem instalado: as classes e métodos Java dele, as funções globais, o próprio Lua e os eventos que ele dispara, mais o que este mod e os seus outros mods ativos definem. O que sobra é uma chamada que falharia naquela linha: uma função que a Build 42 tirou, um método renomeado, um evento que não dispara mais, um erro de digitação. Os achados ficam agrupados em funções globais, métodos e eventos, cada um com o arquivo e a linha, e um "você quis dizer" quando um nome conhecido é parecido. Clique em um para abri-lo no seu editor. Uma chamada que o arquivo também cita sem chamar (`if foo then`) conta como protegida e fica de fora.

A primeira verificação lê os arquivos do jogo e leva alguns segundos; depois disso fica guardada por build. Ela vai só pelo nome: um nome montado com o jogo rodando não é visto, e um mod que traz Java pode definir nomes que o Lua não mostra (o resultado avisa). Abra a linha para avaliar.

## Rascunhos de tradução

A aba **Traduções** mostra o idioma em que o mod está escrito, quantos textos ele tem e os idiomas dele em grupos: os que têm textos faltando, os completos e as pastas do mod que não são idiomas do jogo. Marque os idiomas que você quer e clique em **Escrever rascunhos para…**: o Studio escreve um rascunho de cada texto que falta em cada idioma, traduzido pelo mesmo serviço que traduz as descrições dos mods (**Tradução** em **Configurações do App**), na própria pasta `Translate/<LANG>/` do mod. O que já está lá fica como está. **Abrir a pasta Translate** a abre. Um rascunho é um começo, não uma tradução: revise antes de publicar.

## Publicando na Oficina

A aba **Oficina** envia um mod para a Oficina Steam pelo cliente da Steam aberto neste PC. Ela usa os mesmos arquivos que o próprio enviador do jogo, em `Zomboid/Workshop/<item>/`: `workshop.txt`, `preview.png` e `Contents/mods/<mod>/`.

1. Escolha um item na lista. Para um mod seu em `Zomboid/mods` que ainda não tem item, clique em **Preparar (mod) para a Oficina**: isso cria a pasta do item com uma cópia do mod, um `workshop.txt` a partir do `mod.info` dele e o pôster dele como prévia.
2. Preencha **Título**, **Visibilidade** (**Público**, **Apenas amigos**, **Privado** ou **Não listado (por link)**), **Tags (separadas por ;)** e **Descrição** (a Steam aceita até 8000 caracteres), e clique em **Salvar workshop.txt**.
3. **Ver os arquivos** mostra o que seria enviado e o que mudou desde o último envio pelo Studio. Um `.workshopignore` na pasta do item (um padrão por linha, como `*.psd` ou `.git/`) deixa arquivos de fora.
4. Escreva uma nota de alteração se quiser, e clique em **Publicar na Oficina** (ou **Enviar a atualização** para um item já publicado). Você é consultado antes. A Steam precisa estar aberta e conectada na conta dona do item.

O que é enviado é o que o `workshop.txt` diz: uma edição feita na página da Steam é substituída pelo próximo envio, do jeito que o jogo faz. Se a Steam deixar um item novo oculto até você aceitar o contrato legal da Oficina, o resultado avisa, com um link. **Ver a página da Oficina** abre o item no seu navegador. O Studio não envia o mod de outra pessoa: quando o item traz um mod em que você está inscrito, ele avisa e o botão fica desligado. O botão da lixeira ao lado da lista tira a pasta do item de `Zomboid/Workshop` (para a Lixeira); nada muda na Steam.

## Ferramentas de mapa

Na aba **Mapas**, o TileZed desenha os tiles de cada célula (prédios, cômodos, móveis, chão); o WorldEd coloca as células na grade do mundo e grava os arquivos de mapa que o jogo lê. Eles vêm da The Indie Stone num zip só, que você descompacta em qualquer lugar. O Studio os procura nos lugares de costume (a sua pasta de usuário, Documentos, Área de Trabalho, Downloads, o disco C:) e na pasta que você digitar na caixa abaixo de cada um. Quando encontra um, **Abrir o TileZed** ou **Abrir o WorldEd** o inicia a partir da própria pasta. As células de um mod de mapa ficam em `media/maps/<name>` do mod.

## Bom saber

- Recarregar precisa do jogo rodando com o mod da ponte ativado; as outras ferramentas não precisam do jogo.
- Recarregar no jogo funciona sem reiniciar; uma constante que você mudou numa tabela mantém o valor antigo até um Reset Lua completo.
- Todas as ferramentas, menos **Oficina**, **Mapas** e **Mod novo**, precisam de um mod escolhido cuja pasta o Studio conheça.
