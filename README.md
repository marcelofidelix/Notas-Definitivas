# Notas Definitivas — Rust

Editor de notas Markdown para terminal, com edição modal, SQLite, abas, busca, tabelas e temas. A versão mantida é **Rust 7.0.0**, em `rust/`.

Este espaço de trabalho está preparado para continuar o desenvolvimento em `D:\ChatGPT_Harness\Notas_Definitivas`. Os arquivos Rust foram sincronizados de `D:\PROJETOS\Notas_Definitivas\rust` em 04/11/2026. A origem permanece independente desta cópia.

## Compilar, testar e executar

No PowerShell, a partir da raiz deste projeto:

```powershell
.\build.bat          # compila Rust em release e gera .\notas_rs.exe
.\build.bat run      # compila e abre o editor
.\build.bat test     # executa os testes Rust em release
```

Para desenvolvimento com Cargo:

```powershell
cargo check --manifest-path rust/Cargo.toml --locked
cargo test --manifest-path rust/Cargo.toml --locked
cargo build --manifest-path rust/Cargo.toml --release --locked
```

No Linux/macOS, use `bash build.sh`, `bash build.sh run` ou `bash build.sh test`.

Requisitos: Rust e compilador/linker C para o SQLite embutido. No Windows, o ambiente validado usa Rust MSVC e Visual Studio Build Tools com C++. Essa dependência de compilação do SQLite não depende do aplicativo legado em C nem de `tools/`.

## Organização

- `rust/src/`: código mantido, incluindo testes unitários e de integração.
- `rust/Cargo.toml` e `rust/Cargo.lock`: pacote e dependências fixadas.
- [Manual e arquitetura Rust](rust/README.md): funcionalidades, módulos, atalhos e testes especiais.
- [Orientações para desenvolvimento](AGENTS.md): escopo para os próximos trabalhos neste projeto.
- [Registro da preparação local](PREPARACAO_LOCAL.md): procedência, arquivos e validação.

Os arquivos antigos em C, `backup/`, `tools/` e `DESCRICAO_TECNICA_DETALHADA.md` já existiam neste checkout e são apenas legado. A documentação técnica antiga descreve C; use `rust/README.md` e o código Rust como referência. Os scripts da raiz agora chamam a compilação Rust.

## Banco de notas

O banco pessoal da origem não foi copiado. Ao executar `notas_rs.exe`, o aplicativo cria ou abre `notas.db` ao lado do executável. Para escolher um banco de desenvolvimento:

```powershell
$env:NOTAS_DB = Join-Path $PWD 'notas.db'
.\notas_rs.exe
```

Ao usar `cargo run --manifest-path rust/Cargo.toml`, o executável fica em `rust/target/debug/`; defina `NOTAS_DB` se quiser usar o mesmo banco da raiz. Bancos, arquivos de notas pessoais e artefatos de compilação são ignorados pelo Git. Os testes padrão usam bancos em memória e arquivos temporários.
