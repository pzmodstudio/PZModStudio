# Central de Monitoramento

O Monitor mostra o log do jogo enquanto ele é escrito e transforma cada erro num cartão que diz qual mod o jogo culpou e o que você pode fazer. Ele acompanha o `console.txt` tanto se você iniciou o jogo pelo Studio quanto pela Steam.

## O que você vê

- À esquerda, o visualizador de logs, com uma aba por log.
- À direita, **ERROS E AVISOS**: os cartões de erro e, acima deles, o painel **Achar o mod que causa isso**.
- No topo, **Ver como o jogo está rodando** abre Diagnóstico > Jogo > Ao vivo: memória, CPU e em que tela o jogo está.
- Acima do log, com o jogo rodando (visão completa), o painel **Ao vivo**: veja abaixo.
- No topo, **📁 Pasta de logs** abre a pasta com o `console.txt` e a subpasta `Logs` no Explorador do Windows.
- Ao lado, o botão do filtro de spam. Enquanto ele diz **Filtro de spam: ATIVO**, esconde as linhas sobre traduções faltando; clique para mudar para **Filtro: MOSTRAR TUDO**.

Se a pasta do jogo ainda não estiver definida, a tela só mostra **Configure os caminhos das pastas nas Configurações**.

## Ao vivo (com o jogo rodando)

Na visão completa, com o jogo aberto, um painel **Ao vivo** fica acima do log. A linha de título dele resume tudo (FPS, memória Java, CPU, erros de Lua); um clique o recolhe. Ele é lido a cada 2 segundos:

- **FPS, últimos minutos**: uma linha dos últimos 10 minutos (passe o mouse por cima para ler um momento); a linha tracejada é 30 FPS. Abaixo dela, o quadro médio e o quadro mais longo dos últimos 10 segundos, a memória que o Java usa em relação ao limite dele, a memória do processo inteiro com o que o PC tem livre, e a CPU. Uma segunda linha acompanha a memória.
- **Tempo de Lua agora, por mod**: os 5 mods cujo Lua levou mais tempo nos últimos 10 segundos, em milissegundos por segundo de jogo. Vem do profiler do Studio. Quando o profiler está desligado nada é medido, e o painel avisa com **Ligar no próximo início**: o profiler vai para o começo da lista de mods (e do save que você jogou por último) e mede a partir da próxima vez que você iniciar o jogo.
- **Erros de Lua nesta sessão**: quantos, e quais mods o jogo culpou, do jeito que o Studio Link os conta no jogo; abaixo, os cartões de erro mais novos, marcados como **novo** quando apareceram enquanto você olhava.
- Uma última linha diz se o painel do Link aparece no jogo e quanto o código dele mesmo custa.

FPS, memória Java e erros vêm do Studio Link (versão 1.20 ou mais nova), quando você está dentro de um jogo. O painel some quando o jogo fecha.

### O painel no jogo

O Studio Link pode mostrar os mesmos números por cima do jogo: FPS e tempo de quadro, memória Java em relação ao limite, CPU, os 5 mods com mais tempo de Lua nos últimos 10 segundos (com o profiler ligado) e os erros de Lua da sessão com os mods culpados. Abra pelo menu de pausa com **Painel ao vivo**, ou dê uma tecla a ele nas **Opções > Mods > PZ Mod Studio Link** do jogo (ele não tem nenhuma até você escolher, então nunca pega uma tecla do jogo). Arraste para onde quiser; ele fica lá e volta no próximo jogo. A última linha dele diz quanto ele mesmo custa, em milissegundos por segundo. O limite de memória e a CPU só são conhecidos com o PZ Mod Studio aberto.

## O visualizador de logs

Duas abas estão sempre lá:

- **console.txt (Cliente)**: o log do jogo ao vivo. O Studio começa a ler no fim do arquivo, então você vê o que acontece a partir do momento em que o Studio está aberto. Quando uma nova sessão de jogo começa, o arquivo recomeça e o Studio o lê a partir da primeira linha. A aba guarda as últimas 1.500 linhas, também enquanto você está em outra tela.
- **Servidor dedicado (ao vivo)**: o log do servidor dedicado, enquanto um que você iniciou em **Servidores** está rodando. Mostra as últimas 1.500 linhas dele e é lido de novo a cada 2 segundos.

Para ler outros logs, clique em **Carregar log**. O menu lista os logs encontrados na sua pasta Zomboid (com o tamanho); escolha um para abri-lo numa aba nova. **📂 Procurar outro arquivo (.txt)...** abre qualquer arquivo de texto. Um log carregado mostra as últimas 2.000 linhas dele. Essas abas podem ser fechadas no X; as duas abas ao vivo, não.

Abaixo das abas:

- A caixa de busca deixa só as linhas que contêm o que você digitar.
- As etiquetas **Todos**, **🚨 Erros**, **🌙 Lua** e **☕ Java** deixam só as linhas daquele tipo. **⚡ Ponte** só aparece com os Recursos de desenvolvedor ligados.
- O rodapé mostra quantas linhas estão sendo exibidas do total.

Os botõezinhos à direita das abas:

- O botão do disco salva o que a aba tem como um arquivo novo em `Zomboid/Logs/`.
- O botão de copiar copia as linhas que você está vendo (depois dos filtros) para a área de transferência.
- O botão da lixeira esvazia o visualizador nesta aba. Ele não mexe no arquivo no disco.

As linhas de erro aparecem em vermelho, os avisos em âmbar.

## Os cartões de erro

Um cartão aparece quando o jogo lança um erro, ou quando ele escreve um aviso e cita um mod. Todo o resto fica no log à esquerda. Os cartões que lançaram erro (vermelhos) vêm primeiro, os avisos (âmbar, marcados como **aviso · o jogo continuou**) depois. O mesmo problema aparece uma vez só, não importa quantas vezes o jogo o repita; o número como **600×** diz quantas vezes ele está no log, e os que se repetem são os que vale a pena consertar primeiro.

Acima dos cartões, um selo conta as exceções e outro os avisos (cada cartão diz se há algo a fazer), com uma linha que resume o log. Quando o Studio lembra sessões anteriores, cada cartão diz **novo** ou **visto em N sessões desde** uma data, e **Só os novos** esconde o resto.

Um cartão pode mostrar:

- **O nome do mod**, quando o jogo o citou na linha do erro ou no stack trace abaixo dela.
- **chamado de ...**: os mods cujo código chamou o que falhou, lidos do próprio stack do jogo. A culpa pode ser de qualquer um deles.
- **Trecho do log**: a própria linha.
- Para um `require` que falhou, onde aquele módulo está de verdade agora, quando o Studio o encontra.
- O que fazer em seguida, ou uma nota de que o shim da Build 42 já cobre isso.
- Quando o erro está numa cópia que o seu pacote do Studio tem: uma nota quando o mod mudou aquele arquivo depois que a cópia foi feita, ou quando nenhum mod da sua lista traz mais aquele arquivo.

Quando uma nova sessão de jogo começa, os cartões da anterior são limpos. **Descartar** os limpa à mão.

## Consertando a partir de um cartão

- **Corrigir o caminho** (ou **Corrigir o caminho em File.lua**): aparece quando um mod pede um módulo por um caminho antigo e o Studio conhece o novo. Ele reescreve o `require` em todos os arquivos dos mods ativos que o pedem e os publica no seu pacote do Studio. O mod não é alterado e continua ativado. Uma linha verde diz o que foi corrigido; reinicie o jogo.
- **Corrigir File.lua** ou **Corrigir um arquivo dele**: abre os arquivos daquele mod no Mesclador de Mods para você corrigir um. A sua cópia vai para um pacote que carrega por último, então o jogo a lê no lugar da do mod. Avisos sem correção conhecida não ganham este botão.
- **Remover a cópia de File.lua**: aparece quando o erro está num arquivo que o seu pacote do Studio já corrigiu. Tira aquela cópia do pacote (depois de perguntar), e o jogo volta a ler o próprio arquivo do mod. Qualquer edição que você fez na cópia se perde.
- **Desativar**: pede confirmação, desativa o mod e leva você para a Lista de Mods.
- **Um pacote de mescla pode resolver isto — abrir o Mesclador de Mods**: para erros que o shim da Build 42 resolve. O shim vai dentro de um pacote de mescla publicado; o cartão diz se o seu já está valendo ou ainda é rascunho.

## Relatórios

- **Salvar relatório** grava um arquivo Markdown com o erro, a build do seu jogo, o mod e o código dele, mais as linhas do log em volta. Fica pronto para entregar ao autor de um mod ou a um assistente de IA.
- **Copiar relatório** coloca o mesmo relatório na área de transferência.
- **🔍 Inspecionar nos logs** coloca o arquivo ou a palavra-chave do erro na caixa de busca à esquerda.
- **Copiar a linha** copia só a linha do erro.

## Achar o mod que causa isso

Para problemas que o log não cita: um travamento, uma receita que nunca aparece, um save que não abre.

1. Abra o painel. Se o problema não acontece sempre, marque **Não acontece sempre**. Se o log mostra algum texto do erro, digite-o: depois de cada jogo de teste o app lê o log e sugere a resposta. Clique em **Começar a dividir a lista**, e confirme.
2. Parte dos seus suspeitos é ativada; os que este PC já suspeita (sessões que deram errado, erros que se repetem) são testados primeiro. Mods que outro mod requer ficam ativados em todas as rodadas, para que nada quebre por esse motivo.
3. Clique em **Iniciar o jogo**, jogue do jeito que você jogava quando aconteceu, e feche.
4. Responda **Ainda acontece** ou **Sumiu**. A lista é dividida ao meio de novo. O selo mostra a rodada e quantos inícios faltam (uns 8 para cem mods).
5. Quando sobrar um mod, jogue mais uma vez para confirmar.

No fim, o painel diz qual é o mod, mostra as linhas que o log tem sobre ele (com **Corrigir File.lua** quando uma linha cita um arquivo, e **Mostrar as linhas dele no console**), e oferece **Corrigir um arquivo dele**, **Desativar** ou **Manter ativado**. A sua lista original é devolvida de qualquer jeito que a caça termine. **Parar e devolver a minha lista** termina antes.

Se o último mod não causa o problema sozinho, a caça continua sozinha: ela mantém esse mod ativado e procura o outro mod que causa o problema junto com ele, e então diz qual é o par. Se não encontrar nem um mod nem um par, o painel diz por quê: um dos mods obrigatórios, mais de dois mods, ou nenhum mod.

## Bom saber

- A caça grava a sua Lista de Mods ela mesma. Não ative nem desative mods na Lista de Mods enquanto ela roda, senão a próxima resposta não vale nada.
- Uma caça sobrevive a fechar o Studio; ela fica salva em `Zomboid/PZModStudio_Bisect.json`.
- **Iniciar o jogo** na caça inicia o jogo normalmente, sem o modo Debug.
- As correções publicadas num pacote valem na próxima vez que o jogo iniciar.
