# Diagnóstico

O Diagnóstico junta o que o app sabe sobre os seus mods, o jogo e o servidor. Quatro botões no topo: **Visão geral**, **Mods**, **Jogo** e **Servidor**. Ele sempre abre na Visão geral; cada parte lembra a aba que você olhou por último.

## Visão geral e o Doutor

- Três cartões, um por parte, cada um dizendo como ela está numa linha. **Mods**: algo que quebra o jogo, algo que vale consertar, ou tudo em ordem. **Jogo**: aberto ou fechado, quanta memória usa, o último início, o limite de memória e se alguma sessão ficou sem memória. **Servidor**: se há um rodando neste PC e como ele está. Clique num cartão para abrir aquela parte.
- Abaixo deles, **o Doutor**: ele pergunta o que está errado (**O jogo fecha com erro ou não inicia**, **Um mod não funciona**, **Está lento**, **Outra coisa**) e leva você passo a passo com o que o app já sabe: o que a checagem de mods encontrou, os erros da Central de Monitoramento e as vezes em que o jogo ficou sem memória. Quando um passo manda você para outro lugar, ao voltar você encontra o mesmo passo. **Recomeçar** volta para a pergunta.
- O botão **Doutor** bem no topo abre esta página a partir de qualquer tela.

## Mods

Verifica os seus mods ativos do jeito que o jogo os lê, e lista cada problema que encontra com um botão para consertá-lo. Ele olha o que a build do seu jogo consegue carregar, requisitos faltando ou desativados, a ordem de carregamento, mods incompatíveis, arquivos que dois mods alteram e mods que encheram o console da última vez que você jogou.

- No topo: a **Build do jogo** e quantos mods estão ativos.
- **Notas do Studio** (por exemplo **Notas do Studio: 22 mods**): as regras de ordem de carregamento, os pares incompatíveis e os problemas conhecidos que o app traz dentro dele. Quando você marcou avisos com **Não acontece mais**, um botão ao lado diz quantos e mostra todos de novo.
- **Verificar de novo** roda a checagem de novo. Ela também roda sozinha quando a sua lista muda.
- Cinco abas; as três primeiras dizem quantas linhas pedem alguma coisa:
  - **O que fazer agora**: o que quebra ou vai quebrar o jogo (requisitos faltando ou desativados, ordem de carregamento, mods que a sua build não carrega, pares incompatíveis). **Corrigir N problema(s) automaticamente** fica aqui. Recolhido embaixo: o que o Studio já corrigiu e o que é só bom saber.
  - **Conflitos**: dois mods trazendo o mesmo arquivo, item, receita ou texto; o jogo usa um, e qual é decisão sua. **Mesclar N arquivos automaticamente** fica aqui.
  - **Segurança**: o que um mod poderia fazer além de modificar o jogo (abaixo).
  - **Auditoria**: a sua lista mod a mod, cada um com um veredito: **Quebrado**, **Escolher**, **Desnecessário**, **Conferir** ou **Manter**. Clique num mod para ver por quê, e em **Ir** para ir até o aviso.
  - **Tempo por mod**: quais mods deixam o jogo lento enquanto você joga (abaixo).
- Cada linha mostra o mod, o que está errado, um código curto (como `missing_dependency`) e os botões de correção à direita. **Copiar para um assistente** copia a linha com uma pergunta, para colar em qualquer assistente de chat; nada é enviado pelo Studio. Uma etiqueta de problema na Lista de Mods abre a aba que o tem.

Se não houver nada errado, você vê **Nenhum problema encontrado nos seus N mods ativos.**

### O que fazer com cada linha

Cada linha diz uma de quatro coisas, igual à Lista de Mods:

- **Você precisa agir**: o jogo quebra ou não carrega o mod até você fazer algo. Comece por aqui.
- **Vale fazer**: funciona, mas vale consertar (ordem errada, uma atualização ainda não baixada, um mod enchendo o console). O botão diz o que ele muda.
- **Bom saber**: nada a fazer, só para você saber (dois mods definem a mesma coisa, mods atualizados desde a última vez que você jogou).
- **Já corrigido pelo Studio**: deixe como está, é assim que funciona (uma cópia compatível, um arquivo no seu pacote). Cada um pode ser desfeito se você precisar.

Cada seção pode ser recolhida clicando no título.

### Os problemas que ele encontra

- `missing_mod` (Não instalado): um mod da sua lista não está no disco. Se um mod instalado tem quase o mesmo ID, ele oferece ativar esse no lugar.
- `not_for_build`, `outside_version_range`, `invalid_mod_info`: o jogo não carrega este mod na sua build, ou o `mod.info` dele está quebrado.
- `missing_dependency`: o mod requer outro que não está instalado.
- `dependency_not_enabled`: o mod necessário está instalado, mas desativado.
- `dependency_after`, `load_after_order`, `load_before_order`: dois mods estão na ordem errada.
- `dependency_unavailable`: um mod necessário não consegue carregar.
- `incompatible`: o mod diz que não funciona com outro mod que você tem ativado.
- `duplicate_entry`: o mesmo mod está na lista duas vezes.
- `two_mod_managers`: outro gerenciador de mods que desenha a tela de Mods do jogo está ativado junto com o Studio Link.
- `workshop_update_pending`: a Steam tem uma versão mais nova que ainda não foi baixada. Os servidores recusam jogadores com versões de mods diferentes.
- `file_conflict`: dois mods trazem o mesmo arquivo e o jogo só usa o último.
- `function_overridden` (**Corta outro mod**): um mod substitui uma função do jogo, ou de outro mod, depois que outro mod a alterou, então essa alteração nunca roda. Quando muitas funções de um mod são cortadas por outro (duas versões do mesmo mod ativadas ao mesmo tempo), é uma linha só. Com o Studio Link ativado, isso é verificado no jogo rodando: a linha diz **Visto no jogo rodando**, ou `function_seen_other` (**Outra versão no jogo**) aparece quando o jogo tem a versão de outro mod.
- `java_link_broken` (**Java mais antigo que esta build**): o Java do mod usa classes, métodos ou campos que o jogo instalado não tem mais (uma atualização os mudou). Esse código falha quando roda. Se o Monitor mostrar a falha, procure uma atualização do mod ou desative-o; se funciona, é porque o mod verifica antes.
- `tiledef_clash`: dois mods registram os tiles deles com o mesmo número de `tiledef` e folhas diferentes, então alguns tiles aparecem como os do outro mod ou somem. Deixe um ativado, ou peça a um autor para mudar o número. As linhas `file_conflict` também dizem se o log da última sessão (com esta mesma lista) confirma qual cópia o jogo usou.
- `game_folder_changed` (**A pasta do jogo mudou**): com o jogo rodando, arquivos da própria pasta dele mudaram, onde o jogo nunca grava. O app compara a pasta do jogo, a pasta Zomboid (sem saves nem logs) e a do ZombieBuddy quando o jogo abre e quando fecha, só com o app aberto.
- `script_definition_conflict` e `translation_conflict`: dois mods definem o mesmo item, receita ou nome de jeitos diferentes; vence o que carrega por último. Linhas terminadas em `_stale` querem dizer que você mesclou antes, mas um mod mudou desde então; `_settled` quer dizer que o seu pacote já resolve.
- `floods_console` (aparece na Lista de Mods como **Enche o console**): na sua última sessão, este mod escreveu mais de 1.000 linhas de erros no console. Isso deixa o jogo lento e esconde os erros de outros mods. Veja-os na Central de Monitoramento, ou desative o mod.
- `user_rule_order`: uma das suas próprias regras de ordem de carregamento está quebrada.
- `community_rule_order`, `community_incompatible`, `community_known_issue`, `community_successor`: o que as regras e o índice de mods do Studio dizem sobre os seus mods. Um problema conhecido tem **Não acontece mais**: ele não aparece mais neste PC.
- `learned_issue`: um erro que um mod deu em 3 ou mais das suas sessões com a mesma versão (veja "O que este PC aprende").
- `package_built_for_other_list`: o seu pacote do Studio foi montado com uma lista de mods diferente da deste perfil.
- `package_file_stale`: o seu pacote leva uma cópia antiga de um arquivo de um mod, e o mod mudou esse arquivo desde então. O jogo lê a cópia antiga.
- `package_file_differs`, `package_file_orphan`: o seu pacote leva um arquivo que só um mod da lista traz, ou que nenhum mod da lista traz. Tire-o se você não precisa mais dele.
- `security_executable`, `security_script`, `security_java`, `security_lua`: o que um mod poderia fazer além de modificar o jogo, na aba **Segurança** (abaixo).
- `compatible_copy`, `shadowed_copy`, `updated_since_last_play`: notas sobre qual cópia de um mod é usada e o que mudou.

As linhas sobre a mesma definição podem ser abertas com **Por arquivo** para ver o arquivo de cada mod. Clique numa definição ali para abri-la no Mesclador.

### Os botões de correção

- **Ativar X** ou **Desativar X**: ativa ou desativa um mod. Desativar pergunta antes; o mod continua instalado.
- **Mover X antes de Y**: muda a ordem de carregamento. Numa linha de "decisão sua" (dois mods definem a mesma coisa), ele diz **Usar a versão de Y**.
- **Tirar X da lista**: tira uma entrada da lista.
- **Procurar "X" na Oficina** e **Abrir página da Oficina**: abrem a Oficina Steam no seu navegador.
- **Criar cópia compatível**: faz uma cópia adaptada de um mod da Oficina em `Zomboid/mods` para a sua build. O resultado mostra o que mudou e o que ainda pode quebrar. Inicie o jogo pelo Studio para ele usar a cópia. **Voltar ao original** remove a cópia.
- **Guardar a versão que você tem**: salva o mod como ele está hoje, antes que chegue uma atualização da Oficina. Você pode colocá-lo de volta pela Lista de Mods (botão direito, Versões guardadas).
- **Mesclar automaticamente** e **Abrir no Mesclador**: resolvem um conflito de arquivo ou de definição no seu pacote de patch, que carrega por último.
- **Tirar do pacote**: tira um arquivo do seu pacote de patch, depois de perguntar; o jogo volta a ler o próprio arquivo do mod na próxima vez que iniciar.
- **Ligar o patch X do shim**: liga um patch do shim da Build 42 de que um problema conhecido precisa.

Uma linha verde confirma cada mudança, com **Salvo na sua lista de mods.**

### Consertando vários de uma vez

- **Corrigir N problema(s) automaticamente** ativa os requisitos que faltam e move mods para mais cedo onde for preciso. Ele nunca desativa nada e pula as linhas de "decisão sua". Ele pergunta antes, e a Lista de Mods pode desfazer.
- **Mesclar N arquivos automaticamente** mescla no seu pacote de patch cada arquivo em conflito que não tem mudanças sobrepostas. Os arquivos em que os dois mods mudam as mesmas linhas ficam listados para você decidir no Mesclador.

### As regras e o índice do Studio

O Studio traz regras de ordem de carregamento, pares de mods que não combinam e um índice de problemas conhecidos com a correção deles. Eles vêm com cada versão do app; nada é baixado e nada é enviado. Eles aparecem como linhas `community_*`. Um problema conhecido fica de fora sozinho quando os próprios arquivos do mod mostram que ele sumiu (uma atualização consertou o arquivo de que ele tratava), ou quando o mod mudou depois que o problema foi visto e a linha de log dele não aparece mais na sua sessão. **Não acontece mais** deixa um de fora à mão.

### O que este PC aprende

Além do índice com que vem, cada app aprende com o próprio PC. Em segundo plano, ele lê os logs de sessões do jogo e do servidor que o jogo guarda (`Zomboid/Logs`) e anota quais mods estavam ativados, em que versão, e quais erros o jogo culpou neles (os mesmos que o Monitor mostra). Ele guarda isso em `Zomboid/PZModStudio_Learned.json` e não envia nada.

Quando um mod dá o mesmo erro em 3 ou mais sessões, com a mesma versão e na build instalada, ele aparece no Diagnóstico como **Aprendido neste PC** (`learned_issue`), com a contagem como prova: "em 8 de 14 das suas sessões". Quando o índice do app já fala desse problema, só a linha do índice aparece. Um patch Java que o log culpa em vários mods ao mesmo tempo não é aprendido, porque não dá para saber de quem é. Quando o mod atualiza, a contagem volta a zero; quando as últimas 5 sessões dele passaram sem o erro (consertado, ou uma correção que funcionou), ele não aparece mais. **Não acontece mais** o tira à mão. A página inicial do Diagnóstico diz de quantas sessões ele aprendeu.


### O Doutor e o cérebro

A página inicial do Diagnóstico tem **Perguntar ao Doutor**, um chat que responde sem IA: ele entende o que você pergunta pelas palavras e monta a resposta com o que este PC aprendeu e com as ferramentas do app (as mesmas do servidor MCP). Cada resposta diz de que ferramenta veio. Pergunte o que ele sabe, por que o jogo fechou, o que mudou quando um erro começou, quais mods falham, se o carregamento está mais lento, se as correções funcionaram, o que a última atualização do jogo fez, ou diga o nome de um mod para ver o histórico dele.

Ele perdoa erros de digitação e outros jeitos de dizer ("crashou", "travou", "oq mudou"), entende datas ("o que mudou desde ontem?", "esta semana", "desde 12/11"), "qual demora mais para carregar?" ou "qual mod dá mais erros?", e acompanha a conversa: depois de um mod você pode dizer "e desde quando?", "desativa", "anota" ou "mostra". Quando está em dúvida, ele pergunta "qual destes?" com botões; quando não entende e você mostra com um botão o que queria dizer, ele lembra daquela frase neste PC. Cada resposta começa pela conclusão, depois um conselho com a prova dele (por exemplo, que quase todos os crashes foram o PC ficando sem memória com o jogo e um servidor abertos), os detalhes recolhidos e botões para ir à Lista de Mods ou à parte do Diagnóstico de que se trata.

Também:

- **Qual mod causa isso?** Ele compara as sessões com e sem cada mod: "com X, 3 de 7 deram errado; sem ele, 24 de 196", e diz o quanto tem certeza (bem certo, provável, pode ser acaso). Um mod que está em quase todas as sessões não pode ser comparado e não é citado.
- **Caçar pela metade** pelo chat: ele desativa metade dos seus mods (o que os outros precisam fica ativado), deixa você jogar o teste e você responde "ainda acontece" ou "não acontece mais" até sobrar um. A sua lista fica guardada e volta no fim.
- **O relatório**: quando você abre o chat depois de jogar, ele conta como foi a sua última sessão (tempo jogado, como terminou, erros novos, correções que funcionaram). "O que aconteceu esta semana?" faz o mesmo para um período.
- **Perguntas sobre o app** ("como eu crio um perfil?", "o que é uma cópia compatível?"): ele responde com o parágrafo deste guia e um botão para aquela tela.
- **Grupos, perfis e a Oficina**: "VP Base" conta como aquele grupo está, "perfil Knox Lives" se as últimas sessões dele foram bem; "tem versão nova de X?" e "o que dizem os comentários de X?" leem a página dele.
- **Várias perguntas de uma vez** ("o que mudou e quais mods falham?"), **apelidos** que ele aprende quando você escolhe entre vários mods, **"desfaz isso"** para a última mudança dele, e **memória**: da próxima vez, ele lembra você de qual mod vocês falaram e como ele está agora.

**Com o jogo (Studio Link):**

- No menu principal do jogo, o cartão do PZ Mod Studio (canto superior direito) mostra o que o Doutor diria primeiro: se as últimas sessões com esta lista deram errado, se a última fechou com erro, quais erros se repetem. O app o grava cada vez que verifica, então o cartão diz a última coisa que ele sabia mesmo com o app fechado.
- O Link mantém um diário de qual save você carregou e se você voltou ao menu (em vez de fechar a janela). Assim cada sessão diz "save X · voltou ao menu", também as jogadas com o app fechado.
- No menu de pausa (Esc) há **Marcar este momento**: quando algo estranho acontecer, clique nele. Depois pergunte ao Doutor "o que aconteceu quando eu marquei?" e ele conta o que o log do jogo escreveu naquele minuto (de um minuto e meio antes até meio minuto depois).

Ele também pode mudar coisas, sempre com o seu **Fazer**: criar, renomear ou apagar grupos do perfil ativo e colocar mods neles ou tirá-los ("grupo A, B em Mapas", "tira A do grupo Mapas", "renomeia o grupo Mapas para Mundo", "apaga o grupo Mapas"), sugerir grupos para os seus mods ativos que não têm nenhum (os que compartilham um nome ou o que requerem), e escrever ou remover **notas do Studio** ("nota em A: texto", "remove a nota do Studio de A", "anota o que você aprendeu"). As notas do Studio ficam separadas das suas e aparecem em violeta, com um botão próprio para removê-las; as suas notas ele nunca escreve, muda nem remove.

O que o cérebro tira das sessões:

- **O que mudou**: quando um erro se repete, quando ele começou e o que era diferente da última sessão que passou sem ele (mods adicionados, tirados ou atualizados, o lugar dele na ordem, a build do jogo). Ele diz isso como o que mudou naquela hora, não como a causa. Aparece na linha do Diagnóstico e no histórico do mod.
- **Correções lembradas**: cada correção que você aplica pelo Diagnóstico é anotada, e as sessões depois dela dizem se o erro do mod parou.
- **Antes de jogar**: quando as últimas sessões com quase a mesma lista nem chegaram ao menu ou ficaram sem memória, o app avisa quando você clica em Jogar, com o que é diferente da última que entrou num save, e pergunta se quer jogar mesmo assim.
- **O histórico de um mod**: na Lista de Mods, a seção **Neste PC** do painel da direita.
- **Carregamento mais lento** e uma **atualização do jogo** que fez vários mods falharem de uma vez.
- **Crashes de verdade**: quando o Java cai, ele deixa um relatório (`hs_err_pid…log`) na pasta do jogo; o cérebro o lê e o liga à sessão dele ("caiu: o PC ficou sem memória" ou "um erro fatal do Java naquela biblioteca"). Fechar o jogo com o X ou Alt+F4 não deixa nenhum, então isso nunca conta como crash.

### Segurança

A aba **Segurança** lista o que um mod poderia fazer além de modificar o jogo, lido dos arquivos dele: arquivos de programa ou scripts que ele traz, Java que pode rodar programas ou acessar a internet, e Lua que chama o que os próprios scripts do jogo nunca chamam. Nada disso é um veredito: muitos mods fazem essas coisas por bons motivos. A maioria das linhas é Info; um Aviso marca um padrão que nenhum mod tem bons motivos para usar. A linha de um mod da Oficina tem **Abrir página da Oficina**.

### Tempo por mod

Em **Mods** > **Tempo por mod**: quais mods deixam o jogo lento enquanto você joga. **Medir** coloca um pequeno mod do Studio (PZ Mod Studio Profiler) em primeiro na sua lista de mods e no save que você jogou por último; inicie o jogo (reinicie se ele estava aberto) e jogue alguns minutos como sempre. Cada handler de evento de cada mod é cronometrado pelo jogo e somado por mod: a parte do tempo real que ele levou, milissegundos por segundo e a chamada mais longa dele (uma longa é um travamento que você vê). Abra uma linha para ver os handlers que mais custam, com arquivo e linha. **Parar de medir** tira o profiler da lista e do save de novo; os últimos números ficam.

As janelas, barras e ações cronometradas de um mod também contam (como Class:render, Class:update); cada função conta só o próprio tempo. O profiler adiciona um pouco de tempo próprio a cada handler, então pare-o quando terminar.

## Jogo

O que o app sabe sobre o jogo em si, além dos mods. Cinco abas: **Ao vivo**, **Memória**, **Inicialização**, **Quadros** e **Disco**.

### Ao vivo

O que o jogo usa agora: no topo, a memória livre do PC (o medidor da barra do topo leva a esta aba). Abaixo, um cartão por jogo aberto: memória em relação ao limite do Java dele, CPU, em que tela ele está (menu, conectando, no jogo), a última linha do log dele, quantos erros ele tem e botões para abrir a pasta e o log dele. Com o jogo fechado, mostra o que se sabe da última vez: o último início, o limite de memória, as sessões que ficaram sem memória e as vezes em que o Windows ficou sem memória.

### Memória

Em **Jogo** > **Memória**: quanta memória o Java do jogo pode usar (`-Xmx` no `ProjectZomboid64.json`), do total do PC, com uma sugestão para os seus mods ativos e um aviso quando uma sessão recente ficou sem memória. Escolha um valor (de 2 GB até metade da memória do PC) e clique em **Aplicar** com o jogo fechado; vale a partir do próximo início, e uma cópia do arquivo de antes da mudança vai para `PZModStudio_Backups`.

Cada perfil guarda o próprio valor: ativar um perfil dá ao jogo a memória dele. Um perfil sem valor usa o último escolhido. Se uma atualização do jogo colocar o próprio valor de volta, o Studio restaura o seu da próxima vez que iniciar o jogo.

Quando o próprio Windows ficou sem memória com o jogo aberto nos últimos 14 dias, isso também aparece aqui, com o que ajuda (um arquivo de paginação maior, fechar outros programas grandes). Isso não é a memória do Java do jogo, então aumentar o valor acima não resolve.

### Inicialização

Em **Jogo** > **Inicialização**: quanto tempo o jogo levou para chegar ao menu principal, etapa por etapa (arquivos dos mods, scripts, Lua, modelos e assim por diante), a partir do log que cada início deixa. Cada início fica ao lado do seu início de costume com mais ou menos a mesma quantidade de mods; uma etapa em âmbar levou bem mais que o normal. Escolha um início anterior para vê-lo.

### Quadros e travadas

Em **Jogo** > **Quadros**: os tempos de quadro enquanto você joga (fps, quadros acima de 25, 50 e 100 ms), os chunks carregados e, por mod, os handlers que seguraram um quadro por mais tempo, com arquivo e linha. O mesmo profiler de **Mods** > **Tempo por mod** os conta, então clique em **Medir** lá antes e jogue alguns minutos. **Ler de novo** lê os números mais novos.

### Disco

No topo, **Liberar espaço**: **Limpar cache** apaga o que o Studio guarda para não ler ou perguntar a mesma coisa de novo (as checagens de cada mod, o que a Steam disse, imagens de mapas, traduções); ele monta tudo de novo sozinho e nunca mexe em mods, saves, perfis ou notas. Depois de um teste de multijogador, **Pasta do jogador 2** manda a pasta `Zomboid2` do segundo jogo (com cópias dos mods do servidor) para a Lixeira; os mods em desenvolvimento linkados lá não são mexidos, e o próximo teste a cria de novo. Não durante o teste.

Em **Jogo** > **Disco**: quanto cada mod instalado ocupa, contado pelo que vai junto (um item da Oficina com todos os mods dele, uma pasta em `Zomboid/mods`). A medição leva uns dois segundos.

- **Nada usa** (âmbar): nenhum perfil o tem, ativado ou desativado; nenhum save o cita na lista; nenhum servidor guardado o carregou; ele não está na lista ativa do jogo.
- **Desativado em todo lugar** (azul-claro): algum perfil o mantém, desativado, e nenhum save ou servidor o usa.
- **Sobra**: uma pasta da Oficina de um item em que você não está mais inscrito; a Steam às vezes as deixa para trás.
- **Seu envio**: um mod seu em `Zomboid/Workshop`. Aparece, mas nunca é oferecido para remoção.

Marque os que você quer tirar. Um item da Oficina vai embora com **Cancelar inscrição em N itens** (a Steam precisa estar aberta, e ela mesma apaga os arquivos). Uma pasta local ou uma sobra vai com **Mandar N pastas para a Lixeira**: dá para restaurá-la da Lixeira. Cada um pergunta antes, com os nomes. Nada vai para a Lixeira com o jogo aberto.

Abaixo, **Pastas do jogo e do Studio**: quanto ocupam o jogo, os mods da Oficina, os mods locais, os saves, os backups de saves, as versões de mods guardadas, os backups de listas de mods, o cache, os downloads pela metade e os logs, com um botão para abrir cada pasta. Nada é apagado dali.

## Servidor

O servidor rodando neste PC (o que você inicia pela tela Servidores, ou o de um teste de multijogador), com o mesmo cartão: memória em relação ao limite dele, CPU, se ele já iniciou, a última linha do log dele e quantos erros ele tem. Durante um teste de multijogador, os jogadores dele também aparecem. Sem servidor rodando, um botão leva para **Servidores** para iniciar um.

Um servidor em outro PC em que você entra não pode ser medido daqui: só o que roda neste PC.

## Bom saber

- As correções em **Mods** são salvas na lista de mods do perfil ativo.
- Os arquivos mesclados e os patches do shim valem na próxima vez que o jogo iniciar.
- "Enche o console" é medido a partir do `console.txt` da última sessão, então ele se atualiza depois que você joga.
- Clicar numa etiqueta de problema na Lista de Mods abre esta tela naquela linha.
