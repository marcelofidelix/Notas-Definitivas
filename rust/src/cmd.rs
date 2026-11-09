//! Linha de comando (`:`), no estilo do Vim.

use crate::app::{App, Focus, Mode};
use crate::notes::contains_caseless;
use crate::theme::{NUM_THEMES, THEMES};

fn arg_after(cmd: &str, prefix_len: usize) -> &str {
    cmd[prefix_len..].trim_start_matches(' ')
}

fn level_from_arg(arg: &str, allow_zero_to_three: bool) -> Option<u8> {
    let c = arg.chars().next()?;
    match c {
        '0'..='3' if allow_zero_to_three => Some(c as u8 - b'0'),
        '1'..='3' => Some(c as u8 - b'0'),
        '4' | 'h' | 'H' => Some(4),
        '5' | 'r' | 'R' => Some(5),
        '6' | 'o' | 'O' => Some(6),
        _ => None,
    }
}

fn filter_message(level: u8, n: usize) -> String {
    match level {
        1 => format!("🔴 Filtrando Nível 1 (Vermelho): {} nota(s)", n),
        2 => format!("🟠 Filtrando Nível 2 (Laranja): {} nota(s)", n),
        3 => format!("🟡 Filtrando Nível 3 (Amarelo): {} nota(s)", n),
        4 => format!("🔵 Filtrando Marcação [H] (Azul): {} nota(s)", n),
        5 => format!("🟣 Filtrando Marcação [R] (Roxo): {} nota(s)", n),
        6 => format!("🟢 Filtrando Marcação [O] (Verde): {} nota(s)", n),
        _ => format!("Filtro removido: exibindo todas as {} notas", n),
    }
}

impl App {
    pub fn execute_command(&mut self, raw: &str) {
        let cmd = raw.trim_start_matches([' ', ':']);
        if cmd.is_empty() {
            return;
        }
        match cmd {
            "w" => {
                self.save_current();
            }
            "w!" => {
                self.save_current_ex(true);
            }
            "e!" | "reload" => self.reload_current(),
            "autosave" => self.set_status("Autosave: ATIVADO (salvamento automático a cada 1 minuto de edições)"),
            "closeall" | "tabcloseall" | "ca" | "fechartudo" | "close_all" | "bdall" => {
                self.close_all_tabs();
            }
            "q" | "qa" => {
                if self.any_tab_modified() {
                    self.set_status("Alterações pendentes. Use :wq para salvar todas ou :q! para descartá-las.");
                } else {
                    self.quit = true;
                }
            }
            "q!" | "qa!" => self.quit = true,
            "wq" | "x" | "wqa" | "wqa!" => {
                if self.save_all_modified() {
                    self.quit = true;
                }
            }
            "zm" | "zM" | "foldall" => self.fold_all(true),
            "zr" | "zR" | "unfoldall" => self.fold_all(false),
            "za" | "fold" => match self.buf_mut().and_then(|b| b.toggle_fold()) {
                Some(folded) => {
                    self.editor_scroll();
                    self.clear_needed = true;
                    self.set_status(format!(
                        "Dobra de seção {}",
                        if folded { "recolhida (za)" } else { "expandida (za)" }
                    ));
                }
                None => self.set_status("Nenhum cabeçalho (#) encontrado para recolher/expandir"),
            },
            "tabnew" => {
                let name = format!("Nova_Nota_{}", self.tabs.len() + 1);
                self.create_new_note(&name);
            }
            "nextlink" | "nl" => self.jump_to_next_link(),
            "table" | "tablealign" | "ta" => {
                let y = self.buf().map_or(0, |b| b.cy);
                if !self.table_align_at(y) {
                    self.table_align_all();
                }
            }
            "tableall" => self.table_align_all(),
            "link" | "copylink" | "cl" => self.copy_note_link(),
            "wrapbar" | "barwrap" | "statusbar" => {
                self.force_status_wrap = match self.force_status_wrap {
                    0 => {
                        self.set_status("Barra exclusiva de botões: MODO COMPACTO FORÇADO");
                        1
                    }
                    1 => {
                        self.set_status("Barra exclusiva de botões: MODO EXPANDIDO FORÇADO");
                        2
                    }
                    _ => {
                        self.set_status("Barra exclusiva de botões: MODO AUTOMÁTICO (< 108 colunas)");
                        0
                    }
                };
                self.clear_needed = true;
            }
            "wrap" => {
                self.wrap = !self.wrap;
                let on = self.wrap;
                self.set_status(format!("Quebra de linha (wrap): {}", if on { "ATIVADO" } else { "DESATIVADO" }));
            }
            "nowrap" | "set nowrap" => {
                self.wrap = false;
                self.set_status("Quebra de linha (wrap): DESATIVADO");
            }
            "set wrap" => {
                self.wrap = true;
                self.set_status("Quebra de linha (wrap): ATIVADO");
            }
            "rescan" => {
                self.rescan_notes();
                let n = self.notes.len();
                self.set_status(format!("Notas atualizadas ({} encontradas)", n));
            }
            "noh" | "nohlsearch" | "semrealce" => {
                // Desliga o realce mas mantem o padrao: `n`/`N` continuam valendo.
                self.search_hl = false;
                self.set_status("Realce da busca desligado (n/N continuam navegando)");
            }
            "tabclose" | "tabc" => {
                if let Some(i) = self.active {
                    self.tab_close(i);
                }
            }
            "tabnext" | "tabn" => self.tab_next(),
            "tabprev" | "tabp" => self.tab_prev(),
            "new" | "nova" | "n" => {
                self.prompt_buf.clear();
                self.prompt_cursor = 0;
                self.mode = Mode::PromptNew;
            }
            "gameoflife" | "life" | "gol" => {
                self.run_life();
                self.set_status("Simulação do Game of Life concluída");
            }
            "delete" | "del" | "excluir" | "d" => {
                if self.selected_note().is_some() {
                    self.mode = Mode::ConfirmDelete;
                } else {
                    self.set_status("Nenhuma nota selecionada para excluir.");
                }
            }
            _ => self.execute_command_with_args(cmd),
        }
    }

    fn execute_command_with_args(&mut self, cmd: &str) {
        if cmd.starts_with("theme") {
            let name = arg_after(cmd, 5);
            if name.is_empty() {
                self.theme = (self.theme + 1) % NUM_THEMES;
                self.set_status(format!("Tema alterado para: {}", THEMES[self.theme].name));
            } else if let Some(i) = THEMES.iter().position(|t| contains_caseless(t.name, name)) {
                self.theme = i;
                self.set_status(format!("Tema: {}", THEMES[i].name));
            } else {
                self.set_status(format!("Tema não encontrado: {}", name));
            }
            self.clear_needed = true;
        } else if cmd.starts_with("rename") || cmd.starts_with("rn") {
            let arg = arg_after(cmd, if cmd.starts_with("rename") { 6 } else { 2 });
            if arg.is_empty() {
                self.set_status("Uso: :rename <novo_nome> ou :rn <novo_nome>");
                return;
            }
            let target = if self.focus == Focus::Left {
                self.selected_key().or_else(|| self.cur_key().cloned())
            } else {
                self.cur_key().cloned()
            };
            match target {
                Some(k) => {
                    self.rename_note(&k, arg);
                }
                None => self.set_status("Nenhuma nota selecionada ou aberta para renomear."),
            }
        } else if cmd.starts_with("import") {
            let arg = arg_after(cmd, 6).to_string();
            if arg.is_empty() {
                self.prompt_buf.clear();
                self.prompt_cursor = 0;
                self.mode = Mode::PromptImport;
            } else {
                self.ui_import(&arg);
            }
        } else if cmd.starts_with("export") {
            let arg = arg_after(cmd, 6).to_string();
            if arg.is_empty() {
                self.prompt_buf.clear();
                self.prompt_cursor = 0;
                self.mode = Mode::PromptExport;
            } else {
                self.ui_export(&arg);
            }
        } else if cmd.starts_with("filter") || cmd.starts_with("filtro") {
            self.filter_urgency = match cmd.find(' ') {
                Some(i) => {
                    let arg = cmd[i..].trim_start_matches(' ');
                    level_from_arg(arg, false).unwrap_or(0)
                }
                None => (self.filter_urgency + 1) % 7,
            };
            self.apply_search();
            let msg = filter_message(self.filter_urgency, self.filtered.len());
            self.set_status(msg);
        } else if cmd.starts_with("atencao") || cmd.starts_with("attention") || cmd.starts_with("mark") {
            let level = cmd
                .find(' ')
                .map(|i| cmd[i..].trim_start_matches(' '))
                .and_then(|a| level_from_arg(a, true));
            match level {
                Some(l) => self.set_attention_of_current(l),
                None => {
                    if let Some(k) = self.cur_key().cloned() {
                        if let Some(i) = self.notes.iter().position(|n| n.key == k) {
                            self.toggle_attention_at(i);
                        }
                    }
                }
            }
        } else if cmd.starts_with("s/") || cmd.starts_with("%s/") || cmd.starts_with("sub/") {
            // O prefixo e medido pelo que foi digitado ("sub/" tambem comeca com 's').
            let p = if let Some(r) = cmd.strip_prefix("%s/") {
                r
            } else if let Some(r) = cmd.strip_prefix("sub/") {
                r
            } else {
                &cmd[2..]
            };
            match p.find('/') {
                Some(i) if i > 0 => {
                    let find = &p[..i];
                    let rest = &p[i + 1..];
                    let rep = rest.split('/').next().unwrap_or("");
                    let (find, rep) = (find.to_string(), rep.to_string());
                    self.replace_in_current(&find, &rep);
                }
                _ => self.set_status("Uso: :s/antigo/novo/ ou :sub/antigo/novo/"),
            }
        } else if let Some(pat) = cmd.strip_prefix('/') {
            let pat = pat.to_string();
            self.search_from_top(&pat);
        } else if cmd.starts_with("new ") || cmd.starts_with("e ") {
            let name = cmd[cmd.find(' ').unwrap_or(0)..].trim_start_matches(' ').to_string();
            if !name.is_empty() {
                self.open_or_create_wikilink(&name);
            }
        } else if cmd.as_bytes()[0].is_ascii_digit() {
            let digits: String = cmd.chars().take_while(char::is_ascii_digit).collect();
            let n: usize = digits.parse().unwrap_or(usize::MAX);
            if n > 0 && self.buf().is_some() {
                if let Some(b) = self.buf_mut() {
                    b.goto_line(n);
                }
                self.editor_scroll();
                self.clear_needed = true;
                let (y, total) = self.buf().map_or((1, 1), |b| (b.cy + 1, b.num_lines()));
                self.set_status(format!("Linha {}/{}", y, total));
            }
        } else {
            self.set_status(format!("Comando não reconhecido: :{}", cmd));
        }
    }
}
