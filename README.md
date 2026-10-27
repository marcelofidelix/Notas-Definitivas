# 📝 Notas Definitivas

[![C99](https://img.shields.io/badge/Language-C99-00599C?logo=c&logoColor=white)](https://en.wikipedia.org/wiki/C99)
[![SQLite3](https://img.shields.io/badge/Database-SQLite%203.45+-003B57?logo=sqlite&logoColor=white)](https://www.sqlite.org/)
[![Platform](https://img.shields.io/badge/Platform-Linux%20%7C%20Windows-blue)](https://github.com/marcelofidelix/Notas-Definitivas)
[![Interface](https://img.shields.io/badge/Interface-TUI%20%28Vim%20%2F%20Neovim%20Modal%29-green)](https://neovim.io)
[![License](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

Editor de texto e gerenciador de notas Markdown para terminal (**TUI**), minimalista, ultrarrápido, de arquivo único em C (`notas.c`), com banco de dados embutido **SQLite 3** (`notas.db`), compatibilidade total multiplataforma (**Linux** e **Windows** nativos) e fluxo de trabalho inspirado no **Vim / Neovim** e **Vimwiki**.

Projetado para oferecer velocidade extrema (0 ms de latência), independência de dependências externas complexas e ergonomia profissional para desenvolvedores, escritores e administradores de sistemas.

---

## 📑 Sumário

- [Visão Geral e Destaques](#-visão-geral-e-destaques)
- [Edição Modal Estilo Neovim / Vim](#-edição-modal-estilo-neovim--vim)
  - [Atalhos de Cópia, Cola e Recorte (`y`, `p`, `x`)](#atalhos-de-cópia-cola-e-recorte-y-p-x)
  - [Modos do Editor](#modos-do-editor)
- [Organização por Criticidade e Prioridade](#-organização-por-criticidade-e-prioridade)
- [Armazenamento SQLite e Importação / Exportação](#-armazenamento-sqlite-e-importação--exportação)
- [Recursos de Markdown e Produtividade](#-recursos-de-markdown-e-produtividade)
  - [Tabelas Auto-Alinhadas](#tabelas-auto-alinhadas-estilo-vimwiki)
  - [Dobragem de Seções (Folds)](#dobragem-de-seções-folds)
  - [Ocultação de Links e Estilização](#ocultação-de-links-e-estilização)
  - [Quebra Automática de Linhas (Word Wrap)](#quebra-automática-de-linhas-word-wrap)
- [Área de Trabalho, Abas e Temas](#-área-de-trabalho-abas-e-temas)
- [Game of Life Interativo](#-game-of-life-interativo)
- [Como Compilar e Executar](#-como-compilar-e-executar)
  - [Compilação no Windows (MinGW)](#no-windows-mingw--gcc)
  - [Compilação no Linux](#no-linux-gcc--clang)
  - [Cross-compilação Linux -> Windows](#cross-compilação-linux---windows)
- [Tabela Completa de Atalhos e Comandos](#-tabela-completa-de-atalhos-e-comandos)
- [Robustez, Integridade e Desempenho](#-robustez-integridade-e-desempenho)

---

## ⚡ Visão Geral e Destaques

- **Zero Dependências de Runtime**: Binário único autocontido; o SQLite 3 é compilado estaticamente, dispensando DLLs externas no Windows ou pacotes pesados no Linux.
- **Edição Modal Eficiente**: Navegação rápida no Modo Normal (`h, j, k, l`, `gg`, `G`, `w`, `b`), Modo Visual (`v`, `V`, `Shift + Setas`), Modo Inserção (`i`, `a`, `o`) e Linha de Comandos (`:`).
- **Clipboard Bidirecional Assíncrono**:
  - Cópia instantânea em segundo plano via `CreateThread` / `pthread` sem travamentos de console (0 ms de latência).
  - Suporte completo a acentos e caracteres UTF-8 (`CF_UNICODETEXT` no Windows, `wl-paste`/`xclip` no Linux, `clip.exe` no WSL).
- **Substituição Fluida com Seleção**:
  - **`y`**: Copia (*yank*) o texto selecionado ou a linha atual.
  - **`p`**: Cola (*paste*) e **substitui** a seleção ativa pelo conteúdo da área de transferência (exatamente como no Neovim).
  - **`x`**: Recorta (*cut*) o caractere sob o cursor ou toda a seleção ativa, copiando para o clipboard e apagando do documento.
- **Banco de Dados SQLite Integrado (`notas.db`)**:
  - Modo WAL ativado para concorrência e gravações atômicas instantâneas.
  - Importação de pastas inteiras de arquivos `.md` e exportação com um clique.
- **Salvamento Automático Silencioso**: Notas modificadas são gravadas periodicamente a cada 1 minuto de inatividade. A digitação contínua é agrupada em um único passo de `u` (undo), como uma sessão de inserção do Vim, sem consumir um snapshot por tecla.
- **8 Temas do VS Code em Cores RGB Reais (24-bit TrueColor)**: *Tokyo Night*, *One Dark Pro*, *Dracula*, *Monokai Pro*, *GitHub Dark*, *GitHub Light*, *VS Code Light+* e *Solarized Light*.

---

## 🎯 Edição Modal Estilo Neovim / Vim

### Atalhos de Cópia, Cola e Recorte (`y`, `p`, `x`)

O gerenciamento de texto opera sob a mesma convenção intuitiva do Neovim:

| Tecla | Sem Seleção (Modo Normal) | Com Seleção Ativa (Modo Visual / Shift+Setas / Mouse) |
| :---: | :--- | :--- |
| **`y`** | **Copia (Yank)** a linha inteira para a área de transferência (`yy` ou `y`) | **Copia (Yank)** o trecho selecionado para a área de transferência do editor e do sistema |
| **`p`** | **Cola (Paste)** o conteúdo da área de transferência após o cursor | **Substitui a seleção**, colando o conteúdo do clipboard exatamente no lugar do texto selecionado |
| **`x`** | **Recorta (Cut)** o caractere UTF-8 sob o cursor para o clipboard | **Recorta (Cut)** toda a seleção de texto para o clipboard, apagando-a do documento |
| **`d`** | `dd` exclui a linha inteira (enviando para o buffer) | Exclui a seleção ativa (enviando para o buffer de corte) |
| **`u`** | **Desfaz (Undo)** a última alteração | — |
| **`Ctrl+R`**| **Refaz (Redo)** a alteração desfeita | — |

> **💡 Dica**: A seleção de texto via arrasto do mouse não sobrescreve destrutivamente o seu clipboard. Você pode copiar um texto em qualquer outro aplicativo (navegador, terminal, IDE), selecionar com o mouse uma palavra no *Notas Definitivas* e pressionar **`p`** para substituí-la na hora!

---

### Modos do Editor

1. **Modo Normal**:
   - Pressione `Esc` a qualquer momento para retornar ao Modo Normal.
   - Navegação: `h` (esquerda), `j` (baixo), `k` (cima), `l` (direita), `w` (próxima palavra), `b` (palavra anterior), `0` (início da linha), `$` (fim da linha), `gg` (início da nota), `G` (fim da nota).
   - Entrada em edição: `i` (inserir antes do cursor), `a` (inserir após o cursor), `I` (início da linha), `A` (fim da linha), `o` (nova linha abaixo), `O` (nova linha acima).
   - Substituição de caractere: **`r`** sob o cursor entra em espera de tecla única (`r<Enter>` quebra a linha, `Esc` cancela).
   - Salvamento rápido: **`Ctrl + S`** ou `:w`.

2. **Modo Visual** (`v` por caractere, `V` por linha):
   - Ativado com `v`, `V`, `Shift + Setas` ou arrasto com o mouse.
   - `y` copia a seleção para o clipboard.
   - `p` cola por cima substituindo a seleção.
   - `x` ou `d` recorta/exclui a seleção.
   - `r` substitui todos os caracteres da seleção pelo caractere seguinte.

3. **Modo Inserção** (`i`):
   - Digitação fluida com cursor em barra vertical piscante.
   - `Ctrl + V`: Cola da área de transferência do sistema (com suporte total a acentuação UTF-8).
   - `Ctrl + C`: Copia texto selecionado com `Shift + Setas`.
   - `Shift + Setas` ou `Ctrl + Shift + Setas`: Seleciona trechos sem sair do modo inserção (`INSERT [SEL]`).
   - `Tab` / `Enter`: Alinha automaticamente tabelas Markdown.
   - Rejeição inteligente de toques acidentais de touchpad (<250ms).

4. **Modo de Busca** (`Ctrl + F` ou `/`):
   - Busca em tempo real insensível a maiúsculas e a acentuação.
   - Navegação completa por setas (`←`, `→`, `Home`, `End`, `Ctrl + ←/→`, `Backspace`, `Delete`).
   - `↑` e `↓` alternam entre as notas encontradas exibindo pré-visualização ao vivo.
   - `Enter` abre a nota selecionada em aba.

5. **Linha de Comandos** (`:`):
   - Digite comandos do editor como `:w`, `:q`, `:wq`, `:theme tokyo`, `:wrap`, `:autosave`, `:table`, `:life`, etc.

---

## 🏷️ Organização por Criticidade e Prioridade

As notas são ordenadas na barra lateral por níveis visuais de criticidade, mantendo sempre as notas prioritárias no topo e agrupadas da mais recente para a mais antiga:

| Nível | Identificador | Cor / Destaque | Significado |
| :---: | :---: | :---: | :--- |
| **1** | `[1]` | **Vermelho (Bold)** | Crítico / Máxima Urgência |
| **2** | `[2]` | **Laranja (Bold)** | Médio / Atenção Intermediária |
| **3** | `[3]` | **Amarelo (Bold)** | Baixo / Acompanhamento |
| **4** | `[H]` | **Azul / Ciano (Bold)** | Destaque H (*Highlight*) |
| **5** | `[R]` | **Roxo / Violeta (Bold)** | Referência R |
| **6** | `[O]` | **Verde (Bold)** | Operacional / Organizado |
| **0** | `[ ]` | Cor Padrão | Sem marcação |

- **Ciclar Criticidade**: Pressione **`m`** ou **`!`** na lista ou no editor para avançar o nível da nota atual (`[ ]` ➔ `[1]` ➔ `[2]` ➔ `[3]` ➔ `[H]` ➔ `[R]` ➔ `[O]` ➔ `[ ]`).
- **Filtros Rápidos no Rodapé da Lista**:
  - Pressione as teclas `1`, `2`, `3`, `4`/`H`, `5`/`R`, `6`/`O`, ou `0` (Todas) na barra lateral.
  - Ou clique diretamente sobre os botões `[Todas] [1] [2] [3] [H] [R] [O]` na base do painel esquerdo (os botões adaptam seus rótulos dinamicamente conforme a largura do painel).

---

## 🗄️ Armazenamento SQLite e Importação / Exportação

A partir da versão 6.29+, todas as notas residem no banco de dados **`notas.db`** (SQLite 3), localizado na mesma pasta do executável:

- **Modo WAL**: O banco utiliza *Write-Ahead Logging* para leituras simultâneas sem bloqueios e transações atômicas ultra-rápidas.
- **Banco Inicial Vazio**: O programa nasce limpo sem arquivos temporários de exemplo.
- **Importação `[⬇ Importar]`**:
  - Clique no botão ou execute `:import <pasta>` para ler uma pasta com arquivos `.md`, `.markdown`, `.txt`.
  - Reimportar uma pasta atualiza notas existentes em vez de duplicá-las.
- **Exportação `[⬆ Exportar]`**:
  - Clique no botão ou execute `:export <pasta>` para despejar todas as notas do banco como arquivos Markdown puros em um diretório à sua escolha.

---

## 📊 Recursos de Markdown e Produtividade

### Tabelas Auto-Alinhadas (Estilo Vimwiki)
- Pressione **`Tab`** dentro de uma linha de tabela (`|`) no Modo Inserção: as colunas são perfeitamente alinhadas e o cursor salta para a célula seguinte (ou cria uma nova linha formatada se estiver na última).
- Pressione **`gqq`** ou use o comando **`:table`** (`:ta`) no Modo Normal para alinhar a tabela sob o cursor. Use **`:tableall`** para reformatar todas as tabelas do documento.

### Dobragem de Seções (Folds)
- **`za`** (ou `:fold`): Alterna o recolhimento do bloco de título Markdown (`#`, `##`, `###`) sob o cursor.
- **`zM`** (ou `:foldall` / `:zm`): Recolhe todos os cabeçalhos da nota de uma só vez.
- **`zR`** (ou `:unfoldall` / `:zr`): Expande todos os cabeçalhos.
- **`:<número>`** (ex: `:120`): Salta para a linha especificada, desdobrando a seção se estiver recolhida.

### Ocultação de Links e Estilização
- No Modo Normal, as URLs longas de links `[Título](url)` e WikiLinks `[[Nota]]` são ocultadas, renderizando de forma limpa apenas o título sublinhado em azul.
- Ao entrar no Modo Inserção (`i`), a sintaxe completa com a URL é reexibida para facilitar a edição.
- Pressione **`Enter`** sobre qualquer link no Modo Normal para abri-lo:
  - URLs da Web (`https://...`): Abrem no navegador padrão.
  - Arquivos locais (`./doc.pdf`, `C:\...`): Abrem no aplicativo padrão do sistema.
  - Pastas locais (`./data`): Abrem diretamente no explorador de arquivos.
  - WikiLinks (`[[Nome da Nota]]`): Abrem diretamente a nota no editor.

### Quebra Automática de Linhas (Word Wrap)
- *Soft Word Wrap* inteligente: quebra linhas longas na margem da janela sem quebrar palavras ao meio e mantendo o arquivo no disco sem quebras artificiais.
- Pressione o botão **`[📄 Wrap]`** ou use os comandos **`:wrap`** e **`:nowrap`**.

---

## 🎨 Área de Trabalho, Abas e Temas

- **Sistema Completo de Abas (Tabs)**:
  - Abra múltiplas notas simultaneamente na barra superior.
  - Navegue entre abas com **`gt`** (próxima) e **`gT`** (anterior), ou clicando diretamente no nome da aba.
  - Feche a aba atual com **`c`** (ou clique com o botão do meio do mouse na aba).
  - Feche todas as abas abertas de uma vez com **`C`** (Shift + C) ou `:closeall`.
  - Quando não houver abas abertas, uma aba interativa **`[➕ Nova Nota]`** fica disponível no topo.
- **Redimensionamento Fluido do Painel Lateral**:
  - Ajuste a largura do painel esquerdo a qualquer momento no Modo Normal pressionando **`Ctrl + Seta Esquerda`** e **`Ctrl + Seta Direita`**.
- **Seletor de Temas**:
  - Alterne entre os 8 temas TrueColor através do botão **`[🎨 Tema]`**, pelo atalho **`Ctrl + T`** ou pelo comando **`:theme <nome>`**.

---

## 🧬 Game of Life Interativo

Um simulador completo do **Jogo da Vida de Conway** integrado como protetor de tela e descanso visual:
- Clique no botão **`[🧬 Life]`** no rodapé ou digite **`:life`** no Modo de Comando.
- Ativação automática após **10 minutos** de inatividade.
- A simulação utiliza as cores oficiais do tema ativo do VS Code e conta com algoritmos anti-estagnação (reinjeta gliders e pulsars caso a população se torne estática).
- **Despertar Instantâneo**: Pressione qualquer tecla ou mova/clique o mouse para voltar imediatamente à edição.

---

## 🚀 Como Compilar e Executar

### No Windows (MinGW / GCC)

O projeto conta com o script de compilação automatizado [`build.bat`](build.bat), que detecta as bibliotecas necessárias:

```cmd
build.bat          REM Compila -> notas.exe
build.bat run      REM Compila e executa notas.exe
build.bat clean    REM Remove notas.exe
```

**Compilação manual com GCC (MinGW-w64)**:
```cmd
gcc -O2 -Wall -Wextra notas.c -o notas.exe -Itools/sqlite3-mingw/include -Ltools/sqlite3-mingw/lib -lsqlite3 -lkernel32 -luser32 -lshell32
notas.exe
```

---

### No Linux (GCC / Clang)

Utilize o script de compilação automatizado [`build.sh`](build.sh):

```bash
./build.sh linux    # Compila -> ./notas
./build.sh run      # Compila e executa
./build.sh clean    # Remove os binários
```

**Compilação manual com GCC**:
```bash
gcc -O2 -Wall -Wextra notas.c -o notas -lsqlite3 -ldl
./notas
```

*Pré-requisito (Linux)*: Biblioteca SQLite e headers de desenvolvimento (`sqlite3.h`).
- Arch / Manjaro / Omarchy: `sudo pacman -S sqlite`
- Ubuntu / Debian: `sudo apt install libsqlite3-dev`
- Fedora: `sudo dnf install sqlite-devel`

---

### Cross-compilação Linux -> Windows

O repositório já inclui a biblioteca SQLite estática para Windows em `tools/sqlite3-mingw/`:

```bash
./build.sh windows   # Cross-compila -> ./notas.exe (requer x86_64-w64-mingw32-gcc)
```

---

## ⌨️ Tabela Completa de Atalhos e Comandos

### Modos e Navegação
| Tecla / Comando | Modo | Ação |
| :--- | :---: | :--- |
| **`Esc`** | Qualquer | Retorna imediatamente ao **Modo Normal** e cancela prompts |
| **`i` / `a`** | Normal | Entra no **Modo Inserção** antes / depois do cursor |
| **`I` / `A`** | Normal | Entra no Modo Inserção no início / fim da linha |
| **`o` / `O`** | Normal | Abre nova linha abaixo / acima e entra no Modo Inserção |
| **`v` / `V`** | Normal | Inicia o **Modo Visual** (caractere) / **Modo Visual Linha** |
| **`h, j, k, l`** | Normal | Move o cursor para a esquerda, baixo, cima e direita |
| **`w` / `b`** | Normal | Salta para o início da próxima palavra / palavra anterior |
| **`0` / `$`** | Normal | Salta para o início / fim da linha atual |
| **`gg` / `G`** | Normal | Salta para a primeira / última linha da nota (ou nota da lista) |
| **`:<número>`** | Normal | Salta diretamente para a linha informada (ex: `:45`) |
| **`Tab` ou `Ctrl+W`** | Normal | Alterna o foco entre a lista de notas (painel esquerdo) e o editor |
| **`Ctrl+←` / `Ctrl+→`** | Normal | Reduz / aumenta a largura da barra lateral de notas |

### Cópia, Cola, Recorte e Edição
| Tecla / Comando | Modo | Ação |
| :--- | :---: | :--- |
| **`y`** | Normal (sem seleção) | **Copia (Yank)** a linha atual para a área de transferência (`yy` ou `y`) |
| **`y`** | Visual (com seleção) | **Copia (Yank)** a seleção para a área de transferência do editor e do sistema |
| **`p`** | Normal (sem seleção) | **Cola (Paste)** o conteúdo da área de transferência após o cursor |
| **`p`** | Visual (com seleção) | **Substitui a seleção**, colando o texto do clipboard sobre o texto selecionado |
| **`x`** | Normal (sem seleção) | **Recorta (Cut)** o caractere sob o cursor para o clipboard |
| **`x`** | Visual (com seleção) | **Recorta (Cut)** a seleção de texto para o clipboard, apagando-a do documento |
| **`dd`** | Normal | Exclui a linha atual enviando-a para o buffer |
| **`r`** | Normal | Substitui o caractere sob o cursor pela próxima tecla digitada |
| **`r`** | Visual | Substitui todos os caracteres da seleção pela tecla digitada |
| **`u`** | Normal | Desfaz a última ação (**Undo**) |
| **`Ctrl + R`** | Normal | Refaz a ação desfeita (**Redo**) |
| **`Ctrl + S`** / **`:w`** | Normal | **Salva a nota aberta** instantaneamente no banco SQLite |
| **`Ctrl + V`** | Inserção | Cola da área de transferência com fidelidade a acentuação UTF-8 |
| **`Ctrl + C`** | Inserção/Visual | Copia a seleção atual para a área de transferência |
| **`Ctrl + X`** | Inserção | Recorta a seleção para a área de transferência |
| **`Ctrl + A`** | Inserção | Seleciona todo o texto do documento |
| **`Shift + Setas`** | Inserção/Normal | Seleciona texto diretamente sem sair do modo atual |

### Gerenciamento de Notas e Abas
| Tecla / Comando | Painel / Modo | Ação |
| :--- | :---: | :--- |
| **`n`** / **`[➕ Nova]`** | Lista / Rodapé | Cria uma nova nota interativamente |
| **`r`** / **`[✏️ Renomear]`** | Lista / Rodapé | Renomeia a nota selecionada |
| **`d`** / **`[🗑️ Excluir]`** | Lista / Rodapé | Exclui a nota selecionada com prompt de confirmação `(s/n)` |
| **`y`** | Lista | Copia o nome da nota selecionada para criação rápida de `[[WikiLinks]]` |
| **`m`** ou **`!`** | Lista / Editor | Cicla a criticidade da nota (`[ ]` ➔ `[1]` ➔ `[2]` ➔ `[3]` ➔ `[H]` ➔ `[R]` ➔ `[O]`) |
| **`1` a `6`, `0`** | Lista | Filtra a lista por criticidade (`1` a `6`, ou `0` para exibir Todas) |
| **`c`** | Normal | Fecha a aba ativa atual |
| **`C`** / **`:closeall`** | Normal | Fecha todas as abas abertas de uma só vez |
| **`gt` / `gT`** | Normal | Salta para a próxima aba / aba anterior |
| **`[⬇ Importar]`** / **`:import [pasta]`** | Rodapé / Comando | Importa arquivos Markdown de uma pasta para o SQLite (em lote com transação atômica) |
| **`[⬆ Exportar]`** / **`:export [pasta]`** | Rodapé / Comando | Exporta todas as notas do SQLite para arquivos Markdown em pasta |
| **`[🎨 Tema]`** / **`Ctrl+T`** | Qualquer | Cicla instantaneamente entre os 8 temas do VS Code |
| **`[📄 Wrap]`** / **`:wrap`** | Qualquer | Alterna quebra automática de linha (*Soft Word Wrap*) |
| **`[🧬 Life]`** / **`:life`** | Qualquer | Inicia a simulação do Game of Life |
| **`:q`** / **`:wq`** / **`:q!`** | Comando | Fecha aba / Salva e sai / Sai forçadamente |

---

## 🛡️ Robustez, Integridade e Desempenho

Esta versão passou por uma rodada de endurecimento focada em falhas reais de memória, persistência e renderização:

| Área | Melhoria |
| :--- | :--- |
| **Escrita de arquivos** | Exportação e o fallback de arquivo real usam gravação atômica (temporário + `fsync` + `rename`), em vez de truncar o destino antes de escrever. |
| **SQLite** | `db_open()` fecha o handle em qualquer falha de schema; a versão do schema é lida antes de gravar; IDs virtuais (`db:<id>`) são validados com `strtol`; importação só confirma com `COMMIT` verificado e faz `ROLLBACK` em erro. |
| **Abas e saída** | `:q` verifica alterações em **todas** as abas; `:wq` salva todas as abas modificadas; fechar aba/`C`/`:closeall` não descarta dados se a gravação falhar. |
| **Tabelas Markdown** | A linha nova deixou de usar buffer fixo de 512 bytes (risco de estouro de pilha); tabelas com mais de 32 colunas são recusadas em vez de perder colunas silenciosamente. |
| **Renderização** | O cursor visual avança por linha/segmento em vez de varrer o documento inteiro a cada linha da tela. Em cadernos grandes isso reduz o custo de `O(linhas × altura)` para `O(linhas + altura)` por quadro. |
| **Prompts e mouse** | Os 7 prompts modais usam fonte única de verdade (`prompt_label`), sincronizando renderização, cálculo de coluna do cursor e hit-testing do mouse (corrigindo o deslocamento de clique em `:import`/`:export`). |
| **Substituição (`:s/`)** | Correção de prefixo em `:sub/` e ampliação dos buffers para 256 bytes com proteção contra truncamento ou perda silenciosa em padrões longos. |
| **Undo/Redo** | Snapshots completos continuam existindo, mas teclas consecutivas na mesma posição são agrupadas (janela de 800 ms), reduzindo memória e tempo por caractere. |
| **Clipboard** | Corrigido o padding do Base64 do OSC 52 (`YQ==`, `YWI=`), que fazia terminais rejeitarem cópias de 1 ou 2 bytes; a leitura do clipboard Win32 é limitada ao tamanho real do bloco. |
| **Links e caminhos** | No Linux, links são abertos com `fork`/`exec` (sem shell, sem injeção); no Windows, caminhos UTF-8 são convertidos para UTF-16 sem limite de 1024 bytes e a URL é validada antes do `ShellExecuteW`. |
| **Terminal e buffers** | Escritas tratam envios parciais e `EINTR`; `LINES`/`COLUMNS` são lidos com `strtol` com validação e limite; buffers de escape ANSI expandidos para 64 bytes e uso de `safe_strcat()`, eliminando alertas de `-Wformat-truncation=2`. |
| **Injeção de escapes** | Conteúdo de notas, nomes, prompts e mensagens passam por `ab_append_text()`, que descarta bytes de controle (C0, DEL, C1): um `.md` importado com `ESC` embutido não consegue mais reescrever o clipboard (OSC 52), o título ou a tela do terminal. |
| **Carga e colagem** | Abrir uma nota recalcula dobras uma única vez (antes O(n²): 50 mil linhas levavam ~2,2 s, agora < 1 ms); colagem multilinha e exclusão de seleção operam em lote e não perdem texto em falha de memória. |
| **Colagem e clipboard** | Bracketed paste e leitura do clipboard limitados a 64 MB; o texto colado é sempre consumido até o marcador final, sem vazar como comandos do Modo Normal; leitura `CF_TEXT` limitada ao tamanho do bloco. |
| **SQLite e índice** | Statement de leitura reaproveitado, gravação com tamanho explícito, `DEFENSIVE` + `trusted_schema=OFF` contra banco adulterado; reaproveitamento do cache de busca por `bsearch` (antes O(n²)). |

### Testes

O projeto conta com testes automatizados compilados diretamente com o próprio `notas.c` como unidade de tradução (sem necessidade de interface gráfica aberta):

1. **Testes de Armazenamento e SQLite (`storage_test.c`)**: cobre CRUD, importação, reexportação, integridade de IDs, Base64 e escrita atômica:
   ```bash
   ./tools/test/run_tests.sh
   ```
2. **Testes de Regressão da Lógica de Edição (`editor_test.c`)**: valida comandos `:s/` e `:sub/` (com strings longas e sem barra final), ciclos de undo/redo com dobras recolhidas, agrupamento de digitação contínua e consistência de rótulos dos prompts modais:
   ```bash
   gcc -O2 tools/test/editor_test.c -o editor_test.exe -Itools/sqlite3-mingw/include -Ltools/sqlite3-mingw/lib -lsqlite3
   ./editor_test.exe
   ```
3. **Validação Visual Golden Master**: garantia de paridade binária estrita (0 bytes divergentes) na emissão de sequências de escape ANSI em todos os modos e combinações de texto (ASCII, acentuação UTF-8 e caracteres CJK).

---

## 📄 Licença

Distribuído sob a licença **MIT**. Consulte o arquivo [LICENSE](LICENSE) para mais detalhes.
