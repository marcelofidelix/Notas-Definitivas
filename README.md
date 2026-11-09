# Notas Definitivas — Rust

Editor de notas Markdown para terminal, com edição modal, SQLite, abas, busca, tabelas e temas. A versão mantida é **Rust 7.0.0**, em `rust/`.

O código-fonte e o manual da aplicação estão em `rust/`.

## Compilar, testar e executar

No PowerShell, a partir da raiz deste projeto, use o Cargo:

```powershell
cargo check --manifest-path rust/Cargo.toml --locked
cargo test --manifest-path rust/Cargo.toml --locked
cargo build --manifest-path rust/Cargo.toml --release --locked
cargo run --manifest-path rust/Cargo.toml --release --locked
```

O executável compilado fica em `rust/target/release/notas.exe` no Windows. Para atualizá-lo na raiz, use `Copy-Item rust/target/release/notas.exe notas_rs.exe`.

No Linux/macOS, use `bash build.sh`, `bash build.sh run` ou `bash build.sh test`.

Requisitos: Rust e compilador/linker C para o SQLite embutido. No Windows, o ambiente validado usa Rust MSVC e Visual Studio Build Tools com C++.

## Organização

- `rust/src/`: código mantido, incluindo testes unitários e de integração.
- `rust/Cargo.toml` e `rust/Cargo.lock`: pacote e dependências fixadas.
- [Manual e arquitetura Rust](rust/README.md): funcionalidades, módulos, atalhos e testes especiais.
- [Registro da preparação local](PREPARACAO_LOCAL.md): procedência, arquivos e validação.

O executável Windows distribuído é `notas_rs.exe`.

## Banco de notas

O banco pessoal da origem não foi copiado. Ao executar `notas_rs.exe`, o aplicativo cria ou abre `notas.db` ao lado do executável. Para escolher um banco de desenvolvimento:

```powershell
$env:NOTAS_DB = Join-Path $PWD 'notas.db'
.\notas_rs.exe
```

Ao usar `cargo run --manifest-path rust/Cargo.toml`, o executável fica em `rust/target/debug/`; defina `NOTAS_DB` se quiser usar o mesmo banco da raiz. Bancos, arquivos de notas pessoais e artefatos de compilação são ignorados pelo Git. Os testes padrão usam bancos em memória e arquivos temporários.
