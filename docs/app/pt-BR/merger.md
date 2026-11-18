# Mesclador de Mods

O Mesclador de Mods conserta arquivos que dois ou mais dos seus mods ativos alteram ao mesmo tempo, e deixa você corrigir um bug no próprio arquivo de um mod. As suas decisões vão para um pacote: um pequeno mod próprio que carrega por último, para que o jogo leia a sua versão desses arquivos.

## O que é um pacote

- Um pacote é uma pasta de mod em `Zomboid/mods` cujo nome começa com `Z_PZModStudio_` (o padrão é `Z_PZModStudio_MergedPatch`).
- Quando você o publica, o Studio grava nele os seus arquivos mesclados (para a Build 41, e na pasta `42` dele para a Build 42), adiciona um pequeno arquivo de compatibilidade e coloca o pacote **por último** na lista de mods do jogo. O mod que carrega por último é aquele cuja cópia de um arquivo o jogo usa, então a sua cópia mesclada vence.
- Um pacote tem dois estados. **🟢 EMPACOTADO / FECHADO** quer dizer que ele está publicado e o jogo o lê. **🟡 ABERTO (rascunho)** quer dizer que você o está editando; enquanto for rascunho, ele sai da lista de mods do jogo, então o jogo não o lê até você publicar de novo.
- Cada pacote pertence aos perfis que o têm entre os mods. Um pacote que você cria com um perfil ativo é adicionado a esse perfil.
- Um pacote lembra o perfil que estava ativo da última vez que foi publicado (**feito com** no cartão dele, em âmbar quando outro perfil está ativo agora): as mesclas dele acertam a lista desse perfil. Com um perfil que não tem pacote próprio, **Pacote para "nome"** na barra do topo cria um com o nome dele.

## O que você vê

Quando nenhum pacote está aberto, você vê a lista **Pacotes de mescla**. Cada cartão mostra o nome do pacote, os arquivos, o estado, os perfis a que pertence e estes botões:

- **🔍 Abrir / Entrar no pacote** o abre na área de trabalho e procura conflitos nos seus mods ativos.
- **Visível na lista de mods** / **Oculto na lista de mods** ativa ou desativa o pacote na lista de mods do jogo (`default.txt`).
- O lápis edita o nome e a descrição; a lixeira o apaga do disco (você é consultado antes). Os perfis que o tinham ficam com o novo nome depois de renomear.
- O ícone de download o salva como um arquivo `.pzmerge` para compartilhar; o ícone de pasta abre a pasta dele no Explorador do Windows.
- **Pacotes de outros perfis** mostra os pacotes que pertencem a outros perfis; **Só "nome"** volta para os do perfil ativo.

A barra do topo tem **Carregar pacote (.pzmerge)** para importar um pacote que alguém mandou para você, e **Novo pacote** para criar um (um nome, uma descrição opcional, depois **Criar pacote**). Quando o perfil ativo não tem pacotes de mescla, a tela avisa e explica quando um é necessário, com um botão para ver os pacotes de outros perfis. Os mods do próprio Studio (a Live IPC Bridge e o Studio Link) não são pacotes de mescla: eles ficam embaixo, recolhidos em **Mods do Studio**, e você pode ver os arquivos deles mas não editar, renomear ou apagar.

## Trabalhando dentro de um pacote

A barra do topo mostra **← Voltar para a lista**, um seletor **Pacote:** para pular para outro pacote, o estado e a visibilidade, e estes botões:

- **Corrigir um arquivo de mod**: abre um arquivo do próprio mod para consertá-lo (veja abaixo).
- **⚡ Mesclar tudo e publicar**: mescla todos os arquivos em conflito com o motor automático e publica o pacote de uma vez: ele é gravado em `Zomboid/mods` e colocado por último na sua lista de mods, e o jogo o lê na próxima vez que iniciar. Revise os arquivos depois.
- **↩️ Desfazer todas as mesclas**: joga fora todas as mesclas salvas neste pacote. Uma cópia fica guardada em `Zomboid/PZModStudio_Backups`, e os arquivos já publicados ficam até você publicar de novo.
- **Todos** / **Sem duplicados**: esconde os arquivos em que os mods trazem cópias idênticas, para listar só as colisões de verdade.
- Ícones de pasta, exportar e procurar de novo.
- **📦 Empacotar e publicar**: o passo final. Grava a decisão atual de cada arquivo no pacote e coloca o pacote por último na lista de mods do jogo.

À esquerda, **Conflitos do VFS** lista cada arquivo em colisão com o tipo dele (Lua, **DEFINIÇÃO** para scripts de itens e receitas, **TEXTOS** para traduções) e o estado: **Precisa de revisão**, **Mesclado automaticamente** ou **Resolvido**.

## Decidindo o que o jogo lê para um arquivo

1. Clique num arquivo da lista. A metade de cima mostra o arquivo vanilla, a cópia de cada mod e um resumo do que cada mod muda em relação ao jogo base; os conflitos de verdade (as mesmas linhas alteradas de jeitos diferentes) aparecem marcados. Clique numa mudança para rolar todos os painéis até ela.
2. Em **O que o jogo deve ler?**, escolha uma resposta:
   - **Manter os dois** (para Lua e outros arquivos): as mudanças de todos os mods juntas. Onde dois mudam as mesmas linhas, vence o que carrega por último e o outro fica guardado num comentário.
   - **Mesclar** (para definições e traduções): o que pode ter os dois valores fica com os dois; onde só cabe um, fica o que o jogo faz hoje e o outro vai para um comentário.
   - **Só (nome do mod)**: a versão daquele mod. Para uma definição, as configurações dos outros mods que este não mexe continuam valendo.
   - **Vanilla**: o próprio arquivo do jogo, como se nenhum mod tivesse mexido nele (não aparece quando o jogo base não tem esse arquivo).
3. Ou edite o código no editor abaixo; ele então mostra **A sua própria edição**. **Anexar** e **Remover** no painel de um mod adicionam ou tiram a cópia inteira daquele mod, e **Carregar base** coloca o arquivo vanilla no editor.
4. Clique em **✓ Salvar a mescla deste arquivo**. O arquivo fica **Resolvido**.
5. Quando todos os arquivos estiverem prontos, clique em **📦 Empacotar e publicar**.

**Restaurar mescla AST** coloca a mescla automática de volta no editor, e **Desfazer mescla deste arquivo** esquece a decisão salva para aquele arquivo. Arraste a barra entre os painéis para mudar o tamanho deles.

## Corrigindo um bug em um mod

Use **Corrigir um arquivo de mod** quando um mod tem um bug e não há nada para mesclar. Escolha o mod, depois o arquivo Lua ou de script, edite e clique em **✓ Salvar a mescla deste arquivo**. A sua cópia vai para o pacote e o jogo a lê no lugar da do mod, só para aquele arquivo; o mod continua ativado e o resto dele fica intacto. Salvar uma cópia sem mudanças é recusado, porque isso só congelaria o arquivo na versão de hoje.

O Monitor e o Diagnóstico podem mandar você direto para cá: o pacote certo abre e o arquivo citado pelo erro abre no editor, às vezes com a correção já aplicada.

## Pacotes empacotados e publicação no lugar

Um pacote empacotado é só leitura: os arquivos dele ficam escondidos atrás de um resumo até você clicar em **🔓 Desfazer mescla / Abrir para editar** (ou **Desempacotar para editar**) e confirmar. Desempacotar tira os arquivos publicados do jogo até você publicar de novo.

O arquivo de um mod aberto para correção é a exceção: num pacote empacotado, salvar **publica no lugar**. Tudo o que o pacote tem é publicado de novo com o seu arquivo ao lado, o pacote continua empacotado e o jogo lê o arquivo novo na próxima vez que iniciar. Quando o Diagnóstico ou o Monitor manda você para um arquivo de um pacote empacotado, esse arquivo é mostrado sem desempacotar.

## Compartilhando pacotes

**Carregar pacote (.pzmerge)** importa um arquivo de pacote. Se ele foi feito com mods que você não tem, com cópias diferentes deles ou em outra build do jogo, uma janela **Antes de publicar** os lista, com **Inscrever-se em todos** para os mods da Oficina que faltam. O conserto mais seguro é os dois terem as cópias mais novas, e então procurar de novo e mesclar de novo antes de publicar.

## Bom saber

- O jogo lê um pacote quando inicia: depois de publicar, reinicie o jogo se ele estiver rodando.
- Se a pasta do jogo não for encontrada, a tela pede para você definir os caminhos em **Configurações do App** antes.
- Quando você sai de um arquivo com edições não salvas, você é consultado se quer deixá-las para trás.
- Um pacote que você apaga é removido do disco; os mods que ele mesclava não são mexidos.
