# Preparação local para desenvolvimento Rust

Data: 04/11/2026.

## Procedência e escopo

- Origem: `D:\PROJETOS\Notas_Definitivas\rust`.
- Destino: `D:\ChatGPT_Harness\Notas_Definitivas\rust`.
- Versão do pacote: `notas 7.0.0`, edição Rust 2021.
- O destino já continha o repositório e estava sem alterações locais. A comparação de texto confirmou que os arquivos Rust existentes eram equivalentes aos da origem, diferindo apenas nas quebras de linha.
- Os arquivos Rust, o manifesto, o lockfile e a documentação foram copiados e conferidos antes das adaptações locais abaixo.
- Não foram importados arquivos do legado, executáveis, cache `target/`, `data/` nem o banco pessoal `notas.db` da origem. Os arquivos do legado anteriormente presentes nesta cópia foram removidos do repositório.
- A pasta de origem não foi alterada. O Git e seu remoto existentes no destino foram preservados; não houve commit nem push.

## Adaptações locais

- README da raiz substituído por instruções de desenvolvimento Rust e uso do banco.
- As instruções locais de desenvolvimento permanecem fora do repositório.
- O script `build.sh` da raiz delega para o script Rust; no Windows, a compilação é feita pelo Cargo.
- Manual Rust atualizado para descrever a aplicação mantida.
- `rust/build.sh`: a condição de execução passou a usar `if`, evitando retorno 1 após uma compilação bem-sucedida sem o argumento `run`.
- Quebras de linha dos fontes, manifesto, lockfile e script Windows compatibilizadas com o checkout local. Não houve mudança de conteúdo no código `.rs`, nas dependências ou no esquema do banco.

## Verificação

- Ambiente: Windows, Rust `1.99.1`, Cargo `1.99.1`, toolchain `stable-x86_64-pc-windows-msvc`.
- `cargo test --manifest-path rust/Cargo.toml --locked`: **153 aprovados, 0 falhas, 6 ignorados** (benchmarks e utilitários opt-in).
- Compilação release concluída e executável local `notas_rs.exe` gerado.
- `notas_rs.exe --version` e `--help`: verificados.
- `git diff --check`: sem erros de whitespace.
- A interface interativa e a compilação Linux/macOS não foram exercitadas nesta preparação.
