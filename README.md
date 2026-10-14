# Notas Definitivas (v6.30.0 - Lista por Prioridade e Correcao dos Filtros de Urgencia)

Editor de texto Markdown em terminal (TUI), minimalista, de arquivo único em C (`notas.c`), com **renderização de bullet lists (* e -) em cor diferenciada e negrito**, **compatibilidade bidirecional total da área de transferência no Windows (copiar com "y" e colar do sistema com "p" e Ctrl+V)**, **cópia para o clipboard assíncrona e instantânea no Windows sem travamentos**, **seleção e cópia no modo inserção com Shift+Setas e Mouse**, **atalho `Ctrl + S` para salvamento instantâneo no modo normal**, **navegação suave e robusta por setas no modo inserção sem caracteres indevidos**, **botões interativos na barra inferior (Quebra de linha, Localizar, Substituir, Renomear e Temas)**, **caixas de marcação de atenção com destaque em cores vibrantes**, **corte de linhas completas com `V + x`**, **fechamento de abas com a tecla `c`**, **salvamento ultra-otimizado**, **gerenciamento completo de notas na barra lateral (`n`, `r`, `d`)**, **sistema completo de abas (tabs) com animação marquee**, **seletor de 8 temas do VS Code**, **cursor em linha vertical piscante no modo inserção**, **auto-alinhamento de tabelas (estilo Vimwiki)**, **quebra de linha visual inteligente (Soft Word Wrap)**, e navegação inspirada no Vim / Vimwiki. Compatível nativamente com **Linux** e **Windows**, sem dependências externas complexas.

---

## 📊 Ordem da Lista por Prioridade (v6.32.0)

- A lista lateral é exibida em **grupos de prioridade**, nesta ordem:
  1. **Nível 1** (crítico / vermelho)
  2. **Nível 2** (médio / laranja)
  3. **Nível 3** (baixo / amarelo)
  4. **Nível 4 - [H]** (azul / ciano)
  5. **Nível 5 - [R]** (roxo / violeta)
  6. **Nível 6 - [O]** (verde)
  7. **Sem marcação** (`[ ]`)
- **Dentro de cada grupo, a nota editada mais recentemente aparece primeiro** — o mesmo critério de data que já existia, agora aplicado por grupo.
- Marcar ou desmarcar com **`m`** ou clicando na caixa da nota (`[ ]` ➔ `[1]` ➔ `[2]` ➔ `[3]` ➔ `[H]` ➔ `[R]` ➔ `[O]` ➔ `[ ]`) reorganiza a lista por prioridade para agrupar as notas no topo, persistindo atomicamente no SQLite enquanto a seleção acompanha a nota.
- Vale também com filtro ativo: em `[1]`, `[2]`, `[3]`, `[H]`, `[R]` ou `[O]`, as notas aparecem da mais recente para a mais antiga.
- Desempate por nome, para a ordem não "dançar" entre redesenhos quando duas notas têm a mesma data.

### 🏷️ Rótulos responsivos nos botões de filtro da barra lateral

Os botões de filtro na base do painel esquerdo adaptam-se automaticamente à largura do painel:

| Largura do painel | Rótulos | Colunas |
| :--- | :--- | :--- |
| 32 ou mais | ` [Todas] [1] [2] [3] [H] [R] [O]` | 32 |
| 28 a 31 | ` [T] [1] [2] [3] [H] [R] [O]` | 28 |
| 21 a 27 | `[T][1][2][3][H][R][O]` | 21 |

Os limites de clique do mouse são recalculados dinamicamente a partir do layout em uso e operam de forma independente da largura do painel, garantindo clique preciso em qualquer resolução ou tamanho de janela.

---

## 🗄️ Backend SQLite, Importacao e Exportacao de Markdown (v6.29.0)

- **As notas passaram a viver em um banco SQLite (`notas.db`)**, em vez de arquivos `.md` dentro de `data/`:
  - Sem dependencia de motor externo: o programa abre o banco diretamente (WAL, para leitura/escrita sem travar a interface).
  - **O banco nasce VAZIO**: na primeira execucao nenhuma nota de exemplo e criada. Use `[➕ Nova]` ou `[⬇ Importar]`.
  - A pasta `data/` de arquivos `.md` deixou de ser o armazenamento principal: ela agora e apenas origem/destino das rotinas de importacao e exportacao.
  - Cada nota guarda nome, conteudo, data de modificacao e o nivel de urgencia (`[1]`/`[2]`/`[3]`), antes espalhado no arquivo `data/.attention`. **O nivel de urgencia vive apenas na coluna `attention` do banco** (veja a limitacao na secao de exportacao abaixo).
  - O `notas.db` fica ao lado do executavel (o programa faz `chdir()` para o diretorio do binario).

- **`[⬇ Importar]` — pasta de `.md`  ➔  banco**:
  - Clique no botao, informe a pasta e pressione `Enter`. Aceita `.md`, `.markdown`, `.mdown`, `.txt` e `.text` (ignora o resto, como `.pdf`).
  - Reconhece acentos e emojis (UTF-8 preservado byte a byte).
  - Reimportar a mesma pasta **atualiza** as notas existentes em vez de duplicar (`UNIQUE` pelo nome).
  - Caminhos com `~` sao expandidos e aspas coladas sao removidas automaticamente.

- **`[⬆ Exportar]` — banco  ➔  pasta de `.md`**:
  - Exporta todas as notas como `<nome>.md` na pasta informada (criada se nao existir).
  - O **texto** da nota faz a ida e volta sem nenhuma alteracao (escrito byte a byte como esta no banco).
  - Nomes de nota sao higienizados ao virar arquivo (`/ \ : * ? " < > |` viram `_`).

- **⚠️ Limitacao conhecida: as marcacoes de urgencia NAO viajam nos arquivos `.md`**:
  - O `.md` exportado e markdown puro, sem metadados: nenhum rastro de `[1]`/`[2]`/`[3]` e escrito no arquivo.
  - Portanto, **exportar e depois importar essa pasta em um banco NOVO faz as notas voltarem sem marcacao alguma** (todas em `[ ]`). Isso vale tambem para migrar de maquina.
  - Ja a **reimportacao sobre notas que ja existem preserva a marcacao**: nesse caso o conteudo e atualizado e a coluna `attention` fica intocada.
  - Para nao perder as marcacoes, o caminho seguro hoje e copiar o proprio `notas.db` (e nao os `.md`).

- **Prompts com edicao completa**: os dois campos aceitam setas, `Home`/`End`, `Ctrl+A`/`Ctrl+E`, `Ctrl+←/→`, `Backspace`, `Delete`, `Ctrl+U` e `Ctrl+K`. `Esc` cancela.

---

## 🎯 Bullet Lists (* e -) em Cores e Negrito + Colagem Bidirecional (v6.27.0)

- **Renderização Destacada de Bullet Lists (`*` e `-`)**:
  - **Destaque Visual com Cor e Negrito**: As listas com marcadores (`*` e `-`) em qualquer nível de indentação agora têm seus marcadores renderizados em **negrito** e em uma **cor destacada e vibrante** (`TH_FG_YELLOW` dourado para `-` e `TH_FG_ACCENT` lavanda para `*`).
  - **Preservação de Sintaxe**: Distingue perfeitamente marcadores de listas contra linhas horizontais (`---`, `***`, `- - -`), itálico/negrito (`*texto*`, `**texto**`) e parâmetros de comando (`--flags`).
  - **Listas de Tarefas**: Compatível com checklists `- [ ]` e `* [x]`.

- **Compatibilidade Bidirecional do Sistema para `y` e `p` (Windows)**:
  - **Colar do Sistema com `p` ou `P`**: Ao copiar textos em navegadores, Word, Bloco de Notas ou qualquer outro programa no Windows com `Ctrl+C`, pressionar **`p`** ou **`P`** no editor cola instantaneamente o conteúdo da área de transferência do Windows para a sua nota.
  - **Fidelidade Total a Acentos (`CF_UNICODETEXT`)**: A leitura do clipboard converte UTF-16 diretamente para UTF-8, garantindo que acentuação em português (`ç`, `ã`, `é`, `ó`, etc.) e caracteres especiais sejam colados perfeitamente.
  - **Colar no Modo Inserção com `Ctrl+V`**: Atalho `Ctrl+V` habilitado diretamente no Modo Inserção para colar da área de transferência do sistema operacional sem precisar voltar ao Modo Normal.
  - **Substituição em Seleção**: No Modo Visual, pressionar `p` substitui a seleção atual pelo texto do clipboard do sistema.

---

## ⚡ Desbloqueio Total e Cópia Instantânea com "y" no Windows (v6.26.0)

- **Eliminação Definitiva do Bloqueio de Vários Segundos no Windows**:
  - **Cópia Instantânea com `y`, `yy` ou `Y`**: Pressionar `y` ou `Y` no Modo Normal agora copia imediatamente a linha inteira para a área de transferência do sistema em 0 ms, sem esperar por uma segunda tecla em chamadas síncronas bloqueantes. Usuários acostumados ao `yy` continuam copiando no primeiro `y` e reconfirmando no segundo sem qualquer trava.
  - **Fim do Bloqueio de Kernel em `ReadFile` no Console do Windows**:
    - No Windows Console (`hStdin`), a soltura de uma tecla (`KEY_UP`) sinalizava o handle do console, fazendo com que `WaitForSingleObject` acordasse e chamasse `ReadFile`. Como `ReadFile` descarta eventos de soltura e aguarda um caractere real (`KEY_DOWN`), o processo ficava bloqueado no kernel por vários segundos até o usuário pressionar a próxima tecla.
    - Implementado filtro com `PeekConsoleInput` e `ReadConsoleInput` que drena instantaneamente eventos de liberação de tecla (`KEY_UP`), movimento de mouse e foco antes do `ReadFile`, garantindo que a leitura de entrada nunca mais congele.
  - **Desacoplamento Seguro da Área de Transferência (`OpenClipboard(NULL)`)**:
    - Uso do identificador universal de processo (`NULL`) no `OpenClipboard`, permitindo que a thread de segundo plano (`CreateThread`) atualize o clipboard sem conflitar com restrições de propriedade de janela do Windows Terminal (`conhost`/`OpenConsole.exe`).
  - **Detecção Expandida para Git Bash e WSL**: Suporte nativo ao `clip.exe` tanto em caminhos WSL (`/mnt/c/Windows/...`) quanto no Git Bash (`/c/Windows/...` e `$PATH`).

---

## ⚡ Cópia Instantânea para Área de Transferência no Windows (Sem Travamento)

- **Eliminação Total da Trava ao Copiar com "y" ou Seleção**:
  - **Thread Assíncrona Dedicada (`CreateThread` / `pthread`)**: A escrita na área de transferência do sistema operacional agora é processada em segundo plano de forma desacoplada da interface TUI.
  - **Resposta Visual Imediata (0 ms de Latência)**: O loop principal do editor nunca mais bloqueia aguardando a sincronização com o gerenciador de clipboard do Windows. A mensagem de confirmação no rodapé (`1 linha copiada para a área de transferência`, `Seleção copiada...`, `Nome copiado p/ clipboard...`) é exibida instantaneamente.
  - **Fidelidade Total a Caracteres UTF-8 / Acentuação (`CF_UNICODETEXT`)**: Migração de `CF_TEXT` (ANSI) para `CF_UNICODETEXT` (UTF-16) nativo no Windows, garantindo que acentos, caracteres em português (`ç`, `ã`, `é`, `ó`, etc.) e emojis sejam copiados perfeitamente para qualquer aplicativo externo (Word, Notepad, navegadores, etc.).
  - **Fim da Concorrência de Bloqueio com o Windows Terminal**: No Windows nativo, a transmissão desnecessária da sequência OSC 52 em sessões locais foi desativada, eliminando a disputa de lock entre o terminal e o processo pelo clipboard.
  - **Suporte Inteligente ao WSL**: Em ambientes WSL, o editor detecta o subsistema e utiliza `clip.exe` nativo de forma assíncrona, prevenindo os timeouts de 2 a 5 segundos do `xclip` em terminais sem servidor X11 ativo.

---

## ✨ Seleção e Cópia de Texto no Modo Inserção (Shift + Setas e Mouse)

- **Seleção Nativa sem Sair do Modo de Edição**:
  - **Shift + Setas**: Pressione `Shift + Seta Esquerda`, `Shift + Seta Direita`, `Shift + Seta Cima` ou `Shift + Seta Baixo` (e `Ctrl + Shift + Setas` para saltar palavras) enquanto estiver escrevendo no Modo Inserção para selecionar trechos de texto instantaneamente.
  - **Seleção com o Mouse no Modo Inserção**: Clique e arraste o mouse sobre qualquer texto enquanto estiver no Modo Inserção para selecioná-lo visualmente com destaque em tempo real, sem ser jogado para o Modo Visual.
  - **Cópia Automática e com `Ctrl + C`**: Ao soltar o mouse após o arrasto, o texto selecionado é copiado imediatamente para a área de transferência do sistema operacional. Você também pode copiar a qualquer momento pressionando **`Ctrl + C`**.
  - **Edição Fluida e Substituição**: Pressionar `Backspace`, `Delete` ou simplesmente continuar digitando qualquer letra substitui o trecho selecionado mantendo o histórico de desfazer (`u`). Setas direcionais comuns desmarcam a seleção e `Ctrl + A` seleciona todo o texto.
  - **Indicação Visual**: O badge de status exibe `INSERT [SEL]` enquanto houver texto marcado, mantendo total clareza do estado da edição.

---

## 🛡️ Estabilidade Total na Escrita, Rejeição de Palma e Blindagem de Prompts

- **Proteção Completa contra Saída Involuntária e Saltos do Cursor**:
  - **Blindagem de Prompts Modais (`:new`, renomear, buscar)**: Enquanto você digita o nome de uma nova nota ou responde a um prompt na barra inferior, toques acidentais no touchpad ou cliques em outras áreas da tela são completamente ignorados, impedindo o cancelamento do prompt ou perda do texto digitado. Atalhos globais (`Tab`, `Ctrl+F`, `Ctrl+T`) não interrompem a digitação.
  - **Rejeição de Palma em Modo Inserção**: Implementado filtro inteligente de rejeição de toques acidentais no trackpad durante a digitação (<250ms), evitando que apoiar a mão sobre o laptop faça o cursor saltar do nada para outra linha ou mude o foco para a barra lateral.
  - **Imunidade a Arrasto no Modo Inserção**: O arrasto com mouse (`btn 32`) nunca mais força a saída do Modo Inserção para o Modo Visual.
  - **Robustez de Sequências de Escape e Foco do Terminal**: Eventos de foco de janela (`\x1b[I`, `\x1b[O`) e sequências de controle não reconhecidas são drenados de forma limpa e nunca mais são confundidos com a tecla `Esc`, acabando com saídas repentinas da edição.
  - **Navegação por Palavras no Modo Inserção**: `Ctrl+Seta Esquerda` e `Ctrl+Seta Direita` agora navegam palavra por palavra dentro da linha durante a escrita, sem acionar o redimensionamento do painel lateral.

---

## 🔍 Abertura Direta de Notas em Prévia ao Clicar na Área de Texto

- **Transição Fluida da Pré-visualização para Edição**:
  - Ao navegar pelas notas na barra lateral (painel esquerdo) ou realizar buscas, a nota selecionada é exibida em modo de pré-visualização (`[Prévia]`) no painel direito.
  - Clicar com o mouse na área de texto da nota em pré-visualização agora abre imediatamente essa nota em uma aba (ou ativa sua aba se já estiver aberta), mantendo o foco no editor e posicionando o cursor na linha e coluna clicadas.
  - Elimina o comportamento anterior que descartava a pré-visualização da nota selecionada e reabria a última nota ativa no editor, proporcionando uma experiência de leitura e edição contínua e sem atritos.
  - Suporte completo tanto quando já existem abas abertas quanto no estado sem abas abertas.

---

## 📋 Cópia Rápida de Nome de Nota com `y` para Criação de Links

- **Cópia Instantânea sem `.md` no Painel Esquerdo**:
  - Quando o foco estiver na barra lateral de notas (painel esquerdo), basta posicionar a seleção sobre qualquer nota e pressionar **`y`**.
  - O nome do arquivo (sem a extensão `.md`) é copiado imediatamente para a área de transferência do sistema operacional.
  - O rodapé exibe a confirmação: `Nome copiado p/ clipboard: <nome> (use [[<nome>]])`.
  - Essa funcionalidade torna a criação de links bidirecionais e WikiLinks (`[[Nome da Nota]]`) extremamente prática e rápida durante a edição.

---

## 🗂️ Fechamento Total de Abas e Aba-Botão `[➕ Nova Nota]`

- **Fechamento de Todas as Abas**:
  - Feche todas as abas abertas de uma só vez utilizando o atalho **`C`** (Shift + C) ou os comandos **`:closeall`**, **`:tabcloseall`**, **`:ca`**, **`:fechartudo`**.
  - Ao fechar a última aba individualmente com **`c`**, o editor não força mais a reabertura de uma nota anterior, permitindo manter uma área de trabalho limpa.
- **Aba-Botão `[➕ Nova Nota]` no Topo**:
  - Quando nenhuma aba estiver aberta, a barra superior de abas exibe uma aba interativa com o botão **`[➕ Nova Nota]`**.
  - Um clique do mouse sobre esta aba (ou pressionar `i`, `n` ou `Enter`) abre imediatamente o prompt para criar uma nova nota.
  - O painel direito apresenta instruções elegantes para guiar o usuário.

---

## 💾 Salvamento Automático Inteligente (Autosave a Cada 1 Minuto)

- **Proteção Total contra Perda de Dados**:
  - Todas as notas abertas em abas que possuam edições não salvas há mais de **1 minuto** (60 segundos) são gravadas automaticamente no disco.
  - **Execução Fluida sem Interrupções**: O salvamento acontece de forma transparente em segundo plano, sem tirar o foco do cursor, sem alterar o modo de edição (mantém o Modo Inserção ou Normal intacto) e preservando todo o histórico de Desfazer (`u`) e Refazer (`Ctrl + R`).
  - **Múltiplas Abas Monitoradas**: O rastreador monitora independentemente a idade da edição em cada aba aberta; se mais de uma aba ultrapassar 1 minuto sem salvar, todas são persistidas com segurança.
  - **Salvamento mesmo com Protetor de Tela**: Caso você se afaste do editor e o Game of Life seja ativado, qualquer modificação pendente continuará sendo salva automaticamente após 1 minuto.
  - **Status e Comandos**: Exibe notificação discreta no rodapé (`💾 Salvo automaticamente: <arquivo>`) e suporta o comando **`:autosave`** para checagem rápida.

---

## ⏱️ Tempo Relativo Preciso desde a Última Edição em Todas as Notas

- **Eliminação do Traço (`-`) na Barra Lateral**:
  - Corrigido o problema onde notas com nomes acentuados (`Análise`, `Descrição`, etc.) ou notas recém-adicionadas podiam exibir um traço (`-`) no lugar do tempo decorrido.
  - O sistema de escaneamento de arquivos agora utiliza leitura nativa de atributos Unicode multiplataforma (`GetFileAttributesExW` no Windows e `stat` no Linux), garantindo a recuperação exata do horário de modificação (`mtime`).
  - O tempo relativo de cada nota é recalculado dinamicamente durante a navegação, exibindo com precisão a idade da última edição (`now`, `5m`, `2h`, `1d`, `3w`, `2mo`, `1y`) para 100% das notas da lista.

---

## ⌨️ Substituição de Caracteres no Estilo Vim (`r`)

- **Substituição sem Pressa de Caractere sob o Cursor (Espera Indefinida)**:
  - No **Modo Normal**, pressionar **`r`** coloca o editor em espera indefinida pelo próximo caractere digitado (sem limite de tempo ou timeout curto), exibindo `-- REPLACE --` no rodapé até que a tecla substituta seja pressionada.
  - Para cancelar a qualquer momento, basta pressionar **`Esc`**, **`Ctrl + C`** ou clicar com o mouse, retornando instantaneamente ao Modo Normal sem modificar o texto.
  - **Suporte Completo a Multibyte UTF-8**: Reconhece caracteres acentuados (`á`, `ç`, `ã`) e emojis tanto no caractere original quanto no novo caractere digitado, ajustando o tamanho da linha em memória sem desalinhar colunas.
  - **Quebra de Linha com `r<Enter>`**: Pressionar `r` e em seguida `Enter` divide a linha exatamente no ponto do cursor, movendo o restante do conteúdo para uma nova linha imediatamente abaixo.
  - **Cancelamento Imediato**: Pressione `Esc` para cancelar a substituição sem efetuar nenhuma modificação.
  - **Histórico de Desfazer (`u`)**: Todas as substituições realizadas com `r` integram-se ao histórico do editor, permitindo desfazer instantaneamente com `u` ou refazer com `Ctrl + R`.
  - **Modo Visual (`v` e `V`)**: Ao selecionar texto no modo visual, pressionar `r` seguido de um caractere substitui todos os caracteres da seleção pelo novo caractere informado.

---

## 🎨 Barra de Botões em Extensão Total (100% Horizontal)

- **Uniformidade Cromática de Ponta a Ponta**:
  - A barra inferior onde ficam todos os botões interativos (`[➕ Nova]`, `[✏️ Renomear]`, `[🗑️ Excluir]`, `[Wrap:ON/OFF]`, `[🔍 Buscar]`, `[🔄 Subst]`, `[🎨 Tema]`) agora ocupa a largura total da janela do terminal (da coluna 1 até a última coluna `E.screen_cols`).
  - Corrigida a divergência de cor na margem esquerda: todo o espaçamento à esquerda e à direita dos botões recebe a cor de fundo uniforme do seletor/barra (`TH_BG_SELECT`), eliminando qualquer faixa de cor diferente e garantindo harmonia visual impecável.

---

## 👾 Game of Life Interativo e Protetor de Tela (Jogo da Vida de Conway)

- **Botão Dedicado `[🧬 Life]` na Barra Inferior**:
  - Inicie a simulação do Game of Life a qualquer momento com um único clique no botão **`[🧬 Life]`** (ou **`[🧬]`** em telas menores) na barra inferior de ações.
- **Parada Instantânea com Tecla ou Mouse**:
  - Ao clicar com o mouse em qualquer parte da tela ou pressionar qualquer tecla no teclado, a simulação do Game of Life é interrompida imediatamente, retornando à tela de edição sem nenhum atraso.
- **Ativação Automática por Inatividade (3 Minutos)**:
  - Se o editor permanecer inativo sem toques no teclado ou eventos de mouse por **10 minutos** (600.000 ms), uma animação fascinante do **Jogo da Vida de Conway** é acionada automaticamente em tela cheia.
- **Harmonia Cromática com o Tema Ativo**:
  - A simulação adapta suas cores dinamicamente ao tema do VS Code em uso (`themes[E.current_theme]`), colorindo células recém-nascidas (`TH_FG_ACCENT`), maduras (`TH_FG_H1`, `TH_FG_H2`) e veteranas (`TH_FG_MUTED`) sobre o fundo oficial do tema (`TH_BG_MAIN`).
- **Mecanismo Anti-Estagnação Dinâmico**:
  - Monitora a densidade de células vivas e a repetição de padrões (osciladores e bloqueios estáticos). Caso a população diminua ou entre em repetição estática, o simulador reinjeta novas sementes ativas (gliders, pulsars, blocos caóticos) para manter a evolução sempre vibrante.
- **Despertar Instantâneo**:
  - Qualquer toque no teclado ou clique/movimento do mouse encerra imediatamente a simulação e retorna instantaneamente para a tela do editor no ponto exato onde você estava.
- **Comando Manual**:
  - Inicie a simulação a qualquer momento no modo de comando digitando **`:life`**, **`:gameoflife`** ou **`:gol`**.

---

## 🗂️ Abertura por Duplo Clique / Enter e Barra Exclusiva de Botões

- **Seleção sem Abertura Automática de Abas (Pré-visualização Pura)**:
  - Clicar uma única vez sobre qualquer nota no painel lateral esquerdo (ou navegar com `j`/`k`) **não cria nem abre uma nova aba**.
  - A seleção exibe instantaneamente o conteúdo completo da nota em modo de pré-visualização ao vivo (`g_preview`) no painel direito, preservando as abas de trabalho já abertas.
  - Para abrir a nota definitivamente em uma aba no topo e transferir o foco para o editor, use **duplo clique do mouse** sobre a nota na lista ou pressione a tecla **`Enter`**.
- **Barra Exclusiva para Botões de Ação no Rodapé**:
  - Todos os botões interativos (`[➕ Nova]`, `[✏️ Renomear]`, `[🗑️ Excluir]`, `[Wrap:ON/OFF]`, `[🔍 Buscar]`, `[🔄 Subst]`, `[🎨 Tema]`) agora ficam posicionados em uma barra totalmente dedicada e exclusiva.
  - A barra de status superior a eles é reservada para o indicador de modo (`NORMAL`, `INSERT`, etc.), o nome do arquivo Markdown (ou indicação de `[Prévia]`) e as informações de posição do cursor e contagem de linhas, garantindo máxima legibilidade visual em qualquer resolução ou largura de tela.

---

## 🗑️ Botão Excluir Nota Selecionada na Barra Inferior (`[🗑️ Excluir]`)

- **Exclusão Rápida com um Clique**:
  - Adicionado o botão interativo **`[🗑️ Excluir]`** (ou **`[🗑️]`** em telas compactas) diretamente na barra inferior de ações do editor, ao lado dos botões de Nova Nota e Renomear.
  - Ao clicar no botão, o editor identifica imediatamente qual nota está selecionada na lista do painel esquerdo (`E.selected_note_idx`) e entra no modo de confirmação de segurança.
- **Confirmação de Segurança Prévia**:
  - Exibe o prompt no rodapé: `Deseja realmente excluir a nota '<titulo>'? (s/n): `.
  - Permite confirmar com as teclas `s` ou `y` (ou clicando com o mouse no próprio prompt de confirmação).
  - Permite cancelar a qualquer momento com as teclas `n`, `Esc` ou clicando com o mouse em qualquer outra área da tela, prevenindo exclusões acidentais.
  - Ao confirmar, o arquivo é removido do disco, abas abertas da nota excluída são encerradas, a prévia é atualizada e a lista do painel lateral é recalculada atomicamente.
- **Comandos Equivalentes na Barra de Comandos**:
  - Suporte aos comandos `:delete`, `:del`, `:excluir` e `:d` no modo de comando (`:`).

---

## 🎯 Transição Rápida para Modo Normal com Clique no Texto da Nota

- **Saída Direta do Modo de Busca para Edição**:
  - Quando você estiver no **Modo de Busca (`Ctrl + F`)** na barra lateral e clicar com o mouse sobre qualquer parte da área de texto da nota aberta (painel direito), o editor encerra imediatamente a busca e transita de forma fluida para o **Modo Normal**.
  - O cursor do editor se posiciona exatamente sobre a linha e coluna clicadas, pronto para navegação com `h, j, k, l` ou entrada direta no Modo Inserção (`i`), dispensando a necessidade de apertar `Esc` ou `Tab` para sair da busca.
  - A mesma comodidade se aplica ao clicar sobre as abas no topo da tela ou ao selecionar texto com o mouse no painel direito.

---

## 🌟 Ocultação de Asteriscos no Modo Normal e Navegação Fluida por Setas na Busca

- **Ocultação Automática de Asteriscos (`**` e `*`) no Modo Normal**:
  - No Modo Normal (e na Linha de Comandos `:`), os marcadores asterisco de negrito e itálico são ocultados da visualização, exibindo o texto estilizado com acabamento limpo e profissional, semelhante aos visualizadores modernos de Markdown.
  - No **Modo Inserção (`i`)**, os asteriscos permanecem visíveis para que você possa editar a sintaxe normalmente.
  - A quebra de linha visual (*Word Wrap*) e o mapeamento de colunas acompanham a largura real visível do texto sem gerar desvios.

- **Navegação com Setas no Campo de Busca (`Ctrl + F`)**:
  - Ao digitar na caixa de busca superior da barra lateral, agora é possível navegar livremente com as setas:
    - **`←` e `→`**: Movem o cursor de digitação caractere a caractere UTF-8 no texto da pesquisa.
    - **`Home` / `Ctrl + A`** e **`End` / `Ctrl + E`**: Saltam para o início ou final da consulta.
    - **`Ctrl + ←` e `Ctrl + →`**: Avançam e recuam por palavras completas.
    - **`Delete` e `Backspace`**: Excluem caracteres na posição exata do cursor.
    - **Inserção no Meio do Texto**: Digitar qualquer letra insere no ponto exato onde o cursor estiver posicionado, atualizando a filtragem instantaneamente.
    - **`↑` e `↓`**: Continuam navegando verticalmente pela lista de notas filtradas para pré-visualização.

---

## ✨ Suporte a Negrito e Itálico em Markdown e Correção de Cores Após Links

- **Negrito e Itálico Nativos com Asteriscos (`**` e `*`)**:
  - Textos delimitados por dois asteriscos (**`**negrito**`**) agora são destacados visualmente em **Negrito** (`STYLE_BOLD`).
  - Textos delimitados por um asterisco (*`*itálico*`*) agora são destacados visualmente em *Itálico* (`STYLE_ITALIC`).
  - Textos delimitados por três asteriscos (***`***negrito e itálico***`***) combinam os dois estilos tipográficos simultaneamente.
  - Os marcadores asterisco são exibidos em tom sutil e atenuado, destacando as palavras formatadas sem poluir o fluxo de leitura.
  - Funciona perfeitamente integrado à navegação por caracteres UTF-8, sem causar desalinhamento de colunas ou falhas no posicionamento do cursor.

- **Uniformidade de Cores Após Links na Mesma Linha**:
  - Corrigido o problema em que, ao renderizar um link no modo normal ou de comando, os textos subsequentes na mesma linha perdiam a cor principal do tema ativo (ficando em cor padrão branca/cinza).
  - O restaurador de estilo agora preserva estritamente a cor de texto definida pelo tema (`TH_FG_MAIN` ou a cor dos títulos `H1`, `H2`, `H3`), garantindo harmonia visual contínua antes e depois de qualquer link em todos os 8 temas do VS Code.

---

## 🔗 Ocultação Contínua de URLs de Links na Linha de Comandos (`:`)

- **Visualização Estável e Fluida ao Inserir Comandos**:
  - Ao pressionar **`:`** no Modo Normal para digitar qualquer comando (`:w`, `:q`, `:wrap`, `:theme`, etc.), o texto da nota aberta permanece renderizado de forma limpa, ocultando os endereços completos de links longos (`[Título](url)` e `[[Nota]]`).
  - O título do link permanece visível com destaque colorido, eliminando qualquer salto visual, expansão de linhas ou deslocamento indesejado do texto enquanto o usuário digita na barra de comandos inferior.
  - A exibição do endereço completo do link fica reservada exclusivamente para o **Modo Inserção (`i`)**, onde a edição direta da sintaxe do link é necessária.

---

## 🔍 Limpeza Automática do Campo de Busca (`Ctrl + F`)

- **Reinício Imediato para Novas Buscas**:
  - Ao pressionar **`Ctrl + F`** no editor (ou no painel lateral), qualquer termo da busca anterior é automaticamente descartado e apagado da barra de pesquisa.
  - O campo de pesquisa no topo da barra lateral é redefinido imediatamente para o estado limpo (` 🔍 ` com o cursor piscante posicionado na primeira coluna), permitindo digitar um novo termo instantaneamente sem a necessidade de apagar manualmente caractere por caractere com Backspace ou `Ctrl + U`.
  - A lista lateral restaura a visualização de todas as notas do acervo no mesmo milissegundo, pronta para responder à digitação do novo filtro.
  - O mesmo comportamento foi estendido ao clique do mouse sobre a caixa de busca (` 🔍 Buscar notas...`) e ao atalho **`/`** na navegação do painel lateral.

---

## 📱 Quebra Dinâmica da Barra Inferior (Wrap Bar) e Redimensionamento Fluido

- **Quebra Automática em Duas Linhas para Janelas Menores**:
  - Em terminais estreitos ou janelas menores (< 115 colunas), a barra inferior se desdobra automaticamente em duas linhas dedicadas:
    - **Linha 1 (Status & Posição)**: Badge de Modo (`NORMAL`, `INSERÇÃO`, `VISUAL`), nome completo da nota aberta (com indicador `[+]` de alterações não salvas) e posição do cursor / total de linhas (`L:C Linhas`).
    - **Linha 2 (Ações Rápidas & Botões Interativos)**: Todos os botões de ação ficam acessíveis, visíveis e centralizados:
      - `[➕ Nova]`: Criação rápida de novas notas em prompt dedicado.
      - `[✏️ Renomear]`: Renomeação física do arquivo e do título Markdown.
      - `[📄 Wrap: ON/OFF]`: Alternância de quebra automática com ocultação inteligente de URLs.
      - `[🔍 Buscar]`: Busca de termos dentro da nota.
      - `[🔄 Subst]`: Assistente de localização e substituição em duas etapas com Undo (`u`).
      - `[🎨 Tema]`: Ciclo instantâneo entre os 8 temas visuais integrados.
  - **Suporte Total ao Mouse na Linha 2**: Todos os botões na linha de quebra continuam 100% clicáveis pelo mouse, bem como os botões de filtro de urgência no painel lateral esquerdo.
  - **Telas Largas (≥ 115 colunas)**: A barra de status mantém-se elegante e condensada em uma única linha horizontal.
  - **Comando `:wrapbar`**: Permite alternar o comportamento sob demanda:
    - `:wrapbar` ou `:barwrap`: Alterna entre os modos automático, sempre 2 linhas e sempre 1 linha.
    - `:wrapbar on` / `:wrapbar 2`: Fixa a barra em 2 linhas.
    - `:wrapbar off` / `:wrapbar 1`: Fixa a barra em 1 linha.
    - `:wrapbar auto`: Restaura o ajuste dinâmico automático por resolução.

- **Correção Definitiva do Bug de Redimensionamento de Janela (Window Resize)**:
  - **Sem Travamentos ou Linhas Vazias (`~`)**: Redimensionar ou maximizar/restaurar a janela do terminal agora atualiza o layout imediatamente na mesma fração de segundo (captura de `SIGWINCH`), eliminando a necessidade de rolar a roda do mouse para desengasgar a tela.
  - **Clamping Preventivo de Rolagem**: O cálculo de rolagem (`editor_scroll()`) agora avalia as novas dimensões de tela antes de qualquer renderização e reposiciona automaticamente o deslocamento vertical (`scroll_y`) se o redimensionamento reduzir o número total de linhas virtuais visíveis.
  - **Limpeza Total do Buffer de Rolagem**: Emissão automática de `[2J[3J[H` no evento de redimensionamento, impedindo que resquícios de texto fiquem flutuando no scrollback do terminal.

---

## ✏️ Navegação Completa por Setas e Edição na Barra Inferior (Renomeação e Prompts)

- **Navegação Fluida por Setas no Nome da Nota**:
  - **`←` (Seta Esquerda) e `→` (Seta Direita)**: Movem o cursor de digitação de forma precisa por cada caractere (respeitando limites de caracteres UTF-8 multibyte), permitindo alterar qualquer parte do título da nota sem ter que apagar tudo.
  - **`Home` / `Ctrl + A`**: Salta o cursor diretamente para o início do texto do prompt.
  - **`End` / `Ctrl + E`**: Salta o cursor diretamente para o final do texto.
  - **`Ctrl + ←` e `Ctrl + →`**: Saltam palavras inteiras para trás e para frente.
- **Edição e Deleção Cirúrgica**:
  - **`Backspace`**: Apaga o caractere imediatamente anterior à posição do cursor, puxando o restante do texto suavemente.
  - **`Delete`**: Apaga o caractere exatamente sob o cursor.
  - **Inserção de Caracteres no Meio do Texto**: Digitar qualquer caractere ou sequência acentuada insere o texto exatamente na posição atual do cursor, sem jogar o texto para o final.
  - **`Ctrl + U`**: Limpa todo o campo de digitação instantaneamente.
  - **`Ctrl + K`**: Apaga do cursor até o fim da linha.
- **Cursor Terminal Sincronizado**: O cursor do console (`[5 q`) reflete com precisão milimétrica a coluna visual correspondente à posição do texto, eliminando qualquer travamento visual.
- **Aplicado Universalmente em Todos os Prompts**: O mesmo mecanismo de edição e navegação foi integrado a todos os prompts interativos da barra inferior: **Renomear Nota (`r`)**, **Nova Nota (`n`)**, **Localizar na Nota (`[🔍 Buscar]`)**, **Localizar e Substituir (`[🔄 Subst]`)** e na **Barra de Comandos (`:`)**.

---

## 📂 Navegação Rápida com `gg` / `G` e Dobragem de Seções (`zM`, `zR`, `za`)

- **Navegação `gg` e `G` no Editor**:
  - **`gg`**: Salta instantaneamente para o topo da nota (primeira linha visível), ajustando a rolagem visual para a primeira linha (`scroll_y = 0`).
  - **`G`**: Salta com segurança para o fim da nota (última linha visível). Se seções estiverem recolhidas, o cursor se posiciona exatamente sobre a última linha visível ou cabeçalho ativo, sem deixar linhas em branco nem perder o cursor de vista.
  - **Comando `:<linha>` (ex: `:150`)**: Salta diretamente para qualquer número de linha da nota aberta, desdobrando automaticamente a seção caso ela esteja recolhida.
- **Navegação `gg` e `G` na Barra Lateral (Lista de Notas)**:
  - Quando o foco está no painel esquerdo (`FOCUS_LEFT`), **`gg`** pula para a primeira nota da lista e **`G`** para a última nota.
- **Dobragem Instantânea de Títulos (`zM`, `zR`, `za`, `zc`, `zo`)**:
  - **`zM`** ou **`zm`** (ou comando **`:foldall`** / **`:zm`**): Recolhe todos os cabeçalhos Markdown da nota aberta (`#`, `##`, etc.). Se o cursor estiver em uma linha do corpo que foi ocultada, ele é reposicionado automaticamente no cabeçalho visível correspondente.
  - **`zR`** ou **`zr`** (ou comando **`:unfoldall`** / **`:zr`**): Expande todos os cabeçalhos Markdown da nota aberta.
  - **`za`** (ou comando **`:fold`** / **`:za`**): Alterna entre recolher e expandir o bloco do cabeçalho sob o cursor.
  - **`zc`** e **`zo`**: Fecham e abrem, respectivamente, a seção atual.
- **Janela de Digitação Confortável (1000ms)**:
  - O tempo limite de leitura para sequências compostas (`gg`, `gT`, `gt`, `gl`, `gq`, `zR`, `zM`, `za`) foi calibrado para 1000ms (1 segundo), eliminando o descarte acidental de comandos por digitação natural de letras com Shift. Caso uma tecla inesperada seja pressionada, ela não é descartada e é repassada ao editor.

---

## 🔍 Suporte Completo a Caracteres Especiais e Acentos na Busca

- **Entrada UTF-8 Multibyte Atômica**: Ao digitar caracteres acentuados ou especiais (`ç`, `á`, `é`, `í`, `ó`, `ú`, `ã`, `õ`, etc.) no campo de busca (`Ctrl + F` ou `/`), os bytes da sequência UTF-8 são montados de forma atômica no buffer da pesquisa.
- **Eliminação do Caracter de Substituição ()**: Corrigido o problema em que terminais Windows/Linux exibiam o losango com ponto de interrogação (`U+FFFD`) devido a leituras parciais de 1 byte ou renderização intermediária de bytes soltos.
- **Compatibilidade com Windows (UTF-8, CP1252 e CP850)**: Buffer de leitura do console expandido para capturar sequências completas do console Windows com fallback automático para conversão de páginas de código ANSI/OEM, garantindo funcionamento impecável tanto no Windows Terminal quanto no Prompt de Comando (CMD) e PowerShell.
- **Busca Insensível a Acentos**: A busca multi-palavras continua encontrando termos com ou sem acento (ex: buscar "cafe" ou "café" encontra notas contendo ambos).
- **Cursor e Backspace Precisos**: O cursor visual do campo de busca e o apagamento via `Backspace` respeitam os limites reais dos caracteres UTF-8 de múltiplos bytes.

---

## 💾 Novo Atalho `Ctrl + S` para Salvamento no Modo Normal

- No modo normal, basta pressionar **`Ctrl + S`** para salvar a nota aberta instantaneamente.
- Dispensa a necessidade de digitar `:w`, proporcionando fluxo de trabalho contínuo e ágil.
- O salvamento mantém o mecanismo de alta performance, atualizando metadados em memória sem engasgos de I/O.

---

## 🧭 Navegação Aperfeiçoada por Setas no Modo Inserção

- **Correção Definitiva de Caracteres Indevidos**: Corrigido o tratamento de códigos de tecla estendidos (ANSI/VT100 e SS3 `\x1bO...`) que podiam gerar caracteres espúrios no início ou fim de linhas em terminais específicos.
- **Navegação Suave e Encadeada**: Pressionar `←` no início de uma linha move o cursor para o final da linha anterior; pressionar `→` no final da linha move para o início da linha seguinte.
- **Bloqueio Rigoroso de Injeção de Bytes**: Qualquer código de controle acima de 255 é estritamente tratado como tecla funcional, impedindo qualquer injeção no buffer de texto.

---

## 🔘 Botões Interativos na Barra Inferior (Status Bar)

Localizados na barra de status inferior, acessíveis por clique do mouse ou comandos:
- **`[➕ Nova]`**: Cria uma nova nota interativamente via clique no rodapé, abrindo o prompt `📝 Nova nota:` pré-configurado com navegação completa por setas.
- **`[✏️ Renomear]`**: Renomeia a nota atual (arquivo físico e cabeçalho Markdown) via prompt pré-preenchido.
- **`[🗑️ Excluir]`**: Exclui a nota selecionada no painel esquerdo com confirmação prévia no rodapé (`Deseja realmente excluir a nota '<título>'? (s/n):`). Confirma com `s`/`y` e cancela com `n`/`Esc` ou clique fora. Comandos: `:delete`, `:del`, `:excluir`, `:d`.
- **`[📄 Wrap: ON/OFF]`**: Alterna a quebra de texto visual automática (*Soft Word Wrap*), mantendo URLs de links ocultas no Modo Normal.
- **`[🔍 Buscar]`**: Abre prompt interativo para localizar texto dentro do arquivo aberto.
- **`[🔄 Subst]`**: Assistente em duas etapas para localizar e substituir strings em toda a nota com suporte a Undo (`u`).
- **`[🎨 NomeDoTema]`**: Cicla instantaneamente entre os 8 temas do VS Code.
- *Layout Adaptativo*: Em telas menores (< 125 colunas), os botões compactam suavemente para `[➕]`, `[✏️]`, `[🗑️]`, `[W:ON]`, `[🔍]`, `[🔄]`, `[🎨]`, preservando 100% da usabilidade sem quebra de linhas no terminal.

---

## ⚠️ Marcação de Notas em Seis Níveis de Criticidade / Urgência

- **Classificação em 6 Níveis de Criticidade**:
  - **Nível 1 (Crítico / Máxima Urgência)**: Marcador **`[1]`** e título em **Vermelho** vibrante em negrito.
  - **Nível 2 (Médio / Atenção Intermediária)**: Marcador **`[2]`** e título em **Laranja** em negrito.
  - **Nível 3 (Baixo / Acompanhamento)**: Marcador **`[3]`** e título em **Amarelo** em negrito.
  - **Nível 4 - [H] (Azul / Ciano)**: Marcador **`[H]`** e título em **Azul / Ciano** em negrito.
  - **Nível 5 - [R] (Roxo / Violeta)**: Marcador **`[R]`** e título em **Roxo / Violeta** em negrito.
  - **Nível 6 - [O] (Verde)**: Marcador **`[O]`** e título em **Verde** em negrito.
  - **Sem Marcação (Normal)**: Marcador `[ ]` e título na cor padrão da listagem.
- **Como Alternar**:
  - Pressione o atalho **`m`** ou **`!`** na barra lateral ou no editor (modo normal) para ciclar: `Normal [ ]` ➔ `Nível 1` ➔ `Nível 2` ➔ `Nível 3` ➔ `[H]` ➔ `[R]` ➔ `[O]` ➔ `Normal [ ]`.
  - Ou clique com o mouse diretamente sobre a caixa da nota (colunas 1 a 5 da listagem).
  - Ou execute **`:atencao`** para ciclar, ou defina diretamente com **`:atencao 1`**, **`:atencao 2`**, **`:atencao 3`**, **`:atencao 4`** (ou `h`), **`:atencao 5`** (ou `r`), **`:atencao 6`** (ou `o`), ou **`:atencao 0`**.
- **Persistência**: Armazenado diretamente na coluna `attention` do banco SQLite (`notas.db`).

---

## 🎯 Botões Inferiores de Filtro por Criticidade na Barra Esquerda

- Na parte inferior da barra lateral de notas, há botões de clique rápido:
  - **`[Todas]`**: Exibe todas as notas.
  - **`[1]` (Vermelho)**: Filtra apenas notas de Criticidade Nível 1.
  - **`[2]` (Laranja)**: Filtra apenas notas de Criticidade Nível 2.
  - **`[3]` (Amarelo)**: Filtra apenas notas de Criticidade Nível 3.
  - **`[H]` (Azul)**: Filtra apenas notas de Criticidade Nível 4 (`[H]`).
  - **`[R]` (Roxo)**: Filtra apenas notas de Criticidade Nível 5 (`[R]`).
  - **`[O]` (Verde)**: Filtra apenas notas de Criticidade Nível 6 (`[O]`).
- **Acionamento**: Clique com o mouse no botão desejado, ou use as teclas **`1`**, **`2`**, **`3`**, **`4`** (ou `h`/`H`), **`5`** (ou `r`/`R`), **`6`** (ou `o`/`O`), ou **`0`** com o foco no painel esquerdo, ou os comandos **`:filtro 1/2/3/4/5/6/h/r/o/0`**. Clicar novamente no botão ativo desativa o filtro.

---

## ✂️ Corte de Linha Inteira com `V + x` (Pull Up)

- No modo Visual Linha (**`V`** / `V-LINE`), pressionar **`x`** corta as linhas selecionadas para a área de transferência do sistema e remove-as completamente do documento.
- A linha de baixo sobe imediatamente, preenchendo o espaço de forma limpa e fluida.

---

## 📑 Fechamento Rápido de Abas com a Tecla `c`

- No modo normal, basta pressionar **`c`** para fechar a aba ativa atual instantaneamente, transferindo o foco para a aba adjacente.

---

## 🔗 Suporte Completo a Links e WikiLinks (Arquivos, Pastas e Web)

O editor conta com resolução nativa de caminhos no Windows e Linux para todos os tipos de links Markdown com ocultação visual e suporte a URLs longas:
- **Ocultação Confiável de Links Longos (até 4096 caracteres)**: Corrigido o limite anterior de 500 caracteres que fazia links longos (como URLs de pesquisas, tokens, deep links e links da web) vazarem no Modo Normal. Agora, qualquer link de até 4096 bytes tem sua URL ocultada no Modo Normal, exibindo unicamente o título do link formatado em azul sublinhado.
- **Parênteses Balanceados em URLs**: Suporte nativo a URLs com parênteses internos (como links da Wikipédia `https://pt.wikipedia.org/wiki/C_(linguagem_de_programa%C3%A7%C3%A3o)`), capturando a URL completa sem cortes acidentais.
- **Links com `<URL>` e Atributos de Título**: Compatibilidade com a sintaxe Markdown estendida `[Título](<https://...>)` e com títulos secundários opcionais `[Título](url "descrição")`.
- **Buffer de Linhas Expandido (16 KB)**: O carregamento de arquivos foi ampliado para suportar linhas de até 16.384 bytes, impedindo que linhas longas sejam partidas ao meio.
- **Arquivos Locais**: `[Relatório](./relatorio.pdf)` ou `[Doc](C:\MeusDocumentos\dados.xlsx)` — abre no programa padrão do sistema (Word, Excel, Leitor de PDF, etc.).
- **Pastas Locais**: `[Pasta de Notas](./data)` ou `[Downloads](C:\Users\...\Downloads)` — abre a pasta diretamente no **Windows Explorer**.
- **Pastas Superiores**: `[Documentos](../documentos)` ou `[Backup](../../backup)` — navegação relativa pela árvore de diretórios.
- **Caminhos com Espaços**: `[Meu Arquivo](./minha pasta/meu doc.docx)` ou `[Arquivo](<./minha pasta/meu doc.docx>)`.
- **Links Web**: `[Google](https://www.google.com)` — abre no navegador padrão.
- **WikiLinks Internos**: `[[Nome da Nota]]` — abre a nota correspondente em aba ou cria automaticamente o arquivo `data/<Nome da Nota>.md` caso não exista.
- **Como Ativar**: No Modo Normal, posicione o cursor sobre o link e pressione **`Enter`** (ou use **`Ctrl + N`** / **`gl`** para saltar de link em link).

---

## ⚡ Salvamento de Notas Ultra-Rápido e Otimizado

- Tanto o comando **`:w`** quanto o atalho **`Ctrl + S`** utilizam cache em memória RAM e validação de `mtime`:
  - Apenas o arquivo salvo tem seu cache de busca e metadados atualizado, eliminando varreduras redundantes de disco.
  - O salvamento é executado em menos de 1 milissegundo, mesmo em pastas com centenas de notas.

---

## 🗄️ Onde as notas ficam agora (v6.29.0+)

As notas residem em **`notas.db`** (SQLite), ao lado do executavel — **nao**
mais em `data/*.md`. Isso vale para criacao (`n`), renomeacao (`r`), exclusao
(`d`) e salvamento. O diretorio `data/` e ignorado; para aproveitar um acervo
antigo de `.md`, use `[⬇ Importar]` uma vez.

A data de modificacao exibida na lateral vem da coluna `mtime` do banco, e os
niveis de urgencia (`[1]`/`[2]`/`[3]`) vem da coluna `attention` — o antigo
arquivo `data/.attention` nao e mais usado.

## 📂 Gerenciamento Completo de Notas na Barra Lateral

- **`n` (Nova Nota)**: Solicita o nome no rodapé (`📝 Nova nota:`), cria `data/<nome>.md` com cabeçalho `# <nome>` e abre imediatamente em aba no editor.
- **`r` (Renomear Nota)**: Prompt pré-preenchido que renomeia o arquivo `.md` e atualiza o cabeçalho `# Novo Nome`.
- **`d` (Excluir Nota)**: Confirmação prévia no rodapé (`(s/n)`), fecha as abas associadas e remove o arquivo.
- **`R` (Recarregar Lista)**: Reescaneia o diretório `data/` refletindo alterações externas.
- **Comandos `:rename <nome>` e `:rn <nome>`**: Renomeação via barra de comandos.

---

## 📊 Tabelas Markdown Auto-Alinhadas (Estilo Vimwiki)

- Pressione **`Tab`** no modo inserção para auto-alinhar as colunas e saltar para a próxima célula (cria nova linha automaticamente).
- Pressione **`Enter`** no modo inserção para auto-alinhar e inserir uma nova linha formatada.
- Pressione **`gqq`** ou use **`:table`** (`:ta`) no modo normal para alinhar a tabela sob o cursor. Para todas as tabelas, use **`:tableall`**.

---

## 📄 Quebra de Linha Automática (Word Wrap Inteligente)

- Ativado por padrão (*Soft Wrap*): quebra linhas na margem sem cortar palavras.
- Navegação vertical suave: `j` e `k` percorrem cada linha visual do parágrafo.
- Alterne via botão **`[Wrap]`** ou comandos **`:wrap`** e **`:nowrap`**.

---

## 🎨 Temas do VS Code (Claros e Escuros)

8 temas em TrueColor (24-bit RGB) ANSI:
- **Escuros**: *Tokyo Night* (Padrão), *One Dark Pro*, *Dracula Official*, *Monokai Pro*, *GitHub Dark*.
- **Claros**: *GitHub Light*, *VS Code Light+*, *Solarized Light*.
- Alterne pelo botão **`[🎨 Tema]`**, pelo atalho **`Ctrl + T`** ou pelo comando **`:theme [nome]`**.

---

## 🚀 Como Compilar e Executar

> **A partir da v6.29.0 a libsqlite3 e obrigatoria** (as notas vivem em `notas.db`).

### O jeito recomendado: os scripts de build

```bash
./build.sh linux      # -> ./notas
./build.sh windows    # -> ./notas.exe  (requer mingw-w64 + libsqlite3 p/ MinGW)
./build.sh all        # os dois
./build.sh run        # compila e executa
./build.sh clean      # remove os binarios
```

No Windows: `build.bat` (ou `build.bat run`).

### No Linux, manualmente

```bash
gcc -O2 -Wall -Wextra notas.c -o notas -lsqlite3 -ldl
./notas
```

A libsqlite3 vem no pacote `sqlite` (o header `sqlite3.h` esta em `/usr/include`).
No Arch/Omarchy: `sudo pacman -S sqlite`.

### No Windows (MinGW / GCC), manualmente

```cmd
gcc -O2 -Wall -Wextra notas.c -o notas.exe -lsqlite3 -lkernel32 -luser32 -lshell32
notas.exe
```

### Cross-compilacao a partir do Linux (MinGW-w64)

O repositorio **nao** tem um pacote `mingw-w64-sqlite3` no Arch, entao a
libsqlite3 para Windows e gerada uma vez a partir da amalgamation oficial e
versionada em `tools/sqlite3-mingw/` (~1,3 MB). Para regenerar:

```bash
./tools/build_sqlite_mingw.sh     # baixa, cross-compila e instala
./build.sh windows                # usa tools/sqlite3-mingw/ e gera notas.exe
```

A biblioteca entra **estaticamente**: o `notas.exe` fica com ~1,5 MB e nao
precisa de nenhum DLL ao lado. Detalhes em `tools/BUILD_WINDOWS_SQLITE.md`.

---

## ⌨️ Tabela Resumo de Atalhos e Comandos

| Atalho / Comando | Ação |
| :--- | :--- |
| **`Ctrl + S`** (modo normal) | **Salva a nota aberta instantaneamente** |
| **`Setas (←, →, ↑, ↓)`** (inserção) | **Navegação fluida sem caracteres indevidos** |
| **`[➕ Nova]`** (clique) | **Cria uma nova nota** via prompt interativo |
| **`:new`** / **`:nova`** | **Abre prompt de nova nota** a partir da barra de comandos |
| **`[📄 Wrap: ON/OFF]`** (clique) | **Alterna quebra de linha visual** |
| **`[🔍 Buscar]`** (clique) | **Abre prompt para localizar na nota** |
| **`[🔄 Subst]`** (clique) | **Abre assistente para substituir texto** |
| **`[✏️ Renomear]`** (clique) | **Renomeia a nota atual** |
| **`[🎨 Tema]`** (clique) | **Cicla tema do VS Code** |
| **`[🧬 Life]`** (clique) | **Roda o Game of Life sob demanda** (para ao clicar qualquer tecla ou mouse) |
| **`[⬇ Importar]`** (clique) | **Importa uma pasta de arquivos `.md` para o banco SQLite** (pede a pasta no rodape) |
| **`[⬆ Exportar]`** (clique) | **Exporta o banco para uma pasta de arquivos `.md`** (pede a pasta no rodape) |
| **`c`** (no modo normal) | **Fecha a aba atual ativa** |
| **`V + x`** (no visual linha) | **Corta a linha inteira fazendo a de baixo subir** |
| **`m`** ou **`!`** (na lista / editor) | **Cicla criticidade em 6 níveis (1=Vermelho, 2=Laranja, 3=Amarelo, 4/H=Azul, 5/R=Roxo, 6/O=Verde, Normal)** |
| **`:atencao [1/2/3/4/5/6/h/r/o/0]`** | **Define ou cicla nível de criticidade** |
| **`:filtro [1/2/3/4/5/6/h/r/o/0]`** | **Filtra notas na lista por criticidade** |
| **`:s/antigo/novo/`** | **Substitui string na nota ativa** |
| **`n`** (na lista) | **Cria nova nota** com prompt de nome |
| **`r`** (na lista) | **Renomeia nota selecionada** |
| **`d`** (na lista) | **Exclui nota** com confirmação prévia |
| **`y`** (na lista) | **Copia nome da nota** (sem `.md`) para a área de transferência |
| **`C`** (no editor/lista) | **Fecha todas as abas abertas** (exibe aba `[➕ Nova Nota]`) |
| **`:closeall`** / **`:ca`** | **Fecha todas as abas abertas** via comando |
| **`[🗑️ Excluir]`** (barra inferior) | **Exclui nota selecionada no painel esquerdo** com confirmação prévia |
| **`:delete`** / **`:del`** / **`:excluir`** | **Exclui nota selecionada** via comando |
| **`r`** (no modo normal) | **Substitui caractere sob o cursor** (estilo Vim; `r<Enter>` divide linha, `Esc` cancela) |
| **`r`** (no modo visual) | **Substitui todos os caracteres da seleção** pelo caractere digitado |
| **`:life`** / **`:gameoflife`** / **`:gol`** | **Inicia simulação do Game of Life** (protetor de tela após 10min ocioso) |
| **`R`** (na lista) | **Atualiza lista de notas** do disco |
| **`:rename <nome>`** / **`:rn`** | **Renomeia nota** via comando |
| **`:w`** | **Salva a nota instantaneamente** |
| **`:autosave`** | **Exibe status do salvamento automático** (1 minuto de edições) |
| **`gg`** / **`G`** (no editor) | **Pula para o topo / fim da nota** (respeitando seções recolhidas) |
| **`gg`** / **`G`** (na lista) | **Pula para a primeira / última nota** da barra lateral |
| **`zM`** / **`zm`** (ou `:foldall`) | **Recolhe todas as seções / cabeçalhos** da nota |
| **`zR`** / **`zr`** (ou `:unfoldall`) | **Expande todas as seções / cabeçalhos** da nota |
| **`za`** (ou `:fold`) | **Alterna recolhimento da seção** sob o cursor |
| **`:<número>`** (ex: `:150`) | **Pula diretamente para a linha indicada** |
| **`Espaço`** (no modo normal) | Insere um caractere de espaço no cursor |
| **`Ctrl + N`** (ou `gl`) | Pula para o próximo link na nota |
| **`Tab`** / **`Enter`** (na tabela) | Auto-alinha tabela Markdown (estilo Vimwiki) |
| **`gqq`** / **`:table`** (`:ta`) | Reformatar tabela sob o cursor |
| **`:tableall`** | Alinhar todas as tabelas do documento |
| **`:wrap`** / **`:nowrap`** | Ativa / desativa quebra de texto visual |
| **`Esc` (1 toque)** | Retorna imediatamente ao modo Normal |
| **Mouse Wheel Click** (na aba) | Fecha a aba clicada |
| **`gt`** / **`gT`** | Próxima / anterior aba |
| **`Tab`** ou **`Ctrl + W`** | Alterna foco entre notas e editor |
| **`Ctrl + F`** ou `/` | Busca multi-palavras em tempo real |
| **`u`** / **`Ctrl + R`** | Desfazer / Refazer |
| **`:q`** / **`:wq`** / **`:q!`** | Fechar aba / Salvar e fechar / Forçar saída |
