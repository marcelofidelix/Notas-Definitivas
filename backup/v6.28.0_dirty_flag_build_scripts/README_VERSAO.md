# Snapshot v6.28.0 — dirty-flag de renderizacao + scripts de build

Copia congelada do estado do projeto em 13/10/2026, imediatamente antes do
inicio do desenvolvimento do backend SQLite (v6.29.0).

## Base

Deriva de **v6.27.0** (`../v6.27.0_source_original/notas.c`), que e a fonte
original intacta, sem nenhuma alteracao.

## O que mudou em relacao ao v6.27.0

9 hunks no fonte, +110 / -5 linhas (a maior parte comentarios).

1. **Dirty-flag de renderizacao** (`should_refresh_now`, globais
   `g_needs_redraw` / `g_marquee_active` / `g_last_marquee_tick` /
   `g_last_heartbeat_draw`, `IDLE_HEARTBEAT_MS`):
   o loop principal deixou de redesenshar a tela inteira a cada 150 ms. Agora
   so desenha quando ha mudanca real de estado, marquee de titulo longo ativo,
   expiracao da mensagem de status (5 s) ou batimento de manutencao (30 s, que
   mantem o relogio relativo da barra lateral atualizado).

   Efeito medido (pty 24x100):
   - ocioso sem marquee: 41,6 KB/s -> 1,4 KB/s (~30x menos trafego ANSI)
   - com marquee ativo: 41 quadros/6 s -> 22 quadros/6 s, com animacao
     visualmente identica (21 fatias distintas do titulo em ambos)

2. **Correcao de aviso no build Windows** (`open_link_target`):
   `combined[4096] -> [8192]` e `wcomb[4096] -> [8192]` (com o argumento de
   `MultiByteToWideChar`), eliminando o warning `-Wformat-truncation` que
   apontava truncagem silenciosa de caminho longo em links relativos.

## Arquivos

| Arquivo | Descricao |
| :--- | :--- |
| `notas.c` | fonte completo desta versao (9.441 linhas) |
| `notas` | binario Linux x86-64 ja compilado (188.904 bytes) |
| `notas.exe` | binario Windows x86-64 ja compilado (315.537 bytes) |
| `build.sh` | script de build Linux/Windows (`linux`, `windows`, `all`, `run`, `clean`) |
| `build.bat` | script de build no Windows (`windows`, `run`, `clean`) |
| `README.md` | documentacao de uso que acompanhava esta versao |
| `DESCRICAO_TECNICA_DETALHADA_V6.27.0.md` | manual tecnico (ainda descrevendo a base v6.27.0) |

## Como recompilar esta versao

```sh
./build.sh linux      # -> notas
./build.sh windows    # -> notas.exe (requer mingw-w64)
```

## Como rodar

```sh
./notas
```

A pasta `data/` (notas do usuario) e criada ao lado do binario na primeira
execucao, pois o programa faz `chdir()` para o diretorio do executavel.
