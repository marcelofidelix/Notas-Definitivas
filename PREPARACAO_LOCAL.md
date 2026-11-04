# Preparação local para desenvolvimento Rust

Data: 04/11/2026.

## Procedência e escopo

- Origem: `D:\PROJETOS\Notas_Definitivas\rust`.
- Destino: `D:\ChatGPT_Harness\Notas_Definitivas\rust`.
- Commit presente nos dois repositórios na preparação: `cc4ba0a` (`feat(rust): tabelas em grade, historico por deltas e prefixo numerico`).
- Versão do pacote: `notas 7.0.0`, edição Rust 2021.
- O destino já continha o repositório e estava sem alterações locais. A comparação de texto confirmou que os arquivos Rust existentes eram equivalentes aos da origem, diferindo apenas nas quebras de linha.
- Foram copiados 28 arquivos: 23 fontes `.rs`, `Cargo.toml`, `Cargo.lock`, `README.md`, `build.bat` e `build.sh`. Cada cópia foi conferida por SHA-256 antes das adaptações locais abaixo.
- Não foram importados código C, backups, ferramentas do legado, executáveis, cache `target/`, `data/` nem o banco pessoal `notas.db` da origem. Arquivos do legado já existentes no destino foram preservados e estão fora do escopo de desenvolvimento.
- A pasta de origem não foi alterada. O Git e seu remoto existentes no destino foram preservados; não houve commit nem push.

## Adaptações locais

- README da raiz substituído por instruções de desenvolvimento Rust e uso do banco.
- `AGENTS.md` criado para orientar trabalhos futuros exclusivamente sobre Rust.
- `build.bat` e `build.sh` da raiz agora delegam para os scripts Rust.
- Referência do manual Rust ao antigo README principal ajustada para manter a descrição de funcionalidades independente do manual C.
- `rust/build.sh`: a condição de execução passou a usar `if`, evitando retorno 1 após uma compilação bem-sucedida sem o argumento `run`.
- Quebras de linha dos fontes, manifesto, lockfile e script Windows compatibilizadas com o checkout local. Não houve mudança de conteúdo no código `.rs`, nas dependências ou no esquema do banco.

## Verificação

- Ambiente: Windows, Rust `1.99.1`, Cargo `1.99.1`, toolchain `stable-x86_64-pc-windows-msvc`.
- `cargo test --manifest-path rust/Cargo.toml --locked`: **153 aprovados, 0 falhas, 6 ignorados** (benchmarks e utilitários opt-in).
- `.\build.bat`: compilação release concluída, executável local `notas_rs.exe` gerado.
- `notas_rs.exe --version` e `--help`: verificados.
- `git diff --check`: sem erros de whitespace.
- A interface interativa e a compilação Linux/macOS não foram exercitadas nesta preparação.
