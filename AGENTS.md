# Desenvolvimento do Notas Definitivas

## Escopo

- Desenvolver exclusivamente a aplicação Rust em `rust/`. A aplicação em C foi abandonada.
- Trabalhar nesta cópia em `D:\ChatGPT_Harness\Notas_Definitivas`; a origem `D:\PROJETOS\Notas_Definitivas` é referência de importação, não destino de alterações.
- Consultar `README.md` e `rust/README.md`. `DESCRICAO_TECNICA_DETALHADA.md`, `notas.c`, `backup/` e `tools/` descrevem ou suportam o legado em C e não orientam a implementação atual.
- Preservar `rust/Cargo.lock` no versionamento. Não versionar `rust/target/`, binários gerados ou notas pessoais.

## Comandos na raiz

- Verificar compilação: `cargo check --manifest-path rust/Cargo.toml --locked`.
- Testar: `cargo test --manifest-path rust/Cargo.toml --locked`.
- Compilar release: `cargo build --manifest-path rust/Cargo.toml --release --locked`.
- Compilar e copiar o executável para a raiz no Windows: `.\build.bat`.
- Executar no Windows: `.\build.bat run`.
- No Windows, usar MSVC com Visual Studio Build Tools C++; SQLite é compilado por `rusqlite` com a feature `bundled`.

## Cuidados do projeto

- Não utilizar o banco pessoal da origem para testes. Usar bancos em memória, temporários ou `NOTAS_DB` apontando para um banco de desenvolvimento.
- Preservar limites de caracteres UTF-8, desfazer/refazer, sanitização do terminal e proteção contra perda de dados e gravações concorrentes.
- Os testes ficam nos módulos e em `rust/src/tests.rs`. Rodar a suíte padrão após alterações de comportamento.
- Testes ignorados incluem benchmarks e geração de quadros/banco de demonstração; executá-los apenas quando relevantes, com destinos temporários explícitos.
- Evitar reformatação em massa sem relação com a alteração solicitada.
