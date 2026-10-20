#include <locale.h>
/*
 * Notas Definitivas - Markdown Text Editor & Wiki
 * Single-file, cross-platform (Linux & Windows) TUI Markdown Editor
 * Features: Tokyo Night & VS Code Themes, Multi-Tabs with Marquee Title,
 *           Middle-Click Tab Close, Multi-Word Accent-Insensitive Search,
 *           Full Vim Keybindings, WikiLinks, and Zero Memory Leaks.
 */

#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>
#include <stdbool.h>
#include <stdint.h>
#include <limits.h>
#include <time.h>
#include <sys/stat.h>
#include <signal.h>
#include <errno.h>
#include <stdarg.h>
#include <sqlite3.h>   /* Backend de armazenamento SQLite (v6.29.0) */

#ifdef _WIN32
  #ifndef _WIN32_WINNT
    #define _WIN32_WINNT 0x0A00
  #endif
  #define WIN32_LEAN_AND_MEAN
  #include <windows.h>
  #include <shellapi.h>
  #include <direct.h>
  #include <io.h>
  #include <wchar.h>
  #define PATH_SEP '\\'
  #define mkdir_cross(dir) mkdir_utf8(dir)

  /* Fallback definitions for Windows 10+ Virtual Terminal console flags (MinGW compatibility) */
  #ifndef ENABLE_VIRTUAL_TERMINAL_PROCESSING
    #define ENABLE_VIRTUAL_TERMINAL_PROCESSING 0x0004
  #endif
  #ifndef DISABLE_NEWLINE_AUTO_RETURN
    #define DISABLE_NEWLINE_AUTO_RETURN 0x0008
  #endif
  #ifndef ENABLE_VIRTUAL_TERMINAL_INPUT
    #define ENABLE_VIRTUAL_TERMINAL_INPUT 0x0200
  #endif
  #ifndef ENABLE_WINDOW_INPUT
    #define ENABLE_WINDOW_INPUT 0x0008
  #endif
#else
  #include <unistd.h>
  #include <termios.h>
  #include <sys/ioctl.h>
  #include <dirent.h>
  #include <sys/select.h>
  #include <sys/time.h>
  #include <sys/types.h>
  #include <sys/wait.h>
  #include <fcntl.h>
  #include <pthread.h>
  #define PATH_SEP '/'
  #define mkdir_cross(dir) mkdir_utf8(dir)
#endif

static void safe_strcpy(char *dest, size_t dest_sz, const char *src) {
    if (!dest || dest_sz == 0) return;
    if (!src) { dest[0] = '\0'; return; }
    size_t len = strlen(src);
    if (len >= dest_sz) len = dest_sz - 1;
    memcpy(dest, src, len);
    dest[len] = '\0';
}


/* =========================================================================
 * VS Code Popular Themes (ANSI 24-bit TrueColor)
 * ========================================================================= */
typedef struct {
    const char *name;
    bool is_dark;
    const char *bg_main;
    const char *bg_sidebar;
    const char *bg_status;
    const char *bg_select;
    const char *bg_cur_line;
    const char *bg_tab_active;
    const char *bg_tab_inactive;
    const char *bg_search;
    const char *bg_badge_nor;
    const char *bg_badge_ins;
    const char *bg_badge_vis;
    const char *bg_badge_cmd;
    const char *fg_main;
    const char *fg_muted;
    const char *fg_border;
    const char *fg_h1;
    const char *fg_h2;
    const char *fg_h3;
    const char *fg_link;
    const char *fg_wiki;
    const char *fg_date;
    const char *fg_dark;
    const char *fg_white;
    const char *fg_warn;
    const char *fg_yellow;
    const char *fg_accent;
} Theme;

#define NUM_THEMES 8
static const Theme themes[NUM_THEMES] = {
    {
        "Tokyo Night", true,
        "\x1b[48;2;26;27;38m",   /* #1a1b26 */
        "\x1b[48;2;22;22;30m",   /* #16161e */
        "\x1b[48;2;31;35;53m",   /* #1f2335 */
        "\x1b[48;2;51;70;124m",  /* #33467c */
        "\x1b[48;2;36;40;59m",   /* #24283b */
        "\x1b[48;2;41;46;66m",   /* #292e42 */
        "\x1b[48;2;22;22;30m",   /* #16161e */
        "\x1b[48;2;61;89;161m",  /* #3d59a1 */
        "\x1b[48;2;122;162;247m",/* #7aa2f7 */
        "\x1b[48;2;158;206;106m",/* #9ece6a */
        "\x1b[48;2;255;158;100m",/* #ff9e64 */
        "\x1b[48;2;187;154;247m",/* #bb9af7 */
        "\x1b[38;2;192;202;245m",/* #c0caf5 */
        "\x1b[38;2;86;95;137m",  /* #565f89 */
        "\x1b[38;2;65;72;104m",  /* #414868 */
        "\x1b[38;2;122;162;247m",/* #7aa2f7 */
        "\x1b[38;2;125;207;255m",/* #7dcfff */
        "\x1b[38;2;187;154;247m",/* #bb9af7 */
        "\x1b[38;2;115;218;202m",/* #73daca */
        "\x1b[38;2;187;154;247m",/* #bb9af7 */
        "\x1b[38;2;158;206;106m",/* #9ece6a */
        "\x1b[38;2;22;22;30m",   /* #16161e */
        "\x1b[38;2;255;255;255m",/* #ffffff */
        "\x1b[38;2;247;118;142m",/* #f7768e */
        "\x1b[38;2;224;175;104m",/* #e0af68 */
        "\x1b[38;2;187;154;247m" /* #bb9af7 */
    },
    {
        "One Dark Pro", true,
        "\x1b[48;2;40;44;52m",   /* #282c34 */
        "\x1b[48;2;33;37;43m",   /* #21252b */
        "\x1b[48;2;30;34;39m",   /* #1e2227 */
        "\x1b[48;2;62;68;81m",   /* #3e4451 */
        "\x1b[48;2;44;49;58m",   /* #2c313a */
        "\x1b[48;2;56;62;74m",   /* #383e4a */
        "\x1b[48;2;33;37;43m",   /* #21252b */
        "\x1b[48;2;77;86;102m",  /* #4d5666 */
        "\x1b[48;2;97;175;239m", /* #61afef */
        "\x1b[48;2;152;195;121m",/* #98c379 */
        "\x1b[48;2;229;192;123m",/* #e5c07b */
        "\x1b[48;2;198;120;221m",/* #c678dd */
        "\x1b[38;2;171;178;191m",/* #abb2bf */
        "\x1b[38;2;92;99;112m",  /* #5c6370 */
        "\x1b[38;2;75;82;99m",   /* #4b5263 */
        "\x1b[38;2;97;175;239m", /* #61afef */
        "\x1b[38;2;86;182;194m", /* #56b6c2 */
        "\x1b[38;2;198;120;221m",/* #c678dd */
        "\x1b[38;2;152;195;121m",/* #98c379 */
        "\x1b[38;2;198;120;221m",/* #c678dd */
        "\x1b[38;2;152;195;121m",/* #98c379 */
        "\x1b[38;2;33;37;43m",   /* #21252b */
        "\x1b[38;2;255;255;255m",/* #ffffff */
        "\x1b[38;2;224;108;117m",/* #e06c75 */
        "\x1b[38;2;229;192;123m",/* #e5c07b */
        "\x1b[38;2;198;120;221m" /* #c678dd */
    },
    {
        "Dracula", true,
        "\x1b[48;2;40;42;54m",   /* #282a36 */
        "\x1b[48;2;33;34;44m",   /* #21222c */
        "\x1b[48;2;25;26;33m",   /* #191a21 */
        "\x1b[48;2;68;71;90m",   /* #44475a */
        "\x1b[48;2;52;55;70m",   /* #343746 */
        "\x1b[48;2;68;71;90m",   /* #44475a */
        "\x1b[48;2;33;34;44m",   /* #21222c */
        "\x1b[48;2;98;114;164m", /* #6272a4 */
        "\x1b[48;2;189;147;249m",/* #bd93f9 */
        "\x1b[48;2;80;250;123m", /* #50fa7b */
        "\x1b[48;2;255;184;108m",/* #ffb86c */
        "\x1b[48;2;255;121;198m",/* #ff79c6 */
        "\x1b[38;2;248;248;242m",/* #f8f8f2 */
        "\x1b[38;2;98;114;164m", /* #6272a4 */
        "\x1b[38;2;80;85;110m",  /* #50556e */
        "\x1b[38;2;189;147;249m",/* #bd93f9 */
        "\x1b[38;2;139;233;253m",/* #8be9fd */
        "\x1b[38;2;255;121;198m",/* #ff79c6 */
        "\x1b[38;2;80;250;123m", /* #50fa7b */
        "\x1b[38;2;255;121;198m",/* #ff79c6 */
        "\x1b[38;2;80;250;123m", /* #50fa7b */
        "\x1b[38;2;33;34;44m",   /* #21222c */
        "\x1b[38;2;255;255;255m",/* #ffffff */
        "\x1b[38;2;255;85;85m",  /* #ff5555 */
        "\x1b[38;2;241;250;140m",/* #f1fa8c */
        "\x1b[38;2;189;147;249m" /* #bd93f9 */
    },
    {
        "Monokai", true,
        "\x1b[48;2;39;40;34m",   /* #272822 */
        "\x1b[48;2;30;31;28m",   /* #1e1f1c */
        "\x1b[48;2;23;24;20m",   /* #171814 */
        "\x1b[48;2;73;72;62m",   /* #49483e */
        "\x1b[48;2;62;61;50m",   /* #3e3d32 */
        "\x1b[48;2;62;61;50m",   /* #3e3d32 */
        "\x1b[48;2;30;31;28m",   /* #1e1f1c */
        "\x1b[48;2;117;113;94m", /* #75715e */
        "\x1b[48;2;102;217;239m",/* #66d9ef */
        "\x1b[48;2;166;226;46m", /* #a6e22e */
        "\x1b[48;2;253;151;31m", /* #fd971f */
        "\x1b[48;2;249;38;114m", /* #f92672 */
        "\x1b[38;2;248;248;242m",/* #f8f8f2 */
        "\x1b[38;2;117;113;94m", /* #75715e */
        "\x1b[38;2;90;88;75m",   /* #5a584b */
        "\x1b[38;2;166;226;46m", /* #a6e22e */
        "\x1b[38;2;102;217;239m",/* #66d9ef */
        "\x1b[38;2;249;38;114m", /* #f92672 */
        "\x1b[38;2;230;219;116m",/* #e6db74 */
        "\x1b[38;2;249;38;114m", /* #f92672 */
        "\x1b[38;2;166;226;46m", /* #a6e22e */
        "\x1b[38;2;30;31;28m",   /* #1e1f1c */
        "\x1b[38;2;255;255;255m",/* #ffffff */
        "\x1b[38;2;249;38;114m", /* #f92672 */
        "\x1b[38;2;230;219;116m",/* #e6db74 */
        "\x1b[38;2;174;129;255m" /* #ae81ff */
    },
    {
        "GitHub Dark", true,
        "\x1b[48;2;13;17;23m",   /* #0d1117 */
        "\x1b[48;2;1;4;9m",      /* #010409 */
        "\x1b[48;2;22;27;34m",   /* #161b22 */
        "\x1b[48;2;33;38;45m",   /* #21262d */
        "\x1b[48;2;22;27;34m",   /* #161b22 */
        "\x1b[48;2;48;54;61m",   /* #30363d */
        "\x1b[48;2;1;4;9m",      /* #010409 */
        "\x1b[48;2;56;139;253m", /* #388bfd */
        "\x1b[48;2;88;166;255m", /* #58a6ff */
        "\x1b[48;2;63;185;80m",  /* #3fb950 */
        "\x1b[48;2;210;153;34m", /* #d29922 */
        "\x1b[48;2;188;140;255m",/* #bc8cff */
        "\x1b[38;2;201;209;217m",/* #c9d1d9 */
        "\x1b[38;2;139;148;158m",/* #8b949e */
        "\x1b[38;2;48;54;61m",   /* #30363d */
        "\x1b[38;2;88;166;255m", /* #58a6ff */
        "\x1b[38;2;126;231;135m",/* #7ee787 */
        "\x1b[38;2;188;140;255m",/* #bc8cff */
        "\x1b[38;2;88;166;255m", /* #58a6ff */
        "\x1b[38;2;188;140;255m",/* #bc8cff */
        "\x1b[38;2;63;185;80m",  /* #3fb950 */
        "\x1b[38;2;1;4;9m",      /* #010409 */
        "\x1b[38;2;255;255;255m",/* #ffffff */
        "\x1b[38;2;248;81;73m",  /* #f85149 */
        "\x1b[38;2;210;153;34m", /* #d29922 */
        "\x1b[38;2;188;140;255m" /* #bc8cff */
    },
    {
        "GitHub Light", false,
        "\x1b[48;2;255;255;255m",/* #ffffff */
        "\x1b[48;2;246;248;250m",/* #f6f8fa */
        "\x1b[48;2;234;238;242m",/* #eaeef2 */
        "\x1b[48;2;208;215;222m",/* #d0d7de */
        "\x1b[48;2;246;248;250m",/* #f6f8fa */
        "\x1b[48;2;255;255;255m",/* #ffffff */
        "\x1b[48;2;234;238;242m",/* #eaeef2 */
        "\x1b[48;2;9;105;218m",  /* #0969da */
        "\x1b[48;2;9;105;218m",  /* #0969da */
        "\x1b[48;2;26;127;55m",  /* #1a7f37 */
        "\x1b[48;2;154;103;0m",  /* #9a6700 */
        "\x1b[48;2;130;80;223m", /* #8250df */
        "\x1b[38;2;36;41;47m",   /* #24292f */
        "\x1b[38;2;110;119;129m",/* #6e7781 */
        "\x1b[38;2;208;215;222m",/* #d0d7de */
        "\x1b[38;2;9;105;218m",  /* #0969da */
        "\x1b[38;2;17;99;41m",   /* #116329 */
        "\x1b[38;2;130;80;223m", /* #8250df */
        "\x1b[38;2;9;105;218m",  /* #0969da */
        "\x1b[38;2;130;80;223m", /* #8250df */
        "\x1b[38;2;26;127;55m",  /* #1a7f37 */
        "\x1b[38;2;36;41;47m",   /* #24292f */
        "\x1b[38;2;255;255;255m",/* #ffffff */
        "\x1b[38;2;207;34;46m",  /* #cf222e */
        "\x1b[38;2;154;103;0m",  /* #9a6700 */
        "\x1b[38;2;130;80;223m"  /* #8250df */
    },
    {
        "VS Code Light+", false,
        "\x1b[48;2;255;255;255m",/* #ffffff */
        "\x1b[48;2;243;243;243m",/* #f3f3f3 */
        "\x1b[48;2;0;122;204m",  /* #007acc */
        "\x1b[48;2;173;214;255m",/* #add6ff */
        "\x1b[48;2;248;248;248m",/* #f8f8f8 */
        "\x1b[48;2;255;255;255m",/* #ffffff */
        "\x1b[48;2;236;236;236m",/* #ececec */
        "\x1b[48;2;0;122;204m",  /* #007acc */
        "\x1b[48;2;0;122;204m",  /* #007acc */
        "\x1b[48;2;0;128;0m",    /* #008000 */
        "\x1b[48;2;175;0;219m",  /* #af00db */
        "\x1b[48;2;163;21;21m",  /* #a31515 */
        "\x1b[38;2;0;0;0m",      /* #000000 */
        "\x1b[38;2;110;110;110m",/* #6e6e6e */
        "\x1b[38;2;204;204;204m",/* #cccccc */
        "\x1b[38;2;0;0;255m",    /* #0000ff */
        "\x1b[38;2;0;128;0m",    /* #008000 */
        "\x1b[38;2;175;0;219m",  /* #af00db */
        "\x1b[38;2;0;122;204m",  /* #007acc */
        "\x1b[38;2;175;0;219m",  /* #af00db */
        "\x1b[38;2;0;128;0m",    /* #008000 */
        "\x1b[38;2;0;0;0m",      /* #000000 */
        "\x1b[38;2;255;255;255m",/* #ffffff */
        "\x1b[38;2;205;49;49m",  /* #cd3131 */
        "\x1b[38;2;163;21;21m",  /* #a31515 */
        "\x1b[38;2;0;122;204m"   /* #007acc */
    },
    {
        "Solarized Light", false,
        "\x1b[48;2;253;246;227m",/* #fdf6e3 */
        "\x1b[48;2;238;232;213m",/* #eee8d5 */
        "\x1b[48;2;238;232;213m",/* #eee8d5 */
        "\x1b[48;2;214;208;191m",/* #d6d0bf */
        "\x1b[48;2;238;232;213m",/* #eee8d5 */
        "\x1b[48;2;253;246;227m",/* #fdf6e3 */
        "\x1b[48;2;224;218;198m",/* #e0dac6 */
        "\x1b[48;2;38;139;210m", /* #268bd2 */
        "\x1b[48;2;38;139;210m", /* #268bd2 */
        "\x1b[48;2;133;153;0m",  /* #859900 */
        "\x1b[48;2;181;137;0m",  /* #b58900 */
        "\x1b[48;2;108;113;196m",/* #6c71c4 */
        "\x1b[38;2;101;123;131m",/* #657b83 */
        "\x1b[38;2;147;161;161m",/* #93a1a1 */
        "\x1b[38;2;214;208;191m",/* #d6d0bf */
        "\x1b[38;2;38;139;210m", /* #268bd2 */
        "\x1b[38;2;42;161;152m", /* #2aa198 */
        "\x1b[38;2;108;113;196m",/* #6c71c4 */
        "\x1b[38;2;42;161;152m", /* #2aa198 */
        "\x1b[38;2;108;113;196m",/* #6c71c4 */
        "\x1b[38;2;133;153;0m",  /* #859900 */
        "\x1b[38;2;7;54;66m",    /* #073642 */
        "\x1b[38;2;255;255;255m",/* #ffffff */
        "\x1b[38;2;220;50;47m",  /* #dc322f */
        "\x1b[38;2;181;137;0m",  /* #b58900 */
        "\x1b[38;2;108;113;196m" /* #6c71c4 */
    }
};

#define TH_BG_MAIN      (themes[E.current_theme].bg_main)
#define TH_BG_SIDEBAR   (themes[E.current_theme].bg_sidebar)
#define TH_BG_STATUS    (themes[E.current_theme].bg_status)
#define TH_BG_SELECT    (themes[E.current_theme].bg_select)
#define TH_BG_CUR_LINE  (themes[E.current_theme].bg_cur_line)
#define TH_BG_TAB_ACT   (themes[E.current_theme].bg_tab_active)
#define TH_BG_TAB_INACT (themes[E.current_theme].bg_tab_inactive)
#define TH_BG_SEARCH    (themes[E.current_theme].bg_search)
#define TH_BG_BADGE_NOR (themes[E.current_theme].bg_badge_nor)
#define TH_BG_BADGE_INS (themes[E.current_theme].bg_badge_ins)
#define TH_BG_BADGE_VIS (themes[E.current_theme].bg_badge_vis)
#define TH_BG_BADGE_CMD (themes[E.current_theme].bg_badge_cmd)

#define TH_FG_MAIN      (themes[E.current_theme].fg_main)
#define TH_FG_MUTED     (themes[E.current_theme].fg_muted)
#define TH_FG_BORDER    (themes[E.current_theme].fg_border)
#define TH_FG_H1        (themes[E.current_theme].fg_h1)
#define TH_FG_H2        (themes[E.current_theme].fg_h2)
#define TH_FG_H3        (themes[E.current_theme].fg_h3)
#define TH_FG_LINK      (themes[E.current_theme].fg_link)
#define TH_FG_WIKI      (themes[E.current_theme].fg_wiki)
#define TH_FG_DATE      (themes[E.current_theme].fg_date)
#define TH_FG_DARK      (themes[E.current_theme].fg_dark)
#define TH_FG_WHITE     (themes[E.current_theme].fg_white)
#define TH_FG_WARN      (themes[E.current_theme].fg_warn)
#define TH_FG_YELLOW    (themes[E.current_theme].fg_yellow)
#define TH_FG_ACCENT    (themes[E.current_theme].fg_accent)

#define STYLE_BOLD      "\x1b[1m"
#define STYLE_ITALIC    "\x1b[3m"
#define STYLE_UNDERLINE "\x1b[4m"
#define STYLE_RESET     "\x1b[0m"

/* Rotulos dos prompts de importacao/exportacao. Sao macros para que o desenho
 * do rodape e o calculo da coluna do cursor usem a MESMA string. */
#define PROMPT_IMPORT_LABEL " \xe2\xac\x87 Importar de: "
#define PROMPT_EXPORT_LABEL " \xe2\xac\x86 Exportar para: "

/* =========================================================================
 * Constants and Keycodes
 * ========================================================================= */
#define DATA_DIR "data"
#define DB_PATH "notas.db"       /* Banco SQLite unico com todas as notas (v6.29.0) */
#define DB_SCHEMA_VERSION 1
#define DEFAULT_LEFT_WIDTH 40
#define MIN_LEFT_WIDTH 18
#define MAX_HISTORY 64
#define MAX_UNDO 100
#define MAX_TABS 32
#define NOTE_ID_MAX 2147483647
/* Janela para agrupar digitacao continua em um unico passo de undo. Sem isso,
 * cada tecla copiava o documento inteiro e consumia um dos 100 niveis. */
#ifndef TYPING_UNDO_COALESCE_MS
#define TYPING_UNDO_COALESCE_MS 800
#endif
#ifndef AUTOSAVE_DELAY_MS
#define AUTOSAVE_DELAY_MS 60000
#endif
#ifndef GAMEOFLIFE_IDLE_TIMEOUT_MS
#define GAMEOFLIFE_IDLE_TIMEOUT_MS 600000 /* 10 minutes (600,000 ms) */
#endif

enum EditorMode {
    MODE_NORMAL = 0,
    MODE_INSERT,
    MODE_VISUAL_CHAR,
    MODE_VISUAL_LINE,
    MODE_COMMAND,
    MODE_SEARCH_LEFT,
    MODE_CONFIRM_DELETE_NOTE,
    MODE_PROMPT_NEW_NOTE,
    MODE_PROMPT_RENAME_NOTE,
    MODE_PROMPT_FIND,
    MODE_PROMPT_REPLACE_FIND,
    MODE_PROMPT_REPLACE_WITH,
    MODE_PROMPT_IMPORT_DIR,
    MODE_PROMPT_EXPORT_DIR
};

enum EditorFocus {
    FOCUS_LEFT = 0,
    FOCUS_RIGHT
};

enum KeyCodes {
    KEY_ESC = 27,
    KEY_BACKSPACE = 127,
    KEY_CTRL_BACKSPACE = 1000,
    KEY_ARROW_UP,
    KEY_ARROW_DOWN,
    KEY_ARROW_LEFT,
    KEY_ARROW_RIGHT,
    KEY_SHIFT_UP,
    KEY_SHIFT_DOWN,
    KEY_SHIFT_LEFT,
    KEY_SHIFT_RIGHT,
    KEY_CTRL_SHIFT_LEFT,
    KEY_CTRL_SHIFT_RIGHT,
    KEY_CTRL_LEFT,
    KEY_CTRL_RIGHT,
    KEY_CTRL_TAB,
    KEY_PAGE_UP,
    KEY_PAGE_DOWN,
    KEY_HOME,
    KEY_END,
    KEY_DELETE,
    KEY_MOUSE_EVENT
};

/* =========================================================================
 * Structures
 * ========================================================================= */

/* Dynamic String / Append Buffer for Flicker-Free Rendering */
typedef struct {
    char *b;
    int len;
    int cap;
    bool failed;  /* true se uma alocacao/limite falhou; o buffer fica consistente */
} ABuffer;

static void ab_init(ABuffer *ab) {
    if (!ab) return;
    ab->b = NULL;
    ab->len = 0;
    ab->cap = 0;
    ab->failed = false;
}

/* Acrescenta bytes ao buffer. Em caso de falha de memoria ou overflow de int,
 * marca ab->failed e mantem o conteudo ja acumulado intacto: o chamador pode
 * abortar a operacao em vez de gravar/desenhar um resultado truncado. */
static void ab_append(ABuffer *ab, const char *s, int len) {
    if (!ab || ab->failed || !s) return;
    if (len < 0) {
        size_t slen = strlen(s);
        if (slen > (size_t)INT_MAX) { ab->failed = true; return; }
        len = (int)slen;
    }
    if (len <= 0) return;
    if (ab->len > INT_MAX - len - 1) { ab->failed = true; return; }

    int needed = ab->len + len + 1;
    if (needed > ab->cap) {
        int ncap = ab->cap > 0 ? ab->cap : 1024;
        while (ncap < needed) {
            if (ncap > INT_MAX / 2) { ncap = needed; break; }
            ncap *= 2;
        }
        char *nb = (char *)realloc(ab->b, (size_t)ncap);
        if (!nb) { ab->failed = true; return; }
        ab->b = nb;
        ab->cap = ncap;
    }
    memcpy(ab->b + ab->len, s, (size_t)len);
    ab->len += len;
    ab->b[ab->len] = '\0';
}

static void ab_free(ABuffer *ab) {
    if (!ab) return;
    if (ab->b) free(ab->b);
    ab->b = NULL;
    ab->len = 0;
    ab->cap = 0;
    ab->failed = false;
}

/* Line representation */
typedef struct {
    char *chars;
    int len;
    int heading_lvl;    /* 1-6 if line starts with '#', 0 otherwise */
    bool folded;        /* If this heading is collapsed */
    bool hidden;        /* If line is inside a collapsed heading block */
    bool is_code_fence; /* Line is ``` or ```lang */
    bool in_code_block; /* Line is inside ``` ... ``` */
} Line;

/* Undo Snapshot */
typedef struct {
    Line *lines;
    int num_lines;
    int cursor_y;
    int cursor_x;
} UndoState;

/* Tab representation */
typedef struct {
    char cur_filename[256];
    char cur_filepath[512];
    char title[256];
    Line *lines;
    int num_lines;
    int cap_lines;
    bool modified;
    int64_t first_unsaved_time;

    int cursor_y;
    int cursor_x;
    int scroll_y;
    int scroll_x;

    int visual_start_y;
    int visual_start_x;
    bool has_selection;

    UndoState undo_stack[MAX_UNDO];
    int undo_count;
    UndoState redo_stack[MAX_UNDO];
    int redo_count;
} Tab;

typedef struct {
    int start_col;
    int end_col;
    int close_col;
} TabCoord;

/* Note File Information with High-Performance Search Cache */
typedef struct {
    char filename[512];
    char filepath[512];
    char title[256];
    time_t mtime;
    char rel_time[32];
    /* High-Performance In-Memory Search Cache (Normalized: lowercase & accent-folded) */
    char *search_norm;       /* Full normalized search text (filename + title + header + content) */
    int search_norm_len;     /* Length of cached normalized text */
    int attention_level;     /* Urgency: 0 = none, 1 = Crítica (Vermelho), 2 = Média (Laranja), 3 = Baixa (Amarelo), 4 = H (Azul), 5 = R (Roxo), 6 = O (Verde) */
    bool needs_attention;    /* True if attention_level > 0 */
} NoteInfo;

/* Editor Global State */
typedef struct {
    int screen_rows;
    int screen_cols;
    int left_width;

    enum EditorFocus focus;
    enum EditorMode mode;

    /* Multi-Tab State */
    Tab *tabs;
    int num_tabs;
    int cap_tabs;
    int active_tab;
    TabCoord tab_coords[MAX_TABS];
    int tab_new_btn_start;
    int tab_new_btn_end;

    /* Active buffer state (synchronized with tabs[active_tab]) */
    char cur_filename[256];
    char cur_filepath[512];
    Line *lines;
    int num_lines;
    int cap_lines;
    bool modified;
    int64_t first_unsaved_time;

    int cursor_y;
    int cursor_x;
    int scroll_y;
    int scroll_x;

    int visual_start_y;
    int visual_start_x;
    bool has_selection;

    UndoState undo_stack[MAX_UNDO];
    int undo_count;
    UndoState redo_stack[MAX_UNDO];
    int redo_count;

    /* Theme State */
    int current_theme;
    int theme_btn_start_col;
    int theme_btn_end_col;

    /* Bottom Bar Action Buttons & Layout */
    int status_bar_lines;       /* 1 or 2 lines */
    int force_status_bar_wrap;  /* 0 = auto, 1 = always wrap, 2 = never wrap */
    int btn_new_start, btn_new_end;
    int btn_rename_start, btn_rename_end;
    int btn_delete_start, btn_delete_end;
    int btn_wrap_start, btn_wrap_end;
    int btn_find_start, btn_find_end;
    int btn_replace_start, btn_replace_end;
    int btn_life_start, btn_life_end;
    int btn_import_start, btn_import_end;   /* [Importar] - pasta .md -> SQLite */
    int btn_export_start, btn_export_end;   /* [Exportar] - SQLite -> pasta .md */

    /* Urgency / Criticality Filter State (0 = all, 1 = Red, 2 = Orange, 3 = Yellow, 4 = Blue (H), 5 = Purple (R), 6 = Green (O)) */
    int filter_urgency;
    int btn_filter_all_start, btn_filter_all_end;
    int btn_filter_1_start, btn_filter_1_end;
    int btn_filter_2_start, btn_filter_2_end;
    int btn_filter_3_start, btn_filter_3_end;
    int btn_filter_h_start, btn_filter_h_end;
    int btn_filter_r_start, btn_filter_r_end;
    int btn_filter_o_start, btn_filter_o_end;

    /* Search & Replace Buffers */
    char replace_find_buf[256];
    char replace_with_buf[256];

    /* Notes in data folder */
    NoteInfo *notes;
    int num_notes;
    int selected_note_idx;
    int sidebar_scroll_y;

    /* Filtered notes (during Ctrl+F search) */
    int *filtered_indices;
    int num_filtered;
    char search_query[256];
    int search_cursor;

    /* Prompt buffer for new note and rename */
    char prompt_buf[256];
    int prompt_cursor;
    char rename_target_path[512];

    /* Yank buffer */
    char *yank_buf;
    bool yank_is_linewise;

    /* Pending operator for 'd' */
    char pending_op;

    /* Command line at bottom (:w, :q, etc.) */
    char cmd_buf[256];
    int cmd_cursor;
    char status_msg[256];
    char last_search_pat[128];
    time_t status_msg_time;

    /* Note History */
    char history[MAX_HISTORY][512];
    int history_count;
    int history_idx;

    /* Mouse reporting state */
    int mouse_col;
    int mouse_row;
    int mouse_btn;
    bool mouse_released;

    bool wrap_mode;
    bool quit_requested;
} Editor;

static Editor E;

#ifdef _WIN32
static HANDLE hStdin = INVALID_HANDLE_VALUE;
static HANDLE hStdout = INVALID_HANDLE_VALUE;
static DWORD orig_stdin_mode = 0;
static DWORD orig_stdout_mode = 0;
static UINT orig_input_cp = 0;
static UINT orig_output_cp = 0;
#ifndef CP_UTF8
#define CP_UTF8 65001
#endif
#else
static struct termios orig_termios;
#endif

/* Forward Declarations */
static void save_undo(void);
static void refresh_screen(void);
static void set_status_msg(const char *fmt, ...);
static void open_note_by_path(const char *path, bool record_history);
static void rescan_notes(void);
static void free_preview(void);
static void load_preview(const char *path);
static void apply_note_search(const char *query);
static void update_folding(void);
static void copy_to_system_clipboard(const char *text, int len);
static char *get_system_clipboard(int *out_len);
static void open_link_target(const char *target);
static void editor_scroll(void);
static void jump_to_next_link(void);
static void yank_visual_selection(bool linewise);
static void delete_visual_selection_ex(bool do_yank);
static void delete_visual_selection(void);
static void paste_replace_selection(void);
static void table_align_all(void);
static bool table_align_at(int line_idx);
static void table_jump_next_cell(bool create_if_at_end);
static bool table_insert_row_and_advance(void);
static bool is_table_line(const Line *l);
static void execute_bottom_command(const char *cmd);
static void run_game_of_life(void);
static void tab_save_active(void);
static void tab_load(int idx);
static void tab_close(int idx);
static bool close_all_tabs(void);
static void create_new_note(const char *raw_name);
static bool rename_note(const char *old_path, const char *raw_new_name);
static void replace_in_current_note(const char *find_str, const char *rep_str);
static void load_attention_flags(void);
static void toggle_note_attention_at(int actual_idx);
static void free_editor(void);
static void mark_modified(void);
static bool save_current_note(void);
static bool save_tab_by_idx(int t_idx);
static void check_autosave(void);

/* =========================================================================
 * Terminal & Raw Mode Control
 * ========================================================================= */


#ifdef _WIN32
/* Converte UTF-8 para UTF-16 sem limite arbitrario de 1024 bytes. O chamador
 * recebe um bloco alocado (ou NULL se o caminho for invalido/sem memoria). */
static WCHAR *utf8_to_wide_alloc(const char *s) {
    if (!s) return NULL;
    int n = MultiByteToWideChar(CP_UTF8, 0, s, -1, NULL, 0);
    if (n <= 0) return NULL;
    WCHAR *out = (WCHAR *)malloc((size_t)n * sizeof(WCHAR));
    if (!out) return NULL;
    if (MultiByteToWideChar(CP_UTF8, 0, s, -1, out, n) != n) {
        free(out);
        return NULL;
    }
    return out;
}
#endif

static FILE *fopen_utf8(const char *path, const char *mode) {
    if (!path || !mode) return NULL;
#ifdef _WIN32
    WCHAR *wpath = utf8_to_wide_alloc(path);
    WCHAR *wmode = utf8_to_wide_alloc(mode);
    bool converted = (wpath != NULL && wmode != NULL);
    FILE *f = converted ? _wfopen(wpath, wmode) : NULL;
    free(wpath);
    free(wmode);
    return converted ? f : fopen(path, mode);
#else
    return fopen(path, mode);
#endif
}

static int unlink_utf8(const char *path) {
    if (!path) return -1;
#ifdef _WIN32
    WCHAR *wpath = utf8_to_wide_alloc(path);
    if (!wpath) return remove(path);
    int rc = DeleteFileW(wpath) ? 0 : -1;
    free(wpath);
    return rc;
#else
    return unlink(path);
#endif
}

static int rename_utf8(const char *oldpath, const char *newpath) {
    if (!oldpath || !newpath) return -1;
#ifdef _WIN32
    WCHAR *wold = utf8_to_wide_alloc(oldpath);
    WCHAR *wnew = utf8_to_wide_alloc(newpath);
    int rc = -1;
    if (wold && wnew) {
        /* _wrename/MoveFileW falham quando o destino existe; a escrita atomica
         * depende de substituir o arquivo antigo no mesmo diretorio. */
        rc = MoveFileExW(wold, wnew, MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH) ? 0 : -1;
    } else {
        rc = rename(oldpath, newpath);
    }
    free(wold);
    free(wnew);
    return rc;
#else
    return rename(oldpath, newpath);
#endif
}

static int mkdir_utf8(const char *dir) {
    if (!dir || !*dir) return -1;
#ifdef _WIN32
    WCHAR *wdir = utf8_to_wide_alloc(dir);
    if (!wdir) return _mkdir(dir);
    if (CreateDirectoryW(wdir, NULL)) { free(wdir); return 0; }
    DWORD err = GetLastError();
    int rc = -1;
    if (err == ERROR_ALREADY_EXISTS) {
        DWORD attrs = GetFileAttributesW(wdir);
        if (attrs != INVALID_FILE_ATTRIBUTES && (attrs & FILE_ATTRIBUTE_DIRECTORY)) rc = 0;
    }
    free(wdir);
    return rc;
#else
    if (mkdir(dir, 0755) == 0) return 0;
    if (errno == EEXIST) {
        struct stat st;
        if (stat(dir, &st) == 0 && S_ISDIR(st.st_mode)) return 0;
    }
    return -1;
#endif
}

#ifdef _WIN32
static inline time_t filetime_to_timet(const FILETIME *ft) {
    ULARGE_INTEGER ull;
    ull.LowPart = ft->dwLowDateTime;
    ull.HighPart = ft->dwHighDateTime;
    if (ull.QuadPart < 116444736000000000ULL) return 0;
    return (time_t)((ull.QuadPart - 116444736000000000ULL) / 10000000ULL);
}
#endif

static int stat_cross(const char *path, struct stat *st) {
    if (!path) return -1;
#ifdef _WIN32
    WCHAR *wpath = utf8_to_wide_alloc(path);
    if (!wpath) return -1;
    WIN32_FILE_ATTRIBUTE_DATA data;
    int rc = -1;
    if (GetFileAttributesExW(wpath, GetFileExInfoStandard, &data)) {
        if (st) {
            memset(st, 0, sizeof(struct stat));
            st->st_mtime = filetime_to_timet(&data.ftLastWriteTime);
            st->st_size = ((int64_t)data.nFileSizeHigh << 32) | data.nFileSizeLow;
            st->st_mode = (data.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) ? S_IFDIR : S_IFREG;
        }
        rc = 0;
    }
    free(wpath);
    return rc;
#else
    return stat(path, st);
#endif
}

/* Escrita atomica para arquivos reais (fallback legado e exportacao):
 * grava em um temporario no mesmo diretorio, sincroniza e so entao substitui
 * o destino. Assim uma falha de disco/energia nao deixa a nota truncada. */
static bool write_file_atomic(const char *path, const char *data, long len) {
    if (!path || !*path || len < 0 || (len > 0 && !data)) return false;

#ifdef _WIN32
    unsigned long pid = (unsigned long)GetCurrentProcessId();
#else
    unsigned long pid = (unsigned long)getpid();
#endif
    size_t tmp_sz = strlen(path) + 40;
    char *tmp = (char *)malloc(tmp_sz);
    if (!tmp) return false;
    snprintf(tmp, tmp_sz, "%s.tmp.%lu", path, pid);

    FILE *f = fopen_utf8(tmp, "wb");
    if (!f) { free(tmp); return false; }

    bool ok = true;
    if (len > 0 && fwrite(data, 1, (size_t)len, f) != (size_t)len) ok = false;
    if (ok && fflush(f) != 0) ok = false;
    if (ok) {
#ifdef _WIN32
        if (_commit(_fileno(f)) != 0) ok = false;
#else
        if (fsync(fileno(f)) != 0) ok = false;
#endif
    }
    if (fclose(f) != 0) ok = false;

    if (!ok) {
        unlink_utf8(tmp);
        free(tmp);
        return false;
    }
    if (rename_utf8(tmp, path) != 0) {
        unlink_utf8(tmp);
        free(tmp);
        return false;
    }
    free(tmp);
    return true;
}

static bool g_terminal_shutdown_done = false;

static void disable_raw_mode(void) {
    /* disable_raw_mode() e registrada em atexit e tambem pode ser chamada
     * explicitamente (erro de inicializacao/saida normal). O guarda evita
     * restaurar o terminal e liberar o estado duas vezes. */
    if (g_terminal_shutdown_done) return;
    g_terminal_shutdown_done = true;

    /* Disable SGR mouse tracking, show cursor, restore screen buffer */
    printf("\x1b[?7h\x1b[?2004l\x1b[?1000l\x1b[?1002l\x1b[?1006l\x1b[0 q\x1b[?25h\x1b[?1049l" STYLE_RESET);
    fflush(stdout);

#ifdef _WIN32
    if (hStdin != INVALID_HANDLE_VALUE) {
        SetConsoleMode(hStdin, orig_stdin_mode);
    }
    if (hStdout != INVALID_HANDLE_VALUE) {
        SetConsoleMode(hStdout, orig_stdout_mode);
    }
    if (orig_input_cp != 0) {
        SetConsoleCP(orig_input_cp);
    }
    if (orig_output_cp != 0) {
        SetConsoleOutputCP(orig_output_cp);
    }
#else
    tcsetattr(STDIN_FILENO, TCSAFLUSH, &orig_termios);
#endif

    free_editor();
}

static volatile sig_atomic_t g_window_resized = 0;
static volatile sig_atomic_t g_screen_clear_needed = 1;

/* =========================================================================
 * Dirty-Flag Rendering (evita redesenho de tela desnecessario)
 * -------------------------------------------------------------------------
 * Antes, o loop principal redesenhava a tela INTEIRA a cada 150 ms (o timeout
 * de leitura de tecla), mesmo sem nenhuma mudanca de estado: ~100 KB/s de
 * trafego ANSI so para ficar parado. Agora o desenho so acontece quando algo
 * realmente muda (g_needs_redraw), quando ha algo animado para desenhar
 * (marquee de titulo longo), quando a mensagem de status expira (5 s) ou no
 * batimento de manutencao de 30 s que mantem o relogio relativo da barra
 * lateral atualizado.
 * ========================================================================= */
static volatile sig_atomic_t g_needs_redraw = 1; /* 1 = estado visual mudou, precisa redesenhar */
static bool g_marquee_active = false;            /* true = algum titulo longo esta rolando na barra de abas */
static int64_t g_last_marquee_tick = -1;         /* ultimo tick (now_ms/300) ja desenhado no marquee */
static int64_t g_last_heartbeat_draw = 0;        /* instante do ultimo redesenho de manutencao */

/* Intervalo do batimento de manutencao: mantem o relogio relativo da barra
 * lateral ("now", "5m", "2h") atualizado mesmo com o editor ocioso. A maior
 * granularidade exibida e de 1 minuto, entao 30 s e mais que suficiente. */
#ifndef IDLE_HEARTBEAT_MS
#define IDLE_HEARTBEAT_MS 30000
#endif

#ifndef _WIN32
static void handle_sigwinch(int sig) {
    (void)sig;
    g_window_resized = 1;
    g_screen_clear_needed = 1;
}
#endif

static void enable_raw_mode(void) {
#ifdef _WIN32
    hStdin = GetStdHandle(STD_INPUT_HANDLE);
    hStdout = GetStdHandle(STD_OUTPUT_HANDLE);
    GetConsoleMode(hStdin, &orig_stdin_mode);
    GetConsoleMode(hStdout, &orig_stdout_mode);

    orig_input_cp = GetConsoleCP();
    orig_output_cp = GetConsoleOutputCP();
    SetConsoleCP(CP_UTF8);
    SetConsoleOutputCP(CP_UTF8);

    DWORD in_mode = orig_stdin_mode;
    in_mode &= ~(ENABLE_ECHO_INPUT | ENABLE_LINE_INPUT | ENABLE_PROCESSED_INPUT | ENABLE_MOUSE_INPUT);
    in_mode |= ENABLE_VIRTUAL_TERMINAL_INPUT | ENABLE_WINDOW_INPUT;
    SetConsoleMode(hStdin, in_mode);

    DWORD out_mode = orig_stdout_mode;
    out_mode |= ENABLE_VIRTUAL_TERMINAL_PROCESSING | DISABLE_NEWLINE_AUTO_RETURN;
    SetConsoleMode(hStdout, out_mode);
#else
    tcgetattr(STDIN_FILENO, &orig_termios);
    struct termios raw = orig_termios;
    raw.c_iflag &= ~(BRKINT | ICRNL | INPCK | ISTRIP | IXON);
    raw.c_oflag &= ~(OPOST);
    raw.c_cflag |= (CS8);
    raw.c_lflag &= ~(ECHO | ICANON | IEXTEN | ISIG);
    raw.c_cc[VMIN] = 1;
    raw.c_cc[VTIME] = 0;
    tcsetattr(STDIN_FILENO, TCSAFLUSH, &raw);

    signal(SIGWINCH, handle_sigwinch);
#endif

    /* Enable alternate screen buffer, UTF-8 mouse tracking (SGR 1006), clear screen */
    printf("\x1b[?1049h\x1b[2J\x1b[H\x1b[?7l\x1b[?2004h\x1b[?1000h\x1b[?1002h\x1b[?1006h");
    fflush(stdout);
    atexit(disable_raw_mode);
}

#ifndef _WIN32
static int get_cursor_position(int *rows, int *cols) {
    if (!isatty(STDIN_FILENO) || !isatty(STDOUT_FILENO)) return -1;
    char buf[32];
    unsigned int i = 0;
    if (write(STDOUT_FILENO, "\x1b[6n", 4) != 4) return -1;

    while (i < sizeof(buf) - 1) {
        fd_set fds;
        FD_ZERO(&fds);
        FD_SET(STDIN_FILENO, &fds);
        struct timeval tv = {0, 50000}; /* 50 ms */
        if (select(STDIN_FILENO + 1, &fds, NULL, NULL, &tv) <= 0) break;

        if (read(STDIN_FILENO, &buf[i], 1) != 1) break;
        if (buf[i] == 'R') { i++; break; }
        i++;
    }
    buf[i] = 0;
    if (buf[0] != '\x1b' || buf[1] != '[') return -1;
    if (sscanf(&buf[2], "%d;%d", rows, cols) != 2) return -1;
    return 0;
}
#endif

static void get_window_size(int *rows, int *cols) {
#ifdef _WIN32
    CONSOLE_SCREEN_BUFFER_INFO csbi;
    if (GetConsoleScreenBufferInfo(GetStdHandle(STD_OUTPUT_HANDLE), &csbi)) {
        *cols = csbi.srWindow.Right - csbi.srWindow.Left + 1;
        *rows = csbi.srWindow.Bottom - csbi.srWindow.Top + 1;
        if (*rows > 0 && *cols > 0) return;
    }
#else
    struct winsize ws;
    if (ioctl(STDOUT_FILENO, TIOCGWINSZ, &ws) == 0 && ws.ws_col > 0 && ws.ws_row > 0) {
        *cols = ws.ws_col;
        *rows = ws.ws_row;
        return;
    }
    if (ioctl(STDIN_FILENO, TIOCGWINSZ, &ws) == 0 && ws.ws_col > 0 && ws.ws_row > 0) {
        *cols = ws.ws_col;
        *rows = ws.ws_row;
        return;
    }
    if (ioctl(STDERR_FILENO, TIOCGWINSZ, &ws) == 0 && ws.ws_col > 0 && ws.ws_row > 0) {
        *cols = ws.ws_col;
        *rows = ws.ws_row;
        return;
    }
    FILE *tty = fopen("/dev/tty", "r+");
    if (tty) {
        if (ioctl(fileno(tty), TIOCGWINSZ, &ws) == 0 && ws.ws_col > 0 && ws.ws_row > 0) {
            *cols = ws.ws_col;
            *rows = ws.ws_row;
            fclose(tty);
            return;
        }
        fclose(tty);
    }
    /* Universal ANSI DSR Query Fallback */
    if (write(STDOUT_FILENO, "\x1b[999C\x1b[999B", 12) == 12) {
        if (get_cursor_position(rows, cols) == 0 && *rows > 0 && *cols > 0) {
            return;
        }
    }
#endif
    const char *lines = getenv("LINES");
    const char *columns = getenv("COLUMNS");
    if (lines && columns) {
        int r = atoi(lines), cl = atoi(columns);
        if (r > 0 && cl > 0) {
            *rows = r;
            *cols = cl;
            return;
        }
    }
    *cols = 80;
    *rows = 24;
}

/* =========================================================================
 * UTF-8 Display Width and Terminal Resize Handling
 * ========================================================================= */

static int utf8_charlen(unsigned char ch) {
    if (ch < 0x80) return 1;
    if ((ch & 0xE0) == 0xC0) return 2;
    if ((ch & 0xF0) == 0xE0) return 3;
    if ((ch & 0xF8) == 0xF0) return 4;
    return 1;
}

static int utf8_decode_codepoint(const char *s, int *bytes_read) {
    if (!s || !*s) {
        if (bytes_read) *bytes_read = 0;
        return 0;
    }
    unsigned char c = (unsigned char)*s;
    int cp = 0;
    int len = 1;
    if (c < 0x80) {
        cp = c;
        len = 1;
    } else if ((c & 0xE0) == 0xC0) {
        len = 2;
        cp = c & 0x1F;
    } else if ((c & 0xF0) == 0xE0) {
        len = 3;
        cp = c & 0x0F;
    } else if ((c & 0xF8) == 0xF0) {
        len = 4;
        cp = c & 0x07;
    } else {
        if (bytes_read) *bytes_read = 1;
        return c;
    }

    int i = 1;
    for (; i < len && s[i]; i++) {
        unsigned char b = (unsigned char)s[i];
        if ((b & 0xC0) != 0x80) break;
        cp = (cp << 6) | (b & 0x3F);
    }
    if (bytes_read) *bytes_read = i;
    return cp;
}

static int utf8_codepoint_width(int cp) {
    if (cp == 0) return 0;
    if (cp < 0x20 || (cp >= 0x7F && cp < 0xA0)) return 0;
    /* Zero width: combining marks and variation selectors */
    if ((cp >= 0x0300 && cp <= 0x036F) || (cp >= 0xFE00 && cp <= 0xFE0F) ||
        (cp >= 0x1AB0 && cp <= 0x1AFF) || (cp >= 0x1DC0 && cp <= 0x1DFF) ||
        (cp >= 0x20D0 && cp <= 0x20FF) || cp == 0x200B || cp == 0x200C ||
        cp == 0x200D || cp == 0xFEFF) {
        return 0;
    }
    /* Wide characters: CJK, Fullwidth, and Emojis */
    if ((cp >= 0x1100 && (cp <= 0x115F || cp == 0x2329 || cp == 0x232A)) ||
        (cp >= 0x2E80 && cp <= 0xA4CF && cp != 0x303F) ||
        (cp >= 0xAC00 && cp <= 0xD7A3) ||
        (cp >= 0xF900 && cp <= 0xFAFF) ||
        (cp >= 0xFE10 && cp <= 0xFE19) ||
        (cp >= 0xFE30 && cp <= 0xFE6F) ||
        (cp >= 0xFF01 && cp <= 0xFF60) ||
        (cp >= 0xFFE0 && cp <= 0xFFE6) ||
        (cp >= 0x1F000 && cp <= 0x1FAFF) ||
        (cp >= 0x20000 && cp <= 0x3FFFF)) {
        return 2;
    }
    return 1;
}

static int utf8_width(const char *s) {
    if (!s) return 0;
    int w = 0;
    while (*s) {
        int bytes = 0;
        int cp = utf8_decode_codepoint(s, &bytes);
        if (bytes <= 0) break;
        int cw = utf8_codepoint_width(cp);
        if (s[bytes]) {
            int next_bytes = 0;
            int next_cp = utf8_decode_codepoint(s + bytes, &next_bytes);
            if (next_cp == 0xFE0F) {
                cw = 2;
                bytes += next_bytes;
            }
        }
        w += cw;
        s += bytes;
    }
    return w;
}

static int utf8_align_start(const char *s, int len, int idx) {
    if (idx <= 0) return 0;
    if (idx >= len) return len;
    while (idx > 0 && ((unsigned char)s[idx] & 0xC0) == 0x80) {
        idx--;
    }
    return idx;
}

static int utf8_align_end(const char *s, int len, int idx) {
    if (idx < 0) return 0;
    if (idx >= len) return len - 1;
    while (idx > 0 && ((unsigned char)s[idx] & 0xC0) == 0x80) {
        idx--;
    }
    int clen = utf8_charlen((unsigned char)s[idx]);
    int end = idx + clen - 1;
    if (end >= len) end = len - 1;
    return end;
}

static int utf8_width_len(const char *s, int len) {
    if (!s || len <= 0) return 0;
    int w = 0;
    int i = 0;
    while (i < len && s[i]) {
        int bytes = 0;
        int cp = utf8_decode_codepoint(s + i, &bytes);
        if (bytes <= 0) break;
        if (i + bytes > len) bytes = len - i;
        int cw = utf8_codepoint_width(cp);
        if (i + bytes < len && s[i + bytes]) {
            int next_bytes = 0;
            int next_cp = utf8_decode_codepoint(s + i + bytes, &next_bytes);
            if (next_cp == 0xFE0F) {
                cw = 2;
                bytes += next_bytes;
            }
        }
        w += cw;
        i += bytes;
    }
    return w;
}

static int byte_offset_for_col(const char *s, int len, int target_col) {
    if (!s || len <= 0 || target_col <= 0) return 0;
    int cur_col = 0;
    int i = 0;
    while (i < len && s[i] && cur_col < target_col) {
        int bytes = 0;
        int cp = utf8_decode_codepoint(s + i, &bytes);
        if (bytes <= 0) break;
        if (i + bytes > len) bytes = len - i;
        int cw = utf8_codepoint_width(cp);
        if (i + bytes < len && s[i + bytes]) {
            int next_bytes = 0;
            int next_cp = utf8_decode_codepoint(s + i + bytes, &next_bytes);
            if (next_cp == 0xFE0F) {
                cw = 2;
                bytes += next_bytes;
            }
        }
        cur_col += cw;
        i += bytes;
    }
    return i;
}

static void utf8_truncate_display(const char *src, int max_cols, char *dst, size_t dst_sz) {
    if (!src || !dst || dst_sz == 0) return;
    int cols = 0;
    size_t out_len = 0;
    while (*src && cols < max_cols && out_len + 5 < dst_sz) {
        int bytes = 0;
        int cp = utf8_decode_codepoint(src, &bytes);
        if (bytes <= 0) break;
        int cw = utf8_codepoint_width(cp);
        if (src[bytes]) {
            int next_bytes = 0;
            int next_cp = utf8_decode_codepoint(src + bytes, &next_bytes);
            if (next_cp == 0xFE0F) {
                cw = 2;
                bytes += next_bytes;
            }
        }
        if (cols + cw > max_cols) break;
        for (int i = 0; i < bytes && out_len + 1 < dst_sz; i++) {
            dst[out_len++] = src[i];
        }
        src += bytes;
        cols += cw;
    }
    dst[out_len] = '\0';
}

static void utf8_marquee_slice(const char *title, int step, int avail_w, char *out, size_t out_sz) {
    if (!title || !out || out_sz == 0 || avail_w <= 0) {
        if (out && out_sz > 0) out[0] = '\0';
        return;
    }
    size_t tlen = strlen(title);
    size_t dlen = tlen * 2 + 16;
    char *doubled = (char *)malloc(dlen);
    if (!doubled) {
        safe_strcpy(out, out_sz, title);
        return;
    }
    snprintf(doubled, dlen, "%s    %s", title, title);

    const char *p = doubled;
    int cur_col = 0;
    while (*p && cur_col < step) {
        int ch_len = utf8_charlen((unsigned char)*p);
        cur_col++;
        p += ch_len;
    }

    size_t o = 0;
    int drawn = 0;
    while (*p && drawn < avail_w && o + 4 < out_sz) {
        int ch_len = utf8_charlen((unsigned char)*p);
        for (int k = 0; k < ch_len; k++) {
            out[o++] = p[k];
        }
        drawn++;
        p += ch_len;
    }
    while (drawn < avail_w && o + 1 < out_sz) {
        out[o++] = ' ';
        drawn++;
    }
    out[o] = '\0';
    free(doubled);
}

static int64_t get_current_time_ms(void) {
#ifdef _WIN32
    /* GetTickCount() retorna 32 bits e da a volta em ~49,7 dias, quebrando
     * autosave/animacoes em sessoes longas. GetTickCount64 nao tem esse limite. */
    return (int64_t)GetTickCount64();
#else
    struct timespec ts;
    if (clock_gettime(CLOCK_MONOTONIC, &ts) != 0) return 0;
    return (int64_t)ts.tv_sec * 1000 + ts.tv_nsec / 1000000;
#endif
}

/* =========================================================================
 * Base64 Encoding for OSC 52 System Clipboard Sharing
 * ========================================================================= */
static const char b64_table[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

static char *base64_encode(const unsigned char *data, size_t input_len) {
    if (input_len > (SIZE_MAX / 4) * 3) return NULL;
    size_t output_len = 4 * ((input_len + 2) / 3);
    if (output_len == SIZE_MAX) return NULL;
    char *encoded = (char *)malloc(output_len + 1);
    if (!encoded) return NULL;

    size_t j = 0;
    for (size_t i = 0; i < input_len; i += 3) {
        uint32_t oct_a = data[i];
        uint32_t oct_b = (i + 1 < input_len) ? data[i + 1] : 0;
        uint32_t oct_c = (i + 2 < input_len) ? data[i + 2] : 0;
        uint32_t triple = (oct_a << 16) | (oct_b << 8) | oct_c;

        encoded[j++] = b64_table[(triple >> 18) & 0x3F];
        encoded[j++] = b64_table[(triple >> 12) & 0x3F];
        encoded[j++] = (i + 1 < input_len) ? b64_table[(triple >> 6) & 0x3F] : '=';
        encoded[j++] = (i + 2 < input_len) ? b64_table[triple & 0x3F] : '=';
    }
    encoded[output_len] = '\0';
    return encoded;
}

#ifdef _WIN32
typedef struct {
    char *text;
    int len;
} WinClipData;

static DWORD WINAPI win32_clipboard_thread(LPVOID param) {
    WinClipData *cd = (WinClipData *)param;
    if (!cd) return 0;

    int retries = 10;
    while (!OpenClipboard(NULL) && retries-- > 0) {
        Sleep(10);
    }

    if (retries >= 0) {
        EmptyClipboard();
        int wlen = MultiByteToWideChar(CP_UTF8, 0, cd->text, cd->len, NULL, 0);
        if (wlen > 0) {
            HGLOBAL hMem = GlobalAlloc(GMEM_MOVEABLE, (wlen + 1) * sizeof(WCHAR));
            if (hMem) {
                WCHAR *pw = (WCHAR *)GlobalLock(hMem);
                if (pw) {
                    MultiByteToWideChar(CP_UTF8, 0, cd->text, cd->len, pw, wlen);
                    pw[wlen] = L'\0';
                    GlobalUnlock(hMem);
                    if (!SetClipboardData(CF_UNICODETEXT, hMem)) {
                        GlobalFree(hMem);
                    }
                } else {
                    GlobalFree(hMem);
                }
            }
        }
        CloseClipboard();
    }

    free(cd->text);
    free(cd);
    return 0;
}
#else
typedef struct {
    char *text;
    int len;
} PosixClipData;

static void *posix_clipboard_thread(void *param) {
    PosixClipData *cd = (PosixClipData *)param;
    if (!cd) return NULL;

    /* Detect WSL, Git Bash or Windows host */
    bool is_windows_host = false;
    const char *clip_cmd = NULL;

    if (access("/mnt/c/Windows/System32/clip.exe", X_OK) == 0) {
        is_windows_host = true;
        clip_cmd = "/mnt/c/Windows/System32/clip.exe 2>/dev/null";
    } else if (access("/c/Windows/System32/clip.exe", X_OK) == 0) {
        is_windows_host = true;
        clip_cmd = "/c/Windows/System32/clip.exe 2>/dev/null";
    } else {
        const char *wsl_env = getenv("WSL_DISTRO_NAME");
        if (wsl_env && *wsl_env) {
            is_windows_host = true;
            clip_cmd = "clip.exe 2>/dev/null";
        }
    }

    FILE *pipe = NULL;
    if (is_windows_host && clip_cmd) {
        pipe = popen(clip_cmd, "w");
    } else {
        const char *wayland = getenv("WAYLAND_DISPLAY");
        const char *x11 = getenv("DISPLAY");
        if (wayland && *wayland) {
            pipe = popen("wl-copy 2>/dev/null", "w");
        } else if (x11 && *x11) {
            pipe = popen("xclip -selection clipboard 2>/dev/null || xsel --clipboard --input 2>/dev/null", "w");
        }
    }

    if (pipe) {
        fwrite(cd->text, 1, cd->len, pipe);
        pclose(pipe);
    }

    free(cd->text);
    free(cd);
    return NULL;
}
#endif

static void copy_to_system_clipboard(const char *text, int len) {
    if (!text || len <= 0) return;

    /* 1. OSC 52 Terminal Clipboard sequence:
     * - On Linux / WSL / SSH, terminals (Windows Terminal, tmux, iTerm2, WezTerm, etc.)
     *   intercept this and copy to client clipboard.
     * - On Windows, if running over an SSH session, send OSC 52 to reach the client.
     *   When running locally on Windows, skip OSC 52 to avoid redundant lock contention
     *   with Windows Terminal since CF_UNICODETEXT directly updates the Windows clipboard.
     */
#ifdef _WIN32
    bool send_osc52 = (getenv("SSH_CLIENT") != NULL || getenv("SSH_TTY") != NULL || getenv("SSH_CONNECTION") != NULL);
#else
    bool send_osc52 = true;
#endif

    if (send_osc52) {
        char *b64 = base64_encode((const unsigned char *)text, len);
        if (b64) {
            printf("\x1b]52;c;%s\x07", b64);
            fflush(stdout);
            free(b64);
        }
    }

    /* 2. Asynchronous Native System Clipboard writing (Non-blocking):
     * The main TUI thread returns in 0ms so status messages appear instantaneously.
     */
#ifdef _WIN32
    WinClipData *cd = (WinClipData *)malloc(sizeof(WinClipData));
    if (cd) {
        cd->text = (char *)malloc(len + 1);
        if (cd->text) {
            memcpy(cd->text, text, len);
            cd->text[len] = '\0';
            cd->len = len;
            HANDLE hThread = CreateThread(NULL, 0, win32_clipboard_thread, cd, 0, NULL);
            if (hThread) {
                CloseHandle(hThread);
            } else {
                free(cd->text);
                free(cd);
            }
        } else {
            free(cd);
        }
    }
#else
    PosixClipData *cd = (PosixClipData *)malloc(sizeof(PosixClipData));
    if (cd) {
        cd->text = (char *)malloc(len + 1);
        if (cd->text) {
            memcpy(cd->text, text, len);
            cd->text[len] = '\0';
            cd->len = len;
            pthread_t th;
            pthread_attr_t attr;
            pthread_attr_init(&attr);
            pthread_attr_setdetachstate(&attr, PTHREAD_CREATE_DETACHED);
            if (pthread_create(&th, &attr, posix_clipboard_thread, cd) != 0) {
                free(cd->text);
                free(cd);
            }
            pthread_attr_destroy(&attr);
        } else {
            free(cd);
        }
    }
#endif
}

#ifdef _WIN32
static char *get_system_clipboard(int *out_len) {
    if (out_len) *out_len = 0;
    int retries = 10;
    while (!OpenClipboard(NULL) && retries-- > 0) {
        Sleep(5);
    }
    if (retries < 0) return NULL;

    HANDLE hData = GetClipboardData(CF_UNICODETEXT);
    if (hData) {
        WCHAR *pw = (WCHAR *)GlobalLock(hData);
        if (pw) {
            /* O conteudo do clipboard nao e obrigado a terminar em NUL dentro
             * do bloco alocado; limita a leitura ao tamanho real do handle. */
            SIZE_T bytes = GlobalSize(hData);
            size_t max_wchars = (bytes >= sizeof(WCHAR)) ? (size_t)(bytes / sizeof(WCHAR)) : 0;
            size_t wlen = 0;
            while (wlen < max_wchars && pw[wlen] != L'\0') wlen++;
            if (wlen > 0 && wlen <= (size_t)INT_MAX) {
                int utf8_len = WideCharToMultiByte(CP_UTF8, 0, pw, (int)wlen,
                                                   NULL, 0, NULL, NULL);
                if (utf8_len > 0) {
                    char *res = (char *)malloc((size_t)utf8_len + 1);
                    if (res) {
                        WideCharToMultiByte(CP_UTF8, 0, pw, (int)wlen,
                                            res, utf8_len, NULL, NULL);
                        res[utf8_len] = '\0';
                        GlobalUnlock(hData);
                        CloseClipboard();
                        if (out_len) *out_len = utf8_len;
                        return res;
                    }
                }
            }
            GlobalUnlock(hData);
        }
    } else {
        /* Fallback to ANSI CF_TEXT */
        hData = GetClipboardData(CF_TEXT);
        if (hData) {
            char *pText = (char *)GlobalLock(hData);
            if (pText) {
                char *res = strdup(pText);
                GlobalUnlock(hData);
                CloseClipboard();
                if (res && out_len) *out_len = (int)strlen(res);
                return res;
            }
        }
    }

    CloseClipboard();
    return NULL;
}
#else
static char *get_system_clipboard(int *out_len) {
    if (out_len) *out_len = 0;

    FILE *pipe = NULL;
    const char *wayland = getenv("WAYLAND_DISPLAY");
    const char *x11 = getenv("DISPLAY");

    if (wayland && *wayland) {
        pipe = popen("wl-paste 2>/dev/null", "r");
    } else if (x11 && *x11) {
        pipe = popen("xclip -selection clipboard -o 2>/dev/null || xsel --clipboard --output 2>/dev/null", "r");
    } else {
        /* Check if running in WSL or Git Bash with access to Windows clipboard via powershell */
        const char *wsl = getenv("WSL_DISTRO_NAME");
        if (wsl || access("/mnt/c/Windows/System32/clip.exe", X_OK) == 0) {
            pipe = popen("/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe -NoProfile -Command Get-Clipboard 2>/dev/null || powershell.exe -NoProfile -Command Get-Clipboard 2>/dev/null", "r");
        } else if (access("/c/Windows/System32/clip.exe", X_OK) == 0) {
            pipe = popen("/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe -NoProfile -Command Get-Clipboard 2>/dev/null || powershell.exe -NoProfile -Command Get-Clipboard 2>/dev/null", "r");
        }
    }

    if (!pipe) return NULL;

    int cap = 4096;
    int len = 0;
    char *buf = (char *)malloc(cap);
    if (!buf) {
        pclose(pipe);
        return NULL;
    }

    char tmp[512];
    size_t nr;
    while ((nr = fread(tmp, 1, sizeof(tmp), pipe)) > 0) {
        if (len + (int)nr + 1 >= cap) {
            cap = (cap * 2) + (int)nr;
            char *nb = (char *)realloc(buf, cap);
            if (!nb) break;
            buf = nb;
        }
        memcpy(&buf[len], tmp, nr);
        len += (int)nr;
    }
    pclose(pipe);

    if (len <= 0) {
        free(buf);
        return NULL;
    }
    buf[len] = '\0';
    if (out_len) *out_len = len;
    return buf;
}
#endif

/* =========================================================================
 * System / External Links Execution
 * ========================================================================= */
static void open_link_target(const char *raw_target) {
    if (!raw_target || !*raw_target) return;

    /* 1. Trim leading and trailing whitespace */
    char target[4096];
    int start = 0;
    while (raw_target[start] && (unsigned char)raw_target[start] <= 32) start++;
    safe_strcpy(target, sizeof(target), &raw_target[start]);
    int tlen = (int)strlen(target);
    while (tlen > 0 && (unsigned char)target[tlen - 1] <= 32) {
        target[--tlen] = '\0';
    }
    if (tlen == 0) return;

    /* 2. Strip surrounding angle brackets <...> if present (Markdown extended link syntax) */
    if (target[0] == '<' && target[tlen - 1] == '>') {
        target[tlen - 1] = '\0';
        memmove(target, target + 1, tlen - 1);
        tlen -= 2;
    }

#ifdef _WIN32
    /* 3. Check for web URLs */
    bool is_web = (strncasecmp(target, "http://", 7) == 0 ||
                   strncasecmp(target, "https://", 8) == 0 ||
                   strncasecmp(target, "ftp://", 6) == 0 ||
                   strncasecmp(target, "mailto:", 7) == 0 ||
                   strncasecmp(target, "www.", 4) == 0);
    if (is_web) {
        /* 4096 + 8 para o prefixo: com 4096 o gcc alertava truncagem
         * silenciosa (o alvo pode ter ate 4095 bytes). */
        char url_buf[4104];
        if (strncasecmp(target, "www.", 4) == 0) {
            snprintf(url_buf, sizeof(url_buf), "https://%s", target);
        } else {
            safe_strcpy(url_buf, sizeof(url_buf), target);
        }
        set_status_msg("Abrindo no navegador: %s", url_buf);
        WCHAR *wurl = utf8_to_wide_alloc(url_buf);
        if (!wurl) {
            set_status_msg("URL longa demais/invalida: %s", url_buf);
            return;
        }
        HINSTANCE h = ShellExecuteW(NULL, L"open", wurl, NULL, NULL, SW_SHOWNORMAL);
        free(wurl);
        if ((INT_PTR)h <= 32) {
            set_status_msg("Erro ao abrir navegador para: %s", url_buf);
        }
        return;
    }

    /* Local File / Directory processing */
    const char *p = target;
    if (strncasecmp(p, "file:///", 8) == 0) {
        p += 8;
    } else if (strncasecmp(p, "file://", 7) == 0) {
        p += 7;
    }

    /* Decode URL percent-encoding (e.g. %20 -> space) and normalize slashes */
    char clean_path[4096];
    int ci = 0;
    for (int i = 0; p[i] && ci < 4090; i++) {
        if (p[i] == '%' && isxdigit((unsigned char)p[i+1]) && isxdigit((unsigned char)p[i+2])) {
            char hex[3] = { p[i+1], p[i+2], '\0' };
            clean_path[ci++] = (char)strtol(hex, NULL, 16);
            i += 2;
        } else if (p[i] == '/') {
            clean_path[ci++] = '\\';
        } else {
            clean_path[ci++] = p[i];
        }
    }
    clean_path[ci] = '\0';

    /* Convert to wide char (UTF-16) for complete Windows Unicode support */
    WCHAR wpath[4096];
    if (MultiByteToWideChar(CP_UTF8, 0, clean_path, -1, wpath, 4096) <= 0) {
        set_status_msg("Erro ao decodificar caminho: %s", clean_path);
        return;
    }

    /* Resolve canonical full absolute path */
    WCHAR full_wpath[4096];
    DWORD len = GetFullPathNameW(wpath, 4096, full_wpath, NULL);
    if (len == 0 || len >= 4096) {
        wcsncpy(full_wpath, wpath, 4095);
        full_wpath[4095] = L'\0';
    }

    /* Check existence; if not found relative to process/exe dir, try relative to current note dir */
    DWORD attrs = GetFileAttributesW(full_wpath);
    if (attrs == INVALID_FILE_ATTRIBUTES && E.cur_filepath[0]) {
        char note_dir[4096];
        safe_strcpy(note_dir, sizeof(note_dir), E.cur_filepath);
        char *last_sep = strrchr(note_dir, '\\');
        if (!last_sep) last_sep = strrchr(note_dir, '/');
        if (last_sep) {
            *last_sep = '\0';
            /* Dimensionado para o pior caso: note_dir (ate 4095) + '\' +
             * clean_path (ate 4095) = 8191. Com 4096 o compilador do MinGW
             * alertava sobre truncagem silenciosa (-Wformat-truncation), e um
             * caminho muito longo era cortado no meio, fazendo o link "nao
             * abrir" sem nenhuma explicacao ao usuario. */
            char combined[8192];
            snprintf(combined, sizeof(combined), "%s\\%s", note_dir, clean_path);
            /* Alinhado com combined: MultiByteToWideChar falha (nao trunca)
             * quando o destino e insuficiente, entao o buffer precisa comportar
             * todo o pior caso que combined aceita. */
            WCHAR wcomb[8192];
            if (MultiByteToWideChar(CP_UTF8, 0, combined, -1, wcomb, 8192) > 0) {
                WCHAR full_comb[4096];
                if (GetFullPathNameW(wcomb, 4096, full_comb, NULL) > 0) {
                    DWORD cattrs = GetFileAttributesW(full_comb);
                    if (cattrs != INVALID_FILE_ATTRIBUTES) {
                        wcsncpy(full_wpath, full_comb, 4095);
                        full_wpath[4095] = L'\0';
                        attrs = cattrs;
                    }
                }
            }
        }
    }

    if (attrs != INVALID_FILE_ATTRIBUTES) {
        if (attrs & FILE_ATTRIBUTE_DIRECTORY) {
            /* Open directory in Windows File Explorer */
            HINSTANCE h = ShellExecuteW(NULL, L"explore", full_wpath, NULL, NULL, SW_SHOWNORMAL);
            if ((INT_PTR)h <= 32) {
                h = ShellExecuteW(NULL, L"open", full_wpath, NULL, NULL, SW_SHOWNORMAL);
            }
            if ((INT_PTR)h > 32) {
                set_status_msg("Pasta aberta no Explorer: %s", clean_path);
            } else {
                set_status_msg("Erro ao abrir pasta: %s", clean_path);
            }
        } else {
            /* Open file with default application associated in Windows */
            HINSTANCE h = ShellExecuteW(NULL, L"open", full_wpath, NULL, NULL, SW_SHOWNORMAL);
            if ((INT_PTR)h > 32) {
                set_status_msg("Arquivo aberto: %s", clean_path);
            } else if ((INT_PTR)h == SE_ERR_NOASSOC) {
                /* If no application is associated, show Open With dialog */
                ShellExecuteW(NULL, L"openas", full_wpath, NULL, NULL, SW_SHOWNORMAL);
                set_status_msg("Abrindo diálogo 'Abrir com...' para: %s", clean_path);
            } else {
                set_status_msg("Erro ao abrir arquivo: %s", clean_path);
            }
        }
    } else {
        set_status_msg("Arquivo ou pasta não encontrado: %s", clean_path);
    }
#else
    set_status_msg("Abrindo: %s", target);
    /* fork+exec em vez de system(): o alvo vem do conteudo da nota e nao pode
     * ser interpretado pelo shell (evita injecao de comandos via link). */
    pid_t pid = fork();
    if (pid == 0) {
        setsid();
        pid_t pid2 = fork();
        if (pid2 == 0) {
            int devnull = open("/dev/null", O_WRONLY);
            if (devnull >= 0) {
                dup2(devnull, STDOUT_FILENO);
                dup2(devnull, STDERR_FILENO);
                close(devnull);
            }
            execlp("xdg-open", "xdg-open", "--", target, (char *)NULL);
            _exit(127);
        }
        _exit(0);
    } else if (pid > 0) {
        while (waitpid(pid, NULL, 0) < 0 && errno == EINTR) { /* retry */ }
    } else {
        set_status_msg("Nao foi possivel abrir o link: %s", target);
    }
#endif
}


/* =========================================================================
 * Line & Buffer Operations
 * ========================================================================= */

static void detect_heading(Line *l) {
    l->heading_lvl = 0;
    if (!l->chars || l->len == 0) return;
    int i = 0;
    while (i < l->len && l->chars[i] == '#') i++;
    if (i >= 1 && i <= 6 && i < l->len && l->chars[i] == ' ') {
        l->heading_lvl = i;
    }
}

static void line_init(Line *l, const char *s, int len) {
    l->chars = (char *)malloc(len + 1);
    if (s && len > 0) memcpy(l->chars, s, len);
    l->chars[len] = '\0';
    l->len = len;
    l->folded = false;
    l->hidden = false;
    l->is_code_fence = false;
    l->in_code_block = false;
    detect_heading(l);
}

static void line_free(Line *l) {
    if (l->chars) free(l->chars);
    l->chars = NULL;
    l->len = 0;
}

static void mark_modified(void) {
    int64_t now = get_current_time_ms();
    if (!E.modified || E.first_unsaved_time == 0) {
        E.first_unsaved_time = now;
    }
    E.modified = true;
    if (E.num_tabs > 0 && E.active_tab >= 0 && E.active_tab < E.num_tabs) {
        E.tabs[E.active_tab].modified = true;
        if (E.tabs[E.active_tab].first_unsaved_time == 0) {
            E.tabs[E.active_tab].first_unsaved_time = now;
        }
    }
}

static void buf_insert_line(int at, const char *s, int len) {
    if (at < 0 || at > E.num_lines) return;
    if (E.num_lines + 1 > E.cap_lines) {
        int new_cap = E.cap_lines ? E.cap_lines * 2 : 32;
        Line *new_lines = (Line *)realloc(E.lines, sizeof(Line) * new_cap);
        if (!new_lines) return;
        E.lines = new_lines;
        E.cap_lines = new_cap;
    }
    memmove(&E.lines[at + 1], &E.lines[at], sizeof(Line) * (E.num_lines - at));
    line_init(&E.lines[at], s, len);
    E.num_lines++;
    mark_modified();
    update_folding();
}

static void buf_delete_line(int at) {
    if (at < 0 || at >= E.num_lines) return;
    line_free(&E.lines[at]);
    memmove(&E.lines[at], &E.lines[at + 1], sizeof(Line) * (E.num_lines - at - 1));
    E.num_lines--;
    mark_modified();
    if (E.num_lines == 0) {
        buf_insert_line(0, "", 0);
    }
    update_folding();
}

static void line_insert_char(Line *l, int at, char c) {
    if (at < 0 || at > l->len) at = l->len;
    char *nc = (char *)realloc(l->chars, l->len + 2);
    if (!nc) return;
    l->chars = nc;
    memmove(&l->chars[at + 1], &l->chars[at], l->len - at + 1);
    l->chars[at] = c;
    l->len++;
    l->chars[l->len] = '\0';
    detect_heading(l);
    mark_modified();
}

static void line_replace_range(Line *l, int at, int del_len, const char *new_str, int new_len) {
    if (!l || !l->chars || at < 0 || at > l->len) return;
    if (del_len < 0) del_len = 0;
    if (new_len < 0) new_len = 0;
    if (new_str == NULL) new_len = 0;
    if (del_len > l->len - at) del_len = l->len - at;

    int diff = new_len - del_len;
    if (diff > 0 && l->len > INT_MAX - diff - 1) return;
    if (at > INT_MAX - new_len - 1) return;

    /* new_str pode apontar para dentro da propria linha (ex.: substituir um
     * trecho por outro dela). O realloc invalidaria esse ponteiro. */
    char *alias_copy = NULL;
    uintptr_t src = (uintptr_t)(const void *)new_str;
    uintptr_t begin = (uintptr_t)(void *)l->chars;
    uintptr_t end = begin + (uintptr_t)l->len;
    if (new_len > 0 && src >= begin && src < end) {
        alias_copy = (char *)malloc((size_t)new_len);
        if (!alias_copy) return;
        memcpy(alias_copy, new_str, (size_t)new_len);
        new_str = alias_copy;
    }

    if (diff > 0) {
        char *nc = (char *)realloc(l->chars, (size_t)l->len + (size_t)diff + 1);
        if (!nc) { free(alias_copy); return; }
        l->chars = nc;
    }
    memmove(&l->chars[at + new_len], &l->chars[at + del_len],
            (size_t)(l->len - (at + del_len)) + 1);
    if (new_len > 0) {
        memcpy(&l->chars[at], new_str, (size_t)new_len);
    }
    l->len += diff;
    if (diff < 0) {
        char *nc = (char *)realloc(l->chars, (size_t)l->len + 1);
        if (nc) l->chars = nc;
    }
    l->chars[l->len] = '\0';
    free(alias_copy);
    detect_heading(l);
    mark_modified();
}

static void line_delete_char(Line *l, int at) {
    if (at < 0 || at >= l->len) return;
    int bytes = 1;
    utf8_decode_codepoint(&l->chars[at], &bytes);
    if (bytes <= 0) bytes = 1;
    if (at + bytes > l->len) bytes = l->len - at;
    line_replace_range(l, at, bytes, NULL, 0);
}

static void line_append_string(Line *l, const char *s, int len) {
    if (len <= 0) return;
    char *nc = (char *)realloc(l->chars, l->len + len + 1);
    if (!nc) return;
    l->chars = nc;
    memcpy(&l->chars[l->len], s, len);
    l->len += len;
    l->chars[l->len] = '\0';
    detect_heading(l);
    mark_modified();
}

/* Clean Multi-line Paste Handler (prevents staircasing and buffer overflows) */
static void editor_paste_string(const char *text, int len) {
    if (!text || len <= 0) return;

    if (E.mode == MODE_SEARCH_LEFT) {
        int qlen = (int)strlen(E.search_query);
        for (int j = 0; j < len && qlen < 240; j++) {
            if (text[j] == '\r' || text[j] == '\n') break;
            E.search_query[qlen++] = text[j];
        }
        E.search_query[qlen] = '\0';
        E.search_cursor = qlen;
        apply_note_search(E.search_query);
        if (E.num_filtered > 0) {
            int actual = E.filtered_indices[E.selected_note_idx];
            load_preview(E.notes[actual].filepath);
        } else {
            free_preview();
        }
        return;
    }

    if (E.mode == MODE_COMMAND) {
        int clen = (int)strlen(E.cmd_buf);
        for (int j = 0; j < len && clen < 240; j++) {
            if (text[j] == '\r' || text[j] == '\n') break;
            E.cmd_buf[clen++] = text[j];
        }
        E.cmd_buf[clen] = '\0';
        E.cmd_cursor = clen;
        return;
    }

    if (E.mode >= MODE_PROMPT_NEW_NOTE && E.mode <= MODE_PROMPT_EXPORT_DIR) {
        int plen = (int)strlen(E.prompt_buf);
        for (int j = 0; j < len && plen < 240; j++) {
            if (text[j] == '\r' || text[j] == '\n') break;
            E.prompt_buf[plen++] = text[j];
        }
        E.prompt_buf[plen] = '\0';
        E.prompt_cursor = plen;
        return;
    }

    if (E.num_lines <= 0) {
        buf_insert_line(0, "", 0);
        E.cursor_y = 0;
        E.cursor_x = 0;
    }
    if (E.cursor_y >= E.num_lines) E.cursor_y = E.num_lines - 1;
    if (E.cursor_y < 0) E.cursor_y = 0;
    if (E.cursor_x > E.lines[E.cursor_y].len) E.cursor_x = E.lines[E.cursor_y].len;
    if (E.cursor_x < 0) E.cursor_x = 0;

    save_undo();

    int i = 0;
    while (i < len) {
        int start = i;
        while (i < len && text[i] != '\n' && text[i] != '\r') {
            i++;
        }
        int chunk_len = i - start;

        if (chunk_len > 0) {
            Line *cur = &E.lines[E.cursor_y];
            char *nc = (char *)realloc(cur->chars, cur->len + chunk_len + 1);
            if (!nc) break;
            cur->chars = nc;
            memmove(&cur->chars[E.cursor_x + chunk_len], &cur->chars[E.cursor_x], cur->len - E.cursor_x + 1);
            memcpy(&cur->chars[E.cursor_x], &text[start], chunk_len);
            cur->len += chunk_len;
            E.cursor_x += chunk_len;
            detect_heading(cur);
        }

        if (i < len && (text[i] == '\n' || text[i] == '\r')) {
            char nl_ch = text[i++];
            if (nl_ch == '\r' && i < len && text[i] == '\n') {
                i++; /* Skip \n in CRLF */
            }

            Line *cur = &E.lines[E.cursor_y];
            buf_insert_line(E.cursor_y + 1, &cur->chars[E.cursor_x], cur->len - E.cursor_x);
            cur = &E.lines[E.cursor_y];
            cur->chars[E.cursor_x] = '\0';
            cur->len = E.cursor_x;
            detect_heading(cur);

            E.cursor_y++;
            E.cursor_x = 0;
        }
    }

    update_folding();
    mark_modified();
    g_screen_clear_needed = 1;
}

/* =========================================================================
 * Section Folding (Requirement 16: za, zM, zR)
 * ========================================================================= */

static void update_lines_syntax(Line *lines, int num_lines) {
    int hide_lvl = 0;
    bool in_code = false;
    for (int i = 0; i < num_lines; i++) {
        Line *l = &lines[i];

        const char *p = l->chars ? l->chars : "";
        while (*p == ' ' || *p == '\t') p++;
        if (strncmp(p, "```", 3) == 0) {
            l->is_code_fence = true;
            l->in_code_block = false;
            in_code = !in_code;
        } else {
            l->is_code_fence = false;
            l->in_code_block = in_code;
        }

        if (l->heading_lvl > 0 && !l->in_code_block) {
            if (hide_lvl > 0 && l->heading_lvl <= hide_lvl) {
                hide_lvl = 0;
            }
            if (hide_lvl > 0) {
                l->hidden = true;
            } else {
                l->hidden = false;
                if (l->folded) hide_lvl = l->heading_lvl;
            }
        } else {
            l->hidden = (hide_lvl > 0);
        }
    }
}

static void update_folding(void) {
    update_lines_syntax(E.lines, E.num_lines);
}

static void toggle_fold_at_cursor(void) {
    if (E.cursor_y < 0 || E.cursor_y >= E.num_lines) return;
    int target_idx = E.cursor_y;
    if (E.lines[target_idx].heading_lvl == 0) {
        for (int i = target_idx; i >= 0; i--) {
            if (E.lines[i].heading_lvl > 0) {
                target_idx = i;
                break;
            }
        }
    }
    if (E.lines[target_idx].heading_lvl > 0) {
        E.lines[target_idx].folded = !E.lines[target_idx].folded;
        update_folding();
        if (E.lines[target_idx].folded) {
            E.cursor_y = target_idx;
        }
        editor_scroll();
        g_screen_clear_needed = 1;
        set_status_msg("Dobra de seção %s", E.lines[target_idx].folded ? "recolhida (za)" : "expandida (za)");
    } else {
        set_status_msg("Nenhum cabeçalho (#) encontrado para recolher/expandir");
    }
}

static void collapse_all_headings(void) {
    int count = 0;
    for (int i = 0; i < E.num_lines; i++) {
        if (E.lines[i].heading_lvl > 0) {
            E.lines[i].folded = true;
            count++;
        }
    }
    update_folding();
    while (E.cursor_y > 0 && E.lines[E.cursor_y].hidden) {
        E.cursor_y--;
    }
    editor_scroll();
    g_screen_clear_needed = 1;
    if (count > 0) {
        set_status_msg("Todas as %d seções recolhidas (zM)", count);
    } else {
        set_status_msg("Nenhum cabeçalho (#) encontrado nesta nota");
    }
}

static void uncollapse_all_headings(void) {
    int count = 0;
    for (int i = 0; i < E.num_lines; i++) {
        if (E.lines[i].heading_lvl > 0) {
            E.lines[i].folded = false;
            count++;
        }
    }
    update_folding();
    editor_scroll();
    g_screen_clear_needed = 1;
    if (count > 0) {
        set_status_msg("Todas as %d seções expandidas (zR)", count);
    } else {
        set_status_msg("Nenhum cabeçalho (#) encontrado nesta nota");
    }
}

/* =========================================================================
 * Undo / Redo Stacks (Requirements 8 & Vim compatibility)
 * ========================================================================= */

static void free_undo_state(UndoState *us) {
    if (!us->lines) return;
    for (int i = 0; i < us->num_lines; i++) {
        line_free(&us->lines[i]);
    }
    free(us->lines);
    us->lines = NULL;
    us->num_lines = 0;
}

/* Estado da coalescencia de digitacao: uma sequencia continua de caracteres
 * gera um unico snapshot de undo em vez de um por tecla. Qualquer outra edicao
 * (Enter, Backspace, colar, tabela, troca de aba) chama save_undo() e invalida
 * este estado, preservando a granularidade dos demais comandos. */
static bool g_typing_undo_valid = false;
static int64_t g_typing_undo_deadline = 0;
static int g_typing_undo_line = -1;
static int g_typing_undo_expect_x = -1;
static char g_typing_undo_path[512];

static void invalidate_typing_undo(void) {
    g_typing_undo_valid = false;
    g_typing_undo_line = -1;
    g_typing_undo_expect_x = -1;
    g_typing_undo_deadline = 0;
    g_typing_undo_path[0] = '\0';
}

static void save_undo(void) {
    invalidate_typing_undo();
    if (E.undo_count >= MAX_UNDO) {
        free_undo_state(&E.undo_stack[0]);
        memmove(&E.undo_stack[0], &E.undo_stack[1], sizeof(UndoState) * (MAX_UNDO - 1));
        E.undo_count = MAX_UNDO - 1;
    }
    UndoState *us = &E.undo_stack[E.undo_count++];
    us->num_lines = E.num_lines;
    us->cursor_y = E.cursor_y;
    us->cursor_x = E.cursor_x;
    us->lines = (Line *)malloc(sizeof(Line) * (E.num_lines > 0 ? E.num_lines : 1));
    if (!us->lines) {
        E.undo_count--;
        return;
    }
    for (int i = 0; i < E.num_lines; i++) {
        line_init(&us->lines[i], E.lines[i].chars, E.lines[i].len);
        us->lines[i].heading_lvl = E.lines[i].heading_lvl;
        us->lines[i].folded = E.lines[i].folded;
        us->lines[i].hidden = E.lines[i].hidden;
    }
    for (int i = 0; i < E.redo_count; i++) {
        free_undo_state(&E.redo_stack[i]);
    }
    E.redo_count = 0;
}

/* Antes de inserir um caractere. Se a tecla anterior foi outro caractere, na
 * mesma linha/posicao esperada, dentro da janela e na mesma nota, reaproveita o
 * snapshot: um "burst" de digitacao vira um unico passo de undo. */
static void save_undo_for_typing(int line, int at, int bytes) {
    int64_t now = get_current_time_ms();
    if (g_typing_undo_valid && now <= g_typing_undo_deadline &&
        g_typing_undo_line == line && g_typing_undo_expect_x == at &&
        strcmp(g_typing_undo_path, E.cur_filepath) == 0) {
        g_typing_undo_expect_x = at + bytes;
        g_typing_undo_deadline = now + TYPING_UNDO_COALESCE_MS;
        return;
    }

    save_undo();
    g_typing_undo_valid = true;
    g_typing_undo_line = line;
    g_typing_undo_expect_x = at + bytes;
    g_typing_undo_deadline = now + TYPING_UNDO_COALESCE_MS;
    safe_strcpy(g_typing_undo_path, sizeof(g_typing_undo_path), E.cur_filepath);
}

static void apply_undo(void) {
    invalidate_typing_undo();
    if (E.undo_count <= 0) {
        set_status_msg("Já está na alteração mais antiga");
        return;
    }
    if (E.redo_count >= MAX_UNDO) {
        free_undo_state(&E.redo_stack[0]);
        memmove(&E.redo_stack[0], &E.redo_stack[1], sizeof(UndoState) * (MAX_UNDO - 1));
        E.redo_count = MAX_UNDO - 1;
    }
    UndoState *rs = &E.redo_stack[E.redo_count++];
    rs->num_lines = E.num_lines;
    rs->cursor_y = E.cursor_y;
    rs->cursor_x = E.cursor_x;
    rs->lines = (Line *)malloc(sizeof(Line) * (E.num_lines > 0 ? E.num_lines : 1));
    if (!rs->lines) {
        E.redo_count--;
        return;
    }
    for (int i = 0; i < E.num_lines; i++) {
        line_init(&rs->lines[i], E.lines[i].chars, E.lines[i].len);
        rs->lines[i].heading_lvl = E.lines[i].heading_lvl;
        rs->lines[i].folded = E.lines[i].folded;
        rs->lines[i].hidden = E.lines[i].hidden;
    }

    UndoState *us = &E.undo_stack[--E.undo_count];
    for (int i = 0; i < E.num_lines; i++) line_free(&E.lines[i]);
    if (E.lines) free(E.lines);
    E.num_lines = us->num_lines;
    E.cap_lines = us->num_lines > 0 ? us->num_lines : 1;
    E.lines = us->lines;
    E.cursor_y = us->cursor_y;
    E.cursor_x = us->cursor_x;
    us->lines = NULL;
    us->num_lines = 0;
    update_folding();
    mark_modified();
    set_status_msg("Desfazer (u)");
}

static void apply_redo(void) {
    invalidate_typing_undo();
    if (E.redo_count <= 0) {
        set_status_msg("Já está na alteração mais recente");
        return;
    }
    if (E.undo_count >= MAX_UNDO) {
        free_undo_state(&E.undo_stack[0]);
        memmove(&E.undo_stack[0], &E.undo_stack[1], sizeof(UndoState) * (MAX_UNDO - 1));
        E.undo_count = MAX_UNDO - 1;
    }
    UndoState *us = &E.undo_stack[E.undo_count++];
    us->num_lines = E.num_lines;
    us->cursor_y = E.cursor_y;
    us->cursor_x = E.cursor_x;
    us->lines = (Line *)malloc(sizeof(Line) * (E.num_lines > 0 ? E.num_lines : 1));
    if (!us->lines) {
        E.undo_count--;
        return;
    }
    for (int i = 0; i < E.num_lines; i++) {
        line_init(&us->lines[i], E.lines[i].chars, E.lines[i].len);
        us->lines[i].heading_lvl = E.lines[i].heading_lvl;
        us->lines[i].folded = E.lines[i].folded;
        us->lines[i].hidden = E.lines[i].hidden;
    }

    UndoState *rs = &E.redo_stack[--E.redo_count];
    for (int i = 0; i < E.num_lines; i++) line_free(&E.lines[i]);
    if (E.lines) free(E.lines);
    E.num_lines = rs->num_lines;
    E.cap_lines = rs->num_lines > 0 ? rs->num_lines : 1;
    E.lines = rs->lines;
    E.cursor_y = rs->cursor_y;
    E.cursor_x = rs->cursor_x;
    rs->lines = NULL;
    rs->num_lines = 0;
    update_folding();
    mark_modified();
    set_status_msg("Refazer (Ctrl+R)");
}

/* =========================================================================
 * Notes Folder Scanner & Multi-Word Diacritic-Insensitive Indexer
 * ========================================================================= */

static void format_rel_time(time_t mtime, time_t now, char *buf, size_t sz) {
    if (mtime <= 0) {
        mtime = now;
    }
    long diff = (long)(now - mtime);
    if (diff < 0) diff = 0;
    if (diff < 60) {
        snprintf(buf, sz, "now");
    } else if (diff < 3600) {
        snprintf(buf, sz, "%ldm", diff / 60);
    } else if (diff < 86400) {
        snprintf(buf, sz, "%ldh", diff / 3600);
    } else if (diff < 604800) {
        snprintf(buf, sz, "%ldd", diff / 86400);
    } else if (diff < 2592000) {
        snprintf(buf, sz, "%ldw", diff / 604800);
    } else if (diff < 31536000) {
        snprintf(buf, sz, "%ldmo", diff / 2592000);
    } else {
        snprintf(buf, sz, "%ldy", diff / 31536000);
    }
}

static void normalize_text(const char *src, char *dst, size_t dst_cap) {
    if (!src || !dst || dst_cap == 0) return;
    size_t out = 0;
    size_t i = 0;
    while (src[i] && out + 1 < dst_cap) {
        unsigned char c1 = (unsigned char)src[i];
        if (c1 == 0xC3 && src[i + 1]) {
            unsigned char c2 = (unsigned char)src[i + 1];
            char rep = 0;
            if ((c2 >= 0x80 && c2 <= 0x85) || (c2 >= 0xA0 && c2 <= 0xA5)) rep = 'a';
            else if (c2 == 0x87 || c2 == 0xA7) rep = 'c';
            else if ((c2 >= 0x88 && c2 <= 0x8B) || (c2 >= 0xA8 && c2 <= 0xAB)) rep = 'e';
            else if ((c2 >= 0x8C && c2 <= 0x8F) || (c2 >= 0xAC && c2 <= 0xAF)) rep = 'i';
            else if (c2 == 0x91 || c2 == 0xB1) rep = 'n';
            else if ((c2 >= 0x92 && c2 <= 0x96) || (c2 >= 0xB2 && c2 <= 0xB6)) rep = 'o';
            else if ((c2 >= 0x99 && c2 <= 0x9C) || (c2 >= 0xB9 && c2 <= 0xBC)) rep = 'u';
            else if (c2 == 0x9D || c2 == 0xBD || c2 == 0xBF) rep = 'y';

            if (rep) {
                dst[out++] = rep;
                i += 2;
                continue;
            }
        } else if (c1 == 0xC4 && src[i + 1]) {
            unsigned char c2 = (unsigned char)src[i + 1];
            char rep = 0;
            if (c2 <= 0x85) rep = 'a';
            else if (c2 <= 0x8D) rep = 'c';
            else if (c2 <= 0x91) rep = 'd';
            else if (c2 <= 0x9B) rep = 'e';
            else if (c2 <= 0xA3) rep = 'g';
            else if (c2 <= 0xA7) rep = 'h';
            else if (c2 <= 0xB1) rep = 'i';
            if (rep) {
                dst[out++] = rep;
                i += 2;
                continue;
            }
        } else if (c1 == 0xC5 && src[i + 1]) {
            unsigned char c2 = (unsigned char)src[i + 1];
            char rep = 0;
            if (c2 <= 0x82) rep = 'l';
            else if (c2 <= 0x89) rep = 'n';
            else if (c2 <= 0x91) rep = 'o';
            else if (c2 <= 0x99) rep = 'r';
            else if (c2 <= 0xA1) rep = 's';
            else if (c2 <= 0xA7) rep = 't';
            else if (c2 <= 0xB3) rep = 'u';
            else if (c2 <= 0xBE) rep = 'z';
            if (rep) {
                dst[out++] = rep;
                i += 2;
                continue;
            }
        }
        if (c1 >= 'A' && c1 <= 'Z') {
            dst[out++] = (char)(c1 + ('a' - 'A'));
        } else {
            dst[out++] = (char)c1;
        }
        i++;
    }
    dst[out] = '\0';
}

static void extract_note_title(const char *filepath, const char *fname, char *out_title, size_t sz) {
    (void)filepath;
    safe_strcpy(out_title, sz, fname);
    char *dot = strrchr(out_title, '.');
    if (dot && (strcasecmp(dot, ".md") == 0 || strcasecmp(dot, ".markdown") == 0 ||
                strcasecmp(dot, ".mdown") == 0 || strcasecmp(dot, ".txt") == 0 ||
                strcasecmp(dot, ".text") == 0)) {
        *dot = '\0';
    }
}

/* Comparador de exibicao da lista de notas:
 *   1) grupo de urgencia: nivel 1, depois 2, depois 3, e por ultimo as sem
 *      marcacao ([ ]);
 *   2) dentro de cada grupo, a editada MAIS RECENTEMENTE primeiro;
 *   3) desempate por nome, para a ordem ser estavel entre redesenhos.
 * Reaproveita compare_notes_mtime() para o criterio de data. A definicao dela
 * vem logo abaixo, por isso a declaracao antecipada aqui. */
static int compare_notes_mtime(const void *a, const void *b);

static int compare_notes_priority(const void *a, const void *b) {
    const NoteInfo *na = (const NoteInfo *)a;
    const NoteInfo *nb = (const NoteInfo *)b;

    int ra = (na->attention_level >= 1 && na->attention_level <= 6)
             ? na->attention_level - 1 : 6;
    int rb = (nb->attention_level >= 1 && nb->attention_level <= 6)
             ? nb->attention_level - 1 : 6;
    if (ra != rb) return ra - rb;

    int c = compare_notes_mtime(a, b);
    if (c != 0) return c;
    return strcmp(na->filename, nb->filename);
}

static int compare_notes_mtime(const void *a, const void *b) {
    const NoteInfo *na = (const NoteInfo *)a;
    const NoteInfo *nb = (const NoteInfo *)b;
    if (nb->mtime > na->mtime) return 1;
    if (nb->mtime < na->mtime) return -1;
    return 0;
}

static char *read_entire_file(const char *path, long *out_len) {
    if (out_len) *out_len = 0;
    FILE *f = fopen_utf8(path, "rb");
    if (!f) return NULL;

#ifdef _WIN32
    if (_fseeki64(f, 0, SEEK_END) != 0) { fclose(f); return NULL; }
    __int64 len64 = _ftelli64(f);
    if (len64 < 0 || len64 > (__int64)LONG_MAX) { fclose(f); return NULL; }
    long len = (long)len64;
    if (_fseeki64(f, 0, SEEK_SET) != 0) { fclose(f); return NULL; }
#else
    if (fseeko(f, 0, SEEK_END) != 0) { fclose(f); return NULL; }
    off_t len64 = ftello(f);
    if (len64 < 0 || (uintmax_t)len64 > (uintmax_t)LONG_MAX) { fclose(f); return NULL; }
    long len = (long)len64;
    if (fseeko(f, 0, SEEK_SET) != 0) { fclose(f); return NULL; }
#endif

    char *buf = (char *)malloc((size_t)len + 1);
    if (!buf) { fclose(f); return NULL; }
    size_t bytes_read = fread(buf, 1, (size_t)len, f);
    bool short_read = (bytes_read != (size_t)len);
    if (short_read) {
        free(buf);
        fclose(f);
        return NULL;
    }
    buf[bytes_read] = '\0';
    if (out_len) *out_len = (long)bytes_read;
    fclose(f);
    return buf;
}


/* =========================================================================
 * Armazenamento SQLite (v6.29.0)
 * -------------------------------------------------------------------------
 * Toda a persistencia de notas passou a ficar em um unico banco SQLite
 * (notas.db), criado ao lado do executavel. O diretorio data/ com arquivos .md
 * deixou de ser o armazenamento principal: passou a ser apenas origem e
 * destino das rotinas de importacao e exportacao.
 *
 * COMO O RESTO DO PROGRAMA CONTINUA FUNCIONANDO SEM ALTERACAO:
 * o campo filepath (NoteInfo.filepath, Tab.cur_filepath, E.cur_filepath)
 * segue sendo a chave usada por toda a interface, mas agora carrega um caminho
 * VIRTUAL no formato "db:<id>". Como toda a logica de abas, undo, preview,
 * historico, busca e links apenas compara essa string, nada mais precisou
 * mudar. Caminhos reais de arquivo continuam aceitos como fallback, o que
 * mantem o argumento de linha de comando (abrir um .md avulso) funcionando.
 * ========================================================================= */

static sqlite3 *g_db = NULL;

static inline bool db_is_open(void) { return g_db != NULL; }

static void db_close(void) {
    sqlite3 *db = g_db;
    g_db = NULL;
    if (!db) return;
    /* close_v2 nunca devolve SQLITE_BUSY: se algum statement ainda existir,
     * o handle e liberado quando ele for finalizado. Evita vazar o banco no
     * encerramento caso uma rotina tenha falhado no meio. */
    sqlite3_close_v2(db);
}

static bool db_exec(const char *sql) {
    if (!g_db || !sql) return false;
    char *err = NULL;
    if (sqlite3_exec(g_db, sql, NULL, NULL, &err) != SQLITE_OK) {
        if (err) sqlite3_free(err);
        return false;
    }
    return true;
}

static int db_user_version(void) {
    if (!g_db) return -1;
    sqlite3_stmt *st = NULL;
    int version = -1;
    if (sqlite3_prepare_v2(g_db, "PRAGMA user_version;", -1, &st, NULL) == SQLITE_OK &&
        sqlite3_step(st) == SQLITE_ROW) {
        version = sqlite3_column_int(st, 0);
    }
    if (st) sqlite3_finalize(st);
    return version;
}

/* Abre (criando se necessario) o banco e garante o schema. O banco nasce
 * VAZIO: nenhuma nota de exemplo e criada. Em qualquer falha o handle e
 * fechado, para que uma tentativa seguinte nao receba um banco meio-aberto. */
static bool db_open(void) {
    if (g_db) return true;

    if (sqlite3_open(DB_PATH, &g_db) != SQLITE_OK) {
        if (g_db) { sqlite3_close(g_db); g_db = NULL; }
        return false;
    }
    sqlite3_busy_timeout(g_db, 3000);

    /* WAL permite leitura e escrita concorrentes sem travar o loop da
     * interface; synchronous=NORMAL e o ajuste recomendado pela propria SQLite
     * para o modo WAL. Se o filesystem nao suportar WAL, a SQLite mantem o
     * journal padrao: o programa continua funcionando, apenas sem esse modo. */
    db_exec("PRAGMA journal_mode=WAL;");
    db_exec("PRAGMA synchronous=NORMAL;");
    db_exec("PRAGMA foreign_keys=ON;");

    if (!db_exec("CREATE TABLE IF NOT EXISTS notes ("
                 "id        INTEGER PRIMARY KEY AUTOINCREMENT,"
                 "name      TEXT    NOT NULL UNIQUE,"
                 "content   TEXT    NOT NULL DEFAULT '',"
                 "mtime     INTEGER NOT NULL DEFAULT 0,"
                 "attention INTEGER NOT NULL DEFAULT 0);")) {
        goto fail;
    }
    if (!db_exec("CREATE INDEX IF NOT EXISTS idx_notes_mtime ON notes(mtime DESC);")) {
        goto fail;
    }

    int version = db_user_version();
    if (version < 0 || version > DB_SCHEMA_VERSION) goto fail;
    if (version < DB_SCHEMA_VERSION) {
        /* Schema v1 e o inicial. Versoes futuras devem aplicar migracoes
         * incrementais aqui, em vez de sobrescrever um banco mais novo. */
        if (!db_exec("PRAGMA user_version=1;")) goto fail;
    }
    return true;

fail:
    db_close();
    return false;
}

/* -------------------------------------------------------------------------
 * Identidade de nota: "db:<id>"
 * ------------------------------------------------------------------------- */
static int note_id_from_path(const char *path) {
    if (!path || strncmp(path, "db:", 3) != 0) return -1;
    const char *p = path + 3;
    if (!*p) return -1;
    /* strtol rejeita "db:abc" (que atoi convertia em 0) e "db:12lixo". */
    char *end = NULL;
    errno = 0;
    long value = strtol(p, &end, 10);
    if (errno == ERANGE || end == p || *end != '\0' ||
        value <= 0 || value > NOTE_ID_MAX) {
        return -1;
    }
    return (int)value;
}

static void note_path_from_id(int id, char *out, size_t out_sz) {
    if (!out || out_sz == 0) return;
    if (id <= 0) { out[0] = '\0'; return; }
    snprintf(out, out_sz, "db:%d", id);
}

/* -------------------------------------------------------------------------
 * Consultas e mutacoes
 * ------------------------------------------------------------------------- */
static int db_find_id_by_name(const char *name) {
    if (!g_db || !name || !*name) return -1;
    sqlite3_stmt *st = NULL;
    int id = -1;
    if (sqlite3_prepare_v2(g_db, "SELECT id FROM notes WHERE name=?1 LIMIT 1;",
                           -1, &st, NULL) != SQLITE_OK) {
        return -1;
    }
    sqlite3_bind_text(st, 1, name, -1, SQLITE_TRANSIENT);
    if (sqlite3_step(st) == SQLITE_ROW) id = sqlite3_column_int(st, 0);
    sqlite3_finalize(st);
    return id;
}

static int db_insert_note(const char *name, const char *content, time_t mtime, int attention) {
    if (!g_db || !name || !*name) return -1;
    if (attention < 0) attention = 0;
    if (attention > 6) attention = 6;
    sqlite3_stmt *st = NULL;
    int id = -1;
    if (sqlite3_prepare_v2(g_db,
            "INSERT INTO notes(name, content, mtime, attention) VALUES(?1,?2,?3,?4);",
            -1, &st, NULL) != SQLITE_OK) {
        return -1;
    }
    sqlite3_bind_text(st, 1, name, -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(st, 2, content ? content : "", -1, SQLITE_TRANSIENT);
    sqlite3_bind_int64(st, 3, (sqlite3_int64)(mtime > 0 ? mtime : time(NULL)));
    sqlite3_bind_int(st, 4, attention);
    if (sqlite3_step(st) == SQLITE_DONE) id = (int)sqlite3_last_insert_rowid(g_db);
    sqlite3_finalize(st);
    return id;
}

static bool db_update_content(int id, const char *content, time_t mtime) {
    if (!g_db || id < 0) return false;
    sqlite3_stmt *st = NULL;
    bool ok = false;
    if (sqlite3_prepare_v2(g_db,
            "UPDATE notes SET content=?1, mtime=?2 WHERE id=?3;",
            -1, &st, NULL) != SQLITE_OK) {
        return false;
    }
    sqlite3_bind_text(st, 1, content ? content : "", -1, SQLITE_TRANSIENT);
    sqlite3_bind_int64(st, 2, (sqlite3_int64)(mtime > 0 ? mtime : time(NULL)));
    sqlite3_bind_int(st, 3, id);
    if (sqlite3_step(st) == SQLITE_DONE) ok = (sqlite3_changes(g_db) > 0);
    sqlite3_finalize(st);
    return ok;
}

static bool db_update_name(int id, const char *newname) {
    if (!g_db || id < 0 || !newname || !*newname) return false;
    sqlite3_stmt *st = NULL;
    bool ok = false;
    if (sqlite3_prepare_v2(g_db, "UPDATE notes SET name=?1 WHERE id=?2;",
                           -1, &st, NULL) != SQLITE_OK) {
        return false;
    }
    sqlite3_bind_text(st, 1, newname, -1, SQLITE_TRANSIENT);
    sqlite3_bind_int(st, 2, id);
    /* Se ja existir outra nota com esse nome, o UNIQUE falha e nada muda. */
    if (sqlite3_step(st) == SQLITE_DONE) ok = (sqlite3_changes(g_db) > 0);
    sqlite3_finalize(st);
    return ok;
}

static bool db_delete_note(int id) {
    if (!g_db || id < 0) return false;
    sqlite3_stmt *st = NULL;
    bool ok = false;
    if (sqlite3_prepare_v2(g_db, "DELETE FROM notes WHERE id=?1;", -1, &st, NULL) != SQLITE_OK) {
        return false;
    }
    sqlite3_bind_int(st, 1, id);
    if (sqlite3_step(st) == SQLITE_DONE) ok = (sqlite3_changes(g_db) > 0);
    sqlite3_finalize(st);
    return ok;
}

static bool db_set_attention(int id, int level) {
    if (!g_db || id <= 0) return false;
    if (level < 0) level = 0;
    if (level > 6) level = 6;
    sqlite3_stmt *st = NULL;
    bool ok = false;
    if (sqlite3_prepare_v2(g_db, "UPDATE notes SET attention=?1 WHERE id=?2;",
                           -1, &st, NULL) != SQLITE_OK) {
        return false;
    }
    sqlite3_bind_int(st, 1, level);
    sqlite3_bind_int(st, 2, id);
    if (sqlite3_step(st) == SQLITE_DONE) ok = true;
    sqlite3_finalize(st);
    return ok;
}

static bool db_get_name(int id, char *out, size_t out_sz) {
    if (out && out_sz) out[0] = '\0';
    if (!g_db || id < 0) return false;
    sqlite3_stmt *st = NULL;
    bool ok = false;
    if (sqlite3_prepare_v2(g_db, "SELECT name FROM notes WHERE id=?1;", -1, &st, NULL) != SQLITE_OK) {
        return false;
    }
    sqlite3_bind_int(st, 1, id);
    if (sqlite3_step(st) == SQLITE_ROW) {
        const char *nm = (const char *)sqlite3_column_text(st, 0);
        if (nm) { safe_strcpy(out, out_sz, nm); ok = true; }
    }
    sqlite3_finalize(st);
    return ok;
}

static char *db_get_content(int id, long *out_len) {
    if (out_len) *out_len = 0;
    if (!g_db || id < 0) return NULL;
    sqlite3_stmt *st = NULL;
    char *out = NULL;
    if (sqlite3_prepare_v2(g_db, "SELECT content FROM notes WHERE id=?1;",
                           -1, &st, NULL) != SQLITE_OK) {
        return NULL;
    }
    sqlite3_bind_int(st, 1, id);
    if (sqlite3_step(st) == SQLITE_ROW) {
        const unsigned char *txt = sqlite3_column_text(st, 0);
        int len = sqlite3_column_bytes(st, 0);
        if (txt && len >= 0) {
            out = (char *)malloc((size_t)len + 1);
            if (out) {
                memcpy(out, txt, (size_t)len);
                out[len] = '\0';
                if (out_len) *out_len = len;
            }
        }
    }
    sqlite3_finalize(st);
    return out;
}

static int db_count_notes(void) {
    if (!g_db) return 0;
    sqlite3_stmt *st = NULL;
    int n = 0;
    if (sqlite3_prepare_v2(g_db, "SELECT COUNT(*) FROM notes;", -1, &st, NULL) != SQLITE_OK) return 0;
    if (sqlite3_step(st) == SQLITE_ROW) n = sqlite3_column_int(st, 0);
    sqlite3_finalize(st);
    return n;
}

/* -------------------------------------------------------------------------
 * Acesso uniforme: banco (caminho "db:<id>") ou arquivo real (fallback)
 * ------------------------------------------------------------------------- */
static char *note_read_content(const char *path, long *out_len) {
    int id = note_id_from_path(path);
    if (id >= 0) return db_get_content(id, out_len);
    return read_entire_file(path, out_len);
}

static bool note_write_content(const char *path, const char *data, long len) {
    int id = note_id_from_path(path);
    if (id >= 0) return db_update_content(id, data, time(NULL));
    return write_file_atomic(path, data, len);
}

/* Serializa o buffer de linhas do editor no formato markdown em disco.
 * Devolve NULL se o buffer de saida falhar, para que o chamador reporte erro
 * em vez de salvar silenciosamente um arquivo truncado. */
static char *lines_to_content(Line *lines, int num_lines, long *out_len) {
    if (out_len) *out_len = 0;
    ABuffer ab;
    ab_init(&ab);
    for (int i = 0; i < num_lines; i++) {
        if (lines[i].chars && lines[i].len > 0) ab_append(&ab, lines[i].chars, lines[i].len);
        ab_append(&ab, "\n", 1);
    }
    if (ab.failed) {
        ab_free(&ab);
        return NULL;
    }
    if (!ab.b) {
        ab.b = (char *)malloc(1);
        if (!ab.b) return NULL;
        ab.b[0] = '\0';
    }
    if (out_len) *out_len = ab.len;
    return ab.b; /* posse transferida para o chamador (nao usar ab_free aqui) */
}

/* Carrega 'content' (memoria) como linhas do editor ativo. Equivale ao antigo
 * loop de fgets(), mas sem depender de arquivo e sem limite de 16 KB por linha. */
static void editor_load_content(const char *content) {
    if (!content) return;
    const char *p = content;
    /* Pula UTF-8 BOM (\xEF\xBB\xBF) se presente no inicio */
    if ((unsigned char)p[0] == 0xEF && (unsigned char)p[1] == 0xBB && (unsigned char)p[2] == 0xBF) {
        p += 3;
    }
    while (*p) {
        const char *nl = strchr(p, '\n');
        int linelen = nl ? (int)(nl - p) : (int)strlen(p);
        while (linelen > 0 && p[linelen - 1] == '\r') linelen--;
        buf_insert_line(E.num_lines, p, linelen);
        if (!nl) break;
        p = nl + 1;
    }
}

/* -------------------------------------------------------------------------
 * Nomes e extensoes
 * ------------------------------------------------------------------------- */
static bool is_markdown_ext(const char *fname) {
    const char *ext = strrchr(fname, '.');
    if (!ext) return true; /* mesma regra do scan original: sem extensao conta */
    return strcasecmp(ext, ".md") == 0 || strcasecmp(ext, ".markdown") == 0 ||
           strcasecmp(ext, ".mdown") == 0 || strcasecmp(ext, ".txt") == 0 ||
           strcasecmp(ext, ".text") == 0;
}

/* Deixa um nome seguro para virar nome de arquivo. */
static void sanitize_note_name(const char *in, char *out, size_t out_sz) {
    size_t j = 0;
    if (!in) { out[0] = '\0'; return; }
    for (size_t i = 0; in[i] && j + 1 < out_sz; i++) {
        char c = in[i];
        if (c == '/' || c == '\\' || c == ':' || c == '*' || c == '?' ||
            c == '"' || c == '<' || c == '>' || c == '|') {
            c = '_';
        }
        out[j++] = c;
    }
    out[j] = '\0';
}

/* Expande "~" no inicio para o diretorio pessoal. */
static void expand_user_path(const char *in, char *out, size_t out_sz) {
    if (in && in[0] == '~' && (in[1] == '/' || in[1] == '\\' || in[1] == '\0')) {
        const char *home = getenv("HOME");
#ifdef _WIN32
        if (!home) home = getenv("USERPROFILE");
#endif
        if (home) {
            snprintf(out, out_sz, "%s%s", home, in + 1);
            return;
        }
    }
    safe_strcpy(out, out_sz, in ? in : "");
}

/* -------------------------------------------------------------------------
 * Importacao: pasta com arquivos .md  ->  banco
 * ------------------------------------------------------------------------- */
static char *join_path_alloc(const char *dir, const char *name) {
    if (!dir || !name) return NULL;
    size_t need = strlen(dir) + strlen(name) + 2;
    char *out = (char *)malloc(need);
    if (!out) return NULL;
    snprintf(out, need, "%s%c%s", dir, PATH_SEP, name);
    return out;
}

/* Extrai o nome logico (sem extensao) de um arquivo. Devolve false quando o
 * nome nao cabe no buffer, evitando que dois arquivos longos colidam por
 * truncamento silencioso. */
static bool note_name_from_filename(const char *fname, char *out, size_t out_sz) {
    if (!fname || !out || out_sz == 0) return false;
    const char *dot = strrchr(fname, '.');
    size_t len = dot ? (size_t)(dot - fname) : strlen(fname);
    if (len == 0 || len >= out_sz) return false;
    memcpy(out, fname, len);
    out[len] = '\0';
    return true;
}

static void import_one_note(const char *name, const char *content, time_t mtime,
                            int *added, int *updated) {
    int existing = db_find_id_by_name(name);
    if (existing > 0) {
        if (db_update_content(existing, content, mtime)) (*updated)++;
    } else if (db_insert_note(name, content, mtime, 0) > 0) {
        (*added)++;
    }
}

static int import_notes_from_dir(const char *dirpath, int *out_added, int *out_updated,
                                 char *errbuf, size_t errsz) {
    if (out_added) *out_added = 0;
    if (out_updated) *out_updated = 0;
    if (errbuf && errsz) errbuf[0] = '\0';

    if (!db_is_open()) {
        if (errbuf && errsz) snprintf(errbuf, errsz, "banco de dados indisponivel");
        return -1;
    }
    if (!dirpath || !*dirpath) {
        if (errbuf && errsz) snprintf(errbuf, errsz, "nenhuma pasta informada");
        return -1;
    }

    int added = 0, updated = 0;
    bool dir_found = false;

    if (!db_exec("BEGIN TRANSACTION;")) {
        if (errbuf && errsz) snprintf(errbuf, errsz, "nao foi possivel iniciar a transacao");
        return -1;
    }

#ifdef _WIN32
    WCHAR *wdir = utf8_to_wide_alloc(dirpath);
    size_t wlen = wdir ? wcslen(wdir) : 0;
    WCHAR *pattern = wdir ? (WCHAR *)malloc((wlen + 3) * sizeof(WCHAR)) : NULL;
    if (!wdir || !pattern) {
        free(wdir);
        free(pattern);
        db_exec("ROLLBACK;");
        if (errbuf && errsz) snprintf(errbuf, errsz, "caminho invalido ou sem memoria");
        return -1;
    }
    wcscpy(pattern, wdir);
    wcscat(pattern, L"\\*");
    free(wdir);

    WIN32_FIND_DATAW fd;
    HANDLE hFind = FindFirstFileW(pattern, &fd);
    free(pattern);
    if (hFind != INVALID_HANDLE_VALUE) {
        dir_found = true;
        do {
            if (fd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) continue;
            char fname[512];
            if (WideCharToMultiByte(CP_UTF8, 0, fd.cFileName, -1, fname, sizeof(fname), NULL, NULL) <= 0) continue;
            if (fname[0] == '.' || !is_markdown_ext(fname)) continue;

            char name[256];
            if (!note_name_from_filename(fname, name, sizeof(name))) continue;

            char *full = join_path_alloc(dirpath, fname);
            if (!full) continue;
            long clen = 0;
            char *content = read_entire_file(full, &clen);
            free(full);
            if (!content) continue;

            time_t mt = filetime_to_timet(&fd.ftLastWriteTime);
            if (mt <= 0) mt = time(NULL);
            import_one_note(name, content, mt, &added, &updated);
            free(content);
        } while (FindNextFileW(hFind, &fd));
        FindClose(hFind);
    }
#else
    DIR *d = opendir(dirpath);
    if (d) {
        dir_found = true;
        struct dirent *ent;
        while ((ent = readdir(d)) != NULL) {
            if (ent->d_name[0] == '.') continue;
            if (!is_markdown_ext(ent->d_name)) continue;

            char name[256];
            if (!note_name_from_filename(ent->d_name, name, sizeof(name))) continue;

            char *full = join_path_alloc(dirpath, ent->d_name);
            if (!full) continue;
            struct stat st;
            if (stat(full, &st) != 0 || S_ISDIR(st.st_mode)) { free(full); continue; }

            long clen = 0;
            char *content = read_entire_file(full, &clen);
            free(full);
            if (!content) continue;

            time_t mt = (st.st_mtime > 0) ? st.st_mtime : time(NULL);
            import_one_note(name, content, mt, &added, &updated);
            free(content);
        }
        closedir(d);
    }
#endif

    if (!dir_found) {
        db_exec("ROLLBACK;");
        if (errbuf && errsz) snprintf(errbuf, errsz, "pasta nao encontrada: %.200s", dirpath);
        return -1;
    }
    if (!db_exec("COMMIT;")) {
        db_exec("ROLLBACK;");
        if (errbuf && errsz) snprintf(errbuf, errsz, "falha ao confirmar a importacao");
        return -1;
    }
    if (out_added) *out_added = added;
    if (out_updated) *out_updated = updated;
    return added + updated;
}

/* -------------------------------------------------------------------------
 * Exportacao: banco  ->  pasta com arquivos .md
 * ------------------------------------------------------------------------- */
static int export_notes_to_dir(const char *dirpath, char *errbuf, size_t errsz) {
    if (errbuf && errsz) errbuf[0] = '\0';

    if (!db_is_open()) {
        if (errbuf && errsz) snprintf(errbuf, errsz, "banco de dados indisponivel");
        return -1;
    }
    if (!dirpath || !*dirpath) {
        if (errbuf && errsz) snprintf(errbuf, errsz, "nenhuma pasta informada");
        return -1;
    }

    /* Cria a pasta de destino se ainda nao existir (mkdir_utf8 devolve 0
     * tambem quando ja existe e e um diretorio). */
    if (mkdir_utf8(dirpath) != 0) {
        if (errbuf && errsz) snprintf(errbuf, errsz, "nao foi possivel criar a pasta: %.200s", dirpath);
        return -1;
    }

    sqlite3_stmt *st = NULL;
    if (sqlite3_prepare_v2(g_db, "SELECT name, content FROM notes ORDER BY mtime DESC;",
                           -1, &st, NULL) != SQLITE_OK) {
        if (errbuf && errsz) snprintf(errbuf, errsz, "falha ao ler o banco");
        return -1;
    }

    int written = 0;
    int failed = 0;
    while (sqlite3_step(st) == SQLITE_ROW) {
        const char *name = (const char *)sqlite3_column_text(st, 0);
        const char *content = (const char *)sqlite3_column_text(st, 1);
        int clen = sqlite3_column_bytes(st, 1);
        if (!name || !*name) { failed++; continue; }

        char safe[256];
        sanitize_note_name(name, safe, sizeof(safe));
        if (!safe[0]) { failed++; continue; }

        char fname[2048];
        int n = snprintf(fname, sizeof(fname), "%s%c%s.md", dirpath, PATH_SEP, safe);
        if (n < 0 || n >= (int)sizeof(fname)) { failed++; continue; }

        if (!write_file_atomic(fname, content ? content : "", (long)clen)) {
            failed++;
            continue;
        }
        written++;
    }
    sqlite3_finalize(st);

    if (failed > 0) {
        if (errbuf && errsz) {
            snprintf(errbuf, errsz, "%d nota(s) nao puderam ser exportadas", failed);
        }
        if (written == 0) return -1;
    }
    return written;
}


/* -------------------------------------------------------------------------
 * Pontos de entrada usados pela interface (botoes [Importar] / [Exportar])
 * ------------------------------------------------------------------------- */
static void ui_import_from_dir(const char *raw_dir) {
    char dir[1024];
    expand_user_path(raw_dir, dir, sizeof(dir));

    /* remove espacos e aspas das pontas (colar caminho costuma trazer aspas) */
    char *s = dir;
    while (*s == ' ' || *s == '\t') s++;
    size_t n = strlen(s);
    while (n > 0 && (s[n - 1] == ' ' || s[n - 1] == '\t')) s[--n] = '\0';
    if (n >= 2 && ((s[0] == '"' && s[n - 1] == '"') || (s[0] == '\'' && s[n - 1] == '\''))) {
        s[n - 1] = '\0';
        s++;
    }

    if (!*s) {
        set_status_msg("Importacao cancelada: nenhuma pasta informada.");
        return;
    }

    int added = 0, updated = 0;
    char err[256];
    int total = import_notes_from_dir(s, &added, &updated, err, sizeof(err));
    if (total < 0) {
        set_status_msg("Importacao falhou: %s", err);
        return;
    }

    rescan_notes();
    g_screen_clear_needed = 1;
    if (total == 0) {
        set_status_msg("Nenhum arquivo markdown encontrado em: %s", s);
    } else {
        set_status_msg("Importadas %d nota(s): %d nova(s), %d atualizada(s) de %s",
                       total, added, updated, s);
    }
}

static void ui_export_to_dir(const char *raw_dir) {
    char dir[1024];
    expand_user_path(raw_dir, dir, sizeof(dir));

    char *s = dir;
    while (*s == ' ' || *s == '\t') s++;
    size_t n = strlen(s);
    while (n > 0 && (s[n - 1] == ' ' || s[n - 1] == '\t')) s[--n] = '\0';
    if (n >= 2 && ((s[0] == '"' && s[n - 1] == '"') || (s[0] == '\'' && s[n - 1] == '\''))) {
        s[n - 1] = '\0';
        s++;
    }

    if (!*s) {
        set_status_msg("Exportacao cancelada: nenhuma pasta informada.");
        return;
    }

    if (db_count_notes() == 0) {
        set_status_msg("Nada para exportar: o banco esta vazio.");
        return;
    }

    char err[256];
    int written = export_notes_to_dir(s, err, sizeof(err));
    if (written < 0) {
        set_status_msg("Exportacao falhou: %s", err);
        return;
    }
    set_status_msg("Exportadas %d nota(s) como arquivos .md em %s", written, s);
}

static void note_update_cache(NoteInfo *n) {
    if (n->search_norm) {
        free(n->search_norm);
        n->search_norm = NULL;
    }
    n->search_norm_len = 0;

    long file_len = 0;
    char *raw = note_read_content(n->filepath, &file_len);

    char header[256] = "";
    if (raw && file_len > 0) {
        char *p = raw;
        if ((unsigned char)p[0] == 0xEF && (unsigned char)p[1] == 0xBB && (unsigned char)p[2] == 0xBF) {
            p += 3;
        }
        while (*p && *p != '\n' && *p != '\r' && (*p == ' ' || *p == '\t')) p++;
        if (*p == '#') {
            while (*p == '#') p++;
            while (*p == ' ' || *p == '\t') p++;
            int hidx = 0;
            while (*p && *p != '\n' && *p != '\r' && hidx < 250) {
                header[hidx++] = *p++;
            }
            header[hidx] = '\0';
        }
    }

    size_t total_raw_len = strlen(n->filename) + 1 + strlen(n->title) + 1 + strlen(header) + 1 + (file_len > 0 ? file_len : 0) + 32;
    char *combined_raw = (char *)malloc(total_raw_len);
    if (!combined_raw) {
        if (raw) free(raw);
        n->search_norm = strdup("");
        return;
    }

    snprintf(combined_raw, total_raw_len, "%s %s %s %s",
             n->filename, n->title, header, (raw && file_len > 0) ? raw : "");

    if (raw) free(raw);

    n->search_norm = (char *)malloc(total_raw_len + 1);
    if (n->search_norm) {
        normalize_text(combined_raw, n->search_norm, total_raw_len + 1);
        n->search_norm_len = (int)strlen(n->search_norm);
    } else {
        n->search_norm = strdup("");
        n->search_norm_len = 0;
    }
    free(combined_raw);
}

static const char *get_attention_color(int level) {
    bool dark = themes[E.current_theme].is_dark;
    switch (level) {
        case 1: /* Nível 1 - Vermelho (Crítico / Máxima Urgência) */
            return dark ? "\x1b[38;2;255;85;85m" : "\x1b[38;2;207;34;46m";
        case 2: /* Nível 2 - Laranja (Médio / Atenção Intermediária) */
            return dark ? "\x1b[38;2;255;158;100m" : "\x1b[38;2;217;119;6m";
        case 3: /* Nível 3 - Amarelo (Baixo / Acompanhamento) */
            return dark ? "\x1b[38;2;241;250;140m" : "\x1b[38;2;165;110;0m";
        case 4: /* Nível 4 - Azul (H) */
            return dark ? "\x1b[38;2;122;162;247m" : "\x1b[38;2;9;105;218m";
        case 5: /* Nível 5 - Roxo (R) */
            return dark ? "\x1b[38;2;187;154;247m" : "\x1b[38;2;130;80;223m";
        case 6: /* Nível 6 - Verde (O) */
            return dark ? "\x1b[38;2;80;250;123m" : "\x1b[38;2;26;127;55m";
        default:
            return TH_FG_MAIN;
    }
}

/* Os niveis de urgencia agora vivem na coluna 'attention' do banco e ja vem
 * carregados por rescan_notes(). Aqui apenas derivamos o atalho booleano
 * needs_attention, usado pelo realce da lista lateral. */
static void load_attention_flags(void) {
    for (int i = 0; i < E.num_notes; i++) {
        E.notes[i].needs_attention = (E.notes[i].attention_level > 0);
    }
}

/* Reordena a lista pela prioridade e mantem a selecao sobre a nota indicada,
 * para o cursor nao "pular" para outra nota quando a ordem muda. */
static void notes_resort_keep(const char *keep_path) {
    if (E.num_notes > 1) {
        qsort(E.notes, E.num_notes, sizeof(NoteInfo), compare_notes_priority);
    }
    apply_note_search(E.search_query);
    if (keep_path && *keep_path) {
        for (int k = 0; k < E.num_filtered; k++) {
            if (strcmp(E.notes[E.filtered_indices[k]].filepath, keep_path) == 0) {
                E.selected_note_idx = k;
                break;
            }
        }
        if (E.focus == FOCUS_LEFT && E.num_filtered > 0) {
            load_preview(E.notes[E.filtered_indices[E.selected_note_idx]].filepath);
        }
    }
}

static void toggle_note_attention_at(int actual_idx) {
    if (actual_idx < 0 || actual_idx >= E.num_notes) return;

    /* Guarda identidade e titulo ANTES de reordenar */
    char keep[512];
    char titulo[256];
    safe_strcpy(keep, sizeof(keep), E.notes[actual_idx].filepath);
    safe_strcpy(titulo, sizeof(titulo), E.notes[actual_idx].title);

    int nivel = (E.notes[actual_idx].attention_level + 1) % 7;

    /* Persiste no banco ANTES de alterar a UI. Se a gravacao falhar, o nivel
     * em memoria nao muda e a lista nao mente sobre o que esta salvo. */
    int id = note_id_from_path(E.notes[actual_idx].filepath);
    if (id > 0 && !db_set_attention(id, nivel)) {
        set_status_msg("Falha ao gravar a marcacao de urgencia no banco.");
        return;
    }

    E.notes[actual_idx].attention_level = nivel;
    E.notes[actual_idx].needs_attention = (nivel > 0);

    /* Reordena a lista por prioridade para agrupar as notas marcadas no topo */
    notes_resort_keep(keep);

    g_screen_clear_needed = 1;
    g_needs_redraw = 1;

    switch (nivel) {
        case 1:
            set_status_msg("🔴 Urgência Nível 1 [Crítica - Vermelho]: %s", titulo);
            break;
        case 2:
            set_status_msg("🟠 Urgência Nível 2 [Média - Laranja]: %s", titulo);
            break;
        case 3:
            set_status_msg("🟡 Urgência Nível 3 [Baixa - Amarelo]: %s", titulo);
            break;
        case 4:
            set_status_msg("🔵 Marcação [H - Azul]: %s", titulo);
            break;
        case 5:
            set_status_msg("🟣 Marcação [R - Roxo]: %s", titulo);
            break;
        case 6:
            set_status_msg("🟢 Marcação [O - Verde]: %s", titulo);
            break;
        default:
            set_status_msg("Marcação de urgência removida: %s", titulo);
            break;
    }
}

static void rescan_notes(void) {

    /* Preserve existing cached notes to avoid redundant disk I/O */
    NoteInfo *old_notes = E.notes;
    int old_num_notes = E.num_notes;

    E.notes = NULL;
    E.num_notes = 0;
    int cap = (old_num_notes > 16) ? old_num_notes + 16 : 16;
    E.notes = (NoteInfo *)malloc(sizeof(NoteInfo) * cap);
    if (!E.notes) {
        if (old_notes) {
            for (int i = 0; i < old_num_notes; i++) {
                if (old_notes[i].search_norm) free(old_notes[i].search_norm);
            }
            free(old_notes);
        }
        if (E.filtered_indices) {
            free(E.filtered_indices);
            E.filtered_indices = NULL;
        }
        E.num_filtered = 0;
        E.selected_note_idx = 0;
        free_preview();
        set_status_msg("Memoria insuficiente para recarregar as notas.");
        return;
    }

    time_t now = time(NULL);

    /* Varredura do banco: cada linha vira um NoteInfo com caminho virtual
     * "db:<id>". A ordenacao por data continua sendo feita logo abaixo. */
    {
        sqlite3_stmt *st = NULL;
        if (sqlite3_prepare_v2(g_db,
                "SELECT id, name, mtime, attention FROM notes ORDER BY mtime DESC;",
                -1, &st, NULL) == SQLITE_OK) {
            while (sqlite3_step(st) == SQLITE_ROW) {
                int nid = sqlite3_column_int(st, 0);
                const char *nm = (const char *)sqlite3_column_text(st, 1);
                time_t mt = (time_t)sqlite3_column_int64(st, 2);
                int att = sqlite3_column_int(st, 3);
                if (!nm || !*nm) continue;

                if (E.num_notes >= cap) {
                    int new_cap = cap * 2;
                    NoteInfo *new_notes = (NoteInfo *)realloc(E.notes, sizeof(NoteInfo) * new_cap);
                    if (!new_notes) break;
                    E.notes = new_notes;
                    cap = new_cap;
                }
                NoteInfo *n = &E.notes[E.num_notes++];
                memset(n, 0, sizeof(NoteInfo));
                snprintf(n->filename, sizeof(n->filename), "%s.md", nm);
                note_path_from_id(nid, n->filepath, sizeof(n->filepath));

                n->mtime = (mt > 0) ? mt : now;
                n->attention_level = (att >= 0 && att <= 6) ? att : 0;
                n->needs_attention = (n->attention_level > 0);
                format_rel_time(n->mtime, now, n->rel_time, sizeof(n->rel_time));

                /* reaproveita o cache de busca se a nota nao mudou de data */
                NoteInfo *found_old = NULL;
                for (int o = 0; o < old_num_notes; o++) {
                    if (old_notes[o].search_norm && strcmp(old_notes[o].filepath, n->filepath) == 0) {
                        found_old = &old_notes[o];
                        break;
                    }
                }

                if (found_old && found_old->mtime == n->mtime && found_old->search_norm) {
                    safe_strcpy(n->title, sizeof(n->title), found_old->title);
                    n->search_norm = found_old->search_norm;
                    n->search_norm_len = found_old->search_norm_len;
                    found_old->search_norm = NULL;
                } else {
                    extract_note_title(n->filepath, n->filename, n->title, sizeof(n->title));
                    n->search_norm = NULL;
                    note_update_cache(n);
                }
            }
            sqlite3_finalize(st);
        }
    }


    if (old_notes) {
        for (int i = 0; i < old_num_notes; i++) {
            if (old_notes[i].search_norm) free(old_notes[i].search_norm);
        }
        free(old_notes);
    }

    load_attention_flags();

    if (E.num_notes > 1) {
        qsort(E.notes, E.num_notes, sizeof(NoteInfo), compare_notes_priority);
    }

    if (E.filtered_indices) free(E.filtered_indices);
    E.filtered_indices = (int *)malloc(sizeof(int) * (E.num_notes > 0 ? E.num_notes : 1));
    if (!E.filtered_indices) {
        E.num_filtered = 0;
        E.selected_note_idx = 0;
        free_preview();
        set_status_msg("Memoria insuficiente para indexar as notas.");
        return;
    }

    apply_note_search(E.search_query);
}

static bool str_contains_caseless(const char *haystack, const char *needle) {
    if (!needle || !*needle) return true;
    if (!haystack) return false;
    size_t nlen = strlen(needle);
    size_t hlen = strlen(haystack);
    if (nlen > hlen) return false;
    for (size_t i = 0; i <= hlen - nlen; i++) {
#ifdef _WIN32
        if (_strnicmp(&haystack[i], needle, nlen) == 0) return true;
#else
        if (strncasecmp(&haystack[i], needle, nlen) == 0) return true;
#endif
    }
    return false;
}

static void apply_note_search(const char *query) {
    E.num_filtered = 0;

    char qnorm[512];
    char *words[32];
    int num_words = 0;
    if (query && *query) {
        normalize_text(query, qnorm, sizeof(qnorm));
        char *tok = strtok(qnorm, " \t\r\n,;\"'()[]{}!?");
        while (tok && num_words < 32) {
            if (*tok) words[num_words++] = tok;
            tok = strtok(NULL, " \t\r\n,;\"'()[]{}!?");
        }
    }

    for (int i = 0; i < E.num_notes; i++) {
        NoteInfo *n = &E.notes[i];

        /* Apply Urgency / Attention Filter */
        if (E.filter_urgency > 0 && n->attention_level != E.filter_urgency) {
            continue;
        }

        /* Apply search words */
        if (num_words > 0) {
            if (!n->search_norm) continue;
            bool all_words_match = true;
            for (int w = 0; w < num_words; w++) {
                if (strstr(n->search_norm, words[w]) == NULL) {
                    all_words_match = false;
                    break;
                }
            }
            if (!all_words_match) continue;
        }

        E.filtered_indices[E.num_filtered++] = i;
    }

    if (E.selected_note_idx < 0 || E.selected_note_idx >= E.num_filtered) {
        E.selected_note_idx = 0;
        E.sidebar_scroll_y = 0;
    }
    if (E.focus == FOCUS_LEFT) {
        if (E.num_filtered > 0) {
            load_preview(E.notes[E.filtered_indices[E.selected_note_idx]].filepath);
        } else {
            free_preview();
        }
    }
}

/* =========================================================================
 * Note Preview Management (for left panel navigation without creating tabs)
 * ========================================================================= */
typedef struct {
    char filepath[512];
    Line *lines;
    int num_lines;
    int cap_lines;
    int scroll_y;
} PreviewState;

static PreviewState g_preview;

static void free_preview(void) {
    if (g_preview.lines) {
        for (int i = 0; i < g_preview.num_lines; i++) {
            line_free(&g_preview.lines[i]);
        }
        free(g_preview.lines);
        g_preview.lines = NULL;
    }
    g_preview.num_lines = 0;
    g_preview.cap_lines = 0;
    g_preview.scroll_y = 0;
    g_preview.filepath[0] = '\0';
}

static void load_preview(const char *path) {
    if (!path || !*path) {
        free_preview();
        return;
    }
    if (strcmp(g_preview.filepath, path) == 0 && g_preview.lines != NULL) {
        return; /* Already loaded */
    }
    free_preview();
    safe_strcpy(g_preview.filepath, sizeof(g_preview.filepath), path);

    long clen = 0;
    char *content = note_read_content(path, &clen);
    if (!content) return;

    const char *p = content;
    if ((unsigned char)p[0] == 0xEF && (unsigned char)p[1] == 0xBB && (unsigned char)p[2] == 0xBF) {
        p += 3;
    }
    while (*p) {
        const char *nl = strchr(p, '\n');
        int linelen = nl ? (int)(nl - p) : (int)strlen(p);
        while (linelen > 0 && p[linelen - 1] == '\r') linelen--;
        if (g_preview.num_lines >= g_preview.cap_lines) {
            int new_cap = g_preview.cap_lines ? g_preview.cap_lines * 2 : 32;
            Line *new_lines = (Line *)realloc(g_preview.lines, sizeof(Line) * new_cap);
            if (!new_lines) break;
            g_preview.lines = new_lines;
            g_preview.cap_lines = new_cap;
        }
        Line *l = &g_preview.lines[g_preview.num_lines++];
        line_init(l, p, linelen);
        if (!nl) break;
        p = nl + 1;
    }
    free(content);

    if (g_preview.num_lines == 0) {
        g_preview.lines = (Line *)malloc(sizeof(Line));
        if (g_preview.lines) {
            g_preview.cap_lines = 1;
            Line *l = &g_preview.lines[g_preview.num_lines++];
            line_init(l, "", 0);
        }
    }
    update_lines_syntax(g_preview.lines, g_preview.num_lines);
}

/* =========================================================================
 * Multi-Tab Management
 * ========================================================================= */

static void tab_save_active(void) {
    if (E.num_tabs == 0 || E.active_tab < 0 || E.active_tab >= E.num_tabs) return;
    Tab *t = &E.tabs[E.active_tab];
    safe_strcpy(t->cur_filename, sizeof(t->cur_filename), E.cur_filename);
    safe_strcpy(t->cur_filepath, sizeof(t->cur_filepath), E.cur_filepath);
    t->lines = E.lines;
    t->num_lines = E.num_lines;
    t->cap_lines = E.cap_lines;
    t->modified = E.modified;
    t->first_unsaved_time = E.first_unsaved_time;
    t->cursor_y = E.cursor_y;
    t->cursor_x = E.cursor_x;
    t->scroll_y = E.scroll_y;
    t->scroll_x = E.scroll_x;
    t->visual_start_y = E.visual_start_y;
    t->visual_start_x = E.visual_start_x;
    t->has_selection = E.has_selection;
    memcpy(t->undo_stack, E.undo_stack, sizeof(E.undo_stack));
    t->undo_count = E.undo_count;
    memcpy(t->redo_stack, E.redo_stack, sizeof(E.redo_stack));
    t->redo_count = E.redo_count;
}

static void tab_load(int idx) {
    if (idx < 0 || idx >= E.num_tabs) return;
    invalidate_typing_undo();
    E.active_tab = idx;
    Tab *t = &E.tabs[idx];
    snprintf(E.cur_filename, sizeof(E.cur_filename), "%s", t->cur_filename);
    snprintf(E.cur_filepath, sizeof(E.cur_filepath), "%s", t->cur_filepath);
    E.lines = t->lines;
    E.num_lines = t->num_lines;
    E.cap_lines = t->cap_lines;
    E.modified = t->modified;
    E.first_unsaved_time = t->first_unsaved_time;
    E.cursor_y = t->cursor_y;
    E.cursor_x = t->cursor_x;
    E.scroll_y = t->scroll_y;
    E.scroll_x = t->scroll_x;
    E.visual_start_y = t->visual_start_y;
    E.visual_start_x = t->visual_start_x;
    E.has_selection = t->has_selection;
    memcpy(E.undo_stack, t->undo_stack, sizeof(t->undo_stack));
    E.undo_count = t->undo_count;
    memcpy(E.redo_stack, t->redo_stack, sizeof(t->redo_stack));
    E.redo_count = t->redo_count;
    g_screen_clear_needed = 1;
}

static void tab_switch(int idx) {
    if (idx < 0 || idx >= E.num_tabs || idx == E.active_tab) return;
    tab_save_active();
    tab_load(idx);
    for (int i = 0; i < E.num_filtered; i++) {
        if (strcmp(E.notes[E.filtered_indices[i]].filepath, E.cur_filepath) == 0) {
            E.selected_note_idx = i;
            break;
        }
    }
    set_status_msg("Aba: %s", E.cur_filename);
}

static void tab_next(void) {
    if (E.num_tabs <= 1) return;
    int next_idx = (E.active_tab + 1) % E.num_tabs;
    tab_switch(next_idx);
}

static void tab_prev(void) {
    if (E.num_tabs <= 1) return;
    int prev_idx = (E.active_tab - 1 + E.num_tabs) % E.num_tabs;
    tab_switch(prev_idx);
}

static void tab_close(int idx) {
    if (idx < 0 || idx >= E.num_tabs) return;

    tab_save_active();

    Tab *t = &E.tabs[idx];
    if (t->modified) {
        if (!save_tab_by_idx(idx)) {
            set_status_msg("Aba nao fechada: falha ao salvar as alteracoes");
            return;
        }
        t = &E.tabs[idx];
    }
    if (t->lines) {
        for (int i = 0; i < t->num_lines; i++) line_free(&t->lines[i]);
        free(t->lines);
        t->lines = NULL;
    }
    for (int i = 0; i < t->undo_count; i++) free_undo_state(&t->undo_stack[i]);
    for (int i = 0; i < t->redo_count; i++) free_undo_state(&t->redo_stack[i]);

    for (int i = idx; i < E.num_tabs - 1; i++) {
        E.tabs[i] = E.tabs[i + 1];
    }
    E.num_tabs--;

    if (E.num_tabs == 0) {
        E.active_tab = -1;
        E.lines = NULL;
        E.num_lines = 0;
        E.cap_lines = 0;
        E.cur_filepath[0] = '\0';
        E.cur_filename[0] = '\0';
        E.modified = false;
        E.first_unsaved_time = 0;
        E.cursor_y = 0;
        E.cursor_x = 0;
        E.scroll_y = 0;
        E.scroll_x = 0;
        E.visual_start_y = 0;
        E.visual_start_x = 0;
        E.undo_count = 0;
        E.redo_count = 0;
        set_status_msg("Última aba fechada. Clique em [➕ Nova Nota] para criar uma nova nota.");
    } else {
        if (idx == E.active_tab) {
            int new_active = idx > 0 ? idx - 1 : 0;
            tab_load(new_active);
            E.focus = FOCUS_RIGHT;
        } else if (idx < E.active_tab) {
            E.active_tab--;
            tab_load(E.active_tab);
        } else {
            tab_load(E.active_tab);
        }
        set_status_msg("Aba fechada");
    }
    g_screen_clear_needed = 1;
}

static bool close_all_tabs(void) {
    tab_save_active();

    /* Salva TODAS as abas antes de liberar qualquer uma. Se uma gravacao
     * falhar, nada e fechado e o usuario nao perde o que ainda nao foi salvo. */
    for (int i = 0; i < E.num_tabs; i++) {
        if (E.tabs[i].modified && !save_tab_by_idx(i)) {
            set_status_msg("Nao foi possivel salvar '%s'; abas mantidas abertas",
                           E.tabs[i].cur_filename);
            return false;
        }
    }

    for (int i = 0; i < E.num_tabs; i++) {
        Tab *t = &E.tabs[i];
        if (t->lines) {
            for (int l = 0; l < t->num_lines; l++) line_free(&t->lines[l]);
            free(t->lines);
            t->lines = NULL;
        }
        for (int u = 0; u < t->undo_count; u++) free_undo_state(&t->undo_stack[u]);
        for (int r = 0; r < t->redo_count; r++) free_undo_state(&t->redo_stack[r]);
    }
    E.num_tabs = 0;
    E.active_tab = -1;
    E.lines = NULL;
    E.num_lines = 0;
    E.cap_lines = 0;
    E.cur_filepath[0] = '\0';
    E.cur_filename[0] = '\0';
    E.modified = false;
    E.first_unsaved_time = 0;
    E.cursor_y = 0;
    E.cursor_x = 0;
    E.scroll_y = 0;
    E.scroll_x = 0;
    E.visual_start_y = 0;
    E.visual_start_x = 0;
    E.undo_count = 0;
    E.redo_count = 0;
    invalidate_typing_undo();
    g_screen_clear_needed = 1;
    set_status_msg("Todas as abas foram fechadas. Clique na aba [➕ Nova Nota] para começar.");
    return true;
}

/* Note History Management (Requirement 19) */
static void push_history(const char *path) {
    if (!path || !*path) return;
    if (E.history_count > 0 && strcmp(E.history[E.history_count - 1], path) == 0) {
        return;
    }
    if (E.history_count >= MAX_HISTORY) {
        memmove(&E.history[0], &E.history[1], sizeof(E.history[0]) * (MAX_HISTORY - 1));
        E.history_count = MAX_HISTORY - 1;
    }
    snprintf(E.history[E.history_count], sizeof(E.history[0]), "%s", path);
    E.history_idx = E.history_count;
    E.history_count++;
}

static void navigate_history(int steps) {
    if (E.history_count <= 0) return;
    E.history_idx -= steps;
    if (E.history_idx < 0) E.history_idx = 0;
    set_status_msg("Histórico: %s", E.history[E.history_idx]);
    open_note_by_path(E.history[E.history_idx], false);
}

/* =========================================================================
 * Loading & Saving Files
 * ========================================================================= */

static void open_note_by_path(const char *path, bool record_history) {
    if (!path || !*path) return;
    invalidate_typing_undo();

    for (int i = 0; i < E.num_tabs; i++) {
        if (strcmp(E.tabs[i].cur_filepath, path) == 0) {
            tab_switch(i);
            if (record_history) push_history(path);
            return;
        }
    }

    tab_save_active();

    if (E.num_tabs >= E.cap_tabs) {
        int new_cap = E.cap_tabs ? E.cap_tabs * 2 : 8;
        Tab *new_tabs = (Tab *)realloc(E.tabs, sizeof(Tab) * new_cap);
        if (!new_tabs) return;
        E.tabs = new_tabs;
        E.cap_tabs = new_cap;
    }
    int new_idx = E.num_tabs++;
    Tab *nt = &E.tabs[new_idx];
    memset(nt, 0, sizeof(Tab));

    safe_strcpy(nt->cur_filepath, sizeof(nt->cur_filepath), path);

    int pid_ = note_id_from_path(path);
    if (pid_ >= 0) {
        /* caminho virtual "db:<id>": o nome vem do banco, nao do caminho */
        char nm[256];
        if (db_get_name(pid_, nm, sizeof(nm))) {
            /* buffer intermediario maior: nome (ate 255) + ".md" nao cabe em
             * 256 bytes; o safe_strcpy abaixo trunca com seguranca se preciso. */
            char fn[300];
            snprintf(fn, sizeof(fn), "%s.md", nm);
            safe_strcpy(nt->cur_filename, sizeof(nt->cur_filename), fn);
        } else {
            safe_strcpy(nt->cur_filename, sizeof(nt->cur_filename), path);
        }
    } else {
        const char *slash = strrchr(path, PATH_SEP);
#ifndef _WIN32
        if (!slash) slash = strrchr(path, '/');
#endif
        safe_strcpy(nt->cur_filename, sizeof(nt->cur_filename), slash ? slash + 1 : path);
    }
    extract_note_title(path, nt->cur_filename, nt->title, sizeof(nt->title));

    E.active_tab = new_idx;
    E.lines = NULL;
    E.num_lines = 0;
    E.cap_lines = 0;
    E.cursor_y = 0;
    E.cursor_x = 0;
    E.scroll_y = 0;
    E.scroll_x = 0;
    E.visual_start_y = 0;
    E.visual_start_x = 0;
    E.has_selection = false;
    E.modified = false;
    E.first_unsaved_time = 0;
    E.undo_count = 0;
    E.redo_count = 0;
    safe_strcpy(E.cur_filepath, sizeof(E.cur_filepath), nt->cur_filepath);
    safe_strcpy(E.cur_filename, sizeof(E.cur_filename), nt->cur_filename);

    long clen = 0;
    char *content = note_read_content(path, &clen);
    if (content) {
        editor_load_content(content);
        free(content);
    }

    if (E.num_lines == 0) {
        buf_insert_line(0, "", 0);
    }
    E.modified = false;
    update_folding();
    tab_save_active();

    if (record_history) {
        push_history(path);
    }
    for (int i = 0; i < E.num_filtered; i++) {
        if (strcmp(E.notes[E.filtered_indices[i]].filepath, path) == 0) {
            E.selected_note_idx = i;
            break;
        }
    }
    g_screen_clear_needed = 1;
}

static bool save_current_note(void) {
    if (E.num_tabs == 0 || !E.cur_filepath[0]) {
        set_status_msg("Nenhuma nota aberta para salvar.");
        return false;
    }
    long clen = 0;
    char *content = lines_to_content(E.lines, E.num_lines, &clen);
    bool wrote_ok = content ? note_write_content(E.cur_filepath, content, clen) : false;
    if (content) free(content);
    if (!wrote_ok) {
        set_status_msg("Erro salvando nota: %s", E.cur_filename);
        return false;
    }
    E.modified = false;
    E.first_unsaved_time = 0;
    tab_save_active();

    /* Fast in-memory update for saved note: only update cache for this specific file */
    time_t now = time(NULL);
    bool found_in_notes = false;
    for (int i = 0; i < E.num_notes; i++) {
        if (strcmp(E.notes[i].filepath, E.cur_filepath) == 0) {
            /* o mtime foi gravado no banco pelo proprio note_write_content() */
            E.notes[i].mtime = now;
            format_rel_time(E.notes[i].mtime, now, E.notes[i].rel_time, sizeof(E.notes[i].rel_time));
            extract_note_title(E.cur_filepath, E.notes[i].filename, E.notes[i].title, sizeof(E.notes[i].title));
            note_update_cache(&E.notes[i]);
            found_in_notes = true;
            break;
        }
    }

    if (!found_in_notes) {
        rescan_notes();
    } else {
        notes_resort_keep(E.cur_filepath);
    }

    set_status_msg("\"%s\" %dL gravado", E.cur_filename, E.num_lines);
    return true;
}

static bool save_tab_by_idx(int t_idx) {
    if (t_idx < 0 || t_idx >= E.num_tabs) return false;
    if (t_idx == E.active_tab) {
        return save_current_note();
    }
    Tab *t = &E.tabs[t_idx];
    if (!t->cur_filepath[0]) return false;
    long clen = 0;
    char *content = lines_to_content(t->lines, t->num_lines, &clen);
    bool wrote_ok = content ? note_write_content(t->cur_filepath, content, clen) : false;
    if (content) free(content);
    if (!wrote_ok) return false;
    t->modified = false;
    t->first_unsaved_time = 0;

    /* Fast in-memory update for saved note: only update cache for this specific file */
    time_t now = time(NULL);
    for (int i = 0; i < E.num_notes; i++) {
        if (strcmp(E.notes[i].filepath, t->cur_filepath) == 0) {
            /* o mtime foi gravado no banco pelo proprio note_write_content() */
            E.notes[i].mtime = now;
            format_rel_time(E.notes[i].mtime, now, E.notes[i].rel_time, sizeof(E.notes[i].rel_time));
            extract_note_title(t->cur_filepath, E.notes[i].filename, E.notes[i].title, sizeof(E.notes[i].title));
            note_update_cache(&E.notes[i]);
            break;
        }
    }
    return true;
}

/* Ha alguma aba com alteracao nao salva? tab_save_active() garante que o
 * buffer ativo esteja refletido na estrutura Tab antes da checagem. */
static bool any_tab_modified(void) {
    tab_save_active();
    for (int i = 0; i < E.num_tabs; i++) {
        if (E.tabs[i].modified) return true;
    }
    return false;
}

/* Salva todas as abas modificadas. Devolve false no primeiro erro, sem
 * fechar nada; usado por :wq e por :q quando ha mais de uma aba. */
static bool save_all_modified_tabs(void) {
    tab_save_active();
    for (int i = 0; i < E.num_tabs; i++) {
        if (E.tabs[i].modified && !save_tab_by_idx(i)) return false;
    }
    return true;
}

static void check_autosave(void) {
    int64_t now = get_current_time_ms();

    /* Safety synchronization for active buffer */
    if (E.modified && E.first_unsaved_time == 0) {
        E.first_unsaved_time = now;
    }
    tab_save_active();

    int autosaved_count = 0;
    char last_saved_name[256] = {0};

    for (int t = 0; t < E.num_tabs; t++) {
        Tab *tab = &E.tabs[t];
        if (tab->modified && tab->first_unsaved_time > 0) {
            if (now - tab->first_unsaved_time >= AUTOSAVE_DELAY_MS) { /* 1 minute (60,000 ms) */
                if (save_tab_by_idx(t)) {
                    autosaved_count++;
                    safe_strcpy(last_saved_name, sizeof(last_saved_name), tab->cur_filename);
                }
            }
        }
    }

    if (autosaved_count > 0) {
        const char *keep_p = NULL;
        if (E.focus == FOCUS_LEFT && E.selected_note_idx >= 0 && E.selected_note_idx < E.num_filtered) {
            keep_p = E.notes[E.filtered_indices[E.selected_note_idx]].filepath;
        } else if (E.cur_filepath[0]) {
            keep_p = E.cur_filepath;
        }
        notes_resort_keep(keep_p);

        if (autosaved_count == 1) {
            set_status_msg("💾 Salvo automaticamente: %s", last_saved_name);
        } else {
            set_status_msg("💾 %d notas salvas automaticamente", autosaved_count);
        }
    }
}

static void open_or_create_wikilink_note(const char *note_name) {
    char clean_name[256];
    int j = 0;
    for (int i = 0; note_name[i] && j < 250; i++) {
        char c = note_name[i];
        if (c == '/' || c == '\\' || c == ':' || c == '*' || c == '?' || c == '"' || c == '<' || c == '>' || c == '|') {
            clean_name[j++] = '_';
        } else {
            clean_name[j++] = c;
        }
    }
    clean_name[j] = '\0';

    int nid = db_find_id_by_name(clean_name);
    if (nid < 0) {
        char body[512];
        snprintf(body, sizeof(body), "# %s\n\nNova nota criada via link.\n", note_name);
        nid = db_insert_note(clean_name, body, time(NULL), 0);
        if (nid < 0) {
            set_status_msg("Erro ao criar nota via link: %s", clean_name);
            return;
        }
        rescan_notes();
    }

    char target_path[512];
    note_path_from_id(nid, target_path, sizeof(target_path));
    open_note_by_path(target_path, true);
    set_status_msg("Nota aberta: [[%s]]", note_name);
}

/* =========================================================================
 * Note Creation and Renaming
 * ========================================================================= */

static void create_new_note(const char *raw_name) {
    if (!raw_name) return;
    char name[256];
    safe_strcpy(name, sizeof(name), raw_name);

    /* Trim spaces */
    char *start = name;
    while (*start == ' ' || *start == '\t') start++;
    char *end = start + strlen(start);
    while (end > start && (*(end - 1) == ' ' || *(end - 1) == '\t')) end--;
    *end = '\0';

    if (*start == '\0') {
        set_status_msg("Nome da nota não pode ser vazio.");
        return;
    }

    char title[256];
    char filename[512];
    safe_strcpy(title, sizeof(title), start);

    /* Check extension */
    const char *ext = strrchr(title, '.');
    if (ext && strcasecmp(ext, ".md") == 0) {
        safe_strcpy(filename, sizeof(filename), title);
        title[ext - title] = '\0'; /* title without .md */
    } else {
        snprintf(filename, sizeof(filename), "%s.md", title);
    }

    /* O nome logico da nota e sanitizado (sem / \ : * ? " < > |), pois ele
     * passa a ser a chave unica na tabela e o nome do arquivo na exportacao. */
    char safe_name[256];
    sanitize_note_name(title, safe_name, sizeof(safe_name));
    if (!safe_name[0]) {
        set_status_msg("Nome de nota invalido.");
        return;
    }

    int existing_id = db_find_id_by_name(safe_name);
    if (existing_id >= 0) {
        char existing_path[512];
        note_path_from_id(existing_id, existing_path, sizeof(existing_path));
        set_status_msg("Nota '%s' já existe! Abrindo...", filename);
        open_note_by_path(existing_path, true);
        E.focus = FOCUS_RIGHT;
        return;
    }

    char initial[512];
    snprintf(initial, sizeof(initial), "# %s\n\n", title);
    int new_id = db_insert_note(safe_name, initial, time(NULL), 0);
    if (new_id < 0) {
        set_status_msg("Erro ao criar nota: %s", safe_name);
        return;
    }

    char filepath[512];
    note_path_from_id(new_id, filepath, sizeof(filepath));

    rescan_notes();
    free_preview();
    open_note_by_path(filepath, true);
    E.focus = FOCUS_RIGHT;
    if (E.num_lines > 1) {
        E.cursor_y = E.num_lines - 1;
        E.cursor_x = 0;
    }
    set_status_msg("Nova nota criada: %s", filename);
}

static void replace_in_current_note(const char *find_str, const char *rep_str) {
    if (!find_str || !*find_str || !rep_str || E.num_lines <= 0) return;

    int find_len = (int)strlen(find_str);
    int rep_len = (int)strlen(rep_str);
    int count = 0;
    bool undo_saved = false;
    bool oom = false;

    for (int i = 0; i < E.num_lines; i++) {
        Line *l = &E.lines[i];
        if (!l->chars || l->len < find_len) continue;
        if (!strstr(l->chars, find_str)) continue;

        ABuffer nb;
        ab_init(&nb);
        int consumed = 0;
        int line_hits = 0;

        /* strstr encontra o proximo bloco de uma vez; antes o loop chamava
         * ab_append byte a byte, o que dominava o custo em notas grandes. */
        while (consumed <= l->len - find_len) {
            const char *hit = strstr(l->chars + consumed, find_str);
            if (!hit) break;
            int idx = (int)(hit - l->chars);
            if (idx > l->len - find_len) break;
            ab_append(&nb, l->chars + consumed, idx - consumed);
            ab_append(&nb, rep_str, rep_len);
            if (nb.failed) break;
            consumed = idx + find_len;
            line_hits++;
        }
        if (!nb.failed && consumed < l->len) {
            ab_append(&nb, l->chars + consumed, l->len - consumed);
        }
        if (nb.failed) {
            ab_free(&nb);
            oom = true;
            break;
        }

        if (line_hits > 0) {
            if (!undo_saved) {
                /* Snapshot antes da primeira linha alterada: assim um comando
                 * sem nenhuma ocorrencia nao consome undo nem apaga o redo. */
                save_undo();
                undo_saved = true;
            }
            line_free(l);
            line_init(l, nb.b ? nb.b : "", nb.len);
            mark_modified();
            count += line_hits;
        }
        ab_free(&nb);
    }

    if (count > 0) {
        if (E.cursor_y >= E.num_lines) E.cursor_y = E.num_lines - 1;
        if (E.cursor_y < 0) E.cursor_y = 0;
        if (E.cursor_x > E.lines[E.cursor_y].len) E.cursor_x = E.lines[E.cursor_y].len;
        tab_save_active();
        update_folding();
        g_screen_clear_needed = 1;
    }

    if (oom) {
        set_status_msg("Substituicao interrompida: memoria insuficiente (%d feitas)", count);
    } else if (count == 0) {
        set_status_msg("Nenhuma ocorrencia de '%s' encontrada", find_str);
    } else {
        set_status_msg("Substituição: %d ocorrência(s) de '%s' substituída(s)", count, find_str);
    }
}

static bool rename_note(const char *old_path, const char *raw_new_name) {
    if (!old_path || !*old_path || !raw_new_name) return false;

    char new_name[256];
    safe_strcpy(new_name, sizeof(new_name), raw_new_name);

    /* Trim spaces */
    char *start = new_name;
    while (*start == ' ' || *start == '\t') start++;
    char *end = start + strlen(start);
    while (end > start && (*(end - 1) == ' ' || *(end - 1) == '\t')) end--;
    *end = '\0';

    if (*start == '\0') {
        set_status_msg("Novo nome não pode ser vazio.");
        return false;
    }

    char new_title[256];
    char new_filename[512];
    safe_strcpy(new_title, sizeof(new_title), start);

    const char *ext = strrchr(new_title, '.');
    if (ext && strcasecmp(ext, ".md") == 0) {
        safe_strcpy(new_filename, sizeof(new_filename), new_title);
        new_title[ext - new_title] = '\0';
    } else {
        snprintf(new_filename, sizeof(new_filename), "%s.md", new_title);
    }

    char safe_new[256];
    sanitize_note_name(new_title, safe_new, sizeof(safe_new));
    if (!safe_new[0]) {
        set_status_msg("Novo nome invalido.");
        return false;
    }

    /* 1024 para caber DATA_DIR + separador + new_filename (ate 512 bytes) */
    char new_filepath[1024];
    int rid = note_id_from_path(old_path);

    if (rid < 0) {
        /* Fallback legado: nota aberta diretamente de um arquivo real
         * (ex.: passar um caminho .md na linha de comando). Renomeia no MESMO
         * diretorio; antes o arquivo era movido para data/, que desde o
         * backend SQLite pode nem existir. */
        const char *last_sep = strrchr(old_path, PATH_SEP);
        if (!last_sep) last_sep = strrchr(old_path, '/');
        if (!last_sep) last_sep = strrchr(old_path, '\\');
        if (last_sep) {
            int dir_len = (int)(last_sep - old_path);
            snprintf(new_filepath, sizeof(new_filepath), "%.*s%c%s",
                     dir_len, old_path, PATH_SEP, new_filename);
        } else {
            safe_strcpy(new_filepath, sizeof(new_filepath), new_filename);
        }

        if (strcmp(old_path, new_filepath) == 0) {
            set_status_msg("O nome da nota permanece o mesmo.");
            return true;
        }

        struct stat st;
        if (stat_cross(new_filepath, &st) == 0) {
            set_status_msg("Já existe uma nota com o nome: %s", new_filename);
            return false;
        }

        tab_save_active();

        if (rename_utf8(old_path, new_filepath) != 0) {
            set_status_msg("Erro ao renomear arquivo no disco.");
            return false;
        }
    } else {
        /* Nota no banco: o id (e portanto o caminho virtual) nao muda; apenas
         * a coluna 'name' e atualizada. */
        char current_name[256] = "";
        for (int i = 0; i < E.num_notes; i++) {
            if (strcmp(E.notes[i].filepath, old_path) == 0) {
                safe_strcpy(current_name, sizeof(current_name), E.notes[i].title);
                break;
            }
        }
        if (current_name[0] && strcmp(current_name, safe_new) == 0) {
            set_status_msg("O nome da nota permanece o mesmo.");
            return true;
        }

        int conflict = db_find_id_by_name(safe_new);
        if (conflict >= 0 && conflict != rid) {
            set_status_msg("Já existe uma nota com o nome: %s", new_filename);
            return false;
        }

        tab_save_active();

        if (!db_update_name(rid, safe_new)) {
            set_status_msg("Erro ao renomear a nota no banco.");
            return false;
        }

        note_path_from_id(rid, new_filepath, sizeof(new_filepath));
    }

    /* Update internal content: replace first line heading # Old Title with # New Title */
    bool is_open_in_tabs = false;
    for (int t = 0; t < E.num_tabs; t++) {
        if (strcmp(E.tabs[t].cur_filepath, old_path) == 0) {
            is_open_in_tabs = true;
            safe_strcpy(E.tabs[t].cur_filepath, sizeof(E.tabs[t].cur_filepath), new_filepath);
            safe_strcpy(E.tabs[t].cur_filename, sizeof(E.tabs[t].cur_filename), new_filename);
            safe_strcpy(E.tabs[t].title, sizeof(E.tabs[t].title), new_title);

            if (E.tabs[t].num_lines > 0 && strncmp(E.tabs[t].lines[0].chars, "# ", 2) == 0) {
                char hbuf[300];
                snprintf(hbuf, sizeof(hbuf), "# %s", new_title);
                line_free(&E.tabs[t].lines[0]);
                line_init(&E.tabs[t].lines[0], hbuf, (int)strlen(hbuf));
            }
        }
    }

    if (strcmp(E.cur_filepath, old_path) == 0) {
        safe_strcpy(E.cur_filepath, sizeof(E.cur_filepath), new_filepath);
        safe_strcpy(E.cur_filename, sizeof(E.cur_filename), new_filename);
        if (E.num_lines > 0 && strncmp(E.lines[0].chars, "# ", 2) == 0) {
            char hbuf[300];
            snprintf(hbuf, sizeof(hbuf), "# %s", new_title);
            line_free(&E.lines[0]);
            line_init(&E.lines[0], hbuf, (int)strlen(hbuf));
        }
        save_current_note();
    } else if (is_open_in_tabs) {
        for (int t = 0; t < E.num_tabs; t++) {
            if (strcmp(E.tabs[t].cur_filepath, new_filepath) == 0) {
                save_tab_by_idx(t);
                break;
            }
        }
    } else {
        /* Not open in any tab: read content, update line 0 heading, write back */
        long clen = 0;
        char *content = note_read_content(new_filepath, &clen);
        if (content) {
            const char *p = content;
            if ((unsigned char)p[0] == 0xEF && (unsigned char)p[1] == 0xBB && (unsigned char)p[2] == 0xBF) {
                p += 3;
            }
            if (strncmp(p, "# ", 2) == 0) {
                const char *nl = strchr(p, '\n');
                const char *rest = nl ? nl : "";
                char hbuf[300];
                snprintf(hbuf, sizeof(hbuf), "# %s", new_title);
                ABuffer full_content;
                ab_init(&full_content);
                ab_append(&full_content, hbuf, (int)strlen(hbuf));
                ab_append(&full_content, rest, (int)strlen(rest));
                note_write_content(new_filepath, full_content.b, full_content.len);
                ab_free(&full_content);
            }
            free(content);
        }
    }

    if (strcmp(g_preview.filepath, old_path) == 0) {
        free_preview();
        load_preview(new_filepath);
    }

    rescan_notes();

    /* Find and select new note in sidebar */
    for (int i = 0; i < E.num_filtered; i++) {
        int actual = E.filtered_indices[i];
        if (strcmp(E.notes[actual].filepath, new_filepath) == 0) {
            E.selected_note_idx = i;
            break;
        }
    }

    set_status_msg("Nota renomeada com sucesso: %s", new_filename);
    return true;
}

/* =========================================================================
 * Links Finder ([title](link) and [[wikilink]])
 * ========================================================================= */

typedef struct {
    int start_raw;
    int end_raw;
    char text[512];
    char target[4096];
    bool is_wikilink;
} LinkSpan;

static inline bool should_hide_links(bool in_preview) {
    if (in_preview) return true;
    return (E.mode != MODE_INSERT);
}

/* =========================================================================
 * Markdown Bold / Italic Styles
 * ========================================================================= */
typedef enum {
    STYLE_SPAN_NONE = 0,
    STYLE_SPAN_BOLD = 1,
    STYLE_SPAN_ITALIC = 2,
    STYLE_SPAN_BOLD_ITALIC = 3
} MarkdownStyleType;

typedef struct {
    int open_start;
    int open_len;
    int content_start;
    int content_end;
    int close_start;
    int close_len;
    MarkdownStyleType type;
} MarkdownStyleSpan;

static inline bool should_hide_markdown_styles(bool in_preview) {
    if (in_preview) return true;
    return (E.mode != MODE_INSERT);
}

static int find_markdown_styles(const Line *l, MarkdownStyleSpan *spans, int max_spans) {
    if (!l || !l->chars || l->len <= 0) return 0;
    int count = 0;
    int len = l->len;
    const char *s = l->chars;
    int i = 0;

    while (i < len && count < max_spans) {
        if (s[i] == '`') {
            i++;
            while (i < len && s[i] != '`') i++;
            if (i < len && s[i] == '`') i++;
            continue;
        }
        if (s[i] == '*') {
            int dlen = 0;
            while (i + dlen < len && s[i + dlen] == '*') dlen++;

            if (dlen >= 1 && dlen <= 3) {
                int next_idx = i + dlen;
                if (next_idx < len && s[next_idx] != ' ' && s[next_idx] != '\t' && s[next_idx] != '\r' && s[next_idx] != '\n') {
                    int j = next_idx;
                    bool found = false;
                    while (j < len) {
                        if (s[j] == '*') {
                            int cdlen = 0;
                            while (j + cdlen < len && s[j + cdlen] == '*') cdlen++;
                            if (cdlen == dlen) {
                                if (j > 0 && s[j - 1] != ' ' && s[j - 1] != '\t') {
                                    MarkdownStyleSpan *sp = &spans[count++];
                                    sp->open_start = i;
                                    sp->open_len = dlen;
                                    sp->content_start = i + dlen;
                                    sp->content_end = j;
                                    sp->close_start = j;
                                    sp->close_len = dlen;
                                    if (dlen == 3) sp->type = STYLE_SPAN_BOLD_ITALIC;
                                    else if (dlen == 2) sp->type = STYLE_SPAN_BOLD;
                                    else sp->type = STYLE_SPAN_ITALIC;
                                    i = j + cdlen;
                                    found = true;
                                    break;
                                }
                            }
                            j += cdlen;
                        } else {
                            j++;
                        }
                    }
                    if (found) continue;
                }
            }
            i += dlen;
        } else {
            i++;
        }
    }
    return count;
}

static int find_links_in_line(const Line *l, LinkSpan *spans, int max_spans) {
    if (!l->chars || l->len <= 0) return 0;
    int count = 0;
    const char *s = l->chars;
    int len = l->len;

    for (int i = 0; i < len && count < max_spans; i++) {
        /* 1. WikiLinks [[Nota]] */
        if (s[i] == '[' && i + 1 < len && s[i + 1] == '[') {
            int start = i;
            i += 2;
            int txt_start = i;
            while (i + 1 < len && !(s[i] == ']' && s[i + 1] == ']')) i++;
            if (i + 1 < len && s[i] == ']' && s[i + 1] == ']') {
                int tlen = i - txt_start;
                if (tlen > 0 && tlen < 500) {
                    LinkSpan *ls = &spans[count++];
                    ls->start_raw = start;
                    ls->end_raw = i + 2;
                    ls->is_wikilink = true;
                    memcpy(ls->text, &s[txt_start], tlen);
                    ls->text[tlen] = '\0';
                    snprintf(ls->target, sizeof(ls->target), "%s", ls->text);
                }
                i += 1;
                continue;
            }
        }

        /* 2. Standard and Extended Markdown Links [Title](URL) */
        if (s[i] == '[') {
            int start = i;
            i++;
            int txt_start = i;
            while (i < len && s[i] != ']') i++;
            if (i < len && s[i] == ']' && i + 1 < len && s[i + 1] == '(') {
                int txt_end = i;
                i += 2;
                int link_start = i;
                int paren_depth = 1;
                while (i < len && paren_depth > 0) {
                    if (s[i] == '(') paren_depth++;
                    else if (s[i] == ')') paren_depth--;
                    if (paren_depth > 0) i++;
                }
                if (i < len && s[i] == ')' && paren_depth == 0) {
                    int tlen = txt_end - txt_start;
                    int raw_llen = i - link_start;
                    if (tlen > 0 && tlen < 500 && raw_llen > 0 && raw_llen < 4000) {
                        /* Check for angle brackets <...> */
                        int l_start = link_start;
                        int l_end = i;
                        if (s[l_start] == '<' && l_end > l_start + 1 && s[l_end - 1] == '>') {
                            l_start++;
                            l_end--;
                        }
                        /* Trim optional title attribute after URL e.g. "title" or 'title' */
                        int llen = l_end - l_start;
                        for (int k = 0; k < llen; k++) {
                            if (s[l_start + k] == ' ' || s[l_start + k] == '\t') {
                                int next_k = k + 1;
                                while (next_k < llen && (s[l_start + next_k] == ' ' || s[l_start + next_k] == '\t')) next_k++;
                                if (next_k < llen && (s[l_start + next_k] == '"' || s[l_start + next_k] == '\'')) {
                                    llen = k;
                                    break;
                                }
                            }
                        }
                        if (llen > 0 && llen < 4000) {
                            LinkSpan *ls = &spans[count++];
                            ls->start_raw = start;
                            ls->end_raw = i + 1;
                            ls->is_wikilink = false;
                            memcpy(ls->text, &s[txt_start], tlen);
                            ls->text[tlen] = '\0';
                            memcpy(ls->target, &s[l_start], llen);
                            ls->target[llen] = '\0';
                        }
                    }
                    continue;
                }
            }
        }
    }
    return count;
}

static void activate_link_at_cursor(void) {
    if (E.cursor_y < 0 || E.cursor_y >= E.num_lines) return;
    Line *l = &E.lines[E.cursor_y];
    LinkSpan spans[16];
    int n = find_links_in_line(l, spans, 16);
    for (int i = 0; i < n; i++) {
        if (E.cursor_x >= spans[i].start_raw && E.cursor_x <= spans[i].end_raw) {
            if (spans[i].is_wikilink) {
                open_or_create_wikilink_note(spans[i].target);
            } else {
                open_link_target(spans[i].target);
            }
            return;
        }
    }
    set_status_msg("Nenhum link sob o cursor (Enter em [título](url) ou [[nota]])");
}


static void yank_visual_selection(bool linewise) {
    if (E.num_lines <= 0) return;
    int start_y = E.visual_start_y, end_y = E.cursor_y;
    int start_x = E.visual_start_x, end_x = E.cursor_x;
    if (start_y > end_y || (start_y == end_y && start_x > end_x)) {
        int ty = start_y; start_y = end_y; end_y = ty;
        int tx = start_x; start_x = end_x; end_x = tx;
    }

    if (start_y < 0) start_y = 0;
    if (end_y >= E.num_lines) end_y = E.num_lines - 1;
    if (start_x < 0) start_x = 0;
    if (end_x < 0) end_x = 0;

    if (!linewise) {
        if (start_y < E.num_lines && E.lines[start_y].len > 0) {
            start_x = utf8_align_start(E.lines[start_y].chars, E.lines[start_y].len, start_x);
        }
        if (end_y < E.num_lines && E.lines[end_y].len > 0) {
            end_x = utf8_align_end(E.lines[end_y].chars, E.lines[end_y].len, end_x);
        }
    }

    ABuffer yb;
    ab_init(&yb);

    if (linewise) {
        for (int i = start_y; i <= end_y; i++) {
            ab_append(&yb, E.lines[i].chars, E.lines[i].len);
            ab_append(&yb, "\n", 1);
        }
        E.yank_is_linewise = true;
    } else {
        if (start_y == end_y) {
            int len = end_x - start_x + 1;
            if (start_x < E.lines[start_y].len) {
                if (start_x + len > E.lines[start_y].len) len = E.lines[start_y].len - start_x;
                if (len > 0) ab_append(&yb, &E.lines[start_y].chars[start_x], len);
            }
        } else {
            /* First line */
            if (start_x < E.lines[start_y].len) {
                ab_append(&yb, &E.lines[start_y].chars[start_x], E.lines[start_y].len - start_x);
            }
            ab_append(&yb, "\n", 1);
            /* Intermediate lines */
            for (int i = start_y + 1; i < end_y; i++) {
                ab_append(&yb, E.lines[i].chars, E.lines[i].len);
                ab_append(&yb, "\n", 1);
            }
            /* Last line */
            int last_len = end_x + 1;
            if (last_len > E.lines[end_y].len) last_len = E.lines[end_y].len;
            if (last_len > 0) {
                ab_append(&yb, E.lines[end_y].chars, last_len);
            }
        }
        E.yank_is_linewise = false;
    }

    if (E.yank_buf) free(E.yank_buf);
    E.yank_buf = yb.b;
    copy_to_system_clipboard(E.yank_buf, yb.len);
}

static void delete_visual_selection_ex(bool do_yank) {
    if (E.num_lines <= 0) return;
    bool linewise = (E.mode == MODE_VISUAL_LINE);

    /* 1. Yank selection first only if requested (e.g. cut/delete, but not paste-replace) */
    if (do_yank) {
        yank_visual_selection(linewise);
    }

    /* 2. Save undo snapshot */
    save_undo();

    int start_y = E.visual_start_y, end_y = E.cursor_y;
    int start_x = E.visual_start_x, end_x = E.cursor_x;
    if (start_y > end_y || (start_y == end_y && start_x > end_x)) {
        int ty = start_y; start_y = end_y; end_y = ty;
        int tx = start_x; start_x = end_x; end_x = tx;
    }

    if (start_y < 0) start_y = 0;
    if (end_y >= E.num_lines) end_y = E.num_lines - 1;
    if (start_x < 0) start_x = 0;
    if (end_x < 0) end_x = 0;

    if (!linewise) {
        if (start_y < E.num_lines && E.lines[start_y].len > 0) {
            start_x = utf8_align_start(E.lines[start_y].chars, E.lines[start_y].len, start_x);
        }
        if (end_y < E.num_lines && E.lines[end_y].len > 0) {
            end_x = utf8_align_end(E.lines[end_y].chars, E.lines[end_y].len, end_x);
        }
    }

    if (linewise) {
        for (int i = end_y; i >= start_y; i--) {
            buf_delete_line(i);
        }
        E.cursor_y = start_y;
        if (E.cursor_y >= E.num_lines) E.cursor_y = E.num_lines - 1;
        if (E.cursor_y < 0) E.cursor_y = 0;
        E.cursor_x = 0;
        char msg[64];
        snprintf(msg, sizeof(msg), "%d linha(s) excluída(s)", end_y - start_y + 1);
        set_status_msg(msg);
    } else {
        if (start_y == end_y) {
            Line *line = &E.lines[start_y];
            if (start_x < line->len) {
                int del_len = end_x - start_x + 1;
                if (start_x + del_len > line->len) del_len = line->len - start_x;
                if (del_len > 0) {
                    memmove(&line->chars[start_x], &line->chars[start_x + del_len], line->len - (start_x + del_len) + 1);
                    line->len -= del_len;
                    detect_heading(line);
                    mark_modified();
                }
            }
            E.cursor_y = start_y;
            E.cursor_x = start_x;
            if (E.mode == MODE_INSERT) {
                if (E.cursor_x > line->len) E.cursor_x = line->len;
            } else {
                if (E.cursor_x >= line->len && line->len > 0) E.cursor_x = line->len - 1;
            }
            if (E.cursor_x < 0) E.cursor_x = 0;
            set_status_msg("Texto selecionado excluído");
        } else {
            Line *first = &E.lines[start_y];
            Line *last = &E.lines[end_y];

            int first_keep = (start_x < first->len) ? start_x : first->len;
            int last_skip = (end_x + 1 < last->len) ? (end_x + 1) : last->len;
            int suffix_len = last->len - last_skip;
            if (suffix_len < 0) suffix_len = 0;

            char *new_chars = (char *)malloc(first_keep + suffix_len + 1);
            if (new_chars) {
                if (first_keep > 0) memcpy(new_chars, first->chars, first_keep);
                if (suffix_len > 0) memcpy(new_chars + first_keep, &last->chars[last_skip], suffix_len);
                new_chars[first_keep + suffix_len] = '\0';

                free(first->chars);
                first->chars = new_chars;
                first->len = first_keep + suffix_len;
                detect_heading(first);
                mark_modified();
            }

            for (int i = end_y; i >= start_y + 1; i--) {
                buf_delete_line(i);
            }

            E.cursor_y = start_y;
            E.cursor_x = first_keep;
            Line *cur_line = &E.lines[start_y];
            if (E.mode == MODE_INSERT) {
                if (E.cursor_x > cur_line->len) E.cursor_x = cur_line->len;
            } else {
                if (E.cursor_x >= cur_line->len && cur_line->len > 0) E.cursor_x = cur_line->len - 1;
            }
            if (E.cursor_x < 0) E.cursor_x = 0;
            set_status_msg("Texto selecionado excluído");
        }
    }

    update_folding();
    tab_save_active();
    g_screen_clear_needed = 1;
    if (E.mode != MODE_INSERT) {
        E.mode = MODE_NORMAL;
    }
}

static void delete_visual_selection(void) {
    delete_visual_selection_ex(true);
}

/* Pastes text from clipboard/yank buffer directly replacing the active selection */
static void paste_replace_selection(void) {
    /* 1. Try to read fresh content from system clipboard */
    int clip_len = 0;
    char *clip_text = get_system_clipboard(&clip_len);
    const char *source_text = NULL;

    if (clip_text && clip_len > 0) {
        source_text = clip_text;
    } else if (E.yank_buf && *E.yank_buf) {
        source_text = E.yank_buf;
    }

    if (!source_text || !*source_text) {
        if (clip_text) free(clip_text);
        set_status_msg("Área de transferência vazia para colar.");
        return;
    }

    /* Keep a dedicated copy to paste so buffer modifications are safe */
    char *paste_copy = strdup(source_text);
    if (clip_text) free(clip_text);

    if (!paste_copy) {
        set_status_msg("Erro de memória ao colar.");
        return;
    }

    /* Keep E.yank_buf synchronized */
    if (!E.yank_buf || strcmp(E.yank_buf, paste_copy) != 0) {
        if (E.yank_buf) free(E.yank_buf);
        E.yank_buf = strdup(paste_copy);
        E.yank_is_linewise = (strchr(paste_copy, '\n') != NULL);
    }

    /* 2. Delete selection without overwriting clipboard/yank buffer */
    delete_visual_selection_ex(false);
    E.mode = MODE_NORMAL;
    E.has_selection = false;

    /* 3. Paste at cursor position (where deleted selection was) */
    editor_paste_string(paste_copy, (int)strlen(paste_copy));
    free(paste_copy);

    editor_scroll();
    set_status_msg("Texto colado sobre a seleção com 'p'.");
}

static void jump_to_next_link(void) {
    if (E.num_lines == 0) return;
    E.focus = FOCUS_RIGHT;
    E.mode = MODE_NORMAL;
    int cur_y = E.cursor_y;
    int cur_x = E.cursor_x;

    LinkSpan spans[16];

    /* 1. Check remainder of current line after cur_x */
    int n = find_links_in_line(&E.lines[cur_y], spans, 16);
    for (int i = 0; i < n; i++) {
        if (spans[i].start_raw > cur_x) {
            E.cursor_y = cur_y;
            E.cursor_x = spans[i].start_raw;
            editor_scroll();
            if (spans[i].is_wikilink) {
                set_status_msg("WikiLink: [[%s]] (Enter para abrir)", spans[i].target);
            } else {
                set_status_msg("Link: [%s](%s) (Enter para abrir)", spans[i].text, spans[i].target);
            }
            return;
        }
    }

    /* 2. Check subsequent lines cur_y + 1 .. num_lines - 1 */
    for (int y = cur_y + 1; y < E.num_lines; y++) {
        n = find_links_in_line(&E.lines[y], spans, 16);
        if (n > 0) {
            E.cursor_y = y;
            E.cursor_x = spans[0].start_raw;
            editor_scroll();
            if (spans[0].is_wikilink) {
                set_status_msg("WikiLink: [[%s]] (Enter para abrir)", spans[0].target);
            } else {
                set_status_msg("Link: [%s](%s) (Enter para abrir)", spans[0].text, spans[0].target);
            }
            return;
        }
    }

    /* 3. Wrap around: check lines 0 .. cur_y - 1 */
    for (int y = 0; y < cur_y; y++) {
        n = find_links_in_line(&E.lines[y], spans, 16);
        if (n > 0) {
            E.cursor_y = y;
            E.cursor_x = spans[0].start_raw;
            editor_scroll();
            if (spans[0].is_wikilink) {
                set_status_msg("WikiLink: [[%s]] (Enter para abrir)", spans[0].target);
            } else {
                set_status_msg("Link: [%s](%s) (Enter para abrir)", spans[0].text, spans[0].target);
            }
            return;
        }
    }

    /* 4. Check current line up to cur_x */
    n = find_links_in_line(&E.lines[cur_y], spans, 16);
    if (n > 0 && spans[0].start_raw != cur_x) {
        E.cursor_y = cur_y;
        E.cursor_x = spans[0].start_raw;
        editor_scroll();
        if (spans[0].is_wikilink) {
            set_status_msg("WikiLink: [[%s]] (Enter para abrir)", spans[0].target);
        } else {
            set_status_msg("Link: [%s](%s) (Enter para abrir)", spans[0].text, spans[0].target);
        }
        return;
    }

    set_status_msg("Nenhum link encontrado nesta nota");
}



/* =========================================================================
 * Vimwiki-style Markdown Table Auto-Alignment
 * ========================================================================= */

enum ColAlign {
    ALIGN_LEFT = 0,
    ALIGN_CENTER,
    ALIGN_RIGHT
};

static bool is_table_line(const Line *l) {
    if (!l || l->len == 0 || !l->chars) return false;
    const char *p = l->chars;
    while (*p == ' ' || *p == '\t') p++;
    if (*p != '|') return false;
    return (strchr(p + 1, '|') != NULL);
}

static bool is_delimiter_cell(const char *s) {
    if (!s || !*s) return false;
    bool has_dash = false;
    while (*s) {
        if (*s == '-') has_dash = true;
        else if (*s != ':' && *s != ' ' && *s != '\t') return false;
        s++;
    }
    return has_dash;
}

static char *table_trim_copy(const char *src, int len) {
    if (!src || len <= 0) return strdup("");
    const char *start = src;
    const char *end = src + len;
    while (start < end && (*start == ' ' || *start == '\t')) start++;
    while (end > start && (*(end - 1) == ' ' || *(end - 1) == '\t')) end--;
    int clen = (int)(end - start);
    char *dst = (char *)malloc((size_t)clen + 1);
    if (!dst) return NULL;
    if (clen > 0) memcpy(dst, start, (size_t)clen);
    dst[clen] = '\0';
    return dst;
}

#define MAX_TABLE_COLS 32
#define MAX_TABLE_ROWS 128

typedef struct {
    char *cells[MAX_TABLE_COLS];
    int num_cells;
    bool is_delim;
    int indent;
} ParsedTableRow;

static void align_table_lines(Line *lines, int start_y, int end_y) {
    int num_rows = end_y - start_y + 1;
    if (num_rows < 2) return;

    ParsedTableRow rows[MAX_TABLE_ROWS];
    if (num_rows > MAX_TABLE_ROWS) num_rows = MAX_TABLE_ROWS;

    int max_cols = 0;
    int col_widths[MAX_TABLE_COLS] = {0};
    enum ColAlign col_aligns[MAX_TABLE_COLS];
    for (int c = 0; c < MAX_TABLE_COLS; c++) col_aligns[c] = ALIGN_LEFT;
    int base_indent = -1;
    int parsed_rows = 0;

    for (int r = 0; r < num_rows; r++) {
        Line *l = &lines[start_y + r];
        const char *p = l->chars ? l->chars : "";
        int indent = 0;
        while (*p == ' ' || *p == '\t') {
            indent += (*p == '\t') ? 4 : 1;
            p++;
        }
        if (base_indent == -1) base_indent = indent;
        rows[r].indent = indent;
        rows[r].num_cells = 0;
        rows[r].is_delim = false;
        parsed_rows = r + 1;

        if (*p == '|') p++;
        bool all_delim = true;
        bool truncated = false;

        while (*p && rows[r].num_cells < MAX_TABLE_COLS) {
            const char *next_bar = strchr(p, '|');
            int clen = next_bar ? (int)(next_bar - p) : (int)strlen(p);
            char *trimmed = table_trim_copy(p, clen);
            if (!trimmed) {
                for (int rr = 0; rr < parsed_rows; rr++) {
                    for (int cc = 0; cc < rows[rr].num_cells; cc++) free(rows[rr].cells[cc]);
                }
                return; /* memoria insuficiente: nao altera nenhuma linha */
            }
            int cell_idx = rows[r].num_cells++;
            rows[r].cells[cell_idx] = trimmed;

            if (!is_delimiter_cell(trimmed)) {
                all_delim = false;
            }

            if (!next_bar) { p = NULL; break; }
            p = next_bar + 1;
            const char *check = p;
            while (*check == ' ' || *check == '\t') check++;
            if (!*check) { p = NULL; break; }
        }

        /* Se ainda sobrou conteudo depois da 32a coluna, o alinhador nao
         * consegue representar a linha: aborta sem perder as colunas extras. */
        if (p) {
            const char *rest = p;
            while (*rest == ' ' || *rest == '\t') rest++;
            if (*rest) truncated = true;
        }
        if (truncated) {
            for (int rr = 0; rr < parsed_rows; rr++) {
                for (int cc = 0; cc < rows[rr].num_cells; cc++) free(rows[rr].cells[cc]);
            }
            return;
        }

        if (rows[r].num_cells > 0 && all_delim) {
            rows[r].is_delim = true;
        }

        if (rows[r].num_cells > max_cols) {
            max_cols = rows[r].num_cells;
        }
    }

    if (max_cols == 0) return;

    for (int r = 0; r < num_rows; r++) {
        if (rows[r].is_delim) {
            for (int c = 0; c < rows[r].num_cells; c++) {
                char *s = rows[r].cells[c];
                bool l_colon = (s[0] == ':');
                int slen = (int)strlen(s);
                bool r_colon = (slen > 1 && s[slen - 1] == ':');
                if (l_colon && r_colon) col_aligns[c] = ALIGN_CENTER;
                else if (r_colon) col_aligns[c] = ALIGN_RIGHT;
                else if (l_colon) col_aligns[c] = ALIGN_LEFT;
                else col_aligns[c] = ALIGN_LEFT;
            }
        }
    }

    for (int c = 0; c < max_cols; c++) {
        col_widths[c] = 3;
    }

    for (int r = 0; r < num_rows; r++) {
        if (!rows[r].is_delim) {
            for (int c = 0; c < rows[r].num_cells; c++) {
                int w = utf8_width(rows[r].cells[c]);
                if (w > col_widths[c]) col_widths[c] = w;
            }
        }
    }

    for (int r = 0; r < num_rows; r++) {
        ABuffer row_ab;
        ab_init(&row_ab);

        for (int i = 0; i < base_indent && i < 2000; i++) {
            ab_append(&row_ab, " ", 1);
        }

        if (rows[r].is_delim) {
            for (int c = 0; c < max_cols; c++) {
                ab_append(&row_ab, "| ", 2);
                int w = col_widths[c];
                enum ColAlign al = col_aligns[c];
                if (al == ALIGN_CENTER) {
                    ab_append(&row_ab, ":", 1);
                    for (int k = 0; k < w - 2; k++) ab_append(&row_ab, "-", 1);
                    ab_append(&row_ab, ":", 1);
                } else if (al == ALIGN_RIGHT) {
                    for (int k = 0; k < w - 1; k++) ab_append(&row_ab, "-", 1);
                    ab_append(&row_ab, ":", 1);
                } else if (al == ALIGN_LEFT) {
                    ab_append(&row_ab, ":", 1);
                    for (int k = 0; k < w - 1; k++) ab_append(&row_ab, "-", 1);
                } else {
                    for (int k = 0; k < w; k++) ab_append(&row_ab, "-", 1);
                }
                ab_append(&row_ab, " ", 1);
            }
            ab_append(&row_ab, "|", 1);
        } else {
            for (int c = 0; c < max_cols; c++) {
                ab_append(&row_ab, "| ", 2);
                const char *cell_text = (c < rows[r].num_cells) ? rows[r].cells[c] : "";
                int w = utf8_width(cell_text);
                int pad = col_widths[c] - w;
                if (pad < 0) pad = 0;
                int tlen = (int)strlen(cell_text);

                enum ColAlign al = col_aligns[c];
                if (al == ALIGN_RIGHT) {
                    for (int k = 0; k < pad; k++) ab_append(&row_ab, " ", 1);
                    if (tlen > 0) ab_append(&row_ab, cell_text, tlen);
                } else if (al == ALIGN_CENTER) {
                    int pad_left = pad / 2;
                    int pad_right = pad - pad_left;
                    for (int k = 0; k < pad_left; k++) ab_append(&row_ab, " ", 1);
                    if (tlen > 0) ab_append(&row_ab, cell_text, tlen);
                    for (int k = 0; k < pad_right; k++) ab_append(&row_ab, " ", 1);
                } else {
                    if (tlen > 0) ab_append(&row_ab, cell_text, tlen);
                    for (int k = 0; k < pad; k++) ab_append(&row_ab, " ", 1);
                }
                ab_append(&row_ab, " ", 1);
            }
            ab_append(&row_ab, "|", 1);
        }

        if (row_ab.b) {
            free(lines[start_y + r].chars);
            lines[start_y + r].chars = row_ab.b;
            lines[start_y + r].len = row_ab.len;
        }
    }

    for (int r = 0; r < num_rows; r++) {
        for (int c = 0; c < rows[r].num_cells; c++) {
            free(rows[r].cells[c]);
        }
    }
}

static bool table_align_at(int line_idx) {
    if (line_idx < 0 || line_idx >= E.num_lines) return false;

    int target_y = line_idx;
    if (!is_table_line(&E.lines[target_y])) {
        if (target_y > 0 && is_table_line(&E.lines[target_y - 1])) {
            target_y--;
        } else if (target_y + 1 < E.num_lines && is_table_line(&E.lines[target_y + 1])) {
            target_y++;
        } else {
            return false;
        }
    }

    int start_y = target_y;
    while (start_y > 0 && is_table_line(&E.lines[start_y - 1])) start_y--;
    int end_y = target_y;
    while (end_y + 1 < E.num_lines && is_table_line(&E.lines[end_y + 1])) end_y++;

    if (end_y <= start_y) return false;

    save_undo();
    align_table_lines(E.lines, start_y, end_y);
    mark_modified();
    set_status_msg("Tabela alinhada (%d linhas)", end_y - start_y + 1);
    return true;
}

static void table_align_all(void) {
    bool has_complete_table = false;
    for (int y = 0; y < E.num_lines; y++) {
        if (!is_table_line(&E.lines[y])) continue;
        int end = y;
        while (end + 1 < E.num_lines && is_table_line(&E.lines[end + 1])) end++;
        if (end > y) { has_complete_table = true; break; }
        y = end;
    }
    if (!has_complete_table) {
        set_status_msg("Nenhuma tabela com 2+ linhas encontrada nesta nota");
        return;
    }

    int aligned_count = 0;
    save_undo();
    for (int y = 0; y < E.num_lines; y++) {
        if (is_table_line(&E.lines[y])) {
            int start_y = y;
            while (y + 1 < E.num_lines && is_table_line(&E.lines[y + 1])) {
                y++;
            }
            int end_y = y;
            if (end_y > start_y) {
                align_table_lines(E.lines, start_y, end_y);
                aligned_count++;
            }
        }
    }
    if (aligned_count > 0) {
        mark_modified();
        set_status_msg("%d tabela(s) alinhada(s)", aligned_count);
    } else {
        set_status_msg("Nenhuma tabela encontrada nesta nota");
    }
}

static int table_count_columns(const Line *l) {
    if (!l || !l->chars || l->len <= 0) return 2;
    int start = 0, end = l->len;
    while (start < end && (l->chars[start] == ' ' || l->chars[start] == '\t')) start++;
    while (end > start && (l->chars[end - 1] == ' ' || l->chars[end - 1] == '\t')) end--;
    if (start < end && l->chars[start] == '|') start++;
    if (end > start && l->chars[end - 1] == '|') end--;

    int bars = 0;
    for (int i = start; i < end; i++) {
        if (l->chars[i] == '|') bars++;
    }
    return bars + 1;
}

/* Insere uma linha vazia de tabela. Antes esta linha era montada em um buffer
 * fixo de 512 bytes: uma linha com muitos '|' estourava a pilha. O ABuffer
 * elimina o limite e o numero de colunas e limitado ao que o alinhador aceita. */
static bool table_insert_blank_row(int at, int max_cols) {
    if (max_cols <= 0) max_cols = 2;
    if (max_cols > MAX_TABLE_COLS) return false;

    ABuffer row;
    ab_init(&row);
    ab_append(&row, "|", 1);
    for (int c = 0; c < max_cols; c++) ab_append(&row, "   |", 4);
    if (row.failed || !row.b) {
        ab_free(&row);
        return false;
    }
    buf_insert_line(at, row.b, row.len);
    ab_free(&row);
    return true;
}

static void table_jump_next_cell(bool create_if_at_end) {
    if (E.num_lines == 0) return;
    if (!is_table_line(&E.lines[E.cursor_y])) return;

    int start_y = E.cursor_y;
    while (start_y > 0 && is_table_line(&E.lines[start_y - 1])) start_y--;
    int end_y = E.cursor_y;
    while (end_y + 1 < E.num_lines && is_table_line(&E.lines[end_y + 1])) end_y++;

    save_undo();
    align_table_lines(E.lines, start_y, end_y);
    mark_modified();

    Line *cur = &E.lines[E.cursor_y];

    const char *p = (E.cursor_x < cur->len) ? strchr(&cur->chars[E.cursor_x], '|') : NULL;
    if (p && p[1] != '\0' && strchr(p + 1, '|') != NULL) {
        int next_pos = (int)(p - cur->chars) + 1;
        while (next_pos < cur->len && cur->chars[next_pos] == ' ') next_pos++;
        E.cursor_x = next_pos;
        return;
    }

    if (E.cursor_y + 1 <= end_y) {
        E.cursor_y++;
        Line *next_l = &E.lines[E.cursor_y];
        const char *np = next_l->chars ? next_l->chars : "";
        while (*np == ' ' || *np == '\t') np++;
        if (*np == '|') np++;
        while (*np == ' ' || *np == '\t') np++;
        if (*np == '-' || *np == ':') {
            if (E.cursor_y + 1 <= end_y) {
                E.cursor_y++;
                next_l = &E.lines[E.cursor_y];
            } else if (create_if_at_end) {
                int max_cols = table_count_columns(cur);
                if (!table_insert_blank_row(E.cursor_y + 1, max_cols)) return;
                align_table_lines(E.lines, start_y, end_y + 1);
                E.cursor_y++;
                next_l = &E.lines[E.cursor_y];
            }
        }

        np = next_l->chars ? next_l->chars : "";
        const char *first_bar = strchr(np, '|');
        if (first_bar) {
            int pos = (int)(first_bar - np) + 1;
            while (pos < next_l->len && next_l->chars[pos] == ' ') pos++;
            E.cursor_x = pos;
        } else {
            E.cursor_x = 0;
        }
        return;
    }

    if (create_if_at_end) {
        int max_cols = table_count_columns(cur);
        if (!table_insert_blank_row(E.cursor_y + 1, max_cols)) return;
        align_table_lines(E.lines, start_y, end_y + 1);
        E.cursor_y++;
        Line *new_l = &E.lines[E.cursor_y];
        const char *first_bar = strchr(new_l->chars, '|');
        if (first_bar) {
            int pos = (int)(first_bar - new_l->chars) + 1;
            while (pos < new_l->len && new_l->chars[pos] == ' ') pos++;
            E.cursor_x = pos;
        } else {
            E.cursor_x = 0;
        }
    }
}

static bool table_insert_row_and_advance(void) {
    if (E.num_lines == 0) return false;
    if (!is_table_line(&E.lines[E.cursor_y])) return false;

    int start_y = E.cursor_y;
    while (start_y > 0 && is_table_line(&E.lines[start_y - 1])) start_y--;
    int end_y = E.cursor_y;
    while (end_y + 1 < E.num_lines && is_table_line(&E.lines[end_y + 1])) end_y++;

    save_undo();
    align_table_lines(E.lines, start_y, end_y);
    mark_modified();

    Line *cur = &E.lines[E.cursor_y];
    int max_cols = table_count_columns(cur);

    if (!table_insert_blank_row(E.cursor_y + 1, max_cols)) return false;
    align_table_lines(E.lines, start_y, end_y + 1);

    E.cursor_y++;
    Line *new_l = &E.lines[E.cursor_y];
    const char *first_bar = strchr(new_l->chars, '|');
    if (first_bar) {
        int pos = (int)(first_bar - new_l->chars) + 1;
        while (pos < new_l->len && new_l->chars[pos] == ' ') pos++;
        E.cursor_x = pos;
    } else {
        E.cursor_x = 0;
    }
    set_status_msg("Nova linha de tabela inserida");
    return true;
}

/* =========================================================================
 * Visual Soft Wrap Segmentation & Coordinate Mapping
 * ========================================================================= */

typedef struct {
    int byte_offset;
    int byte_len;
} LineSegment;

static int get_line_segments(const Line *l, int max_cols, LineSegment *segs, int max_segs, bool hide_links, bool hide_styles) {
    if (!l || l->len == 0 || !l->chars) {
        if (max_segs > 0) {
            segs[0].byte_offset = 0;
            segs[0].byte_len = 0;
            return 1;
        }
        return 0;
    }

    if (max_cols <= 5) max_cols = 5;

    LinkSpan spans[16];
    int nspans = hide_links ? find_links_in_line(l, spans, 16) : 0;
    MarkdownStyleSpan style_spans[16];
    int nstyles = (!l->is_code_fence && hide_styles) ? find_markdown_styles(l, style_spans, 16) : 0;

    int n_segs = 0;
    int i = 0;
    int len = l->len;
    const char *s = l->chars;

    while (i < len && n_segs < max_segs) {
        int seg_start = i;
        int cols = 0;
        int last_space_byte = -1;
        int last_space_next = -1;

        int p = i;
        while (p < len) {
            /* If links are hidden, a link is treated as its visible text only (URL takes 0 columns) */
            if (nspans > 0) {
                int link_idx = -1;
                for (int sp = 0; sp < nspans; sp++) {
                    if (p >= spans[sp].start_raw && p < spans[sp].end_raw) {
                        link_idx = sp;
                        break;
                    }
                }
                if (link_idx >= 0) {
                    int link_w = utf8_width(spans[link_idx].text) + (spans[link_idx].is_wikilink ? 2 : 0);
                    if (cols + link_w > max_cols && cols > 0) {
                        /* Link does not fit on this visual line; wrap before the link */
                        break;
                    }
                    cols += link_w;
                    p = spans[link_idx].end_raw;
                    if (p < len && s[p] == ' ') {
                        last_space_byte = p;
                        last_space_next = p + 1;
                    }
                    continue;
                }
            }

            /* If styles are hidden, delimiters take 0 columns */
            if (nstyles > 0) {
                int st_open = -1;
                int st_close = -1;
                for (int st = 0; st < nstyles; st++) {
                    if (p >= style_spans[st].open_start && p < style_spans[st].content_start) {
                        st_open = st;
                        break;
                    }
                    if (p >= style_spans[st].close_start && p < style_spans[st].close_start + style_spans[st].close_len) {
                        st_close = st;
                        break;
                    }
                }
                if (st_open >= 0) {
                    p = style_spans[st_open].content_start;
                    continue;
                }
                if (st_close >= 0) {
                    p = style_spans[st_close].close_start + style_spans[st_close].close_len;
                    if (p < len && s[p] == ' ') {
                        last_space_byte = p;
                        last_space_next = p + 1;
                    }
                    continue;
                }
            }

            unsigned char ch = (unsigned char)s[p];
            int clen = utf8_charlen(ch);
            if (clen <= 0) clen = 1;
            if (p + clen > len) clen = len - p;

            if (ch == ' ' && p > seg_start) {
                last_space_byte = p;
                last_space_next = p + 1;
            }

            if (cols + 1 > max_cols) {
                break;
            }

            cols++;
            p += clen;
        }

        if (p >= len) {
            segs[n_segs].byte_offset = seg_start;
            segs[n_segs].byte_len = len - seg_start;
            n_segs++;
            break;
        }

        if (last_space_byte > seg_start) {
            segs[n_segs].byte_offset = seg_start;
            segs[n_segs].byte_len = last_space_byte - seg_start;
            n_segs++;
            i = last_space_next;
        } else {
            segs[n_segs].byte_offset = seg_start;
            segs[n_segs].byte_len = p - seg_start;
            n_segs++;
            i = p;
        }
    }

    if (n_segs == 0 && max_segs > 0) {
        segs[0].byte_offset = 0;
        segs[0].byte_len = 0;
        n_segs = 1;
    }
    return n_segs;
}

static int get_visual_col_in_seg(const Line *l, int seg_start, int x_in_seg, bool hide_links, bool hide_styles) {
    if (!l || !l->chars || x_in_seg <= 0) return 0;
    LinkSpan spans[16];
    int nspans = hide_links ? find_links_in_line(l, spans, 16) : 0;
    MarkdownStyleSpan style_spans[16];
    int nstyles = (!l->is_code_fence && hide_styles) ? find_markdown_styles(l, style_spans, 16) : 0;

    int cols = 0;
    int target_byte = seg_start + x_in_seg;
    int p = seg_start;
    const char *s = l->chars;

    while (p < target_byte && p < l->len) {
        if (nspans > 0) {
            int link_idx = -1;
            for (int sp = 0; sp < nspans; sp++) {
                if (p >= spans[sp].start_raw && p < spans[sp].end_raw) {
                    link_idx = sp;
                    break;
                }
            }
            if (link_idx >= 0) {
                if (p == spans[link_idx].start_raw) {
                    if (target_byte >= spans[link_idx].end_raw) {
                        cols += utf8_width(spans[link_idx].text) + (spans[link_idx].is_wikilink ? 2 : 0);
                    } else {
                        cols += (spans[link_idx].is_wikilink ? 1 : 0);
                        int tlen = (int)strlen(spans[link_idx].text);
                        int bytes_into = target_byte - (spans[link_idx].start_raw + (spans[link_idx].is_wikilink ? 2 : 1));
                        if (bytes_into > tlen) bytes_into = tlen;
                        if (bytes_into > 0) {
                            cols += utf8_width_len(spans[link_idx].text, bytes_into);
                        }
                    }
                }
                p = spans[link_idx].end_raw;
                continue;
            }
        }

        if (nstyles > 0) {
            int st_open = -1;
            int st_close = -1;
            for (int st = 0; st < nstyles; st++) {
                if (p >= style_spans[st].open_start && p < style_spans[st].content_start) {
                    st_open = st;
                    break;
                }
                if (p >= style_spans[st].close_start && p < style_spans[st].close_start + style_spans[st].close_len) {
                    st_close = st;
                    break;
                }
            }
            if (st_open >= 0) {
                if (target_byte < style_spans[st_open].content_start) {
                    break;
                }
                p = style_spans[st_open].content_start;
                continue;
            }
            if (st_close >= 0) {
                if (target_byte < style_spans[st_close].close_start + style_spans[st_close].close_len) {
                    break;
                }
                p = style_spans[st_close].close_start + style_spans[st_close].close_len;
                continue;
            }
        }

        unsigned char ch = (unsigned char)s[p];
        int clen = utf8_charlen(ch);
        if (clen <= 0) clen = 1;
        cols++;
        p += clen;
    }
    return cols;
}

static int get_byte_offset_in_seg(const Line *l, int seg_start, int seg_len, int target_col, bool hide_links, bool hide_styles) {
    if (!l || !l->chars || seg_len <= 0 || target_col <= 0) return 0;
    LinkSpan spans[16];
    int nspans = hide_links ? find_links_in_line(l, spans, 16) : 0;
    MarkdownStyleSpan style_spans[16];
    int nstyles = (!l->is_code_fence && hide_styles) ? find_markdown_styles(l, style_spans, 16) : 0;

    int cur_col = 0;
    int p = seg_start;
    int end = seg_start + seg_len;
    const char *s = l->chars;

    while (p < end && cur_col < target_col) {
        if (nspans > 0) {
            int link_idx = -1;
            for (int sp = 0; sp < nspans; sp++) {
                if (p >= spans[sp].start_raw && p < spans[sp].end_raw) {
                    link_idx = sp;
                    break;
                }
            }
            if (link_idx >= 0) {
                int link_w = utf8_width(spans[link_idx].text) + (spans[link_idx].is_wikilink ? 2 : 0);
                if (cur_col + link_w > target_col) {
                    int col_into = target_col - cur_col - (spans[link_idx].is_wikilink ? 1 : 0);
                    if (col_into <= 0) return spans[link_idx].start_raw - seg_start;
                    int b_into = byte_offset_for_col(spans[link_idx].text, (int)strlen(spans[link_idx].text), col_into);
                    return (spans[link_idx].start_raw + (spans[link_idx].is_wikilink ? 2 : 1) + b_into) - seg_start;
                }
                cur_col += link_w;
                p = spans[link_idx].end_raw;
                continue;
            }
        }

        if (nstyles > 0) {
            int st_open = -1;
            int st_close = -1;
            for (int st = 0; st < nstyles; st++) {
                if (p >= style_spans[st].open_start && p < style_spans[st].content_start) {
                    st_open = st;
                    break;
                }
                if (p >= style_spans[st].close_start && p < style_spans[st].close_start + style_spans[st].close_len) {
                    st_close = st;
                    break;
                }
            }
            if (st_open >= 0) {
                p = style_spans[st_open].content_start;
                continue;
            }
            if (st_close >= 0) {
                p = style_spans[st_close].close_start + style_spans[st_close].close_len;
                continue;
            }
        }

        unsigned char ch = (unsigned char)s[p];
        int clen = utf8_charlen(ch);
        if (clen <= 0) clen = 1;
        cur_col++;
        p += clen;
    }
    return p - seg_start;
}

static bool find_visual_row(bool in_preview, int target_vrow, int avail_w,
                            int *out_line_idx, int *out_sub_idx, LineSegment *out_seg, int *out_total_vrows) {
    Line *render_lines = in_preview ? g_preview.lines : E.lines;
    int num_render_lines = in_preview ? g_preview.num_lines : E.num_lines;

    int current_vrow = 0;
    bool found = false;

    for (int i = 0; i < num_render_lines; i++) {
        if (render_lines[i].hidden) continue;

        LineSegment segs[128];
        int nsegs = 1;
        bool hide_links = should_hide_links(in_preview);
        bool hide_styles = should_hide_markdown_styles(in_preview);
        if (E.wrap_mode && !render_lines[i].is_code_fence && !is_table_line(&render_lines[i])) {
            nsegs = get_line_segments(&render_lines[i], avail_w, segs, 128, hide_links, hide_styles);
        } else {
            segs[0].byte_offset = 0;
            segs[0].byte_len = render_lines[i].len;
        }

        if (!found && current_vrow + nsegs > target_vrow) {
            if (out_line_idx) *out_line_idx = i;
            int sub = target_vrow - current_vrow;
            if (out_sub_idx) *out_sub_idx = sub;
            if (out_seg) *out_seg = segs[sub];
            found = true;
            /* Sem total solicitado, nao ha motivo para continuar ate o fim. */
            if (!out_total_vrows) return true;
        }
        current_vrow += nsegs;
    }

    if (out_total_vrows) *out_total_vrows = current_vrow;
    return found;
}

static void get_cursor_visual_pos(int avail_w, int *out_vrow, int *out_vcol) {
    int vrow = 0;
    int vcol = 0;
    bool hide_links = should_hide_links(false);
    bool hide_styles = should_hide_markdown_styles(false);

    for (int i = 0; i < E.num_lines; i++) {
        if (E.lines[i].hidden) continue;

        LineSegment segs[128];
        int nsegs = 1;
        if (E.wrap_mode && !E.lines[i].is_code_fence && !is_table_line(&E.lines[i])) {
            nsegs = get_line_segments(&E.lines[i], avail_w, segs, 128, hide_links, hide_styles);
        } else {
            segs[0].byte_offset = 0;
            segs[0].byte_len = E.lines[i].len;
        }

        if (i == E.cursor_y) {
            int cur_sub = 0;
            for (int s = 0; s < nsegs; s++) {
                int seg_end = segs[s].byte_offset + segs[s].byte_len;
                if (E.cursor_x >= segs[s].byte_offset && (E.cursor_x <= seg_end || s == nsegs - 1)) {
                    cur_sub = s;
                    if (s + 1 < nsegs && E.cursor_x >= segs[s + 1].byte_offset) continue;
                    break;
                }
            }
            vrow += cur_sub;
            int seg_start = segs[cur_sub].byte_offset;
            int x_in_seg = E.cursor_x - seg_start;
            if (x_in_seg < 0) x_in_seg = 0;
            if (x_in_seg > segs[cur_sub].byte_len) x_in_seg = segs[cur_sub].byte_len;
            vcol = get_visual_col_in_seg(&E.lines[i], seg_start, x_in_seg, hide_links, hide_styles);
            break;
        }

        vrow += nsegs;
    }

    if (out_vrow) *out_vrow = vrow;
    if (out_vcol) *out_vcol = vcol;
}

static void move_cursor_visual_down(void) {
    int editor_width = E.screen_cols - E.left_width - 1;
    int avail_text_w = editor_width - 7;
    if (avail_text_w <= 5) avail_text_w = 5;

    Line *cur = &E.lines[E.cursor_y];
    LineSegment segs[128];
    int nsegs = 1;
    bool hide_links = should_hide_links(false);
    bool hide_styles = should_hide_markdown_styles(false);
    if (E.wrap_mode && !cur->is_code_fence && !is_table_line(cur)) {
        nsegs = get_line_segments(cur, avail_text_w, segs, 128, hide_links, hide_styles);
    } else {
        segs[0].byte_offset = 0;
        segs[0].byte_len = cur->len;
    }

    int cur_sub = 0;
    for (int s = 0; s < nsegs; s++) {
        int seg_end = segs[s].byte_offset + segs[s].byte_len;
        if (E.cursor_x >= segs[s].byte_offset && (E.cursor_x <= seg_end || s == nsegs - 1)) {
            cur_sub = s;
            if (s + 1 < nsegs && E.cursor_x >= segs[s + 1].byte_offset) continue;
            break;
        }
    }

    if (cur_sub + 1 < nsegs) {
        int col_offset = get_visual_col_in_seg(cur, segs[cur_sub].byte_offset, E.cursor_x - segs[cur_sub].byte_offset, hide_links, hide_styles);
        int next_start = segs[cur_sub + 1].byte_offset;
        int next_len = segs[cur_sub + 1].byte_len;
        int target_x = next_start + get_byte_offset_in_seg(cur, next_start, next_len, col_offset, hide_links, hide_styles);
        E.cursor_x = target_x;
    } else {
        if (E.cursor_y + 1 < E.num_lines) {
            E.cursor_y++;
            while (E.cursor_y + 1 < E.num_lines && E.lines[E.cursor_y].hidden) {
                E.cursor_y++;
            }
            if (E.cursor_x > E.lines[E.cursor_y].len) {
                E.cursor_x = E.lines[E.cursor_y].len;
            }
        }
    }
}

static void move_cursor_visual_up(void) {
    int editor_width = E.screen_cols - E.left_width - 1;
    int avail_text_w = editor_width - 7;
    if (avail_text_w <= 5) avail_text_w = 5;

    Line *cur = &E.lines[E.cursor_y];
    LineSegment segs[128];
    int nsegs = 1;
    bool hide_links = should_hide_links(false);
    bool hide_styles = should_hide_markdown_styles(false);
    if (E.wrap_mode && !cur->is_code_fence && !is_table_line(cur)) {
        nsegs = get_line_segments(cur, avail_text_w, segs, 128, hide_links, hide_styles);
    } else {
        segs[0].byte_offset = 0;
        segs[0].byte_len = cur->len;
    }

    int cur_sub = 0;
    for (int s = 0; s < nsegs; s++) {
        int seg_end = segs[s].byte_offset + segs[s].byte_len;
        if (E.cursor_x >= segs[s].byte_offset && (E.cursor_x <= seg_end || s == nsegs - 1)) {
            cur_sub = s;
            if (s + 1 < nsegs && E.cursor_x >= segs[s + 1].byte_offset) continue;
            break;
        }
    }

    if (cur_sub > 0) {
        int col_offset = get_visual_col_in_seg(cur, segs[cur_sub].byte_offset, E.cursor_x - segs[cur_sub].byte_offset, hide_links, hide_styles);
        int prev_start = segs[cur_sub - 1].byte_offset;
        int prev_len = segs[cur_sub - 1].byte_len;
        int target_x = prev_start + get_byte_offset_in_seg(cur, prev_start, prev_len, col_offset, hide_links, hide_styles);
        E.cursor_x = target_x;
    } else {
        if (E.cursor_y > 0) {
            E.cursor_y--;
            while (E.cursor_y > 0 && E.lines[E.cursor_y].hidden) {
                E.cursor_y--;
            }
            if (E.cursor_x > E.lines[E.cursor_y].len) {
                E.cursor_x = E.lines[E.cursor_y].len;
            }
        }
    }
}

/* =========================================================================
 * Screen Rendering (VS Code Themes & Tabs)
 * ========================================================================= */

static void render_left_panel(ABuffer *ab, int row) {
    int w = E.left_width;
    int cols_drawn = 0;

    /* Row 0: App Title & Author Credit */
    if (row == 0) {
        ab_append(ab, TH_BG_STATUS, -1);
        ab_append(ab, STYLE_BOLD, -1);
        ab_append(ab, TH_FG_ACCENT, -1);
        ab_append(ab, " NotaZ", -1);
        cols_drawn += 6;
        if (w >= 41) {
            ab_append(ab, "\x1b[24m", 4);
            ab_append(ab, TH_BG_STATUS, -1);
            ab_append(ab, TH_FG_MUTED, -1);
            ab_append(ab, ", desenvolvido por ", -1);
            cols_drawn += 19;
            ab_append(ab, TH_FG_WHITE, -1);
            ab_append(ab, STYLE_BOLD, -1);
            ab_append(ab, "Marcelo Fidelix", -1);
            cols_drawn += 15;
        } else if (w >= 28) {
            ab_append(ab, "\x1b[24m", 4);
            ab_append(ab, TH_BG_STATUS, -1);
            ab_append(ab, TH_FG_MUTED, -1);
            ab_append(ab, " por ", -1);
            cols_drawn += 5;
            ab_append(ab, TH_FG_WHITE, -1);
            ab_append(ab, STYLE_BOLD, -1);
            ab_append(ab, "Marcelo Fidelix", -1);
            cols_drawn += 15;
        }
        ab_append(ab, STYLE_RESET, -1);
        ab_append(ab, TH_BG_STATUS, -1);
        while (cols_drawn < w) {
            ab_append(ab, " ", 1);
            cols_drawn++;
        }
        ab_append(ab, TH_FG_BORDER, -1);
        ab_append(ab, "│" STYLE_RESET, -1);
        return;
    }

    /* Row 1: Search Entry (Modern Styled Box) */
    if (row == 1) {
        if (E.mode == MODE_SEARCH_LEFT) {
            /* Active search box: Vibrant glowing frame */
            ab_append(ab, TH_BG_SELECT, -1);
            ab_append(ab, TH_FG_YELLOW, -1);
            ab_append(ab, STYLE_BOLD, -1);
            ab_append(ab, " 🔍 ", -1);
            cols_drawn += 4;

            ab_append(ab, TH_FG_WHITE, -1);
            int avail = w - cols_drawn - 2;
            if (avail < 2) avail = 2;
            int q_w = utf8_width(E.search_query);
            if (q_w > avail) {
                char trunc[256];
                utf8_truncate_display(E.search_query, avail, trunc, sizeof(trunc));
                ab_append(ab, trunc, (int)strlen(trunc));
                cols_drawn += utf8_width(trunc);
            } else {
                ab_append(ab, E.search_query, (int)strlen(E.search_query));
                cols_drawn += q_w;
            }
            ab_append(ab, "\x1b[7m \x1b[27m", -1);
            cols_drawn += 1;

            while (cols_drawn < w) {
                ab_append(ab, " ", 1);
                cols_drawn++;
            }
            ab_append(ab, STYLE_RESET, -1);
            ab_append(ab, TH_FG_H2, -1);
            ab_append(ab, "┃" STYLE_RESET, -1);
            return;
        } else {
            /* Inactive search box: Styled modern container */
            ab_append(ab, TH_BG_SIDEBAR, -1);
            ab_append(ab, " ", 1);
            cols_drawn += 1;

            ab_append(ab, TH_BG_CUR_LINE, -1);
            ab_append(ab, TH_FG_ACCENT, -1);
            ab_append(ab, STYLE_BOLD, -1);
            ab_append(ab, " ⌕ ", -1);
            cols_drawn += 3;

            ab_append(ab, STYLE_RESET, -1);
            ab_append(ab, TH_BG_CUR_LINE, -1);
            ab_append(ab, TH_FG_MUTED, -1);
            const char *placeholder = (E.search_query[0] != '\0') ? E.search_query : "Buscar notas...";
            int p_w = utf8_width(placeholder);
            int shortcut_w = 9; /* " [Ctrl+F]" */
            int max_text = w - cols_drawn - shortcut_w - 2;
            if (max_text > 0) {
                if (p_w > max_text) {
                    char trunc[128];
                    utf8_truncate_display(placeholder, max_text, trunc, sizeof(trunc));
                    ab_append(ab, trunc, (int)strlen(trunc));
                    cols_drawn += utf8_width(trunc);
                } else {
                    ab_append(ab, placeholder, (int)strlen(placeholder));
                    cols_drawn += p_w;
                }
            }

            int inner_pad = w - cols_drawn - shortcut_w - 1;
            while (inner_pad-- > 0) {
                ab_append(ab, " ", 1);
                cols_drawn++;
            }

            ab_append(ab, TH_FG_YELLOW, -1);
            ab_append(ab, " [Ctrl+F]", -1);
            cols_drawn += 9;

            ab_append(ab, " ", 1);
            cols_drawn += 1;

            ab_append(ab, STYLE_RESET, -1);
            ab_append(ab, TH_BG_SIDEBAR, -1);
            while (cols_drawn < w) {
                ab_append(ab, " ", 1);
                cols_drawn++;
            }
            ab_append(ab, TH_FG_BORDER, -1);
            ab_append(ab, "│" STYLE_RESET, -1);
            return;
        }
    }

    /* Row 2: Header / Notes separator */
    if (row == 2) {
        char hdr[128];
        if (E.search_query[0] != '\0') {
            snprintf(hdr, sizeof(hdr), " 🔍 %d %s encontrada(s)", E.num_filtered, E.num_filtered == 1 ? "nota" : "notas");
            ab_append(ab, TH_BG_SELECT, -1);
            ab_append(ab, TH_FG_WHITE, -1);
            ab_append(ab, STYLE_BOLD, -1);
        } else if (E.filter_urgency > 0) {
            ab_append(ab, TH_BG_SELECT, -1);
            ab_append(ab, get_attention_color(E.filter_urgency), -1);
            ab_append(ab, STYLE_BOLD, -1);
            const char *lbl = (E.filter_urgency == 1) ? "🔴 NÍVEL 1" :
                              ((E.filter_urgency == 2) ? "🟠 NÍVEL 2" :
                              ((E.filter_urgency == 3) ? "🟡 NÍVEL 3" :
                              ((E.filter_urgency == 4) ? "🔵 MARCAÇÃO H" :
                              ((E.filter_urgency == 5) ? "🟣 MARCAÇÃO R" : "🟢 MARCAÇÃO O"))));
            snprintf(hdr, sizeof(hdr), " %s (%d/%d) ", lbl, E.num_filtered, E.num_notes);
        } else if (E.focus == FOCUS_LEFT) {
            ab_append(ab, TH_BG_SELECT, -1);
            ab_append(ab, TH_FG_WHITE, -1);
            ab_append(ab, STYLE_BOLD, -1);
            snprintf(hdr, sizeof(hdr), " 📂 NOTAS (%d) [ATIVO] ", E.num_notes);
        } else {
            ab_append(ab, TH_BG_SIDEBAR, -1);
            ab_append(ab, TH_FG_MUTED, -1);
            ab_append(ab, STYLE_BOLD, -1);
            snprintf(hdr, sizeof(hdr), "  NOTAS (%d) ", E.num_notes);
        }

        int h_w = utf8_width(hdr);
        if (h_w > w) {
            char trunc[128];
            utf8_truncate_display(hdr, w, trunc, sizeof(trunc));
            ab_append(ab, trunc, (int)strlen(trunc));
            cols_drawn += utf8_width(trunc);
        } else {
            ab_append(ab, hdr, (int)strlen(hdr));
            cols_drawn += h_w;
        }

        while (cols_drawn < w) {
            ab_append(ab, " ", 1);
            cols_drawn++;
        }
        ab_append(ab, STYLE_RESET, -1);
        if (E.focus == FOCUS_LEFT || E.mode == MODE_SEARCH_LEFT) {
            ab_append(ab, TH_FG_H2, -1);
            ab_append(ab, "┃" STYLE_RESET, -1);
        } else {
            ab_append(ab, TH_FG_BORDER, -1);
            ab_append(ab, "│" STYLE_RESET, -1);
        }
        return;
    }

    int sb_lines = (E.status_bar_lines >= 1) ? E.status_bar_lines : 1;
    int editor_rows = E.screen_rows - (1 + sb_lines);

    /* Bottom row of left panel: Urgency Filter Buttons */
    if (row == editor_rows - 1) {
        ab_append(ab, TH_BG_SIDEBAR, -1);
        int cur_col = 1;

        /* Rotulos compactos quando o painel e estreito:
         *   completo       " [Todas] [1] [2] [3] [H] [R] [O]" = 32 colunas
         *   compacto       " [T] [1] [2] [3] [H] [R] [O]"     = 28 colunas
         *   ultra-compacto "[T][1][2][3][H][R][O]"           = 21 colunas
         * Os limites (campos _start/_end) sao gravados a partir do rotulo usado,
         * entao o clique continua correto em qualquer largura. */
        int spacing = (w >= 28) ? 1 : 0;
        int leading = (w >= 28) ? 1 : 0;
        const char *lbl_all = (w >= 32) ? "[Todas]" : "[T]";
        int w_all = (w >= 32) ? 7 : 3;

        if (leading) {
            ab_append(ab, " ", 1);
            cols_drawn += 1;
            cur_col += 1;
        }

        /* Button [Todas] / [T] */
        E.btn_filter_all_start = cur_col;
        if (E.filter_urgency == 0) {
            ab_append(ab, TH_BG_SELECT, -1);
            ab_append(ab, TH_FG_WHITE, -1);
            ab_append(ab, STYLE_BOLD, -1);
            ab_append(ab, lbl_all, -1);
            ab_append(ab, STYLE_RESET, -1);
            ab_append(ab, TH_BG_SIDEBAR, -1);
        } else {
            ab_append(ab, TH_FG_MUTED, -1);
            ab_append(ab, lbl_all, -1);
            ab_append(ab, STYLE_RESET, -1);
            ab_append(ab, TH_BG_SIDEBAR, -1);
        }
        cols_drawn += w_all;
        cur_col += w_all;
        E.btn_filter_all_end = cur_col;

        if (spacing) {
            ab_append(ab, " ", 1);
            cols_drawn += 1;
            cur_col += 1;
        }

        /* Button [1] (Crítico - Vermelho) */
        E.btn_filter_1_start = cur_col;
        const char *c1 = get_attention_color(1);
        ab_append(ab, c1, -1);
        ab_append(ab, STYLE_BOLD, -1);
        if (E.filter_urgency == 1) {
            ab_append(ab, TH_BG_SELECT, -1);
            ab_append(ab, "[1]", 3);
            ab_append(ab, STYLE_RESET, -1);
            ab_append(ab, TH_BG_SIDEBAR, -1);
        } else {
            ab_append(ab, "[1]", 3);
            ab_append(ab, STYLE_RESET, -1);
            ab_append(ab, TH_BG_SIDEBAR, -1);
        }
        cols_drawn += 3;
        cur_col += 3;
        E.btn_filter_1_end = cur_col;

        if (spacing) {
            ab_append(ab, " ", 1);
            cols_drawn += 1;
            cur_col += 1;
        }

        /* Button [2] (Médio - Laranja) */
        E.btn_filter_2_start = cur_col;
        const char *c2 = get_attention_color(2);
        ab_append(ab, c2, -1);
        ab_append(ab, STYLE_BOLD, -1);
        if (E.filter_urgency == 2) {
            ab_append(ab, TH_BG_SELECT, -1);
            ab_append(ab, "[2]", 3);
            ab_append(ab, STYLE_RESET, -1);
            ab_append(ab, TH_BG_SIDEBAR, -1);
        } else {
            ab_append(ab, "[2]", 3);
            ab_append(ab, STYLE_RESET, -1);
            ab_append(ab, TH_BG_SIDEBAR, -1);
        }
        cols_drawn += 3;
        cur_col += 3;
        E.btn_filter_2_end = cur_col;

        if (spacing) {
            ab_append(ab, " ", 1);
            cols_drawn += 1;
            cur_col += 1;
        }

        /* Button [3] (Baixo - Amarelo) */
        E.btn_filter_3_start = cur_col;
        const char *c3 = get_attention_color(3);
        ab_append(ab, c3, -1);
        ab_append(ab, STYLE_BOLD, -1);
        if (E.filter_urgency == 3) {
            ab_append(ab, TH_BG_SELECT, -1);
            ab_append(ab, "[3]", 3);
            ab_append(ab, STYLE_RESET, -1);
            ab_append(ab, TH_BG_SIDEBAR, -1);
        } else {
            ab_append(ab, "[3]", 3);
            ab_append(ab, STYLE_RESET, -1);
            ab_append(ab, TH_BG_SIDEBAR, -1);
        }
        cols_drawn += 3;
        cur_col += 3;
        E.btn_filter_3_end = cur_col;

        if (spacing) {
            ab_append(ab, " ", 1);
            cols_drawn += 1;
            cur_col += 1;
        }

        /* Button [H] (Azul) */
        E.btn_filter_h_start = cur_col;
        const char *ch = get_attention_color(4);
        ab_append(ab, ch, -1);
        ab_append(ab, STYLE_BOLD, -1);
        if (E.filter_urgency == 4) {
            ab_append(ab, TH_BG_SELECT, -1);
            ab_append(ab, "[H]", 3);
            ab_append(ab, STYLE_RESET, -1);
            ab_append(ab, TH_BG_SIDEBAR, -1);
        } else {
            ab_append(ab, "[H]", 3);
            ab_append(ab, STYLE_RESET, -1);
            ab_append(ab, TH_BG_SIDEBAR, -1);
        }
        cols_drawn += 3;
        cur_col += 3;
        E.btn_filter_h_end = cur_col;

        if (spacing) {
            ab_append(ab, " ", 1);
            cols_drawn += 1;
            cur_col += 1;
        }

        /* Button [R] (Roxo) */
        E.btn_filter_r_start = cur_col;
        const char *cr = get_attention_color(5);
        ab_append(ab, cr, -1);
        ab_append(ab, STYLE_BOLD, -1);
        if (E.filter_urgency == 5) {
            ab_append(ab, TH_BG_SELECT, -1);
            ab_append(ab, "[R]", 3);
            ab_append(ab, STYLE_RESET, -1);
            ab_append(ab, TH_BG_SIDEBAR, -1);
        } else {
            ab_append(ab, "[R]", 3);
            ab_append(ab, STYLE_RESET, -1);
            ab_append(ab, TH_BG_SIDEBAR, -1);
        }
        cols_drawn += 3;
        cur_col += 3;
        E.btn_filter_r_end = cur_col;

        if (spacing) {
            ab_append(ab, " ", 1);
            cols_drawn += 1;
            cur_col += 1;
        }

        /* Button [O] (Verde) */
        E.btn_filter_o_start = cur_col;
        const char *co = get_attention_color(6);
        ab_append(ab, co, -1);
        ab_append(ab, STYLE_BOLD, -1);
        if (E.filter_urgency == 6) {
            ab_append(ab, TH_BG_SELECT, -1);
            ab_append(ab, "[O]", 3);
            ab_append(ab, STYLE_RESET, -1);
            ab_append(ab, TH_BG_SIDEBAR, -1);
        } else {
            ab_append(ab, "[O]", 3);
            ab_append(ab, STYLE_RESET, -1);
            ab_append(ab, TH_BG_SIDEBAR, -1);
        }
        cols_drawn += 3;
        cur_col += 3;
        E.btn_filter_o_end = cur_col;

        while (cols_drawn < w) {
            ab_append(ab, " ", 1);
            cols_drawn++;
        }
        ab_append(ab, TH_FG_BORDER, -1);
        ab_append(ab, "│" STYLE_RESET, -1);
        return;
    }

    /* Notes list rows */
    int note_idx = (row - 3) + E.sidebar_scroll_y;

    if (E.num_filtered == 0 && row == 3) {
        ab_append(ab, TH_BG_SIDEBAR, -1);
        ab_append(ab, TH_FG_MUTED, -1);
        ab_append(ab, "  (Nenhum resultado)", -1);
        cols_drawn += 20;
        while (cols_drawn < w) {
            ab_append(ab, " ", 1);
            cols_drawn++;
        }
        ab_append(ab, TH_FG_BORDER, -1);
        ab_append(ab, "│" STYLE_RESET, -1);
        return;
    }

    if (note_idx >= 0 && note_idx < E.num_filtered) {
        int actual_idx = E.filtered_indices[note_idx];
        NoteInfo *n = &E.notes[actual_idx];
        bool is_selected = (note_idx == E.selected_note_idx);
        bool is_open = (strcmp(n->filepath, E.cur_filepath) == 0);

        ab_append(ab, is_selected ? TH_BG_SELECT : TH_BG_SIDEBAR, -1);

        /* Marker and Attention Checkbox [ ] / [!] */
        if (is_open) {
            ab_append(ab, TH_FG_H1, -1);
            ab_append(ab, "●", -1);
        } else if (is_selected) {
            ab_append(ab, TH_FG_YELLOW, -1);
            ab_append(ab, "▸", -1);
        } else {
            ab_append(ab, " ", 1);
        }
        cols_drawn += 1;

        if (n->attention_level > 0) {
            const char *color = get_attention_color(n->attention_level);
            const char *box = "[?] ";
            if (n->attention_level == 1) box = "[1] ";
            else if (n->attention_level == 2) box = "[2] ";
            else if (n->attention_level == 3) box = "[3] ";
            else if (n->attention_level == 4) box = "[H] ";
            else if (n->attention_level == 5) box = "[R] ";
            else if (n->attention_level == 6) box = "[O] ";
            ab_append(ab, color, -1);
            ab_append(ab, STYLE_BOLD, -1);
            ab_append(ab, box, 4);
            ab_append(ab, STYLE_RESET, -1);
            ab_append(ab, is_selected ? TH_BG_SELECT : TH_BG_SIDEBAR, -1);
        } else {
            ab_append(ab, TH_FG_MUTED, -1);
            ab_append(ab, "[ ] ", 4);
            ab_append(ab, STYLE_RESET, -1);
            ab_append(ab, is_selected ? TH_BG_SELECT : TH_BG_SIDEBAR, -1);
        }
        cols_drawn += 4;

        /* Refresh relative time dynamically */
        if (n->mtime <= 0) {
            struct stat st;
            if (stat_cross(n->filepath, &st) == 0 && st.st_mtime > 0) {
                n->mtime = st.st_mtime;
            } else {
                n->mtime = time(NULL);
            }
        }
        format_rel_time(n->mtime, time(NULL), n->rel_time, sizeof(n->rel_time));

        /* Relative time width */
        int date_w = utf8_width(n->rel_time);
        int avail_title = w - cols_drawn - 1 - date_w;
        if (avail_title < 3) avail_title = 3;

        int title_w = utf8_width(n->title);
        if (n->attention_level > 0) {
            const char *color = get_attention_color(n->attention_level);
            ab_append(ab, color, -1);
            ab_append(ab, STYLE_BOLD, -1);
        } else if (is_selected) {
            ab_append(ab, TH_FG_WHITE, -1);
            ab_append(ab, STYLE_BOLD, -1);
        } else {
            ab_append(ab, TH_FG_MAIN, -1);
        }

        if (title_w > avail_title) {
            char trunc[256];
            utf8_truncate_display(n->title, avail_title - 1, trunc, sizeof(trunc));
            ab_append(ab, trunc, (int)strlen(trunc));
            ab_append(ab, "~", 1);
            cols_drawn += utf8_width(trunc) + 1;
        } else {
            ab_append(ab, n->title, (int)strlen(n->title));
            cols_drawn += title_w;
        }

        /* Pad between title and date so date aligns perfectly before divider */
        int target_date_col = w - date_w - 1;
        while (cols_drawn < target_date_col) {
            ab_append(ab, " ", 1);
            cols_drawn++;
        }
        ab_append(ab, " ", 1);
        cols_drawn++;

        /* Date */
        ab_append(ab, TH_FG_DATE, -1);
        ab_append(ab, n->rel_time, (int)strlen(n->rel_time));
        cols_drawn += date_w;

        while (cols_drawn < w) {
            ab_append(ab, " ", 1);
            cols_drawn++;
        }
    } else {
        ab_append(ab, TH_BG_SIDEBAR, -1);
        while (cols_drawn < w) {
            ab_append(ab, " ", 1);
            cols_drawn++;
        }
    }

    /* Vertical Divider - Guaranteed straight at column w + 1 */
    ab_append(ab, TH_FG_BORDER, -1);
    ab_append(ab, "│" STYLE_RESET, -1);
}

/* Render Tab Bar across the top of the editor */
static void render_tab_bar(ABuffer *ab) {
    int bar_width = E.screen_cols - E.left_width - 1;
    if (bar_width <= 0) return;

    /* Dirty-flag: assume que nao ha nada animado. O ramo de marquee abaixo
     * reativa isto se algum titulo longo realmente precisar rolar. */
    g_marquee_active = false;

    if (E.num_tabs == 0) {
        const char *new_tab_btn = " [➕ Nova Nota] ";
        int btn_w = utf8_width(new_tab_btn);
        int cur_screen_col = E.left_width + 2;

        E.tab_new_btn_start = cur_screen_col;
        E.tab_new_btn_end = cur_screen_col + btn_w - 1;

        /* Tab button styled as an inviting active tab */
        ab_append(ab, TH_BG_TAB_ACT, -1);
        ab_append(ab, TH_FG_ACCENT, -1);
        ab_append(ab, STYLE_BOLD, -1);
        ab_append(ab, new_tab_btn, (int)strlen(new_tab_btn));
        ab_append(ab, STYLE_RESET, -1);

        /* Tab Divider */
        ab_append(ab, TH_BG_TAB_INACT, -1);
        ab_append(ab, TH_FG_BORDER, -1);
        ab_append(ab, "│" STYLE_RESET, -1);

        ab_append(ab, TH_BG_TAB_INACT, -1);
        int pad = bar_width - 1 - btn_w - 1;
        while (pad-- > 0) ab_append(ab, " ", 1);
        ab_append(ab, "\x1b[K" STYLE_RESET, -1);
        return;
    }

    int max_tab_w = 24;
    int min_tab_w = 14;
    int tab_w = (bar_width - 1) / E.num_tabs;
    if (tab_w > max_tab_w) tab_w = max_tab_w;
    if (tab_w < min_tab_w) tab_w = min_tab_w;

    int cols_drawn = 0;
    int64_t now_ms = get_current_time_ms();
    int cur_screen_col = E.left_width + 2;

    int max_visible = (bar_width - 1) / min_tab_w;
    if (max_visible <= 0) max_visible = 1;
    int start_tab = 0;
    if (E.active_tab >= max_visible) {
        start_tab = E.active_tab - max_visible + 1;
    }

    for (int i = start_tab; i < E.num_tabs && cols_drawn + min_tab_w <= bar_width - 1; i++) {
        Tab *t = &E.tabs[i];
        bool is_active = (i == E.active_tab);

        int this_w = tab_w;
        if (cols_drawn + this_w > bar_width - 1) {
            this_w = bar_width - 1 - cols_drawn;
        }
        if (this_w < min_tab_w) break;

        /* Record coordinates for mouse click / middle-click */
        E.tab_coords[i].start_col = cur_screen_col;
        E.tab_coords[i].end_col = cur_screen_col + this_w - 1;
        E.tab_coords[i].close_col = cur_screen_col + this_w - 2;

        /* Tab background */
        ab_append(ab, is_active ? TH_BG_TAB_ACT : TH_BG_TAB_INACT, -1);

        /* Indicator (2 cols) */
        if (is_active) {
            ab_append(ab, t->modified ? TH_FG_WARN : TH_FG_H1, -1);
            ab_append(ab, "● ", -1);
        } else {
            if (t->modified) {
                ab_append(ab, TH_FG_WARN, -1);
                ab_append(ab, "+ ", -1);
            } else {
                ab_append(ab, TH_FG_MUTED, -1);
                ab_append(ab, "  ", -1);
            }
        }
        int tab_drawn = 2;

        int avail_title = this_w - 5;
        if (avail_title < 3) avail_title = 3;

        int title_w = utf8_width(t->title);
        if (is_active) {
            ab_append(ab, TH_FG_WHITE, -1);
            ab_append(ab, STYLE_BOLD, -1);
        } else {
            ab_append(ab, TH_FG_MUTED, -1);
        }

        if (title_w <= avail_title) {
            ab_append(ab, t->title, (int)strlen(t->title));
            tab_drawn += title_w;
            while (tab_drawn < this_w - 3) {
                ab_append(ab, " ", 1);
                tab_drawn++;
            }
        } else {
            /* Marquee / Title scroll feature */
            int cycle = title_w + 4;
            int step = (int)((now_ms / 300) % cycle);
            char slice[128];
            utf8_marquee_slice(t->title, step, avail_title, slice, sizeof(slice));
            /* Dirty-flag: ha animacao de fato rolando; registra o tick desenhado
             * para que should_refresh_now() so peca novo desenho quando o passo
             * do marquee realmente mudar (a cada 300 ms), e nao a cada 150 ms. */
            g_marquee_active = true;
            g_last_marquee_tick = now_ms / 300;
            ab_append(ab, slice, (int)strlen(slice));
            tab_drawn += utf8_width(slice);
            while (tab_drawn < this_w - 3) {
                ab_append(ab, " ", 1);
                tab_drawn++;
            }
        }

        /* Close Button (×) */
        ab_append(ab, " ", 1);
        ab_append(ab, is_active ? TH_FG_WARN : TH_FG_MUTED, -1);
        ab_append(ab, "×", -1);
        tab_drawn += 2;

        /* Tab Divider */
        ab_append(ab, TH_FG_BORDER, -1);
        ab_append(ab, "│" STYLE_RESET, -1);
        tab_drawn++;

        cols_drawn += tab_drawn;
        cur_screen_col += tab_drawn;
    }

    ab_append(ab, TH_BG_TAB_INACT, -1);
    while (cols_drawn < bar_width - 1) {
        ab_append(ab, " ", 1);
        cols_drawn++;
    }
    ab_append(ab, "\x1b[K" STYLE_RESET, -1);
}

static void render_code_line(ABuffer *ab, Line *l, int *cols_printed, int max_w) {
    if (!l->chars || l->len == 0) return;

    const char *p = l->chars;
    while (*p == ' ' || *p == '\t') p++;

    /* Section header in ini/conf: [section] */
    if (*p == '[' && strchr(p, ']')) {
        ab_append(ab, TH_FG_H2, -1);
        ab_append(ab, STYLE_BOLD, -1);
        while (*p && *cols_printed < max_w) {
            ab_append(ab, p, 1);
            (*cols_printed)++;
            if (*p == ']') { p++; break; }
            p++;
        }
        ab_append(ab, STYLE_RESET, -1);
        ab_append(ab, TH_BG_CUR_LINE, -1);
        ab_append(ab, TH_FG_MAIN, -1);
    }

    /* Comment line: # or // or ; */
    if (*p == '#' || (p[0] == '/' && p[1] == '/') || *p == ';') {
        ab_append(ab, TH_FG_MUTED, -1);
        while (*p && *cols_printed < max_w) {
            ab_append(ab, p++, 1);
            (*cols_printed)++;
        }
        ab_append(ab, STYLE_RESET, -1);
        ab_append(ab, TH_BG_CUR_LINE, -1);
        ab_append(ab, TH_FG_MAIN, -1);
        return;
    }

    int i = (int)(p - l->chars);
    while (i < l->len && *cols_printed < max_w) {
        char ch = l->chars[i];

        /* Comment starting mid-line */
        if (ch == '#' || (ch == '/' && i + 1 < l->len && l->chars[i + 1] == '/') || ch == ';') {
            ab_append(ab, TH_FG_MUTED, -1);
            while (i < l->len && *cols_printed < max_w) {
                ab_append(ab, &l->chars[i++], 1);
                (*cols_printed)++;
            }
            break;
        }

        /* String: "..." or '...' */
        if (ch == '"' || ch == '\'') {
            char quote = ch;
            ab_append(ab, TH_FG_DATE, -1);
            ab_append(ab, &l->chars[i++], 1);
            (*cols_printed)++;
            while (i < l->len && l->chars[i] != quote && *cols_printed < max_w) {
                if (l->chars[i] == '\\' && i + 1 < l->len) {
                    ab_append(ab, &l->chars[i++], 1);
                    (*cols_printed)++;
                }
                ab_append(ab, &l->chars[i++], 1);
                (*cols_printed)++;
            }
            if (i < l->len && l->chars[i] == quote && *cols_printed < max_w) {
                ab_append(ab, &l->chars[i++], 1);
                (*cols_printed)++;
            }
            ab_append(ab, TH_FG_MAIN, -1);
            continue;
        }

        /* Variable: $VAR or ${VAR} */
        if (ch == '$') {
            ab_append(ab, TH_FG_WARN, -1);
            ab_append(ab, &l->chars[i++], 1);
            (*cols_printed)++;
            if (i < l->len && l->chars[i] == '{') {
                ab_append(ab, &l->chars[i++], 1);
                (*cols_printed)++;
                while (i < l->len && l->chars[i] != '}' && *cols_printed < max_w) {
                    ab_append(ab, &l->chars[i++], 1);
                    (*cols_printed)++;
                }
                if (i < l->len && l->chars[i] == '}' && *cols_printed < max_w) {
                    ab_append(ab, &l->chars[i++], 1);
                    (*cols_printed)++;
                }
            } else {
                while (i < l->len && (isalnum((unsigned char)l->chars[i]) || l->chars[i] == '_') && *cols_printed < max_w) {
                    ab_append(ab, &l->chars[i++], 1);
                    (*cols_printed)++;
                }
            }
            ab_append(ab, TH_FG_MAIN, -1);
            continue;
        }

        /* Flags: -a, --option */
        if (ch == '-' && (i == 0 || isspace((unsigned char)l->chars[i - 1])) && i + 1 < l->len && (isalpha((unsigned char)l->chars[i + 1]) || l->chars[i + 1] == '-')) {
            ab_append(ab, TH_FG_YELLOW, -1);
            while (i < l->len && !isspace((unsigned char)l->chars[i]) && l->chars[i] != '=' && *cols_printed < max_w) {
                ab_append(ab, &l->chars[i++], 1);
                (*cols_printed)++;
            }
            ab_append(ab, TH_FG_MAIN, -1);
            continue;
        }

        /* Word / Keyword */
        if (isalpha((unsigned char)ch) || ch == '_') {
            int wstart = i;
            while (i < l->len && (isalnum((unsigned char)l->chars[i]) || l->chars[i] == '_')) {
                i++;
            }
            int wlen = i - wstart;
            char word[64];
            if (wlen < (int)sizeof(word)) {
                memcpy(word, &l->chars[wstart], wlen);
                word[wlen] = '\0';
            } else {
                word[0] = '\0';
            }

            bool is_kw = false;
            static const char *keywords[] = {
                "sudo", "def", "return", "if", "else", "elif", "for", "while", "in", "import", "from", "class",
                "int", "char", "void", "const", "static", "struct", "bool", "true", "false", "null", "NULL",
                "echo", "cat", "grep", "mkdir", "cd", "ls", "chmod", "chown", "systemctl", "apt", "yum",
                "export", "function", "case", "esac", "fi", "then", "do", "done", "set", "alias", "restart",
                "path", "writeable", "create", "directory", "mask", "public", "yes", "no", "read", "write"
            };
            int nkw = sizeof(keywords) / sizeof(keywords[0]);
            for (int k = 0; k < nkw; k++) {
                if (strcasecmp(word, keywords[k]) == 0) {
                    is_kw = true;
                    break;
                }
            }

            if (is_kw) {
                ab_append(ab, TH_FG_H1, -1);
                ab_append(ab, STYLE_BOLD, -1);
                for (int w = wstart; w < i && *cols_printed < max_w; w++) {
                    ab_append(ab, &l->chars[w], 1);
                    (*cols_printed)++;
                }
                ab_append(ab, STYLE_RESET, -1);
                ab_append(ab, TH_BG_CUR_LINE, -1);
                ab_append(ab, TH_FG_MAIN, -1);
            } else {
                for (int w = wstart; w < i && *cols_printed < max_w; w++) {
                    ab_append(ab, &l->chars[w], 1);
                    (*cols_printed)++;
                }
            }
            continue;
        }

        /* Operators = : | & */
        if (ch == '=' || ch == ':' || ch == '|' || ch == '&') {
            ab_append(ab, TH_FG_ACCENT, -1);
            ab_append(ab, &l->chars[i++], 1);
            (*cols_printed)++;
            ab_append(ab, TH_FG_MAIN, -1);
            continue;
        }

        /* Tab */
        if (ch == '\t') {
            int spaces = 4 - ((*cols_printed - 6) % 4);
            if (*cols_printed + spaces > max_w) spaces = max_w - *cols_printed;
            while (spaces-- > 0) {
                ab_append(ab, " ", 1);
                (*cols_printed)++;
            }
            i++;
            continue;
        }

        /* Regular character */
        int ch_len = utf8_charlen((unsigned char)ch);
        if (ch_len <= 0) ch_len = 1;
        ab_append(ab, &l->chars[i], ch_len);
        (*cols_printed)++;
        i += ch_len;
    }
}

/* =========================================================================
 * Bullet List Detection (* and -)
 * ========================================================================= */
static bool is_bullet_list_line(const Line *l, int *bullet_pos, char *bullet_char) {
    if (!l || !l->chars || l->len == 0) return false;
    if (l->heading_lvl > 0 || l->is_code_fence || l->in_code_block) return false;
    int i = 0;
    while (i < l->len && (l->chars[i] == ' ' || l->chars[i] == '\t')) {
        i++;
    }
    if (i < l->len && (l->chars[i] == '*' || l->chars[i] == '-')) {
        char ch = l->chars[i];
        if (i + 1 == l->len || l->chars[i + 1] == ' ' || l->chars[i + 1] == '\t') {
            /* Check if the line is just a horizontal rule like "- - -" or "* * *" or "---" */
            int non_sp = 0;
            for (int k = i; k < l->len; k++) {
                if (l->chars[k] != ' ' && l->chars[k] != '\t') {
                    if (l->chars[k] == ch) non_sp++;
                    else { non_sp = 0; break; }
                }
            }
            if (non_sp >= 3) return false;

            if (bullet_pos) *bullet_pos = i;
            if (bullet_char) *bullet_char = ch;
            return true;
        }
    }
    return false;
}

/* Cursor de renderizacao por quadro.
 *
 * render_right_panel() e chamado uma vez por linha visivel. Antes, cada chamada
 * refazia find_visual_row() desde a linha 0, ou seja O(linhas) por linha da
 * tela (O(linhas * altura) por quadro). Este cursor guarda a linha/segmento
 * atual e avanca um segmento por chamada; o custo cai para O(linhas + altura)
 * por quadro. E resetado no inicio de cada refresh_screen(), entao nao ha
 * cache entre quadros nem risco de dados obsoletos. */
typedef struct {
    bool valid;
    bool at_end;
    bool preview;
    const Line *lines;
    int num_lines;
    int avail_w;
    int line_idx;
    int line_vrow;
    int sub_idx;
    int nsegs;
    int total_vrows;  /* valido quando a ultima busca chegou ao fim do buffer */
    LineSegment segs[128];
} RenderCursor;

static RenderCursor g_render_cursor;

static void render_cursor_reset(void) {
    memset(&g_render_cursor, 0, sizeof(g_render_cursor));
}

static int build_render_segments(const Line *l, int avail_w, bool in_preview,
                                 LineSegment *segs, int max_segs) {
    if (E.wrap_mode && !l->is_code_fence && !is_table_line(l)) {
        return get_line_segments(l, avail_w, segs, max_segs,
                                 should_hide_links(in_preview),
                                 should_hide_markdown_styles(in_preview));
    }
    if (max_segs > 0) {
        segs[0].byte_offset = 0;
        segs[0].byte_len = l->len;
        return 1;
    }
    return 0;
}

static bool render_cursor_advance(void) {
    RenderCursor *c = &g_render_cursor;
    if (!c->valid) return false;

    if (c->sub_idx + 1 < c->nsegs) {
        c->sub_idx++;
        return true;
    }

    int next = c->line_idx + 1;
    while (next < c->num_lines && c->lines[next].hidden) next++;
    if (next >= c->num_lines) {
        c->at_end = true;
        c->valid = false;
        c->total_vrows = c->line_vrow + c->nsegs;
        return false;
    }

    c->line_vrow += c->nsegs;
    c->line_idx = next;
    c->sub_idx = 0;
    c->nsegs = build_render_segments(&c->lines[next], c->avail_w, c->preview,
                                     c->segs, (int)(sizeof(c->segs) / sizeof(c->segs[0])));
    if (c->nsegs <= 0) {
        c->at_end = true;
        c->valid = false;
        return false;
    }
    return true;
}

static bool render_cursor_locate(bool in_preview, int target_vrow, int avail_w) {
    RenderCursor *c = &g_render_cursor;
    Line *lines = in_preview ? g_preview.lines : E.lines;
    int num_lines = in_preview ? g_preview.num_lines : E.num_lines;

    c->valid = false;
    c->at_end = false;
    c->preview = in_preview;
    c->lines = lines;
    c->num_lines = num_lines;
    c->avail_w = avail_w;
    c->line_idx = -1;
    c->line_vrow = 0;
    c->total_vrows = -1;

    int vrow = 0;
    for (int i = 0; i < num_lines; i++) {
        if (lines[i].hidden) continue;
        int nsegs = build_render_segments(&lines[i], avail_w, in_preview,
                                          c->segs, (int)(sizeof(c->segs) / sizeof(c->segs[0])));
        if (nsegs <= 0) continue;
        if (target_vrow < vrow + nsegs) {
            c->valid = true;
            c->line_idx = i;
            c->line_vrow = vrow;
            c->sub_idx = target_vrow - vrow;
            c->nsegs = nsegs;
            return true;
        }
        vrow += nsegs;
    }

    c->at_end = true;
    c->valid = false;
    c->total_vrows = vrow;
    return false;
}

static bool render_cursor_get(bool in_preview, int target_vrow, int avail_w,
                              int *out_line_idx, int *out_sub_idx, LineSegment *out_seg) {
    RenderCursor *c = &g_render_cursor;
    Line *lines = in_preview ? g_preview.lines : E.lines;
    int num_lines = in_preview ? g_preview.num_lines : E.num_lines;

    bool same_context = c->lines == lines && c->num_lines == num_lines &&
                        c->preview == in_preview && c->avail_w == avail_w;
    int current = c->line_vrow + c->sub_idx;

    if (!same_context || !c->valid || c->at_end || target_vrow < current) {
        if (!render_cursor_locate(in_preview, target_vrow, avail_w)) return false;
    } else {
        while (c->line_vrow + c->sub_idx < target_vrow) {
            if (!render_cursor_advance()) return false;
        }
    }

    if (!c->valid || c->line_vrow + c->sub_idx != target_vrow) return false;
    if (out_line_idx) *out_line_idx = c->line_idx;
    if (out_sub_idx) *out_sub_idx = c->sub_idx;
    if (out_seg) *out_seg = c->segs[c->sub_idx];
    return true;
}

/* Render editor content row with visual soft wrap */

static void render_right_panel(ABuffer *ab, int screen_row) {
    int editor_width = E.screen_cols - E.left_width - 1;
    if (editor_width <= 0) return;
    int avail_text_w = editor_width - 7;
    if (avail_text_w <= 5) avail_text_w = 5;

    bool in_preview = (E.focus == FOCUS_LEFT && g_preview.lines != NULL);

    if (!in_preview && E.num_tabs == 0) {
        ab_append(ab, TH_BG_MAIN, -1);
        if (screen_row == 2) {
            ab_append(ab, TH_FG_MUTED, -1);
            const char *msg = "  Nenhuma nota aberta no momento.";
            ab_append(ab, msg, (int)strlen(msg));
            int pad = editor_width - 1 - utf8_width(msg);
            while (pad-- > 0) ab_append(ab, " ", 1);
        } else if (screen_row == 4) {
            ab_append(ab, TH_FG_ACCENT, -1);
            ab_append(ab, STYLE_BOLD, -1);
            const char *msg = "  👉 Clique na aba [➕ Nova Nota] acima para criar uma nota.";
            ab_append(ab, msg, (int)strlen(msg));
            ab_append(ab, STYLE_RESET, -1);
            ab_append(ab, TH_BG_MAIN, -1);
            int pad = editor_width - 1 - utf8_width(msg);
            while (pad-- > 0) ab_append(ab, " ", 1);
        } else if (screen_row == 6) {
            ab_append(ab, TH_FG_MUTED, -1);
            const char *msg = "  Ou selecione uma nota no painel esquerdo e pressione Enter.";
            ab_append(ab, msg, (int)strlen(msg));
            int pad = editor_width - 1 - utf8_width(msg);
            while (pad-- > 0) ab_append(ab, " ", 1);
        } else {
            ab_append(ab, TH_FG_MUTED, -1);
            ab_append(ab, "~     ", 6);
            int pad = editor_width - 1 - 6;
            while (pad-- > 0) ab_append(ab, " ", 1);
        }
        ab_append(ab, "\x1b[K" STYLE_RESET, -1);
        return;
    }

    Line *render_lines = in_preview ? g_preview.lines : E.lines;
    int cur_scroll_y = in_preview ? g_preview.scroll_y : E.scroll_y;
    int active_cursor_y = in_preview ? -1 : E.cursor_y;

    int target_vrow = screen_row + cur_scroll_y;
    int visible_line_idx = -1;
    int sub_idx = -1;
    LineSegment seg;
    bool found = render_cursor_get(in_preview, target_vrow, avail_text_w,
                                   &visible_line_idx, &sub_idx, &seg);

    ab_append(ab, TH_BG_MAIN, -1);

    if (found && visible_line_idx >= 0) {
        Line *l = &render_lines[visible_line_idx];
        bool is_cur_line = (!in_preview && visible_line_idx == active_cursor_y);
        if (is_cur_line && (E.mode == MODE_NORMAL || E.mode == MODE_COMMAND)) {
            ab_append(ab, TH_BG_CUR_LINE, -1);
        }

        char lnum_buf[32];
        if (sub_idx == 0) {
            if (l->heading_lvl > 0) {
                snprintf(lnum_buf, sizeof(lnum_buf), "%4d %s", visible_line_idx + 1, l->folded ? "▸" : "▾");
            } else {
                snprintf(lnum_buf, sizeof(lnum_buf), "%4d  ", visible_line_idx + 1);
            }
        } else {
            snprintf(lnum_buf, sizeof(lnum_buf), "      ");
        }

        if (is_cur_line) {
            ab_append(ab, TH_FG_H1, -1);
            ab_append(ab, STYLE_BOLD, -1);
        } else {
            ab_append(ab, TH_FG_MUTED, -1);
        }
        ab_append(ab, lnum_buf, (int)strlen(lnum_buf));
        ab_append(ab, STYLE_RESET, -1);
        if (is_cur_line && (E.mode == MODE_NORMAL || E.mode == MODE_COMMAND)) ab_append(ab, TH_BG_CUR_LINE, -1);
        else ab_append(ab, TH_BG_MAIN, -1);

        if (l->is_code_fence) {
            ab_append(ab, TH_BG_CUR_LINE, -1);
            int cols_printed = 6;
            const char *p = l->chars ? l->chars : "";
            while (*p == ' ' || *p == '\t') {
                ab_append(ab, p++, 1);
                cols_printed++;
            }
            if (strncmp(p, "```", 3) == 0) {
                ab_append(ab, TH_FG_H3, -1);
                ab_append(ab, STYLE_BOLD, -1);
                ab_append(ab, "```", 3);
                cols_printed += 3;
                p += 3;
                if (*p) {
                    ab_append(ab, STYLE_RESET, -1);
                    ab_append(ab, TH_BG_CUR_LINE, -1);
                    ab_append(ab, TH_FG_YELLOW, -1);
                    ab_append(ab, STYLE_BOLD, -1);
                    while (*p && cols_printed < editor_width - 1) {
                        ab_append(ab, p++, 1);
                        cols_printed++;
                    }
                }
            }
            ab_append(ab, STYLE_RESET, -1);
            ab_append(ab, TH_BG_CUR_LINE, -1);
            int pad = editor_width - 1 - cols_printed;
            while (pad-- > 0) ab_append(ab, " ", 1);
            ab_append(ab, "\x1b[K", 3);
            ab_append(ab, STYLE_RESET, -1);
            return;
        }

        if (l->in_code_block) {
            ab_append(ab, TH_BG_CUR_LINE, -1);
            ab_append(ab, TH_FG_MAIN, -1);
            int cols_printed = 6;
            render_code_line(ab, l, &cols_printed, editor_width - 1);
            ab_append(ab, STYLE_RESET, -1);
            ab_append(ab, TH_BG_CUR_LINE, -1);
            int pad = editor_width - 1 - cols_printed;
            while (pad-- > 0) ab_append(ab, " ", 1);
            ab_append(ab, "\x1b[K", 3);
            ab_append(ab, STYLE_RESET, -1);
            return;
        }

        int cols_printed = 6;

        LinkSpan spans[16];
        int nspans = should_hide_links(in_preview) ? find_links_in_line(l, spans, 16) : 0;

        MarkdownStyleSpan style_spans[16];
        int nstyles = (!l->is_code_fence) ? find_markdown_styles(l, style_spans, 16) : 0;
        bool hide_styles = should_hide_markdown_styles(in_preview);

        const char *line_fg = TH_FG_MAIN;
        if (l->heading_lvl == 1) line_fg = TH_FG_H1;
        else if (l->heading_lvl == 2) line_fg = TH_FG_H2;
        else if (l->heading_lvl >= 3) line_fg = TH_FG_H3;

        ab_append(ab, line_fg, -1);
        if (l->heading_lvl >= 1) ab_append(ab, STYLE_BOLD, -1);

        int bullet_pos = -1;
        char bullet_char = 0;
        bool is_bullet = is_bullet_list_line(l, &bullet_pos, &bullet_char);

        int start_char = seg.byte_offset;
        int end_char = seg.byte_offset + seg.byte_len;

        for (int char_idx = start_char; char_idx < end_char && cols_printed < editor_width - 1; ) {
            /* Bullet list marker (* or -) rendered in distinct accent color and bold */
            if (is_bullet && char_idx == bullet_pos) {
                const char *bullet_color = (bullet_char == '*') ? TH_FG_ACCENT : TH_FG_YELLOW;
                ab_append(ab, bullet_color, -1);
                ab_append(ab, STYLE_BOLD, -1);
                ab_append(ab, &l->chars[char_idx], 1);
                ab_append(ab, "\x1b[22m", 4);
                ab_append(ab, line_fg, -1);
                cols_printed += 1;
                char_idx += 1;
                continue;
            }
            if (!in_preview && ((E.mode == MODE_VISUAL_CHAR || E.mode == MODE_VISUAL_LINE) || (E.mode == MODE_INSERT && E.has_selection))) {
                int start_y = E.visual_start_y, end_y = E.cursor_y;
                int start_x = E.visual_start_x, end_x = E.cursor_x;
                if (start_y > end_y || (start_y == end_y && start_x > end_x)) {
                    int ty = start_y; start_y = end_y; end_y = ty;
                    int tx = start_x; start_x = end_x; end_x = tx;
                }
                bool in_sel = false;
                if (E.mode == MODE_VISUAL_LINE) {
                    if (visible_line_idx >= start_y && visible_line_idx <= end_y) in_sel = true;
                } else {
                    if (visible_line_idx > start_y && visible_line_idx < end_y) in_sel = true;
                    else if (visible_line_idx == start_y && visible_line_idx == end_y) {
                        if (char_idx >= start_x && char_idx <= end_x) in_sel = true;
                    } else if (visible_line_idx == start_y && char_idx >= start_x) in_sel = true;
                    else if (visible_line_idx == end_y && char_idx <= end_x) in_sel = true;
                }
                if (in_sel) ab_append(ab, TH_BG_SELECT, -1);
                else {
                    if (is_cur_line && (E.mode == MODE_NORMAL || E.mode == MODE_COMMAND)) ab_append(ab, TH_BG_CUR_LINE, -1);
                    else ab_append(ab, TH_BG_MAIN, -1);
                }
            }

            if (should_hide_links(in_preview) && nspans > 0) {
                bool handled = false;
                for (int s = 0; s < nspans; s++) {
                    if (char_idx >= spans[s].start_raw && char_idx < spans[s].end_raw) {
                        if (char_idx == spans[s].start_raw) {
                        int full_w = utf8_width(spans[s].text) + (spans[s].is_wikilink ? 2 : 0);
                        if (cols_printed + full_w > editor_width - 1) {
                            int avail = editor_width - 1 - cols_printed - (spans[s].is_wikilink ? 2 : 0);
                            if (avail > 0) {
                                char tlink[256];
                                utf8_truncate_display(spans[s].text, avail, tlink, sizeof(tlink));
                                if (spans[s].is_wikilink) {
                                    ab_append(ab, TH_FG_WIKI, -1);
                                    ab_append(ab, STYLE_BOLD "[", -1);
                                    ab_append(ab, tlink, (int)strlen(tlink));
                                    ab_append(ab, "]", 1); ab_append(ab, "\x1b[22m", 4);
                                    cols_printed += utf8_width(tlink) + 2;
                                } else {
                                    ab_append(ab, TH_FG_LINK, -1);
                                    ab_append(ab, STYLE_UNDERLINE, -1);
                                    ab_append(ab, tlink, (int)strlen(tlink));
                                    ab_append(ab, STYLE_RESET, -1);
                                    cols_printed += utf8_width(tlink);
                                }
                            }
                            cols_printed = editor_width - 1;
                        } else {
                            if (spans[s].is_wikilink) {
                                ab_append(ab, TH_FG_WIKI, -1);
                                ab_append(ab, STYLE_BOLD "[", -1);
                                ab_append(ab, spans[s].text, (int)strlen(spans[s].text));
                                ab_append(ab, "]", 1); ab_append(ab, "\x1b[22m", 4);
                                cols_printed += utf8_width(spans[s].text) + 2;
                            } else {
                                ab_append(ab, TH_FG_LINK, -1);
                                ab_append(ab, STYLE_UNDERLINE, -1);
                                ab_append(ab, spans[s].text, (int)strlen(spans[s].text));
                                ab_append(ab, STYLE_RESET, -1);
                                cols_printed += utf8_width(spans[s].text);
                            }
                        }
                        if (is_cur_line && (E.mode == MODE_NORMAL || E.mode == MODE_COMMAND)) ab_append(ab, TH_BG_CUR_LINE, -1);
                        else ab_append(ab, TH_BG_MAIN, -1);

                        /* Restore foreground color after link */
                        ab_append(ab, line_fg, -1);
                        if (l->heading_lvl >= 1) ab_append(ab, STYLE_BOLD, -1);
                        }
                        char_idx = spans[s].end_raw;
                        handled = true;
                        break;
                    }
                }
                if (handled) continue;
            }

            int cur_style_idx = -1;
            for (int st = 0; st < nstyles; st++) {
                if (char_idx >= style_spans[st].open_start &&
                    char_idx < style_spans[st].close_start + style_spans[st].close_len) {
                    cur_style_idx = st;
                    break;
                }
            }

            if (cur_style_idx != -1 && hide_styles) {
                MarkdownStyleSpan *msp = &style_spans[cur_style_idx];
                if (char_idx >= msp->open_start && char_idx < msp->content_start) {
                    if (msp->type == STYLE_SPAN_BOLD || msp->type == STYLE_SPAN_BOLD_ITALIC) ab_append(ab, STYLE_BOLD, -1);
                    if (msp->type == STYLE_SPAN_ITALIC || msp->type == STYLE_SPAN_BOLD_ITALIC) ab_append(ab, STYLE_ITALIC, -1);
                    ab_append(ab, line_fg, -1);
                    char_idx = msp->content_start;
                    continue;
                } else if (char_idx >= msp->close_start && char_idx < msp->close_start + msp->close_len) {
                    ab_append(ab, "\x1b[22;23m", -1);
                    ab_append(ab, line_fg, -1);
                    if (l->heading_lvl >= 1) ab_append(ab, STYLE_BOLD, -1);
                    char_idx = msp->close_start + msp->close_len;
                    continue;
                }
            }

            char c = l->chars[char_idx];
            if (c == '\t') {
                int tab_w = 4;
                if (cols_printed + tab_w > editor_width - 1) {
                    tab_w = editor_width - 1 - cols_printed;
                }
                while (tab_w-- > 0) {
                    ab_append(ab, " ", 1);
                    cols_printed++;
                }
                char_idx++;
            } else {
                int ch_len = 0;
                int cp = utf8_decode_codepoint(&l->chars[char_idx], &ch_len);
                int cw = utf8_codepoint_width(cp);
                if (char_idx + ch_len < l->len && (unsigned char)l->chars[char_idx + ch_len] == 0xEF) {
                    int nb = 0;
                    int ncp = utf8_decode_codepoint(&l->chars[char_idx + ch_len], &nb);
                    if (ncp == 0xFE0F) {
                        cw = 2;
                        ch_len += nb;
                    }
                }
                if (cols_printed + cw > editor_width - 1) break;

                if (cur_style_idx != -1) {
                    MarkdownStyleSpan *msp = &style_spans[cur_style_idx];
                    if (!hide_styles && char_idx >= msp->open_start && char_idx < msp->content_start) {
                        /* Opening delimiter: subtle muted color in Insert mode */
                        ab_append(ab, TH_FG_MUTED, -1);
                        ab_append(ab, &l->chars[char_idx], ch_len);
                        if (char_idx + ch_len >= msp->content_start) {
                            if (msp->type == STYLE_SPAN_BOLD || msp->type == STYLE_SPAN_BOLD_ITALIC) ab_append(ab, STYLE_BOLD, -1);
                            if (msp->type == STYLE_SPAN_ITALIC || msp->type == STYLE_SPAN_BOLD_ITALIC) ab_append(ab, STYLE_ITALIC, -1);
                            ab_append(ab, line_fg, -1);
                        }
                    } else if (char_idx >= msp->content_start && char_idx < msp->content_end) {
                        /* Styled content: apply bold / italic and line_fg */
                        if (msp->type == STYLE_SPAN_BOLD || msp->type == STYLE_SPAN_BOLD_ITALIC) ab_append(ab, STYLE_BOLD, -1);
                        if (msp->type == STYLE_SPAN_ITALIC || msp->type == STYLE_SPAN_BOLD_ITALIC) ab_append(ab, STYLE_ITALIC, -1);
                        ab_append(ab, line_fg, -1);
                        ab_append(ab, &l->chars[char_idx], ch_len);
                        if (char_idx + ch_len >= msp->content_end) {
                            ab_append(ab, "\x1b[22;23m", -1);
                            ab_append(ab, line_fg, -1);
                            if (l->heading_lvl >= 1) ab_append(ab, STYLE_BOLD, -1);
                            if (!hide_styles) ab_append(ab, TH_FG_MUTED, -1);
                        }
                    } else if (!hide_styles && char_idx >= msp->close_start && char_idx < msp->close_start + msp->close_len) {
                        /* Closing delimiter: subtle muted color in Insert mode */
                        ab_append(ab, TH_FG_MUTED, -1);
                        ab_append(ab, &l->chars[char_idx], ch_len);
                        if (char_idx + ch_len >= msp->close_start + msp->close_len) {
                            ab_append(ab, "\x1b[22;23m", -1);
                            ab_append(ab, line_fg, -1);
                            if (l->heading_lvl >= 1) ab_append(ab, STYLE_BOLD, -1);
                        }
                    }
                } else {
                    ab_append(ab, line_fg, -1);
                    if (l->heading_lvl >= 1) ab_append(ab, STYLE_BOLD, -1);
                    ab_append(ab, &l->chars[char_idx], ch_len);
                }

                cols_printed += cw;
                char_idx += ch_len;
            }
        }
        ab_append(ab, STYLE_RESET, -1);
        if (is_cur_line && (E.mode == MODE_NORMAL || E.mode == MODE_COMMAND)) ab_append(ab, TH_BG_CUR_LINE, -1);
        else ab_append(ab, TH_BG_MAIN, -1);

        int pad = editor_width - 1 - cols_printed;
        while (pad-- > 0) ab_append(ab, " ", 1);
        ab_append(ab, "\x1b[K", 3);
    } else {
        ab_append(ab, TH_BG_MAIN, -1);
        ab_append(ab, TH_FG_MUTED, -1);
        ab_append(ab, "~     ", 6);
        int pad = editor_width - 1 - 6;
        while (pad-- > 0) ab_append(ab, " ", 1);
        ab_append(ab, "\x1b[K", 3);
    }
    ab_append(ab, STYLE_RESET, -1);
}

static void render_status_bar(ABuffer *ab) {
    /* Mode badge */
    const char *badge = " NORMAL ";
    const char *badge_color = TH_BG_BADGE_NOR;
    if (E.mode == MODE_INSERT) {
        badge = E.has_selection ? " INSERT [SEL] " : " INSERT ";
        badge_color = TH_BG_BADGE_INS;
    } else if (E.mode == MODE_VISUAL_CHAR) {
        badge = " VISUAL ";
        badge_color = TH_BG_BADGE_VIS;
    } else if (E.mode == MODE_VISUAL_LINE) {
        badge = " V-LINE ";
        badge_color = TH_BG_BADGE_VIS;
    } else if (E.mode == MODE_COMMAND) {
        badge = " COMMAND ";
        badge_color = TH_BG_BADGE_CMD;
    } else if (E.mode == MODE_SEARCH_LEFT) {
        badge = " SEARCH ";
        badge_color = TH_BG_BADGE_CMD;
    } else if (E.mode == MODE_CONFIRM_DELETE_NOTE) {
        badge = " DELETE? ";
        badge_color = TH_BG_BADGE_CMD;
    } else if (E.mode == MODE_PROMPT_NEW_NOTE) {
        badge = " NEW NOTE ";
        badge_color = TH_BG_BADGE_INS;
    } else if (E.mode == MODE_PROMPT_RENAME_NOTE) {
        badge = " RENAME ";
        badge_color = TH_BG_BADGE_CMD;
    }

    char note_desc[1024] = {0};
    int note_desc_w = 0;
    char preview_name[256] = {0};
    if (E.focus == FOCUS_LEFT && g_preview.filepath[0]) {
        int pid_ = note_id_from_path(g_preview.filepath);
        if (pid_ >= 0) {
            if (!db_get_name(pid_, preview_name, sizeof(preview_name))) {
                safe_strcpy(preview_name, sizeof(preview_name), g_preview.filepath);
            } else if (!strchr(preview_name, '.')) {
                strncat(preview_name, ".md", sizeof(preview_name) - strlen(preview_name) - 1);
            }
        } else {
            const char *pname = strrchr(g_preview.filepath, PATH_SEP);
#ifndef _WIN32
            if (!pname) pname = strrchr(g_preview.filepath, '/');
#endif
            safe_strcpy(preview_name, sizeof(preview_name), pname ? pname + 1 : g_preview.filepath);
        }
        snprintf(note_desc, sizeof(note_desc), "[Prévia] %s (Enter ou duplo clique p/ abrir)", preview_name);
    } else {
        snprintf(note_desc, sizeof(note_desc), "%s%s",
                 E.num_tabs == 0 ? "[Sem Abas Abertas]" : (E.cur_filename[0] ? E.cur_filename : "[Sem Arquivo]"),
                 E.modified ? " [+]" : "");
    }
    note_desc_w = utf8_width(note_desc);

    char rinfo[64];
    if (E.focus == FOCUS_LEFT && g_preview.lines) {
        snprintf(rinfo, sizeof(rinfo), " %dL (prévia)", g_preview.num_lines);
    } else {
        snprintf(rinfo, sizeof(rinfo), " %d:%d %dL", E.cursor_y + 1, E.cursor_x + 1, E.num_lines);
    }
    int rlen = utf8_width(rinfo);

    bool compact_btns = (E.force_status_bar_wrap == 1) || (E.force_status_bar_wrap == 0 && E.screen_cols < 108);

    const char *new_btn = (!compact_btns) ? " [➕ Nova] " : " [➕] ";
    int new_w = utf8_width(new_btn);

    const char *ren_btn = (!compact_btns) ? " [✏️ Renomear] " : " [✏️] ";
    int ren_w = utf8_width(ren_btn);

    const char *del_btn = (!compact_btns) ? " [🗑️ Excluir] " : " [🗑️] ";
    int del_w = utf8_width(del_btn);

    char wrap_btn[32];
    if (!compact_btns) {
        snprintf(wrap_btn, sizeof(wrap_btn), " [%s] ", E.wrap_mode ? "Wrap:ON" : "Wrap:OFF");
    } else {
        snprintf(wrap_btn, sizeof(wrap_btn), " [W:%s] ", E.wrap_mode ? "ON" : "OFF");
    }
    int wrap_w = utf8_width(wrap_btn);

    const char *find_btn = (!compact_btns) ? " [🔍 Buscar] " : " [🔍] ";
    int find_w = utf8_width(find_btn);

    const char *rep_btn = (!compact_btns) ? " [🔄 Subst] " : " [🔄] ";
    int rep_w = utf8_width(rep_btn);

    char theme_btn[64];
    if (!compact_btns) {
        snprintf(theme_btn, sizeof(theme_btn), " [🎨 %s] ", themes[E.current_theme].name);
    } else {
        snprintf(theme_btn, sizeof(theme_btn), " [🎨] ");
    }
    int tbtn_w = utf8_width(theme_btn);

    const char *life_btn = (!compact_btns) ? " [🧬 Life] " : " [🧬] ";
    int life_w = utf8_width(life_btn);

    const char *imp_btn = (!compact_btns) ? " [⬇ Importar] " : " [⬇] ";
    const char *exp_btn = (!compact_btns) ? " [⬆ Exportar] " : " [⬆] ";
    int imp_w = utf8_width(imp_btn);
    int exp_w = utf8_width(exp_btn);

    int btns_w = new_w + ren_w + del_w + wrap_w + find_w + rep_w + tbtn_w + life_w + imp_w + exp_w;

    /* -----------------------------------------------------------------
     * LINE 1: Status Bar (Mode Badge + Markdown Note Name + Cursor Info)
     * ----------------------------------------------------------------- */
    ab_append(ab, TH_BG_STATUS, -1);

    /* Badge */
    ab_append(ab, badge_color, -1);
    ab_append(ab, TH_FG_DARK, -1);
    ab_append(ab, STYLE_BOLD, -1);
    ab_append(ab, badge, (int)strlen(badge));
    ab_append(ab, STYLE_RESET, -1);
    ab_append(ab, TH_BG_STATUS, -1);
    ab_append(ab, " ", 1);

    int used1 = (int)strlen(badge) + 1;

    if (E.focus == FOCUS_LEFT && g_preview.filepath[0]) {
        ab_append(ab, TH_FG_YELLOW, -1);
        ab_append(ab, "[Prévia] ", -1);
        ab_append(ab, TH_FG_WHITE, -1);
        ab_append(ab, preview_name[0] ? preview_name : g_preview.filepath, -1);
        ab_append(ab, TH_FG_MUTED, -1);
        ab_append(ab, " (Enter/Duplo clique p/ abrir)", -1);
    } else {
        ab_append(ab, TH_FG_WHITE, -1);
        ab_append(ab, STYLE_BOLD, -1);
        ab_append(ab, E.cur_filename[0] ? E.cur_filename : "[Sem Arquivo]", -1);
        ab_append(ab, STYLE_RESET, -1);
        ab_append(ab, TH_BG_STATUS, -1);
        if (E.modified) {
            ab_append(ab, TH_FG_WARN, -1);
            ab_append(ab, " [+]", -1);
        }
    }
    used1 += note_desc_w;

    int space1 = E.screen_cols - used1 - rlen;
    if (space1 < 1) space1 = 1;
    while (space1-- > 0) ab_append(ab, " ", 1);

    ab_append(ab, TH_FG_MUTED, -1);
    ab_append(ab, rinfo, (int)strlen(rinfo));
    ab_append(ab, "\x1b[K" STYLE_RESET "\r\n", -1);

    /* -----------------------------------------------------------------
     * LINE 2: Exclusive Action Buttons Bar (Centered & Fully Clickable)
     * ----------------------------------------------------------------- */
    ab_append(ab, TH_BG_SELECT, -1);
    int cur_col = 1;

    int btn_margin = (E.screen_cols - btns_w) / 2;
    if (btn_margin < 1) btn_margin = 1;
    for (int k = 1; k < btn_margin; k++) {
        ab_append(ab, " ", 1);
        cur_col++;
    }

    E.btn_new_start = cur_col;
    ab_append(ab, TH_BG_SELECT, -1);
    ab_append(ab, TH_FG_ACCENT, -1);
    ab_append(ab, STYLE_BOLD, -1);
    ab_append(ab, new_btn, (int)strlen(new_btn));
    cur_col += new_w;
    E.btn_new_end = cur_col;

    E.btn_rename_start = cur_col;
    ab_append(ab, TH_BG_SELECT, -1);
    ab_append(ab, TH_FG_WHITE, -1);
    ab_append(ab, STYLE_BOLD, -1);
    ab_append(ab, ren_btn, (int)strlen(ren_btn));
    cur_col += ren_w;
    E.btn_rename_end = cur_col;

    E.btn_delete_start = cur_col;
    ab_append(ab, TH_BG_SELECT, -1);
    ab_append(ab, TH_FG_WARN, -1);
    ab_append(ab, STYLE_BOLD, -1);
    ab_append(ab, del_btn, (int)strlen(del_btn));
    cur_col += del_w;
    E.btn_delete_end = cur_col;

    E.btn_wrap_start = cur_col;
    ab_append(ab, TH_BG_SELECT, -1);
    ab_append(ab, TH_FG_WHITE, -1);
    ab_append(ab, STYLE_BOLD, -1);
    ab_append(ab, wrap_btn, (int)strlen(wrap_btn));
    cur_col += wrap_w;
    E.btn_wrap_end = cur_col;

    E.btn_find_start = cur_col;
    ab_append(ab, find_btn, (int)strlen(find_btn));
    cur_col += find_w;
    E.btn_find_end = cur_col;

    E.btn_replace_start = cur_col;
    ab_append(ab, rep_btn, (int)strlen(rep_btn));
    cur_col += rep_w;
    E.btn_replace_end = cur_col;

    E.theme_btn_start_col = cur_col;
    ab_append(ab, theme_btn, (int)strlen(theme_btn));
    cur_col += tbtn_w;
    E.theme_btn_end_col = cur_col;

    E.btn_life_start = cur_col;
    ab_append(ab, TH_BG_SELECT, -1);
    ab_append(ab, TH_FG_H1, -1);
    ab_append(ab, STYLE_BOLD, -1);
    ab_append(ab, life_btn, (int)strlen(life_btn));
    cur_col += life_w;
    E.btn_life_end = cur_col;

    /* [⬇ Importar]: pasta de arquivos .md  ->  banco */
    E.btn_import_start = cur_col;
    ab_append(ab, TH_BG_SELECT, -1);
    ab_append(ab, TH_FG_H2, -1);
    ab_append(ab, STYLE_BOLD, -1);
    ab_append(ab, imp_btn, (int)strlen(imp_btn));
    cur_col += imp_w;
    E.btn_import_end = cur_col;

    /* [⬆ Exportar]: banco  ->  pasta de arquivos .md */
    E.btn_export_start = cur_col;
    ab_append(ab, TH_BG_SELECT, -1);
    ab_append(ab, TH_FG_H2, -1);
    ab_append(ab, STYLE_BOLD, -1);
    ab_append(ab, exp_btn, (int)strlen(exp_btn));
    cur_col += exp_w;
    E.btn_export_end = cur_col;

    int pad2 = E.screen_cols - cur_col + 1;
    while (pad2-- > 0) ab_append(ab, " ", 1);
    ab_append(ab, "\x1b[K" STYLE_RESET, -1);
}

static void render_command_line(ABuffer *ab) {
    ab_append(ab, TH_BG_STATUS, -1);

    if (E.mode == MODE_COMMAND) {
        ab_append(ab, TH_FG_H1, -1);
        ab_append(ab, STYLE_BOLD ":", -1);
        ab_append(ab, TH_FG_WHITE, -1);
        ab_append(ab, E.cmd_buf, (int)strlen(E.cmd_buf));
        int pad = E.screen_cols - 1 - (int)strlen(E.cmd_buf);
        while (pad-- > 0) ab_append(ab, " ", 1);
    } else if (E.mode == MODE_CONFIRM_DELETE_NOTE) {
        ab_append(ab, TH_FG_WARN, -1);
        ab_append(ab, STYLE_BOLD, -1);
        char prompt[512];
        int actual = (E.num_filtered > 0 && E.selected_note_idx < E.num_filtered) ? E.filtered_indices[E.selected_note_idx] : -1;
        const char *tname = (actual >= 0) ? E.notes[actual].title : "esta nota";
        snprintf(prompt, sizeof(prompt), " Deseja realmente excluir a nota '%s'? (s/n): ", tname);
        ab_append(ab, prompt, -1);
        ab_append(ab, STYLE_RESET, -1);
        int plen = utf8_width(prompt);
        int pad = E.screen_cols - plen;
        while (pad-- > 0) ab_append(ab, " ", 1);
    } else if (E.mode == MODE_PROMPT_NEW_NOTE) {
        ab_append(ab, TH_FG_ACCENT, -1);
        ab_append(ab, STYLE_BOLD, -1);
        const char *prompt = " 📝 Nova nota: ";
        ab_append(ab, prompt, -1);
        ab_append(ab, STYLE_RESET, -1);
        ab_append(ab, TH_FG_WHITE, -1);
        ab_append(ab, E.prompt_buf, (int)strlen(E.prompt_buf));
        int plen = utf8_width(prompt) + utf8_width(E.prompt_buf);
        int pad = E.screen_cols - plen;
        while (pad-- > 0) ab_append(ab, " ", 1);
    } else if (E.mode == MODE_PROMPT_RENAME_NOTE) {
        ab_append(ab, TH_FG_YELLOW, -1);
        ab_append(ab, STYLE_BOLD, -1);
        const char *prompt = " ✏️  Renomear nota: ";
        ab_append(ab, prompt, -1);
        ab_append(ab, STYLE_RESET, -1);
        ab_append(ab, TH_FG_WHITE, -1);
        ab_append(ab, E.prompt_buf, (int)strlen(E.prompt_buf));
        int plen = utf8_width(prompt) + utf8_width(E.prompt_buf);
        int pad = E.screen_cols - plen;
        while (pad-- > 0) ab_append(ab, " ", 1);
    } else if (E.mode == MODE_PROMPT_FIND) {
        ab_append(ab, TH_FG_H1, -1);
        ab_append(ab, STYLE_BOLD, -1);
        const char *prompt = " 🔍 Localizar na nota: ";
        ab_append(ab, prompt, -1);
        ab_append(ab, STYLE_RESET, -1);
        ab_append(ab, TH_FG_WHITE, -1);
        ab_append(ab, E.prompt_buf, (int)strlen(E.prompt_buf));
        int plen = utf8_width(prompt) + utf8_width(E.prompt_buf);
        int pad = E.screen_cols - plen;
        while (pad-- > 0) ab_append(ab, " ", 1);
    } else if (E.mode == MODE_PROMPT_REPLACE_FIND) {
        ab_append(ab, TH_FG_WARN, -1);
        ab_append(ab, STYLE_BOLD, -1);
        const char *prompt = " 🔄 Localizar para substituir: ";
        ab_append(ab, prompt, -1);
        ab_append(ab, STYLE_RESET, -1);
        ab_append(ab, TH_FG_WHITE, -1);
        ab_append(ab, E.prompt_buf, (int)strlen(E.prompt_buf));
        int plen = utf8_width(prompt) + utf8_width(E.prompt_buf);
        int pad = E.screen_cols - plen;
        while (pad-- > 0) ab_append(ab, " ", 1);
    } else if (E.mode == MODE_PROMPT_REPLACE_WITH) {
        ab_append(ab, TH_FG_ACCENT, -1);
        ab_append(ab, STYLE_BOLD, -1);
        char prompt[300];
        snprintf(prompt, sizeof(prompt), " 🔄 Substituir '%s' por: ", E.replace_find_buf);
        ab_append(ab, prompt, -1);
        ab_append(ab, STYLE_RESET, -1);
        ab_append(ab, TH_FG_WHITE, -1);
        ab_append(ab, E.prompt_buf, (int)strlen(E.prompt_buf));
        int plen = utf8_width(prompt) + utf8_width(E.prompt_buf);
        int pad = E.screen_cols - plen;
        while (pad-- > 0) ab_append(ab, " ", 1);
    } else if (E.mode == MODE_PROMPT_IMPORT_DIR) {
        const char *prompt = PROMPT_IMPORT_LABEL;
        ab_append(ab, TH_FG_ACCENT, -1);
        ab_append(ab, STYLE_BOLD, -1);
        ab_append(ab, prompt, -1);
        ab_append(ab, STYLE_RESET, -1);
        ab_append(ab, TH_FG_WHITE, -1);
        ab_append(ab, E.prompt_buf, (int)strlen(E.prompt_buf));
        int plen = utf8_width(prompt) + utf8_width(E.prompt_buf);
        int pad = E.screen_cols - plen;
        while (pad-- > 0) ab_append(ab, " ", 1);
    } else if (E.mode == MODE_PROMPT_EXPORT_DIR) {
        const char *prompt = PROMPT_EXPORT_LABEL;
        ab_append(ab, TH_FG_ACCENT, -1);
        ab_append(ab, STYLE_BOLD, -1);
        ab_append(ab, prompt, -1);
        ab_append(ab, STYLE_RESET, -1);
        ab_append(ab, TH_FG_WHITE, -1);
        ab_append(ab, E.prompt_buf, (int)strlen(E.prompt_buf));
        int plen = utf8_width(prompt) + utf8_width(E.prompt_buf);
        int pad = E.screen_cols - plen;
        while (pad-- > 0) ab_append(ab, " ", 1);
    } else {
        time_t now = time(NULL);
        if (E.status_msg[0] && now - E.status_msg_time < 5) {
            ab_append(ab, TH_FG_YELLOW, -1);
            ab_append(ab, " ", 1);
            ab_append(ab, E.status_msg, (int)strlen(E.status_msg));
            int pad = E.screen_cols - 1 - (int)strlen(E.status_msg);
            while (pad-- > 0) ab_append(ab, " ", 1);
        } else {
            const char *hint = (E.focus == FOCUS_LEFT) ?
                " [j/k]: Navegar | [Enter]: Abrir | [n]: Nova | [r]: Renomear | [d]: Excluir | [/]: Buscar | [Tab]: Editor" :
                " [gt/gT]: Abas | [Ctrl+T]: Tema | [Roda do mouse na aba]: Fechar aba | [:w/:q]: Comandos";
            ab_append(ab, TH_FG_MUTED, -1);
            int hlen = (int)strlen(hint);
            if (hlen > E.screen_cols) hlen = E.screen_cols;
            ab_append(ab, hint, hlen);
            int pad = E.screen_cols - 1 - hlen;
            while (pad-- > 0) ab_append(ab, " ", 1);
        }
    }
    ab_append(ab, STYLE_RESET, -1);
}

static void editor_scroll(void) {
    int editor_width = E.screen_cols - E.left_width - 1;
    int avail_text_w = editor_width - 7;
    if (avail_text_w <= 5) avail_text_w = 5;

    int sb_lines = (E.status_bar_lines >= 1) ? E.status_bar_lines : 1;
    int visible_rows = E.screen_rows - (1 + sb_lines) - 1; /* content rows excluding tab bar */
    if (visible_rows <= 0) visible_rows = 1;

    int cur_vrow = 0, cur_vcol = 0;
    get_cursor_visual_pos(avail_text_w, &cur_vrow, &cur_vcol);

    if (E.scroll_y < 0) E.scroll_y = 0;

    /* Clamp scroll_y sem varrer o documento inteiro: conta apenas as linhas
     * visuais a partir do scroll atual. Se a tela enche, nao ha o que limitar;
     * se o documento termina antes, o total e conhecido. */
    int total_vrows = -1;
    {
        render_cursor_reset();
        int probe = E.scroll_y;
        int counted = 0;
        LineSegment probe_seg;
        int probe_line = -1, probe_sub = -1;
        while (counted < visible_rows &&
               render_cursor_get(false, probe, avail_text_w, &probe_line, &probe_sub, &probe_seg)) {
            counted++;
            probe++;
        }
        if (counted < visible_rows) total_vrows = g_render_cursor.total_vrows;
    }
    if (total_vrows >= 0) {
        int max_scroll = total_vrows - visible_rows;
        if (max_scroll < 0) max_scroll = 0;
        if (E.scroll_y > max_scroll) {
            E.scroll_y = max_scroll;
        }
    }

    if (cur_vrow < E.scroll_y) {
        E.scroll_y = cur_vrow;
    }
    if (cur_vrow >= E.scroll_y + visible_rows) {
        E.scroll_y = cur_vrow - visible_rows + 1;
    }
    if (E.scroll_y < 0) E.scroll_y = 0;

    /* Clamp preview scroll_y as well */
    if (E.focus == FOCUS_LEFT && g_preview.lines) {
        if (g_preview.scroll_y < 0) g_preview.scroll_y = 0;
        render_cursor_reset();
        int probe = g_preview.scroll_y;
        int counted = 0;
        LineSegment probe_seg;
        int probe_line = -1, probe_sub = -1;
        while (counted < visible_rows &&
               render_cursor_get(true, probe, avail_text_w, &probe_line, &probe_sub, &probe_seg)) {
            counted++;
            probe++;
        }
        if (counted < visible_rows && g_render_cursor.total_vrows >= 0) {
            int max_pscroll = g_render_cursor.total_vrows - visible_rows;
            if (max_pscroll < 0) max_pscroll = 0;
            if (g_preview.scroll_y > max_pscroll) {
                g_preview.scroll_y = max_pscroll;
            }
        }
    }

    /* Clamp sidebar scroll_y */
    int max_sidebar_visible = E.screen_rows - (1 + sb_lines) - 4;
    if (max_sidebar_visible <= 0) max_sidebar_visible = 1;
    if (E.selected_note_idx < E.sidebar_scroll_y) {
        E.sidebar_scroll_y = E.selected_note_idx;
    }
    if (E.selected_note_idx >= E.sidebar_scroll_y + max_sidebar_visible) {
        E.sidebar_scroll_y = E.selected_note_idx - max_sidebar_visible + 1;
    }
    if (E.sidebar_scroll_y > E.num_filtered - max_sidebar_visible) {
        E.sidebar_scroll_y = E.num_filtered - max_sidebar_visible;
    }
    if (E.sidebar_scroll_y < 0) E.sidebar_scroll_y = 0;
}

/* Escrita completa no stdout. write()/WriteFile() podem aceitar apenas parte
 * do buffer (console, pipe, sinal); ignorar o retorno deixava o quadro pela
 * metade. Este laco trata EINTR e escribas parciais. */
static bool write_stdout_all(const char *data, int len) {
    if (!data || len <= 0) return true;
    int off = 0;
    while (off < len) {
        int chunk = len - off;
        if (chunk > 32768) chunk = 32768;
#ifdef _WIN32
        DWORD written = 0;
        if (!WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), data + off, (DWORD)chunk,
                       &written, NULL) || written == 0) {
            return false;
        }
        off += (int)written;
#else
        ssize_t n = write(STDOUT_FILENO, data + off, (size_t)chunk);
        if (n < 0) {
            if (errno == EINTR) continue;
            return false;
        }
        if (n == 0) return false;
        off += (int)n;
#endif
    }
    return true;
}

static void refresh_screen(void) {
    /* Dirty-flag: refresh_screen() e o unico ponto que de fato desenha, entao e
     * aqui que a flag e baixada e o batimento de manutencao e reiniciado. */
    g_needs_redraw = 0;
    g_last_heartbeat_draw = get_current_time_ms();

    int prev_rows = E.screen_rows;
    int prev_cols = E.screen_cols;
    get_window_size(&E.screen_rows, &E.screen_cols);

    bool size_changed = (prev_rows != E.screen_rows || prev_cols != E.screen_cols || g_window_resized);
    if (size_changed) {
        g_screen_clear_needed = 1;
        g_window_resized = 0;
    }

    /* Clamp left panel width if window shrank */
    if (E.screen_cols < 60) {
        if (E.left_width > E.screen_cols / 2) E.left_width = E.screen_cols / 2;
    }
    if (E.left_width < MIN_LEFT_WIDTH && E.screen_cols >= 50) {
        E.left_width = MIN_LEFT_WIDTH;
    }
    if (E.left_width >= E.screen_cols - 15) {
        E.left_width = E.screen_cols - 15;
        if (E.left_width < 10) E.left_width = 10;
    }

    /* Buttons are ALWAYS on an exclusive bar: status_bar_lines is always 2 */
    E.status_bar_lines = 2;

    /* Calculate scroll after window dimensions and status bar lines are resolved */
    editor_scroll();

    ABuffer ab;
    ab_init(&ab);

    if (g_screen_clear_needed || size_changed) {
        ab_append(&ab, "\x1b[2J\x1b[3J\x1b[H", 11);
        g_screen_clear_needed = 0;
    } else {
        ab_append(&ab, "\x1b[H", 3);
    }

    ab_append(&ab, "\x1b[?25l", 6);

    render_cursor_reset();
    int editor_rows = E.screen_rows - (1 + E.status_bar_lines);

    for (int r = 0; r < editor_rows; r++) {
        render_left_panel(&ab, r);
        if (r == 0) {
            render_tab_bar(&ab);
        } else {
            render_right_panel(&ab, r - 1);
        }
        ab_append(&ab, "\r\n", 2);
    }

    render_status_bar(&ab);
    ab_append(&ab, "\r\n", 2);
    render_command_line(&ab);

    /* Position Cursor */
    if (E.mode == MODE_COMMAND) {
        int cur_pos = E.cmd_cursor;
        int clen = (int)strlen(E.cmd_buf);
        if (cur_pos < 0) cur_pos = 0;
        if (cur_pos > clen) cur_pos = clen;
        int col = 1 + utf8_width_len(E.cmd_buf, cur_pos) + 1;
        char cpos[32];
        snprintf(cpos, sizeof(cpos), "\x1b[%d;%dH\x1b[?25h\x1b[5 q", E.screen_rows, col);
        ab_append(&ab, cpos, (int)strlen(cpos));
    } else if (E.mode == MODE_CONFIRM_DELETE_NOTE) {
        int actual = (E.num_filtered > 0 && E.selected_note_idx < E.num_filtered) ? E.filtered_indices[E.selected_note_idx] : -1;
        const char *tname = (actual >= 0) ? E.notes[actual].title : "esta nota";
        char prompt[512];
        snprintf(prompt, sizeof(prompt), " Deseja realmente excluir a nota '%s'? (s/n): ", tname);
        int col = utf8_width(prompt) + 1;
        char cpos[32];
        snprintf(cpos, sizeof(cpos), "\x1b[%d;%dH\x1b[?25h\x1b[2 q", E.screen_rows, col);
        ab_append(&ab, cpos, (int)strlen(cpos));
    } else if (E.mode == MODE_PROMPT_NEW_NOTE) {
        int pwidth = utf8_width(" 📝 Nova nota: ");
        int cur_pos = E.prompt_cursor;
        int plen = (int)strlen(E.prompt_buf);
        if (cur_pos < 0) cur_pos = 0;
        if (cur_pos > plen) cur_pos = plen;
        int col = pwidth + utf8_width_len(E.prompt_buf, cur_pos) + 1;
        char cpos[32];
        snprintf(cpos, sizeof(cpos), "\x1b[%d;%dH\x1b[?25h\x1b[5 q", E.screen_rows, col);
        ab_append(&ab, cpos, (int)strlen(cpos));
    } else if (E.mode == MODE_PROMPT_RENAME_NOTE) {
        int pwidth = utf8_width(" ✏️  Renomear nota: ");
        int cur_pos = E.prompt_cursor;
        int plen = (int)strlen(E.prompt_buf);
        if (cur_pos < 0) cur_pos = 0;
        if (cur_pos > plen) cur_pos = plen;
        int col = pwidth + utf8_width_len(E.prompt_buf, cur_pos) + 1;
        char cpos[32];
        snprintf(cpos, sizeof(cpos), "\x1b[%d;%dH\x1b[?25h\x1b[5 q", E.screen_rows, col);
        ab_append(&ab, cpos, (int)strlen(cpos));
    } else if (E.mode == MODE_PROMPT_FIND) {
        int pwidth = utf8_width(" 🔍 Localizar na nota: ");
        int cur_pos = E.prompt_cursor;
        int plen = (int)strlen(E.prompt_buf);
        if (cur_pos < 0) cur_pos = 0;
        if (cur_pos > plen) cur_pos = plen;
        int col = pwidth + utf8_width_len(E.prompt_buf, cur_pos) + 1;
        char cpos[32];
        snprintf(cpos, sizeof(cpos), "\x1b[%d;%dH\x1b[?25h\x1b[5 q", E.screen_rows, col);
        ab_append(&ab, cpos, (int)strlen(cpos));
    } else if (E.mode == MODE_PROMPT_REPLACE_FIND) {
        int pwidth = utf8_width(" 🔄 Localizar para substituir: ");
        int cur_pos = E.prompt_cursor;
        int plen = (int)strlen(E.prompt_buf);
        if (cur_pos < 0) cur_pos = 0;
        if (cur_pos > plen) cur_pos = plen;
        int col = pwidth + utf8_width_len(E.prompt_buf, cur_pos) + 1;
        char cpos[32];
        snprintf(cpos, sizeof(cpos), "\x1b[%d;%dH\x1b[?25h\x1b[5 q", E.screen_rows, col);
        ab_append(&ab, cpos, (int)strlen(cpos));
    } else if (E.mode == MODE_PROMPT_REPLACE_WITH) {
        char prompt[300];
        snprintf(prompt, sizeof(prompt), " 🔄 Substituir '%s' por: ", E.replace_find_buf);
        int pwidth = utf8_width(prompt);
        int cur_pos = E.prompt_cursor;
        int plen = (int)strlen(E.prompt_buf);
        if (cur_pos < 0) cur_pos = 0;
        if (cur_pos > plen) cur_pos = plen;
        int col = pwidth + utf8_width_len(E.prompt_buf, cur_pos) + 1;
        char cpos[32];
        snprintf(cpos, sizeof(cpos), "\x1b[%d;%dH\x1b[?25h\x1b[5 q", E.screen_rows, col);
        ab_append(&ab, cpos, (int)strlen(cpos));
    } else if (E.mode == MODE_PROMPT_IMPORT_DIR || E.mode == MODE_PROMPT_EXPORT_DIR) {
        const char *prompt = (E.mode == MODE_PROMPT_IMPORT_DIR) ? PROMPT_IMPORT_LABEL : PROMPT_EXPORT_LABEL;
        int pwidth = utf8_width(prompt);
        int cur_pos = E.prompt_cursor;
        int plen = (int)strlen(E.prompt_buf);
        if (cur_pos < 0) cur_pos = 0;
        if (cur_pos > plen) cur_pos = plen;
        int col = pwidth + utf8_width_len(E.prompt_buf, cur_pos) + 1;
        char cpos[32];
        snprintf(cpos, sizeof(cpos), "\x1b[%d;%dH\x1b[?25h\x1b[5 q", E.screen_rows, col);
        ab_append(&ab, cpos, (int)strlen(cpos));
    } else if (E.mode == MODE_SEARCH_LEFT) {
        char cpos[32];
        int q_w = utf8_width(E.search_query);
        int avail = E.left_width - 6;
        if (avail < 2) avail = 2;
        int disp_w = (q_w > avail) ? avail : q_w;
        snprintf(cpos, sizeof(cpos), "\x1b[2;%dH\x1b[?25h\x1b[5 q", 5 + disp_w);
        ab_append(&ab, cpos, (int)strlen(cpos));
    } else if (E.focus == FOCUS_LEFT) {
        int crow = (E.selected_note_idx - E.sidebar_scroll_y) + 4;
        if (crow < 4) crow = 4;
        if (crow > editor_rows) crow = editor_rows;
        char cpos[32];
        snprintf(cpos, sizeof(cpos), "\x1b[%d;3H\x1b[?25h", crow);
        ab_append(&ab, cpos, (int)strlen(cpos));
    } else if (E.focus == FOCUS_RIGHT && E.num_tabs == 0) {
        ab_append(&ab, "\x1b[?25l", 6);
    } else {
        int editor_width = E.screen_cols - E.left_width - 1;
        int avail_text_w = editor_width - 7;
        if (avail_text_w <= 5) avail_text_w = 5;

        int cur_vrow = 0, cur_vcol = 0;
        get_cursor_visual_pos(avail_text_w, &cur_vrow, &cur_vcol);

        int cy = cur_vrow - E.scroll_y + 2;
        int cx = E.left_width + 1 + 6 + cur_vcol + 1;
        if (cy < 2) cy = 2;
        if (cy > editor_rows) cy = editor_rows;
        if (cx < E.left_width + 2) cx = E.left_width + 2;
        if (cx > E.screen_cols) cx = E.screen_cols;
        char cpos[32];
        const char *cursor_shape = (E.mode == MODE_INSERT) ? "\x1b[5 q" : "\x1b[2 q";
        snprintf(cpos, sizeof(cpos), "\x1b[%d;%dH\x1b[?25h%s", cy, cx, cursor_shape);
        ab_append(&ab, cpos, (int)strlen(cpos));
    }

    if (ab.failed || !ab.b) {
        /* Nunca emite um quadro truncado: agenda uma nova tentativa completa. */
        g_screen_clear_needed = 1;
        g_needs_redraw = 1;
    } else if (!write_stdout_all(ab.b, ab.len)) {
        g_screen_clear_needed = 1;
        g_needs_redraw = 1;
    }
    ab_free(&ab);
}

static void set_status_msg(const char *fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    vsnprintf(E.status_msg, sizeof(E.status_msg), fmt, ap);
    va_end(ap);
    E.status_msg_time = time(NULL);
    /* Dirty-flag: a mensagem precisa aparecer na hora, mesmo quando vem de
     * ponto assincrono como o autosave (check_autosave), que roda sem tecla. */
    g_needs_redraw = 1;
}

/* =========================================================================
 * Input Processing & Key Parsing
 * ========================================================================= */

static int g_unget_byte = -1;
static unsigned char g_input_queue[512];
static int g_input_q_head = 0;
static int g_input_q_tail = 0;
static int64_t g_last_typing_time = 0;

static void queue_input_byte(unsigned char b) {
    if (g_input_q_tail < (int)sizeof(g_input_queue)) {
        g_input_queue[g_input_q_tail++] = b;
    }
}

static bool has_pending_input(void) {
    if (g_unget_byte != -1) return true;
    if (g_input_q_head < g_input_q_tail) return true;
#ifdef _WIN32
    DWORD num_events = 0;
    if (GetNumberOfConsoleInputEvents(hStdin, &num_events) && num_events > 0) {
        INPUT_RECORD records[16];
        DWORD read_count = 0;
        DWORD to_peek = (num_events > 16) ? 16 : num_events;
        if (PeekConsoleInput(hStdin, records, to_peek, &read_count) && read_count > 0) {
            for (DWORD i = 0; i < read_count; i++) {
                if (records[i].EventType == KEY_EVENT && records[i].Event.KeyEvent.bKeyDown) {
                    return true;
                }
            }
            /* Discard non-keydown events (key-up, mouse, focus) so they do not cause ReadFile to block */
            INPUT_RECORD discard[16];
            DWORD discarded = 0;
            ReadConsoleInput(hStdin, discard, read_count, &discarded);
        }
    }
    return false;
#else
    fd_set fds;
    FD_ZERO(&fds);
    FD_SET(STDIN_FILENO, &fds);
    struct timeval tv = {0, 0};
    int r = select(STDIN_FILENO + 1, &fds, NULL, NULL, &tv);
    return (r > 0);
#endif
}

static int read_raw_byte(int timeout_ms) {
    if (g_unget_byte != -1) {
        int b = g_unget_byte;
        g_unget_byte = -1;
        return b;
    }

    if (g_input_q_head < g_input_q_tail) {
        unsigned char b = g_input_queue[g_input_q_head++];
        if (g_input_q_head == g_input_q_tail) {
            g_input_q_head = 0;
            g_input_q_tail = 0;
        }
        return b;
    }
    g_input_q_head = 0;
    g_input_q_tail = 0;

#ifdef _WIN32
    if (timeout_ms >= 0) {
        DWORD start_time = GetTickCount();
        while (1) {
            DWORD elapsed = GetTickCount() - start_time;
            if (elapsed >= (DWORD)timeout_ms) return -1;
            DWORD remain = (DWORD)timeout_ms - elapsed;
            DWORD wait_res = WaitForSingleObject(hStdin, remain);
            if (wait_res != WAIT_OBJECT_0) return -1;

            /* Check if there are real key down events */
            DWORD num_events = 0;
            if (GetNumberOfConsoleInputEvents(hStdin, &num_events) && num_events > 0) {
                INPUT_RECORD records[32];
                DWORD read_count = 0;
                DWORD to_peek = (num_events > 32) ? 32 : num_events;
                if (PeekConsoleInput(hStdin, records, to_peek, &read_count) && read_count > 0) {
                    bool has_key_down = false;
                    for (DWORD i = 0; i < read_count; i++) {
                        if (records[i].EventType == KEY_EVENT && records[i].Event.KeyEvent.bKeyDown) {
                            has_key_down = true;
                            break;
                        }
                    }
                    if (has_key_down) {
                        break;
                    } else {
                        /* Discard non-keydown events (key-up, mouse, focus) so ReadFile doesn't block */
                        INPUT_RECORD discard[32];
                        DWORD discarded = 0;
                        ReadConsoleInput(hStdin, discard, read_count, &discarded);
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }
    char buf[128];
    DWORD read_bytes = 0;
    if (ReadFile(hStdin, buf, sizeof(buf), &read_bytes, NULL) && read_bytes > 0) {
        for (DWORD i = 0; i < read_bytes; i++) {
            queue_input_byte((unsigned char)buf[i]);
        }
        if (g_input_q_head < g_input_q_tail) {
            unsigned char b = g_input_queue[g_input_q_head++];
            if (g_input_q_head == g_input_q_tail) {
                g_input_q_head = 0;
                g_input_q_tail = 0;
            }
            return b;
        }
    }
    return -1;
#else
    if (timeout_ms >= 0) {
        fd_set fds;
        FD_ZERO(&fds);
        FD_SET(STDIN_FILENO, &fds);
        struct timeval tv;
        tv.tv_sec = timeout_ms / 1000;
        tv.tv_usec = (timeout_ms % 1000) * 1000;
        int r = select(STDIN_FILENO + 1, &fds, NULL, NULL, &tv);
        if (r == -1 && errno == EINTR) {
            if (g_window_resized) return -2;
            return -1;
        }
        if (r <= 0) return -1;
    }
    char buf[128];
    int n = read(STDIN_FILENO, buf, sizeof(buf));
    if (n > 0) {
        for (int i = 0; i < n; i++) {
            queue_input_byte((unsigned char)buf[i]);
        }
        if (g_input_q_head < g_input_q_tail) {
            unsigned char b = g_input_queue[g_input_q_head++];
            if (g_input_q_head == g_input_q_tail) {
                g_input_q_head = 0;
                g_input_q_tail = 0;
            }
            return b;
        }
    }
    if (n == -1 && errno == EINTR && g_window_resized) return -2;
    return -1;
#endif
}

static int convert_ansi_to_utf8(unsigned char c, char *out) {
    /* CP850 special characters common in Portuguese / Western Europe */
    if (c == 0x87) { out[0] = (char)0xC3; out[1] = (char)0xA7; out[2] = '\0'; return 2; } /* ç */
    if (c == 0x80) { out[0] = (char)0xC3; out[1] = (char)0x87; out[2] = '\0'; return 2; } /* Ç */
    if (c == 0xA0) { out[0] = (char)0xC3; out[1] = (char)0xA1; out[2] = '\0'; return 2; } /* á */
    if (c == 0x82) { out[0] = (char)0xC3; out[1] = (char)0xA9; out[2] = '\0'; return 2; } /* é */
    if (c == 0xA1) { out[0] = (char)0xC3; out[1] = (char)0xAD; out[2] = '\0'; return 2; } /* í */
    if (c == 0xA2) { out[0] = (char)0xC3; out[1] = (char)0xB3; out[2] = '\0'; return 2; } /* ó */
    if (c == 0xA3) { out[0] = (char)0xC3; out[1] = (char)0xBA; out[2] = '\0'; return 2; } /* ú */
    if (c == 0xC6) { out[0] = (char)0xC3; out[1] = (char)0xA3; out[2] = '\0'; return 2; } /* ã */
    if (c == 0xE4) { out[0] = (char)0xC3; out[1] = (char)0xB5; out[2] = '\0'; return 2; } /* õ */
    if (c == 0x83) { out[0] = (char)0xC3; out[1] = (char)0xA2; out[2] = '\0'; return 2; } /* â */
    if (c == 0x88) { out[0] = (char)0xC3; out[1] = (char)0xAA; out[2] = '\0'; return 2; } /* ê */
    if (c == 0x93) { out[0] = (char)0xC3; out[1] = (char)0xB4; out[2] = '\0'; return 2; } /* ô */
    if (c == 0x85) { out[0] = (char)0xC3; out[1] = (char)0xA0; out[2] = '\0'; return 2; } /* à */

    if (c >= 0xA0) {
        out[0] = (char)(0xC0 | (c >> 6));
        out[1] = (char)(0x80 | (c & 0x3F));
        out[2] = '\0';
        return 2;
    }
    if (c == 0x80) { out[0] = (char)0xE2; out[1] = (char)0x82; out[2] = (char)0xAC; out[3] = '\0'; return 3; } /* € */

    out[0] = (char)c;
    out[1] = '\0';
    return 1;
}

static char g_last_utf8_char[8] = {0};
static int g_last_utf8_len = 0;

static int editor_read_key(void) {
    g_last_utf8_char[0] = '\0';
    g_last_utf8_len = 0;

    int c = read_raw_byte(150); /* 150ms timeout for smooth marquee animation when idle */
    if (c == -1) return -1;
    if (c == -2) return -2;

    if (c == KEY_ESC) {
        int seq[8] = {0};
        /* Check if this is the start of an escape sequence */
        seq[0] = read_raw_byte(60);
        if (seq[0] == -1) return KEY_ESC;

        if (seq[0] == '[') {
            seq[1] = read_raw_byte(80);
            if (seq[1] == -1) return -1; /* Incomplete sequence, do not return fake KEY_ESC! */

            /* Focus reporting: \x1b[I (focus in), \x1b[O (focus out) - ignore safely */
            if (seq[1] == 'I' || seq[1] == 'O') {
                return -1;
            }

            if (seq[1] == '<') {
                char mouse_str[64];
                int idx = 0;
                int ch;
                bool completed = false;
                while ((ch = read_raw_byte(80)) != -1 && idx < 60) {
                    mouse_str[idx++] = (char)ch;
                    if (ch == 'M' || ch == 'm') {
                        completed = true;
                        break;
                    }
                }
                mouse_str[idx] = '\0';
                if (!completed) return -1; /* Incomplete mouse sequence, discard safely */

                int btn = 0, col = 0, row = 0;
                char release = 'M';
                if (sscanf(mouse_str, "%d;%d;%d%c", &btn, &col, &row, &release) != 4) {
                    return -1;
                }
                E.mouse_btn = btn;
                E.mouse_col = col;
                E.mouse_row = row;
                E.mouse_released = (release == 'm');
                return KEY_MOUSE_EVENT;
            }

            if (seq[1] >= '0' && seq[1] <= '9') {
                seq[2] = read_raw_byte(80);
                if (seq[1] == '9' && seq[2] == ';') {
                    int u1 = read_raw_byte(80);
                    int u2 = read_raw_byte(80);
                    if (u1 == '5' && u2 == 'u') return KEY_CTRL_TAB;
                }
                if (seq[1] == '2' && seq[2] == '0') {
                    seq[3] = read_raw_byte(80);
                    seq[4] = read_raw_byte(80);
                    if (seq[3] == '0' && seq[4] == '~') {
                        /* Start of Bracketed Paste */
                        int cap = 8192;
                        char *pbuf = (char *)malloc(cap);
                        if (!pbuf) return -1;
                        int plen = 0;
                        while (1) {
                            int b = read_raw_byte(200);
                            if (b == -1) break;
                            if (b == KEY_ESC) {
                                int b1 = read_raw_byte(80);
                                if (b1 == '[') {
                                    int b2 = read_raw_byte(80);
                                    int b3 = read_raw_byte(80);
                                    int b4 = read_raw_byte(80);
                                    int b5 = read_raw_byte(80);
                                    if (b2 == '2' && b3 == '0' && b4 == '1' && b5 == '~') {
                                        break; /* End of Bracketed Paste */
                                    }
                                    if (plen + 6 >= cap) {
                                        int new_cap = cap * 2;
                                        char *np = (char *)realloc(pbuf, new_cap);
                                        if (!np) break;
                                        pbuf = np;
                                        cap = new_cap;
                                    }
                                    pbuf[plen++] = '\x1b';
                                    if (b1 != -1) pbuf[plen++] = (char)b1;
                                    if (b2 != -1) pbuf[plen++] = (char)b2;
                                    if (b3 != -1) pbuf[plen++] = (char)b3;
                                    if (b4 != -1) pbuf[plen++] = (char)b4;
                                    if (b5 != -1) pbuf[plen++] = (char)b5;
                                    continue;
                                } else if (b1 != -1) {
                                    if (plen + 2 >= cap) {
                                        int new_cap = cap * 2;
                                        char *np = (char *)realloc(pbuf, new_cap);
                                        if (!np) break;
                                        pbuf = np;
                                        cap = new_cap;
                                    }
                                    pbuf[plen++] = '\x1b';
                                    pbuf[plen++] = (char)b1;
                                    continue;
                                }
                            }
                            if (plen + 2 >= cap) {
                                int new_cap = cap * 2;
                                char *np = (char *)realloc(pbuf, new_cap);
                                if (!np) break;
                                pbuf = np;
                                cap = new_cap;
                            }
                            pbuf[plen++] = (char)b;
                        }
                        pbuf[plen] = '\0';
                        editor_paste_string(pbuf, plen);
                        free(pbuf);
                        return -1;
                    }
                }
                if (seq[2] == '~') {
                    switch (seq[1]) {
                        case '1': return KEY_HOME;
                        case '3': return KEY_DELETE;
                        case '4': return KEY_END;
                        case '5': return KEY_PAGE_UP;
                        case '6': return KEY_PAGE_DOWN;
                        case '7': return KEY_HOME;
                        case '8': return KEY_END;
                    }
                } else if (seq[2] == ';') {
                    seq[3] = read_raw_byte(80);
                    seq[4] = read_raw_byte(80);
                    if (seq[3] == '2') { /* Shift modifier */
                        if (seq[4] == 'A') return KEY_SHIFT_UP;
                        if (seq[4] == 'B') return KEY_SHIFT_DOWN;
                        if (seq[4] == 'C') return KEY_SHIFT_RIGHT;
                        if (seq[4] == 'D') return KEY_SHIFT_LEFT;
                    } else if (seq[3] == '5') { /* Ctrl modifier */
                        if (seq[4] == 'D') return KEY_CTRL_LEFT;
                        if (seq[4] == 'C') return KEY_CTRL_RIGHT;
                        if (seq[4] == 'I') return KEY_CTRL_TAB;
                        if (seq[4] == '5') return KEY_PAGE_UP;
                        if (seq[4] == '6') return KEY_PAGE_DOWN;
                    } else if (seq[3] == '6') { /* Ctrl+Shift modifier */
                        if (seq[4] == 'D') return KEY_CTRL_SHIFT_LEFT;
                        if (seq[4] == 'C') return KEY_CTRL_SHIFT_RIGHT;
                        if (seq[4] == 'A') return KEY_SHIFT_UP;
                        if (seq[4] == 'B') return KEY_SHIFT_DOWN;
                    }
                } else if (seq[1] == '2') {
                    if (seq[2] == 'A') return KEY_SHIFT_UP;
                    if (seq[2] == 'B') return KEY_SHIFT_DOWN;
                    if (seq[2] == 'C') return KEY_SHIFT_RIGHT;
                    if (seq[2] == 'D') return KEY_SHIFT_LEFT;
                }
                /* Drain unrecognized numeric CSI sequence until terminator */
                int dr = seq[2];
                while (dr != -1 && (dr < 0x40 || dr > 0x7E)) {
                    dr = read_raw_byte(50);
                }
                return -1;
            } else {
                switch (seq[1]) {
                    case 'A': return KEY_ARROW_UP;
                    case 'B': return KEY_ARROW_DOWN;
                    case 'C': return KEY_ARROW_RIGHT;
                    case 'D': return KEY_ARROW_LEFT;
                    case 'H': return KEY_HOME;
                    case 'F': return KEY_END;
                    case 'a': return KEY_SHIFT_UP;
                    case 'b': return KEY_SHIFT_DOWN;
                    case 'c': return KEY_SHIFT_RIGHT;
                    case 'd': return KEY_SHIFT_LEFT;
                }
                /* Drain unrecognized letter CSI sequence until terminator */
                int dr = seq[1];
                while (dr != -1 && (dr < 0x40 || dr > 0x7E)) {
                    dr = read_raw_byte(50);
                }
                return -1;
            }
        } else if (seq[0] == 'O') {
            seq[1] = read_raw_byte(80);
            switch (seq[1]) {
                case 'A': return KEY_ARROW_UP;
                case 'B': return KEY_ARROW_DOWN;
                case 'C': return KEY_ARROW_RIGHT;
                case 'D': return KEY_ARROW_LEFT;
                case 'H': return KEY_HOME;
                case 'F': return KEY_END;
            }
            return -1;
        } else {
            /* Standalone Esc followed immediately by another character */
            if (E.mode == MODE_INSERT || (E.mode >= MODE_PROMPT_NEW_NOTE && E.mode <= MODE_PROMPT_EXPORT_DIR)) {
                return -1; /* Don't abort text editing / prompt on stray Alt combinations */
            }
            g_unget_byte = seq[0];
            return KEY_ESC;
        }
        return -1;
    }

    if ((unsigned char)c < 0x80) {
        g_last_utf8_char[0] = (char)c;
        g_last_utf8_char[1] = '\0';
        g_last_utf8_len = 1;
        return c;
    }

    /* Multi-byte UTF-8 character leading byte */
    if ((unsigned char)c >= 0xC2 && (unsigned char)c <= 0xF4) {
        int expected = 1;
        if (((unsigned char)c & 0xE0) == 0xC0) expected = 2;
        else if (((unsigned char)c & 0xF0) == 0xE0) expected = 3;
        else if (((unsigned char)c & 0xF8) == 0xF0) expected = 4;

        g_last_utf8_char[0] = (char)c;
        g_last_utf8_len = 1;

        bool valid = true;
        for (int i = 1; i < expected; i++) {
            int next_b = read_raw_byte(80);
            if (next_b == -1 || ((unsigned char)next_b & 0xC0) != 0x80) {
                if (next_b != -1) g_unget_byte = next_b;
                valid = false;
                break;
            }
            g_last_utf8_char[g_last_utf8_len++] = (char)next_b;
        }
        g_last_utf8_char[g_last_utf8_len] = '\0';

        if (valid && g_last_utf8_len == expected) {
            return (unsigned char)g_last_utf8_char[0];
        }
    }

    /* Fallback for single-byte legacy ANSI / CP1252 / CP850 */
    g_last_utf8_len = convert_ansi_to_utf8((unsigned char)c, g_last_utf8_char);
    return (unsigned char)g_last_utf8_char[0];
}

static void execute_delete_selected_note(void) {
    if (E.num_filtered > 0 && E.selected_note_idx >= 0 &&
        E.selected_note_idx < E.num_filtered) {
        int actual = E.filtered_indices[E.selected_note_idx];
        char target_path[512];
        safe_strcpy(target_path, sizeof(target_path), E.notes[actual].filepath);

        /* Remove a nota do banco (ou o arquivo, se for caminho real). Se a
         * exclusao falhar, mantem tudo como esta em vez de fechar as abas e
         * anunciar sucesso. */
        int did = note_id_from_path(target_path);
        if (did > 0) {
            if (!db_delete_note(did)) {
                set_status_msg("Falha ao excluir a nota do banco.");
                return;
            }
        } else {
            if (unlink_utf8(target_path) != 0) {
                set_status_msg("Falha ao excluir o arquivo: %s", target_path);
                return;
            }
        }

        /* Close tabs associated with this note */
        for (int i = E.num_tabs - 1; i >= 0; i--) {
            if (strcmp(E.tabs[i].cur_filepath, target_path) == 0) {
                E.tabs[i].modified = false;
                tab_close(i);
            }
        }

        /* Clear preview if it was showing this file */
        if (strcmp(g_preview.filepath, target_path) == 0) {
            free_preview();
        }

        rescan_notes();

        if (E.selected_note_idx >= E.num_filtered) {
            E.selected_note_idx = E.num_filtered > 0 ? E.num_filtered - 1 : 0;
        }
        if (E.num_filtered > 0) {
            int nactual = E.filtered_indices[E.selected_note_idx];
            load_preview(E.notes[nactual].filepath);
        } else {
            free_preview();
        }
        set_status_msg("Nota excluída com sucesso.");
    }
}

static int64_t g_last_theme_click_time = 0;
static int64_t g_last_activity_time = 0;
static int64_t g_last_note_click_time = 0;
static int g_last_clicked_note_idx = -1;

static void handle_mouse_event(void) {
    int64_t now = get_current_time_ms();

    /* 1. Modal Prompt Insulation: protect prompt modes from background clicks/drags */
    if (E.mode >= MODE_PROMPT_NEW_NOTE && E.mode <= MODE_PROMPT_EXPORT_DIR) {
        if (E.mouse_btn == 0 && !E.mouse_released && E.mouse_row == E.screen_rows) {
            int pwidth = 0;
            if (E.mode == MODE_PROMPT_NEW_NOTE) pwidth = utf8_width(" 📝 Nova nota: ");
            else if (E.mode == MODE_PROMPT_RENAME_NOTE) pwidth = utf8_width(" ✏️  Renomear nota: ");
            else if (E.mode == MODE_PROMPT_FIND) pwidth = utf8_width(" 🔍 Localizar na nota: ");
            else if (E.mode == MODE_PROMPT_REPLACE_FIND) pwidth = utf8_width(" 🔄 Localizar para substituir: ");
            else if (E.mode == MODE_PROMPT_REPLACE_WITH) {
                char pbuf[300];
                snprintf(pbuf, sizeof(pbuf), " 🔄 Substituir '%s' por: ", E.replace_find_buf);
                pwidth = utf8_width(pbuf);
            }
            int clicked_col = E.mouse_col - pwidth - 1;
            if (clicked_col < 0) clicked_col = 0;
            int plen = (int)strlen(E.prompt_buf);
            int byte_idx = 0;
            int cur_w = 0;
            while (byte_idx < plen) {
                int clen = utf8_charlen((unsigned char)E.prompt_buf[byte_idx]);
                if (clen <= 0) clen = 1;
                int cw = utf8_width_len(&E.prompt_buf[byte_idx], clen);
                if (cur_w + cw > clicked_col) break;
                cur_w += cw;
                byte_idx += clen;
            }
            E.prompt_cursor = byte_idx;
        }
        /* Completely ignore clicks/drags outside the prompt line while modal prompt is open */
        return;
    }

    /* 2. Palm Rejection & Trackpad Tap Protection in Insert Mode */
    if (E.mode == MODE_INSERT) {
        if (E.mouse_btn == 0 && (now - g_last_typing_time < 150)) {
            return; /* Accidental palm tap while typing ignored */
        }
    }

    if (E.mouse_btn == 64) {
        if (E.mouse_col <= E.left_width) {
            if (E.sidebar_scroll_y > 0) E.sidebar_scroll_y--;
        } else {
            if (E.focus == FOCUS_LEFT && g_preview.lines) {
                if (g_preview.scroll_y > 0) g_preview.scroll_y--;
            } else {
                if (E.scroll_y > 0) E.scroll_y--;
            }
        }
        return;
    } else if (E.mouse_btn == 65) {
        if (E.mouse_col <= E.left_width) {
            if (E.sidebar_scroll_y + (E.screen_rows - 5) < E.num_filtered) E.sidebar_scroll_y++;
        } else {
            if (E.focus == FOCUS_LEFT && g_preview.lines) {
                if (g_preview.scroll_y + 1 < g_preview.num_lines) g_preview.scroll_y++;
            } else {
                if (E.scroll_y + 1 < E.num_lines) E.scroll_y++;
            }
        }
        return;
    }

    /* Middle Mouse Button Click (Wheel click = button 1) */
    if (E.mouse_btn == 1 && !E.mouse_released) {
        if (E.mouse_row == 1 && E.mouse_col > E.left_width + 1) {
            for (int i = 0; i < E.num_tabs; i++) {
                if (E.mouse_col >= E.tab_coords[i].start_col && E.mouse_col <= E.tab_coords[i].end_col) {
                    tab_close(i);
                    return;
                }
            }
        }
        return;
    }

    /* Left Click (button 0) */
    if (E.mouse_btn == 0 && !E.mouse_released) {
        /* If in delete confirmation mode, handle mouse clicks on confirm/cancel */
        if (E.mode == MODE_CONFIRM_DELETE_NOTE) {
            if (E.mouse_row == E.screen_rows) {
                int actual = (E.num_filtered > 0 && E.selected_note_idx < E.num_filtered) ? E.filtered_indices[E.selected_note_idx] : -1;
                const char *tname = (actual >= 0) ? E.notes[actual].title : "esta nota";
                char prompt[512];
                snprintf(prompt, sizeof(prompt), " Deseja realmente excluir a nota '%s'? (s/n): ", tname);
                int plen = utf8_width(prompt);
                if (E.mouse_col >= plen - 7 && E.mouse_col <= plen - 5) {
                    E.mode = MODE_NORMAL;
                    execute_delete_selected_note();
                    return;
                } else if (E.mouse_col >= plen - 4 && E.mouse_col <= plen - 2) {
                    E.mode = MODE_NORMAL;
                    set_status_msg("Exclusão cancelada.");
                    return;
                }
            }
            E.mode = MODE_NORMAL;
            set_status_msg("Exclusão cancelada.");
            return;
        }
        /* Check Bottom Bar Buttons click on Status Bar (always row screen_rows - 1) */
        if (E.mouse_row == E.screen_rows - 1) {
            if (now - g_last_theme_click_time > 200) {
                g_last_theme_click_time = now;
                if (E.mouse_col >= E.btn_new_start && E.mouse_col < E.btn_new_end) {
                    /* Create new note */
                    E.prompt_buf[0] = '\0';
                    E.prompt_cursor = 0;
                    E.mode = MODE_PROMPT_NEW_NOTE;
                    return;
                } else if (E.mouse_col >= E.btn_rename_start && E.mouse_col < E.btn_rename_end) {
                    /* Rename current note */
                    const char *target = (E.focus == FOCUS_LEFT && E.num_filtered > 0 && E.selected_note_idx < E.num_filtered) ?
                                         E.notes[E.filtered_indices[E.selected_note_idx]].filepath : E.cur_filepath;
                    if (target && *target) {
                        safe_strcpy(E.rename_target_path, sizeof(E.rename_target_path), target);
                        const char *slash = strrchr(target, PATH_SEP);
#ifndef _WIN32
                        if (!slash) slash = strrchr(target, '/');
#endif
                        const char *fname = slash ? slash + 1 : target;
                        safe_strcpy(E.prompt_buf, sizeof(E.prompt_buf), fname);
                        char *dot = strrchr(E.prompt_buf, '.');
                        if (dot) *dot = '\0';
                        E.prompt_cursor = (int)strlen(E.prompt_buf);
                        E.mode = MODE_PROMPT_RENAME_NOTE;
                    } else {
                        set_status_msg("Nenhuma nota aberta para renomear.");
                    }
                    return;
                } else if (E.mouse_col >= E.btn_delete_start && E.mouse_col < E.btn_delete_end) {
                    /* Excluir nota selecionada no painel esquerdo */
                    if (E.num_filtered > 0 && E.selected_note_idx < E.num_filtered) {
                        E.mode = MODE_CONFIRM_DELETE_NOTE;
                    } else {
                        set_status_msg("Nenhuma nota selecionada para excluir.");
                    }
                    return;
                } else if (E.mouse_col >= E.btn_wrap_start && E.mouse_col < E.btn_wrap_end) {
                    E.wrap_mode = !E.wrap_mode;
                    g_screen_clear_needed = 1;
                    set_status_msg("Quebra de linha (wrap): %s", E.wrap_mode ? "ATIVADO" : "DESATIVADO");
                    return;
                } else if (E.mouse_col >= E.btn_find_start && E.mouse_col < E.btn_find_end) {
                    E.mode = MODE_PROMPT_FIND;
                    E.prompt_buf[0] = '\0';
                    E.prompt_cursor = 0;
                    set_status_msg("Localizar na nota (digite o termo e pressione Enter):");
                    return;
                } else if (E.mouse_col >= E.btn_replace_start && E.mouse_col < E.btn_replace_end) {
                    E.mode = MODE_PROMPT_REPLACE_FIND;
                    E.prompt_buf[0] = '\0';
                    E.prompt_cursor = 0;
                    set_status_msg("Substituir: digite o termo a ser localizado e pressione Enter");
                    return;
                } else if (E.mouse_col >= E.theme_btn_start_col && E.mouse_col < E.theme_btn_end_col) {
                    E.current_theme = (E.current_theme + 1) % NUM_THEMES;
                    g_screen_clear_needed = 1;
                    set_status_msg("Tema alterado para: %s", themes[E.current_theme].name);
                    return;
                } else if (E.mouse_col >= E.btn_life_start && E.mouse_col < E.btn_life_end) {
                    run_game_of_life();
                    g_last_activity_time = get_current_time_ms();
                    set_status_msg("Simulação do Game of Life encerrada.");
                    return;
                } else if (E.mouse_col >= E.btn_import_start && E.mouse_col < E.btn_import_end) {
                    E.mode = MODE_PROMPT_IMPORT_DIR;
                    E.prompt_buf[0] = '\0';
                    E.prompt_cursor = 0;
                    set_status_msg("%sinforme a pasta com os arquivos .md", PROMPT_IMPORT_LABEL);
                    return;
                } else if (E.mouse_col >= E.btn_export_start && E.mouse_col < E.btn_export_end) {
                    E.mode = MODE_PROMPT_EXPORT_DIR;
                    E.prompt_buf[0] = '\0';
                    E.prompt_cursor = 0;
                    set_status_msg("%sinforme a pasta de destino", PROMPT_EXPORT_LABEL);
                    return;
                }
            }
        }

        /* Check Tab Bar click (row 1, col > left_width + 1) */
        if (E.mouse_row == 1 && E.mouse_col > E.left_width + 1) {
            if (E.num_tabs == 0) {
                if (E.mouse_col >= E.tab_new_btn_start && E.mouse_col <= E.tab_new_btn_end) {
                    E.prompt_buf[0] = '\0';
                    E.prompt_cursor = 0;
                    E.mode = MODE_PROMPT_NEW_NOTE;
                    set_status_msg("Criar nova nota:");
                    return;
                }
            } else {
                for (int i = 0; i < E.num_tabs; i++) {
                    if (E.mouse_col >= E.tab_coords[i].start_col && E.mouse_col <= E.tab_coords[i].end_col) {
                        if (E.mouse_col >= E.tab_coords[i].close_col) {
                            tab_close(i);
                        } else {
                            tab_switch(i);
                            free_preview();
                            if (E.mode == MODE_SEARCH_LEFT) {
                                E.mode = MODE_NORMAL;
                                set_status_msg("-- MODO NORMAL --");
                            }
                            E.focus = FOCUS_RIGHT;
                        }
                        return;
                    }
                }
            }
            return;
        }

        /* Botoes de filtro de urgencia: tratados AQUI, FORA do guarda
         * `E.mouse_col <= E.left_width` que envolve o painel esquerdo.
         * Motivo (bug corrigido): os botoes sao desenhados ate a coluna 20,
         * mas o guarda so aceita cliques dentro da largura do painel. Com o
         * painel estreito (minimo 18 colunas via Ctrl+<-), os botoes [2] e [3]
         * caiam fora da area clicavel e nao filtravam nada - exatamente o
         * sintoma relatado no Windows, onde a largura do console era menor. */
        /* Click on bottom urgency filter buttons */
        int filter_btn_row = E.screen_rows - (1 + E.status_bar_lines);
        if (E.mouse_row == filter_btn_row) {
            if (E.mouse_col >= E.btn_filter_1_start && E.mouse_col < E.btn_filter_1_end) {
                E.filter_urgency = (E.filter_urgency == 1) ? 0 : 1;
                apply_note_search(E.search_query);
                if (E.filter_urgency == 1)
                    set_status_msg("🔴 Filtrando notas com Urgência Nível 1 (Crítico - Vermelho): %d encontrada(s)", E.num_filtered);
                else
                    set_status_msg("Filtro de urgência removido (Exibindo todas as %d notas)", E.num_filtered);
                return;
            }
            if (E.mouse_col >= E.btn_filter_2_start && E.mouse_col < E.btn_filter_2_end) {
                E.filter_urgency = (E.filter_urgency == 2) ? 0 : 2;
                apply_note_search(E.search_query);
                if (E.filter_urgency == 2)
                    set_status_msg("🟠 Filtrando notas com Urgência Nível 2 (Médio - Laranja): %d encontrada(s)", E.num_filtered);
                else
                    set_status_msg("Filtro de urgência removido (Exibindo todas as %d notas)", E.num_filtered);
                return;
            }
            if (E.mouse_col >= E.btn_filter_3_start && E.mouse_col < E.btn_filter_3_end) {
                E.filter_urgency = (E.filter_urgency == 3) ? 0 : 3;
                apply_note_search(E.search_query);
                if (E.filter_urgency == 3)
                    set_status_msg("🟡 Filtrando notas com Urgência Nível 3 (Baixo - Amarelo): %d encontrada(s)", E.num_filtered);
                else
                    set_status_msg("Filtro de urgência removido (Exibindo todas as %d notas)", E.num_filtered);
                return;
            }
            if (E.mouse_col >= E.btn_filter_h_start && E.mouse_col < E.btn_filter_h_end) {
                E.filter_urgency = (E.filter_urgency == 4) ? 0 : 4;
                apply_note_search(E.search_query);
                if (E.filter_urgency == 4)
                    set_status_msg("🔵 Filtrando notas com Marcação [H] (Azul): %d encontrada(s)", E.num_filtered);
                else
                    set_status_msg("Filtro de urgência removido (Exibindo todas as %d notas)", E.num_filtered);
                return;
            }
            if (E.mouse_col >= E.btn_filter_r_start && E.mouse_col < E.btn_filter_r_end) {
                E.filter_urgency = (E.filter_urgency == 5) ? 0 : 5;
                apply_note_search(E.search_query);
                if (E.filter_urgency == 5)
                    set_status_msg("🟣 Filtrando notas com Marcação [R] (Roxo): %d encontrada(s)", E.num_filtered);
                else
                    set_status_msg("Filtro de urgência removido (Exibindo todas as %d notas)", E.num_filtered);
                return;
            }
            if (E.mouse_col >= E.btn_filter_o_start && E.mouse_col < E.btn_filter_o_end) {
                E.filter_urgency = (E.filter_urgency == 6) ? 0 : 6;
                apply_note_search(E.search_query);
                if (E.filter_urgency == 6)
                    set_status_msg("🟢 Filtrando notas com Marcação [O] (Verde): %d encontrada(s)", E.num_filtered);
                else
                    set_status_msg("Filtro de urgência removido (Exibindo todas as %d notas)", E.num_filtered);
                return;
            }
            if (E.mouse_col >= E.btn_filter_all_start && E.mouse_col < E.btn_filter_all_end) {
                E.filter_urgency = 0;
                apply_note_search(E.search_query);
                set_status_msg("Filtro removido: exibindo todas as %d notas", E.num_filtered);
                return;
            }
            return;
        }

        /* Left Panel click (notes selection, attention checkbox, or filter buttons) */
        if (E.mouse_col <= E.left_width) {
            E.has_selection = false;
            if (E.mode == MODE_INSERT) {
                E.mode = MODE_NORMAL;
                set_status_msg("-- MODO NORMAL --");
            }
            E.focus = FOCUS_LEFT;
            if (E.mouse_row <= 3) {
                if (E.mouse_row == 2) {
                    E.mode = MODE_SEARCH_LEFT;
                    E.search_query[0] = '\0';
                    E.search_cursor = 0;
                    apply_note_search("");
                    if (E.num_filtered > 0) {
                        int actual = E.filtered_indices[E.selected_note_idx];
                        load_preview(E.notes[actual].filepath);
                    } else {
                        free_preview();
                    }
                    set_status_msg("Busca de notas (digite o termo, Enter para abrir)");
                }
                return;
            }

            int clicked_note = (E.mouse_row - 4) + E.sidebar_scroll_y;
            if (clicked_note >= 0 && clicked_note < E.num_filtered) {
                E.selected_note_idx = clicked_note;
                int actual = E.filtered_indices[clicked_note];
                /* Clicked on checkbox [ ] / [!] / [H] / [R] (cols 1 to 5) */
                if (E.mouse_col >= 1 && E.mouse_col <= 5) {
                    g_last_clicked_note_idx = -1;
                    g_last_note_click_time = 0;
                    toggle_note_attention_at(actual);
                    return;
                }

                if (clicked_note == g_last_clicked_note_idx && (now - g_last_note_click_time < 500)) {
                    /* Double click: Open note in tab and switch focus to editor */
                    g_last_clicked_note_idx = -1;
                    g_last_note_click_time = 0;
                    free_preview();
                    open_note_by_path(E.notes[actual].filepath, true);
                    if (E.mode == MODE_SEARCH_LEFT) {
                        E.mode = MODE_NORMAL;
                    }
                    E.focus = FOCUS_RIGHT;
                    set_status_msg("Aberta em aba: %s", E.cur_filename);
                } else {
                    /* Single click: Select note and show preview only, do not open tab */
                    g_last_clicked_note_idx = clicked_note;
                    g_last_note_click_time = now;
                    E.focus = FOCUS_LEFT;
                    load_preview(E.notes[actual].filepath);
                    set_status_msg("Prévia: %s (Enter ou duplo clique para abrir)", E.notes[actual].title);
                }
            } else {
                E.focus = FOCUS_LEFT;
            }
            return;
        } else {
            /* Editor Click (Right Panel) */
            const char *target_preview = g_preview.filepath[0] ? g_preview.filepath :
                ((E.focus == FOCUS_LEFT && E.num_filtered > 0 && E.selected_note_idx < E.num_filtered) ?
                 E.notes[E.filtered_indices[E.selected_note_idx]].filepath : NULL);

            if (target_preview && (E.focus == FOCUS_LEFT || E.mode == MODE_SEARCH_LEFT)) {
                char preview_path[512];
                safe_strcpy(preview_path, sizeof(preview_path), target_preview);
                int prev_scroll_y = g_preview.scroll_y;
                free_preview();
                open_note_by_path(preview_path, true);
                E.scroll_y = prev_scroll_y;
                if (E.mode == MODE_SEARCH_LEFT) {
                    E.mode = MODE_NORMAL;
                }
                set_status_msg("Aberta em aba: %s", E.cur_filename);
            }
            E.focus = FOCUS_RIGHT;
            int editor_width = E.screen_cols - E.left_width - 1;
            int avail_text_w = editor_width - 7;
            if (avail_text_w <= 5) avail_text_w = 5;

            int max_ed_row = E.screen_rows - (1 + E.status_bar_lines);
            if (E.mouse_row > max_ed_row) return;
            int screen_row = E.mouse_row - 2;
            int target_vrow = screen_row + E.scroll_y;
            int line_idx = -1, sub_idx = -1;
            LineSegment seg;
            if (find_visual_row(false, target_vrow, avail_text_w, &line_idx, &sub_idx, &seg, NULL)) {
                E.cursor_y = line_idx;
                int col = E.mouse_col - (E.left_width + 1 + 6 + 1);
                if (col < 0) col = 0;
                int x_in_seg = get_byte_offset_in_seg(&E.lines[line_idx], seg.byte_offset, seg.byte_len, col, should_hide_links(false), should_hide_markdown_styles(false));
                E.cursor_x = seg.byte_offset + x_in_seg;
                if (E.cursor_x > E.lines[line_idx].len) E.cursor_x = E.lines[line_idx].len;
            }
            E.visual_start_y = E.cursor_y;
            E.visual_start_x = E.cursor_x;
            E.has_selection = false;
            if (E.mode == MODE_VISUAL_CHAR || E.mode == MODE_VISUAL_LINE) {
                E.mode = MODE_NORMAL;
                set_status_msg("-- MODO NORMAL --");
            }
        }
    }

    /* Mouse Drag Selection in Editor (btn == 32) */
    if (E.mouse_btn == 32) {
        if (E.mode != MODE_NORMAL && E.mode != MODE_VISUAL_CHAR && E.mode != MODE_VISUAL_LINE && E.mode != MODE_INSERT) {
            return;
        }
        if (E.mouse_col > E.left_width + 1 && E.mouse_row >= 2 && E.mouse_row <= E.screen_rows - (1 + E.status_bar_lines)) {
            const char *target_preview = g_preview.filepath[0] ? g_preview.filepath :
                ((E.focus == FOCUS_LEFT && E.num_filtered > 0 && E.selected_note_idx < E.num_filtered) ?
                 E.notes[E.filtered_indices[E.selected_note_idx]].filepath : NULL);

            if (target_preview && (E.focus == FOCUS_LEFT || E.mode == MODE_SEARCH_LEFT)) {
                char preview_path[512];
                safe_strcpy(preview_path, sizeof(preview_path), target_preview);
                int prev_scroll_y = g_preview.scroll_y;
                free_preview();
                open_note_by_path(preview_path, true);
                E.scroll_y = prev_scroll_y;
                if (E.mode == MODE_SEARCH_LEFT) {
                    E.mode = MODE_NORMAL;
                }
                set_status_msg("Aberta em aba: %s", E.cur_filename);
            }
            E.focus = FOCUS_RIGHT;
            int editor_width = E.screen_cols - E.left_width - 1;
            int avail_text_w = editor_width - 7;
            if (avail_text_w <= 5) avail_text_w = 5;

            int screen_row = E.mouse_row - 2;
            int target_vrow = screen_row + E.scroll_y;
            int line_idx = -1, sub_idx = -1;
            LineSegment seg;
            if (find_visual_row(false, target_vrow, avail_text_w, &line_idx, &sub_idx, &seg, NULL)) {
                E.cursor_y = line_idx;
                int col = E.mouse_col - (E.left_width + 1 + 6 + 1);
                if (col < 0) col = 0;
                int x_in_seg = get_byte_offset_in_seg(&E.lines[line_idx], seg.byte_offset, seg.byte_len, col, should_hide_links(false), should_hide_markdown_styles(false));
                E.cursor_x = seg.byte_offset + x_in_seg;
                if (E.cursor_x > E.lines[line_idx].len) E.cursor_x = E.lines[line_idx].len;
            }
            if (E.mode == MODE_INSERT) {
                if (E.visual_start_y != E.cursor_y || E.visual_start_x != E.cursor_x) {
                    E.has_selection = true;
                }
                set_status_msg("-- INSERÇÃO (Texto selecionado com mouse) --");
            } else {
                if (E.mode != MODE_VISUAL_CHAR) {
                    E.mode = MODE_VISUAL_CHAR;
                }
                set_status_msg("-- VISUAL (Seleção com Mouse) --");
            }
            return;
        }
    }

    /* Mouse Release: Finish selection */
    if (E.mouse_released) {
        if (E.mode == MODE_VISUAL_CHAR) {
            if (E.visual_start_y != E.cursor_y || E.visual_start_x != E.cursor_x) {
                set_status_msg("Texto selecionado! (y: copiar | p: colar/substituir | x: recortar | Esc: cancelar)");
            }
        } else if (E.mode == MODE_INSERT && E.has_selection) {
            if (E.visual_start_y != E.cursor_y || E.visual_start_x != E.cursor_x) {
                set_status_msg("Texto selecionado! (Ctrl+V: colar e substituir | Ctrl+C: copiar | Esc: cancelar)");
            }
        }
        return;
    }
}

/* =========================================================================
 * Bottom Command Bar Dispatcher (:w, :q, :wq, :/, :theme, :tabnew, :tabclose)
 * ========================================================================= */

static void execute_bottom_command(const char *cmd) {
    if (!cmd || !*cmd) return;
    while (*cmd == ' ' || *cmd == ':') cmd++;

    if (strcmp(cmd, "w") == 0) {
        save_current_note();
    } else if (strcmp(cmd, "autosave") == 0) {
        set_status_msg("Autosave: ATIVADO (salvamento automático a cada 1 minuto de edições)");
    } else if (strcmp(cmd, "closeall") == 0 || strcmp(cmd, "tabcloseall") == 0 ||
               strcmp(cmd, "ca") == 0 || strcmp(cmd, "fechartudo") == 0 ||
               strcmp(cmd, "close_all") == 0 || strcmp(cmd, "bdall") == 0) {
        close_all_tabs();
    } else if (strcmp(cmd, "q") == 0 || strcmp(cmd, "qa") == 0) {
        /* :q tambem verifica abas em segundo plano: antes so E.modified era
         * olhado e uma alteracao em outra aba podia ser perdida. */
        if (any_tab_modified()) {
            set_status_msg("Modificações não salvas! Use :w para salvar ou :q! para forçar");
        } else {
            E.quit_requested = true;
        }
    } else if (strcmp(cmd, "q!") == 0 || strcmp(cmd, "qa!") == 0) {
        E.quit_requested = true;
    } else if (strcmp(cmd, "wq") == 0 || strcmp(cmd, "x") == 0 || strcmp(cmd, "wqa") == 0 || strcmp(cmd, "wqa!") == 0) {
        /* Salva todas as abas alteradas antes de sair; se qualquer gravacao
         * falhar, permanece no editor com a mensagem de erro. */
        if (save_all_modified_tabs()) {
            E.quit_requested = true;
        }
    } else if (strcmp(cmd, "zm") == 0 || strcmp(cmd, "zM") == 0 || strcmp(cmd, "foldall") == 0) {
        collapse_all_headings();
    } else if (strcmp(cmd, "zr") == 0 || strcmp(cmd, "zR") == 0 || strcmp(cmd, "unfoldall") == 0) {
        uncollapse_all_headings();
    } else if (strcmp(cmd, "za") == 0 || strcmp(cmd, "fold") == 0) {
        toggle_fold_at_cursor();
    } else if (strncmp(cmd, "theme", 5) == 0) {
        const char *tname = cmd + 5;
        while (*tname == ' ') tname++;
        if (*tname == '\0') {
            E.current_theme = (E.current_theme + 1) % NUM_THEMES;
            set_status_msg("Tema alterado para: %s", themes[E.current_theme].name);
        } else {
            bool found = false;
            for (int i = 0; i < NUM_THEMES; i++) {
                if (str_contains_caseless(themes[i].name, tname)) {
                    E.current_theme = i;
                    found = true;
                    set_status_msg("Tema: %s", themes[i].name);
                    break;
                }
            }
            if (!found) {
                set_status_msg("Tema não encontrado: %s", tname);
            }
        }
        g_screen_clear_needed = 1;
    } else if (strcmp(cmd, "tabnew") == 0) {
        /* Cria de fato uma nota no SQLite. Antes era aberto um caminho em
         * data/ (que pode nao existir), gerando uma aba vazia que falhava ao
         * salvar. create_new_note() cuida de nome duplicado e de abrir a aba. */
        char init_name[64];
        snprintf(init_name, sizeof(init_name), "Nova_Nota_%d", E.num_tabs + 1);
        create_new_note(init_name);
    } else if (strcmp(cmd, "nextlink") == 0 || strcmp(cmd, "nl") == 0) {
        jump_to_next_link();
    } else if (strcmp(cmd, "table") == 0 || strcmp(cmd, "tablealign") == 0 || strcmp(cmd, "ta") == 0) {
        if (!table_align_at(E.cursor_y)) {
            table_align_all();
        }
    } else if (strcmp(cmd, "tableall") == 0) {
        table_align_all();
    } else if (strcmp(cmd, "wrapbar") == 0 || strcmp(cmd, "barwrap") == 0 || strcmp(cmd, "statusbar") == 0) {
        if (E.force_status_bar_wrap == 0) {
            E.force_status_bar_wrap = 1;
            set_status_msg("Barra exclusiva de botões: MODO COMPACTO FORÇADO");
        } else if (E.force_status_bar_wrap == 1) {
            E.force_status_bar_wrap = 2;
            set_status_msg("Barra exclusiva de botões: MODO EXPANDIDO FORÇADO");
        } else {
            E.force_status_bar_wrap = 0;
            set_status_msg("Barra exclusiva de botões: MODO AUTOMÁTICO (< 92 colunas)");
        }
        g_screen_clear_needed = 1;
    } else if (strcmp(cmd, "wrap") == 0) {
        E.wrap_mode = !E.wrap_mode;
        set_status_msg("Quebra de linha (wrap): %s", E.wrap_mode ? "ATIVADO" : "DESATIVADO");
    } else if (strcmp(cmd, "nowrap") == 0 || strcmp(cmd, "set nowrap") == 0) {
        E.wrap_mode = false;
        set_status_msg("Quebra de linha (wrap): DESATIVADO");
    } else if (strcmp(cmd, "set wrap") == 0) {
        E.wrap_mode = true;
        set_status_msg("Quebra de linha (wrap): ATIVADO");
    } else if (strncmp(cmd, "rename", 6) == 0 || strncmp(cmd, "rn", 2) == 0) {
        const char *arg = (strncmp(cmd, "rename", 6) == 0) ? cmd + 6 : cmd + 2;
        while (*arg == ' ') arg++;
        if (*arg == '\0') {
            set_status_msg("Uso: :rename <novo_nome> ou :rn <novo_nome>");
        } else {
            const char *target_path = NULL;
            if (E.focus == FOCUS_LEFT && E.num_filtered > 0 && E.selected_note_idx < E.num_filtered) {
                int actual = E.filtered_indices[E.selected_note_idx];
                target_path = E.notes[actual].filepath;
            } else if (E.cur_filepath[0]) {
                target_path = E.cur_filepath;
            }
            if (target_path) {
                rename_note(target_path, arg);
            } else {
                set_status_msg("Nenhuma nota selecionada ou aberta para renomear.");
            }
        }
    } else if (strcmp(cmd, "rescan") == 0) {
        rescan_notes();
        set_status_msg("Notas atualizadas (%d encontradas)", E.num_notes);
    } else if (strncmp(cmd, "import", 6) == 0) {
        const char *arg = cmd + 6;
        while (*arg == ' ') arg++;
        if (*arg == '\0') {
            E.prompt_buf[0] = '\0';
            E.prompt_cursor = 0;
            E.mode = MODE_PROMPT_IMPORT_DIR;
        } else {
            ui_import_from_dir(arg);
        }
    } else if (strncmp(cmd, "export", 6) == 0) {
        const char *arg = cmd + 6;
        while (*arg == ' ') arg++;
        if (*arg == '\0') {
            E.prompt_buf[0] = '\0';
            E.prompt_cursor = 0;
            E.mode = MODE_PROMPT_EXPORT_DIR;
        } else {
            ui_export_to_dir(arg);
        }
    } else if (strncmp(cmd, "filter", 6) == 0 || strncmp(cmd, "filtro", 6) == 0) {
        /* Filter notes by urgency level 1, 2, 3, 4(H), 5(R), 6(O) or 0 (all) */
        const char *arg = strchr(cmd, ' ');
        if (arg) {
            while (*arg == ' ') arg++;
            if (*arg == '1') E.filter_urgency = 1;
            else if (*arg == '2') E.filter_urgency = 2;
            else if (*arg == '3') E.filter_urgency = 3;
            else if (*arg == '4' || *arg == 'h' || *arg == 'H') E.filter_urgency = 4;
            else if (*arg == '5' || *arg == 'r' || *arg == 'R') E.filter_urgency = 5;
            else if (*arg == '6' || *arg == 'o' || *arg == 'O') E.filter_urgency = 6;
            else E.filter_urgency = 0;
        } else {
            E.filter_urgency = (E.filter_urgency + 1) % 7;
        }
        apply_note_search(E.search_query);
        if (E.filter_urgency == 1) set_status_msg("🔴 Filtrando Nível 1 (Vermelho): %d nota(s)", E.num_filtered);
        else if (E.filter_urgency == 2) set_status_msg("🟠 Filtrando Nível 2 (Laranja): %d nota(s)", E.num_filtered);
        else if (E.filter_urgency == 3) set_status_msg("🟡 Filtrando Nível 3 (Amarelo): %d nota(s)", E.num_filtered);
        else if (E.filter_urgency == 4) set_status_msg("🔵 Filtrando Marcação [H] (Azul): %d nota(s)", E.num_filtered);
        else if (E.filter_urgency == 5) set_status_msg("🟣 Filtrando Marcação [R] (Roxo): %d nota(s)", E.num_filtered);
        else if (E.filter_urgency == 6) set_status_msg("🟢 Filtrando Marcação [O] (Verde): %d nota(s)", E.num_filtered);
        else set_status_msg("Filtro removido: exibindo todas as %d notas", E.num_filtered);
    } else if (strncmp(cmd, "atencao", 7) == 0 || strncmp(cmd, "attention", 9) == 0 || strncmp(cmd, "mark", 4) == 0) {
        /* Toggle or set specific attention level for current note */
        int target_level = -1;
        const char *arg = strchr(cmd, ' ');
        if (arg) {
            while (*arg == ' ') arg++;
            if (*arg >= '0' && *arg <= '3') target_level = *arg - '0';
            else if (*arg == '4' || *arg == 'h' || *arg == 'H') target_level = 4;
            else if (*arg == '5' || *arg == 'r' || *arg == 'R') target_level = 5;
            else if (*arg == '6' || *arg == 'o' || *arg == 'O') target_level = 6;
        }
        for (int i = 0; i < E.num_notes; i++) {
            if (strcmp(E.notes[i].filepath, E.cur_filepath) == 0) {
                if (target_level >= 0) {
                    int nid = note_id_from_path(E.notes[i].filepath);
                    if (nid > 0 && !db_set_attention(nid, target_level)) {
                        set_status_msg("Falha ao gravar a marcacao de urgencia no banco.");
                        break;
                    }
                    char keep[512];
                    safe_strcpy(keep, sizeof(keep), E.notes[i].filepath);
                    E.notes[i].attention_level = target_level;
                    E.notes[i].needs_attention = (target_level > 0);
                    notes_resort_keep(keep);
                    g_screen_clear_needed = 1;
                    switch (target_level) {
                        case 1: set_status_msg("🔴 Urgência Nível 1 [Crítica - Vermelho]: %s", E.notes[i].title); break;
                        case 2: set_status_msg("🟠 Urgência Nível 2 [Média - Laranja]: %s", E.notes[i].title); break;
                        case 3: set_status_msg("🟡 Urgência Nível 3 [Baixa - Amarelo]: %s", E.notes[i].title); break;
                        case 4: set_status_msg("🔵 Marcação [H - Azul]: %s", E.notes[i].title); break;
                        case 5: set_status_msg("🟣 Marcação [R - Roxo]: %s", E.notes[i].title); break;
                        case 6: set_status_msg("🟢 Marcação [O - Verde]: %s", E.notes[i].title); break;
                        default: set_status_msg("Marcação de urgência removida: %s", E.notes[i].title); break;
                    }
                } else {
                    toggle_note_attention_at(i);
                }
                break;
            }
        }
    } else if (strncmp(cmd, "s/", 2) == 0 || strncmp(cmd, "%s/", 3) == 0 || strncmp(cmd, "sub/", 4) == 0) {
        /* Vim style substitute :s/antigo/novo/ or :%s/antigo/novo/g */
        const char *p = (cmd[0] == '%') ? cmd + 3 : (cmd[0] == 's' ? cmd + 2 : cmd + 4);
        char f_str[128] = {0};
        char r_str[128] = {0};
        const char *slash2 = strchr(p, '/');
        if (slash2) {
            int flen = slash2 - p;
            if (flen < (int)sizeof(f_str)) {
                strncpy(f_str, p, flen);
                const char *r_start = slash2 + 1;
                const char *slash3 = strchr(r_start, '/');
                if (slash3) {
                    int rlen = slash3 - r_start;
                    if (rlen < (int)sizeof(r_str)) strncpy(r_str, r_start, rlen);
                } else {
                    safe_strcpy(r_str, sizeof(r_str), r_start);
                }
                replace_in_current_note(f_str, r_str);
            }
        } else {
            set_status_msg("Uso: :s/antigo/novo/ ou :sub/antigo/novo/");
        }
    } else if (strcmp(cmd, "tabclose") == 0 || strcmp(cmd, "tabc") == 0) {
        tab_close(E.active_tab);
    } else if (strcmp(cmd, "tabnext") == 0 || strcmp(cmd, "tabn") == 0) {
        tab_next();
    } else if (strcmp(cmd, "tabprev") == 0 || strcmp(cmd, "tabp") == 0) {
        tab_prev();
    } else if (cmd[0] == '/') {
        const char *pat = cmd + 1;
        safe_strcpy(E.last_search_pat, sizeof(E.last_search_pat), pat);
        bool found = false;
        for (int i = 0; i < E.num_lines; i++) {
            if (E.lines[i].chars && str_contains_caseless(E.lines[i].chars, pat)) {
                E.cursor_y = i;
                E.cursor_x = 0;
                set_status_msg("Padrão encontrado na linha %d", i + 1);
                found = true;
                break;
            }
        }
        if (!found) set_status_msg("Padrão não encontrado: %s", pat);
    } else if (strcmp(cmd, "new") == 0 || strcmp(cmd, "nova") == 0 || strcmp(cmd, "n") == 0) {
        E.prompt_buf[0] = '\0';
        E.prompt_cursor = 0;
        E.mode = MODE_PROMPT_NEW_NOTE;
    } else if (strcmp(cmd, "gameoflife") == 0 || strcmp(cmd, "life") == 0 || strcmp(cmd, "gol") == 0) {
        run_game_of_life();
        g_last_activity_time = get_current_time_ms();
        set_status_msg("Simulação do Game of Life concluída");
    } else if (strcmp(cmd, "delete") == 0 || strcmp(cmd, "del") == 0 || strcmp(cmd, "excluir") == 0 || strcmp(cmd, "d") == 0) {
        if (E.num_filtered > 0 && E.selected_note_idx < E.num_filtered) {
            E.mode = MODE_CONFIRM_DELETE_NOTE;
        } else {
            set_status_msg("Nenhuma nota selecionada para excluir.");
        }
    } else if (strncmp(cmd, "new ", 4) == 0 || strncmp(cmd, "e ", 2) == 0) {
        const char *target = strchr(cmd, ' ') + 1;
        while (*target == ' ') target++;
        open_or_create_wikilink_note(target);
    } else if (cmd[0] >= '0' && cmd[0] <= '9') {
        int target_line = atoi(cmd);
        if (target_line > 0 && E.num_lines > 0) {
            if (target_line > E.num_lines) target_line = E.num_lines;
            E.cursor_y = target_line - 1;
            E.cursor_x = 0;
            if (E.lines[E.cursor_y].hidden) {
                for (int i = E.cursor_y; i >= 0; i--) {
                    if (E.lines[i].heading_lvl > 0 && E.lines[i].folded) {
                        E.lines[i].folded = false;
                    }
                }
                update_folding();
            }
            editor_scroll();
            g_screen_clear_needed = 1;
            set_status_msg("Linha %d/%d", E.cursor_y + 1, E.num_lines);
        }
    } else {
        set_status_msg("Comando não reconhecido: :%s", cmd);
    }
}

/* =========================================================================
 * Single-Line Prompt Editor Helper (Arrows, Home, End, Delete, UTF-8)
 * ========================================================================= */
static bool handle_prompt_line_edit(char *buf, size_t max_sz, int *cursor, int c) {
    int plen = (int)strlen(buf);
    if (*cursor < 0) *cursor = 0;
    if (*cursor > plen) *cursor = plen;

    if (c == KEY_ARROW_LEFT) {
        if (*cursor > 0) {
            (*cursor)--;
            while (*cursor > 0 && ((unsigned char)buf[*cursor] & 0xC0) == 0x80) {
                (*cursor)--;
            }
        }
        return true;
    }
    if (c == KEY_ARROW_RIGHT) {
        if (*cursor < plen) {
            int clen = utf8_charlen((unsigned char)buf[*cursor]);
            if (clen <= 0) clen = 1;
            *cursor += clen;
            if (*cursor > plen) *cursor = plen;
        }
        return true;
    }
    if (c == KEY_CTRL_LEFT) {
        while (*cursor > 0 && buf[*cursor - 1] == ' ') (*cursor)--;
        while (*cursor > 0 && buf[*cursor - 1] != ' ') {
            (*cursor)--;
            while (*cursor > 0 && ((unsigned char)buf[*cursor] & 0xC0) == 0x80) (*cursor)--;
        }
        return true;
    }
    if (c == KEY_CTRL_RIGHT) {
        while (*cursor < plen && buf[*cursor] != ' ') {
            int clen = utf8_charlen((unsigned char)buf[*cursor]);
            if (clen <= 0) clen = 1;
            *cursor += clen;
        }
        while (*cursor < plen && buf[*cursor] == ' ') (*cursor)++;
        return true;
    }
    if (c == KEY_HOME || c == 1) { /* Home or Ctrl+A */
        *cursor = 0;
        return true;
    }
    if (c == KEY_END || c == 5) { /* End or Ctrl+E */
        *cursor = plen;
        return true;
    }
    if (c == KEY_BACKSPACE || c == 8 || c == 127) {
        if (*cursor > 0) {
            int prev = *cursor - 1;
            while (prev > 0 && ((unsigned char)buf[prev] & 0xC0) == 0x80) {
                prev--;
            }
            memmove(&buf[prev], &buf[*cursor], plen - *cursor + 1);
            *cursor = prev;
        }
        return true;
    }
    if (c == KEY_DELETE) {
        if (*cursor < plen) {
            int clen = utf8_charlen((unsigned char)buf[*cursor]);
            if (clen <= 0) clen = 1;
            memmove(&buf[*cursor], &buf[*cursor + clen], plen - (*cursor + clen) + 1);
        }
        return true;
    }
    if (c == 21) { /* Ctrl+U: clear all */
        buf[0] = '\0';
        *cursor = 0;
        return true;
    }
    if (c == 11) { /* Ctrl+K: clear to end */
        buf[*cursor] = '\0';
        return true;
    }
    if ((unsigned char)c >= 32 && (unsigned char)c != 127) {
        if (plen + g_last_utf8_len < (int)max_sz - 4) {
            memmove(&buf[*cursor + g_last_utf8_len], &buf[*cursor], plen - *cursor + 1);
            memcpy(&buf[*cursor], g_last_utf8_char, g_last_utf8_len);
            *cursor += g_last_utf8_len;
        }
        return true;
    }
    return false;
}

/* =========================================================================
 * Main Keypress Processor
 * ========================================================================= */

static void terminal_write(const char *s, int len) {
    (void)write_stdout_all(s, len);
}

static void run_game_of_life(void) {
    int rows = E.screen_rows;
    int cols = E.screen_cols;
    if (rows < 6 || cols < 20) return;

    int total_cells = rows * cols;
    uint8_t *grid = (uint8_t *)calloc(total_cells, 1);
    uint8_t *next = (uint8_t *)calloc(total_cells, 1);
    uint8_t *age = (uint8_t *)calloc(total_cells, 1);
    if (!grid || !next || !age) {
        if (grid) free(grid);
        if (next) free(next);
        if (age) free(age);
        return;
    }

    /* Seed initial live cells with diverse patterns */
    srand((unsigned int)time(NULL));
    for (int i = 0; i < total_cells; i++) {
        if ((rand() % 100) < 22) { /* 22% initial density */
            grid[i] = 1;
            age[i] = 1;
        }
    }

    /* Clear screen and hide cursor */
    terminal_write("\x1b[?25l\x1b[2J\x1b[3J\x1b[H", 17);

    int generation = 0;
    int stagnant_count = 0;
    int last_live_count = 0;
    int64_t start_time = get_current_time_ms();

    while (1) {
        /* Compute next generation (toroidal wrap on rows - 1) */
        int active_rows = rows - 1; /* leave bottom line for status */
        int live_count = 0;
        for (int r = 0; r < active_rows; r++) {
            int r_up = (r == 0) ? active_rows - 1 : r - 1;
            int r_down = (r == active_rows - 1) ? 0 : r + 1;
            for (int c = 0; c < cols; c++) {
                int c_left = (c == 0) ? cols - 1 : c - 1;
                int c_right = (c == cols - 1) ? 0 : c + 1;

                int neighbors = grid[r_up * cols + c_left] +
                                grid[r_up * cols + c] +
                                grid[r_up * cols + c_right] +
                                grid[r * cols + c_left] +
                                grid[r * cols + c_right] +
                                grid[r_down * cols + c_left] +
                                grid[r_down * cols + c] +
                                grid[r_down * cols + c_right];

                int idx = r * cols + c;
                if (grid[idx]) {
                    if (neighbors == 2 || neighbors == 3) {
                        next[idx] = 1;
                        age[idx] = (age[idx] < 250) ? age[idx] + 1 : 250;
                        live_count++;
                    } else {
                        next[idx] = 0;
                        age[idx] = 0;
                    }
                } else {
                    if (neighbors == 3) {
                        next[idx] = 1;
                        age[idx] = 1;
                        live_count++;
                    } else {
                        next[idx] = 0;
                        age[idx] = 0;
                    }
                }
            }
        }

        /* Detect population stagnation or extinction */
        if (live_count == last_live_count) {
            stagnant_count++;
        } else {
            stagnant_count = 0;
        }
        last_live_count = live_count;

        /* If died out or stagnant, inject new life patterns */
        if (live_count < 10 || stagnant_count > 30) {
            for (int i = 0; i < total_cells; i++) {
                if ((rand() % 100) < 18) {
                    next[i] = 1;
                    age[i] = 1;
                }
            }
            stagnant_count = 0;
        }

        /* Render frame */
        ABuffer ab;
        ab_init(&ab);
        ab_append(&ab, "\x1b[H", 3);
        ab_append(&ab, TH_BG_MAIN, -1);

        for (int r = 0; r < active_rows; r++) {
            for (int c = 0; c < cols; c++) {
                int idx = r * cols + c;
                if (next[idx]) {
                    int a = age[idx];
                    if (a <= 1) {
                        ab_append(&ab, TH_FG_ACCENT, -1);
                        ab_append(&ab, STYLE_BOLD, -1);
                        ab_append(&ab, "●", 3);
                    } else if (a <= 4) {
                        ab_append(&ab, TH_FG_H1, -1);
                        ab_append(&ab, STYLE_BOLD, -1);
                        ab_append(&ab, "o", 1);
                    } else if (a <= 10) {
                        ab_append(&ab, TH_FG_H2, -1);
                        ab_append(&ab, "•", 3);
                    } else {
                        ab_append(&ab, TH_FG_MUTED, -1);
                        ab_append(&ab, "·", 2);
                    }
                } else {
                    ab_append(&ab, " ", 1);
                }
            }
            ab_append(&ab, "\r\n", 2);
        }

        /* Status bar at the bottom row */
        ab_append(&ab, TH_BG_STATUS, -1);
        ab_append(&ab, TH_FG_MUTED, -1);
        char bar[256];
        snprintf(bar, sizeof(bar), " 🧬 Conway's Game of Life [%s] │ Geração: %d │ Células Vivas: %d",
                 themes[E.current_theme].name, ++generation, live_count);
        ab_append(&ab, bar, (int)strlen(bar));
        int blen = utf8_width(bar);
        const char *hint = "[Pressione qualquer tecla ou clique para sair] ";
        int hlen = utf8_width(hint);
        int pad = cols - blen - hlen;
        if (pad < 1) pad = 1;
        while (pad-- > 0) ab_append(&ab, " ", 1);
        ab_append(&ab, TH_FG_H1, -1);
        ab_append(&ab, STYLE_BOLD, -1);
        ab_append(&ab, hint, (int)strlen(hint));
        ab_append(&ab, "\x1b[K" STYLE_RESET, -1);

        terminal_write(ab.b, ab.len);
        ab_free(&ab);

        /* Swap buffer */
        memcpy(grid, next, total_cells);

        /* Automatic background saving during simulation */
        check_autosave();

        /* Check if user pressed any key or mouse to wake up / stop */
        int key = editor_read_key();
        if (key != -1) {
            /* If launched via mouse click, ignore mouse release event within first 350ms */
            if (key == KEY_MOUSE_EVENT && E.mouse_released && (get_current_time_ms() - start_time < 350)) {
                /* Discard initial button release from button click */
            } else {
                break;
            }
        }
        if (g_window_resized) {
            break;
        }
    }

    free(grid);
    free(next);
    free(age);

    /* Drain any remaining input events to prevent unwanted side-effects */
    g_input_q_head = g_input_q_tail = 0;
    g_unget_byte = -1;
#ifdef _WIN32
    FlushConsoleInputBuffer(GetStdHandle(STD_INPUT_HANDLE));
#else
    tcflush(STDIN_FILENO, TCIFLUSH);
#endif

    /* Restore screen and cursor */
    g_screen_clear_needed = 1;
    terminal_write("\x1b[2J\x1b[3J\x1b[H\x1b[?25h", 17);
}

static void editor_process_keypress(void) {
    check_autosave();
    int c = editor_read_key();
    if (c == -2 || g_window_resized) {
        g_window_resized = 0;
        return;
    }
    if (c == -1) {
        int64_t now = get_current_time_ms();
        if (g_last_activity_time > 0 && now - g_last_activity_time >= GAMEOFLIFE_IDLE_TIMEOUT_MS) { /* 10 minutes inactivity */
            run_game_of_life();
            g_last_activity_time = get_current_time_ms();
        }
        return;
    }

    g_last_activity_time = get_current_time_ms();

    /* Dirty-flag: chegou um evento real (tecla ou mouse). Em vez de marcar o
     * desenho em cada ponto de mutacao de estado (seriam centenas de lugares),
     * marca-se uma unica vez aqui. O redesenho acontece no inicio da proxima
     * iteracao do loop principal, ou seja, sempre antes de bloquear de novo. */
    g_needs_redraw = 1;

    if (c == KEY_MOUSE_EVENT) {
        handle_mouse_event();
        return;
    }

    /* ---------------------------------------------------------------------
     * Modal Prompts have highest priority (protected from all shortcuts)
     * --------------------------------------------------------------------- */
    if (E.mode >= MODE_PROMPT_NEW_NOTE && E.mode <= MODE_PROMPT_EXPORT_DIR) {
        g_last_typing_time = get_current_time_ms();
        if (c == KEY_ESC || c == 3) {
            if (E.mode == MODE_PROMPT_NEW_NOTE) set_status_msg("Criação de nota cancelada.");
            else if (E.mode == MODE_PROMPT_RENAME_NOTE) set_status_msg("Renomeação cancelada.");
            else if (E.mode == MODE_PROMPT_FIND) set_status_msg("Busca cancelada.");
            else if (E.mode == MODE_PROMPT_REPLACE_FIND || E.mode == MODE_PROMPT_REPLACE_WITH) set_status_msg("Substituição cancelada.");
            else if (E.mode == MODE_PROMPT_IMPORT_DIR) set_status_msg("Importação cancelada.");
            else if (E.mode == MODE_PROMPT_EXPORT_DIR) set_status_msg("Exportação cancelada.");
            E.mode = MODE_NORMAL;
            E.prompt_buf[0] = '\0';
            E.prompt_cursor = 0;
            E.rename_target_path[0] = '\0';
            return;
        }
        if (c == '\r' || c == '\n') {
            if (E.mode == MODE_PROMPT_IMPORT_DIR) {
                char dir_in[256];
                safe_strcpy(dir_in, sizeof(dir_in), E.prompt_buf);
                E.mode = MODE_NORMAL;
                E.prompt_buf[0] = '\0';
                E.prompt_cursor = 0;
                ui_import_from_dir(dir_in);
                E.focus = FOCUS_LEFT;
                g_screen_clear_needed = 1;
            } else if (E.mode == MODE_PROMPT_EXPORT_DIR) {
                char dir_out[256];
                safe_strcpy(dir_out, sizeof(dir_out), E.prompt_buf);
                E.mode = MODE_NORMAL;
                E.prompt_buf[0] = '\0';
                E.prompt_cursor = 0;
                ui_export_to_dir(dir_out);
            } else if (E.mode == MODE_PROMPT_NEW_NOTE) {
                E.mode = MODE_NORMAL;
                create_new_note(E.prompt_buf);
                E.prompt_buf[0] = '\0';
                E.prompt_cursor = 0;
            } else if (E.mode == MODE_PROMPT_RENAME_NOTE) {
                E.mode = MODE_NORMAL;
                rename_note(E.rename_target_path, E.prompt_buf);
                E.prompt_buf[0] = '\0';
                E.prompt_cursor = 0;
                E.rename_target_path[0] = '\0';
            } else if (E.mode == MODE_PROMPT_FIND) {
                E.mode = MODE_NORMAL;
                if (E.prompt_buf[0]) {
                    safe_strcpy(E.last_search_pat, sizeof(E.last_search_pat), E.prompt_buf);
                    bool found = false;
                    for (int i = E.cursor_y; i < E.num_lines; i++) {
                        if (E.lines[i].chars && str_contains_caseless(E.lines[i].chars, E.last_search_pat)) {
                            E.cursor_y = i;
                            E.cursor_x = 0;
                            set_status_msg("Padrão encontrado na linha %d (use n para próximo)", i + 1);
                            found = true;
                            break;
                        }
                    }
                    if (!found) {
                        for (int i = 0; i < E.cursor_y; i++) {
                            if (E.lines[i].chars && str_contains_caseless(E.lines[i].chars, E.last_search_pat)) {
                                E.cursor_y = i;
                                E.cursor_x = 0;
                                set_status_msg("Padrão encontrado na linha %d (reinício)", i + 1);
                                found = true;
                                break;
                            }
                        }
                    }
                    if (!found) set_status_msg("Padrão não encontrado: %s", E.last_search_pat);
                }
                E.prompt_buf[0] = '\0';
                E.prompt_cursor = 0;
            } else if (E.mode == MODE_PROMPT_REPLACE_FIND) {
                if (E.prompt_buf[0] == '\0') {
                    E.mode = MODE_NORMAL;
                    set_status_msg("Substituição cancelada (termo vazio).");
                } else {
                    safe_strcpy(E.replace_find_buf, sizeof(E.replace_find_buf), E.prompt_buf);
                    E.prompt_buf[0] = '\0';
                    E.prompt_cursor = 0;
                    E.mode = MODE_PROMPT_REPLACE_WITH;
                    set_status_msg("Substituir '%s' por (digite o novo texto e pressione Enter):", E.replace_find_buf);
                }
            } else if (E.mode == MODE_PROMPT_REPLACE_WITH) {
                safe_strcpy(E.replace_with_buf, sizeof(E.replace_with_buf), E.prompt_buf);
                E.mode = MODE_NORMAL;
                replace_in_current_note(E.replace_find_buf, E.replace_with_buf);
                E.prompt_buf[0] = '\0';
                E.prompt_cursor = 0;
            }
            return;
        }
        if (handle_prompt_line_edit(E.prompt_buf, sizeof(E.prompt_buf), &E.prompt_cursor, c)) {
            return;
        }
        return; /* Consume any other key during modal prompt */
    }

    if (E.mode == MODE_CONFIRM_DELETE_NOTE) {
        if (c == 's' || c == 'S' || c == 'y' || c == 'Y') {
            E.mode = MODE_NORMAL;
            execute_delete_selected_note();
            return;
        } else if (c == 'n' || c == 'N' || c == KEY_ESC || c == 3 || c == '\r' || c == '\n') {
            E.mode = MODE_NORMAL;
            set_status_msg("Exclusão cancelada.");
            return;
        }
        return;
    }

    /* Requirement: Esc key ALWAYS immediately returns to Normal mode in Editor */
    if (c == KEY_ESC) {
        if (E.mode == MODE_SEARCH_LEFT) {
            E.search_query[0] = '\0';
            apply_note_search("");
            if (E.num_filtered > 0) {
                int actual = E.filtered_indices[E.selected_note_idx];
                open_note_by_path(E.notes[actual].filepath, false);
            }
        } else if (E.mode == MODE_COMMAND) {
            E.cmd_buf[0] = '\0';
        } else if (E.mode == MODE_CONFIRM_DELETE_NOTE) {
            set_status_msg("Exclusão cancelada.");
        } else if (E.mode == MODE_PROMPT_NEW_NOTE) {
            E.prompt_buf[0] = '\0';
            set_status_msg("Criação de nota cancelada.");
        } else if (E.mode == MODE_PROMPT_RENAME_NOTE) {
            E.prompt_buf[0] = '\0';
            E.rename_target_path[0] = '\0';
            set_status_msg("Renomeação cancelada.");
        } else if (E.mode == MODE_INSERT) {
            if (E.cursor_x > 0) E.cursor_x--;
        }
        E.mode = MODE_NORMAL;
        E.focus = FOCUS_RIGHT;
        E.pending_op = 0;
        set_status_msg("-- MODO NORMAL --");
        return;
    }

    /* Shortcut Ctrl+S (19): Save current open note in Normal mode */
    if (c == 19 && E.mode == MODE_NORMAL) {
        save_current_note();
        return;
    }

    /* Shortcut Ctrl+T: Cycle Theme */
    if (c == 20 && E.mode == MODE_NORMAL) {
        E.current_theme = (E.current_theme + 1) % NUM_THEMES;
        g_screen_clear_needed = 1;
        set_status_msg("Tema alterado para: %s", themes[E.current_theme].name);
        return;
    }

    /* Requirement 20: Adjust left panel width using Ctrl+Left and Ctrl+Right */
    if (c == KEY_CTRL_LEFT && E.mode == MODE_NORMAL) {
        if (E.left_width > MIN_LEFT_WIDTH) E.left_width -= 2;
        set_status_msg("Largura do painel: %d", E.left_width);
        return;
    }
    if (c == KEY_CTRL_RIGHT && E.mode == MODE_NORMAL) {
        if (E.left_width < E.screen_cols - 25) E.left_width += 2;
        set_status_msg("Largura do painel: %d", E.left_width);
        return;
    }

    /* Requirement 3: Ctrl+F to activate search entry at top of left panel */
    if (c == 6 && (E.mode == MODE_NORMAL || E.mode == MODE_SEARCH_LEFT)) { /* Ctrl+F */
        E.mode = MODE_SEARCH_LEFT;
        E.focus = FOCUS_LEFT;
        E.search_query[0] = '\0';
        E.search_cursor = 0;
        apply_note_search("");
        if (E.num_filtered > 0) {
            int actual = E.filtered_indices[E.selected_note_idx];
            load_preview(E.notes[actual].filepath);
        } else {
            free_preview();
        }
        set_status_msg("Busca de notas (digite o termo, Enter para abrir)");
        return;
    }

    /* Shortcut Ctrl+N (14): Jump to next link in note */
    if (c == 14 && E.mode == MODE_NORMAL) {
        jump_to_next_link();
        return;
    }

    /* Tab or Ctrl+W: Toggle focus between left and right panel (or align table if on table line) */
    if ((c == '\t' || c == 23) && (E.mode == MODE_NORMAL || E.mode == MODE_SEARCH_LEFT)) {
        if (c == '\t' && E.focus == FOCUS_RIGHT && is_table_line(&E.lines[E.cursor_y])) {
            table_jump_next_cell(false);
            return;
        }
        if (E.focus == FOCUS_LEFT) {
            free_preview();
            E.focus = FOCUS_RIGHT;
        } else {
            E.focus = FOCUS_LEFT;
            if (E.num_filtered > 0 && E.selected_note_idx < E.num_filtered) {
                int actual = E.filtered_indices[E.selected_note_idx];
                load_preview(E.notes[actual].filepath);
            }
        }
        set_status_msg("Painel ativo: %s", E.focus == FOCUS_LEFT ? "Notas (use j/k e Enter)" : "Editor de Texto");
        return;
    }



    /* ---------------------------------------------------------------------
     * Search Left Panel Mode
     * --------------------------------------------------------------------- */
    if (E.mode == MODE_SEARCH_LEFT) {
        if (c == KEY_ESC) {
            free_preview();
            E.mode = MODE_NORMAL;
            E.search_query[0] = '\0';
            apply_note_search("");
            E.focus = FOCUS_RIGHT;
            return;
        }
        if (c == '\r' || c == '\n') {
            E.mode = MODE_NORMAL;
            if (E.num_filtered > 0) {
                int actual = E.filtered_indices[E.selected_note_idx];
                free_preview();
                open_note_by_path(E.notes[actual].filepath, true);
                E.focus = FOCUS_RIGHT;
                set_status_msg("Aberta em aba: %s", E.cur_filename);
            }
            return;
        }
        if (c == KEY_ARROW_UP) {
            if (E.selected_note_idx > 0) {
                E.selected_note_idx--;
                int actual = E.filtered_indices[E.selected_note_idx];
                load_preview(E.notes[actual].filepath);
            }
            return;
        }
        if (c == KEY_ARROW_DOWN) {
            if (E.selected_note_idx + 1 < E.num_filtered) {
                E.selected_note_idx++;
                int actual = E.filtered_indices[E.selected_note_idx];
                load_preview(E.notes[actual].filepath);
            }
            return;
        }
        if (c == 21) { /* Ctrl+U: clear query */
            E.search_query[0] = '\0';
            E.search_cursor = 0;
            apply_note_search("");
            if (E.num_filtered > 0) {
                int actual = E.filtered_indices[E.selected_note_idx];
                load_preview(E.notes[actual].filepath);
            } else {
                free_preview();
            }
            return;
        }
        if (c == KEY_BACKSPACE || c == 8 || c == 127) {
            int qlen = (int)strlen(E.search_query);
            if (qlen > 0) {
                qlen--;
                while (qlen > 0 && (unsigned char)E.search_query[qlen] >= 0x80 && (unsigned char)E.search_query[qlen] < 0xC0) {
                    qlen--;
                }
                E.search_query[qlen] = '\0';
                apply_note_search(E.search_query);
                if (E.num_filtered > 0) {
                    int actual = E.filtered_indices[E.selected_note_idx];
                    load_preview(E.notes[actual].filepath);
                } else {
                    free_preview();
                }
            }
            return;
        }
        if ((unsigned char)c >= 32 && (unsigned char)c != 127) {
            int qlen = (int)strlen(E.search_query);
            if (qlen + g_last_utf8_len < 250) {
                memcpy(&E.search_query[qlen], g_last_utf8_char, g_last_utf8_len);
                E.search_query[qlen + g_last_utf8_len] = '\0';
                apply_note_search(E.search_query);
                if (E.num_filtered > 0) {
                    int actual = E.filtered_indices[E.selected_note_idx];
                    load_preview(E.notes[actual].filepath);
                } else {
                    free_preview();
                }
            }
            return;
        }
        return;
    }

    /* ---------------------------------------------------------------------
     * Command Mode (:w, :q, :wq, :/, :theme, :tabclose)
     * --------------------------------------------------------------------- */
    if (E.mode == MODE_COMMAND) {
        if (c == KEY_ESC) {
            E.mode = MODE_NORMAL;
            E.cmd_buf[0] = '\0';
            E.cmd_cursor = 0;
            return;
        }
        if (c == '\r' || c == '\n') {
            E.mode = MODE_NORMAL;
            execute_bottom_command(E.cmd_buf);
            E.cmd_buf[0] = '\0';
            E.cmd_cursor = 0;
            return;
        }
        if (c == KEY_BACKSPACE || c == 8 || c == 127) {
            if (E.cmd_cursor == 0 && E.cmd_buf[0] == '\0') {
                E.mode = MODE_NORMAL;
                return;
            }
        }
        if (handle_prompt_line_edit(E.cmd_buf, sizeof(E.cmd_buf), &E.cmd_cursor, c)) {
            return;
        }
        return;
    }

    /* ---------------------------------------------------------------------
     * Left Panel Focused Navigation
     * --------------------------------------------------------------------- */
    if (E.focus == FOCUS_LEFT) {
        if (c == 'j' || c == KEY_ARROW_DOWN) {
            if (E.selected_note_idx + 1 < E.num_filtered) {
                E.selected_note_idx++;
                if (E.selected_note_idx >= E.sidebar_scroll_y + (E.screen_rows - 5)) {
                    E.sidebar_scroll_y++;
                }
                int actual = E.filtered_indices[E.selected_note_idx];
                load_preview(E.notes[actual].filepath);
            }
            return;
        }
        if (c == 'k' || c == KEY_ARROW_UP) {
            if (E.selected_note_idx > 0) {
                E.selected_note_idx--;
                if (E.selected_note_idx < E.sidebar_scroll_y) {
                    E.sidebar_scroll_y = E.selected_note_idx;
                }
                int actual = E.filtered_indices[E.selected_note_idx];
                load_preview(E.notes[actual].filepath);
            }
            return;
        }
        if (c == '\r' || c == '\n') {
            if (E.num_filtered > 0) {
                int actual = E.filtered_indices[E.selected_note_idx];
                free_preview();
                open_note_by_path(E.notes[actual].filepath, true);
                E.focus = FOCUS_RIGHT;
                set_status_msg("Aberta em aba: %s", E.cur_filename);
            }
            return;
        }
        if (c == 'l' || c == KEY_ARROW_RIGHT) {
            if (E.num_filtered > 0) {
                int actual = E.filtered_indices[E.selected_note_idx];
                int prev_scroll_y = g_preview.scroll_y;
                free_preview();
                open_note_by_path(E.notes[actual].filepath, true);
                E.scroll_y = prev_scroll_y;
                set_status_msg("Aberta em aba: %s", E.cur_filename);
            } else {
                free_preview();
                set_status_msg("Foco no Editor (Tab para voltar às Notas)");
            }
            E.focus = FOCUS_RIGHT;
            return;
        }
        if (c == 'i' || c == 'a') {
            if (E.num_filtered > 0) {
                int actual = E.filtered_indices[E.selected_note_idx];
                free_preview();
                open_note_by_path(E.notes[actual].filepath, true);
                E.focus = FOCUS_RIGHT;
                E.mode = MODE_INSERT;
                set_status_msg("Modo Inserção: %s", E.cur_filename);
            }
            return;
        }
        if (c == 'h' || c == KEY_ARROW_LEFT || c == KEY_ESC) {
            free_preview();
            E.focus = FOCUS_RIGHT;
            set_status_msg("Foco no Editor (Tab para voltar às Notas)");
            return;
        }
        if (c == 'c') {
            if (E.num_tabs > 0) {
                tab_close(E.active_tab);
            } else {
                set_status_msg("Nenhuma aba aberta.");
            }
            return;
        }
        if (c == 'C') {
            close_all_tabs();
            return;
        }
        if (c == 'y' || c == 'Y') {
            if (E.num_filtered > 0 && E.selected_note_idx < E.num_filtered) {
                int actual = E.filtered_indices[E.selected_note_idx];
                char title_name[512];
                extract_note_title(E.notes[actual].filepath, E.notes[actual].filename, title_name, sizeof(title_name));
                copy_to_system_clipboard(title_name, (int)strlen(title_name));
                set_status_msg("Nome copiado p/ clipboard: %s (use [[%s]])", title_name, title_name);
            } else {
                set_status_msg("Nenhuma nota selecionada para copiar.");
            }
            return;
        }
        if (c == 'd') {
            if (E.num_filtered > 0 && E.selected_note_idx < E.num_filtered) {
                E.mode = MODE_CONFIRM_DELETE_NOTE;
            } else {
                set_status_msg("Nenhuma nota selecionada para excluir.");
            }
            return;
        }
        if (c == 'n') {
            E.prompt_buf[0] = '\0';
            E.prompt_cursor = 0;
            E.mode = MODE_PROMPT_NEW_NOTE;
            return;
        }
        if (c == 'r') {
            if (E.num_filtered > 0 && E.selected_note_idx < E.num_filtered) {
                int actual = E.filtered_indices[E.selected_note_idx];
                safe_strcpy(E.rename_target_path, sizeof(E.rename_target_path), E.notes[actual].filepath);
                const char *cur_name = E.notes[actual].title;
                safe_strcpy(E.prompt_buf, sizeof(E.prompt_buf), cur_name);
                E.prompt_cursor = (int)strlen(E.prompt_buf);
                E.mode = MODE_PROMPT_RENAME_NOTE;
            } else {
                set_status_msg("Nenhuma nota selecionada para renomear.");
            }
            return;
        }
        if (c == 'm' || c == '!') {
            if (E.num_filtered > 0 && E.selected_note_idx < E.num_filtered) {
                int actual = E.filtered_indices[E.selected_note_idx];
                toggle_note_attention_at(actual);
            }
            return;
        }
        if (c == '1') {
            E.filter_urgency = (E.filter_urgency == 1) ? 0 : 1;
            apply_note_search(E.search_query);
            if (E.filter_urgency == 1)
                set_status_msg("🔴 Filtrando notas com Urgência Nível 1 (Crítico - Vermelho): %d encontrada(s)", E.num_filtered);
            else
                set_status_msg("Filtro removido: exibindo todas as %d notas", E.num_filtered);
            return;
        }
        if (c == '2') {
            E.filter_urgency = (E.filter_urgency == 2) ? 0 : 2;
            apply_note_search(E.search_query);
            if (E.filter_urgency == 2)
                set_status_msg("🟠 Filtrando notas com Urgência Nível 2 (Médio - Laranja): %d encontrada(s)", E.num_filtered);
            else
                set_status_msg("Filtro removido: exibindo todas as %d notas", E.num_filtered);
            return;
        }
        if (c == '3') {
            E.filter_urgency = (E.filter_urgency == 3) ? 0 : 3;
            apply_note_search(E.search_query);
            if (E.filter_urgency == 3)
                set_status_msg("🟡 Filtrando notas com Urgência Nível 3 (Baixo - Amarelo): %d encontrada(s)", E.num_filtered);
            else
                set_status_msg("Filtro removido: exibindo todas as %d notas", E.num_filtered);
            return;
        }
        if (c == '4' || c == 'H') {
            E.filter_urgency = (E.filter_urgency == 4) ? 0 : 4;
            apply_note_search(E.search_query);
            if (E.filter_urgency == 4)
                set_status_msg("🔵 Filtrando notas com Marcação [H] (Azul): %d encontrada(s)", E.num_filtered);
            else
                set_status_msg("Filtro removido: exibindo todas as %d notas", E.num_filtered);
            return;
        }
        if (c == '5') {
            E.filter_urgency = (E.filter_urgency == 5) ? 0 : 5;
            apply_note_search(E.search_query);
            if (E.filter_urgency == 5)
                set_status_msg("🟣 Filtrando notas com Marcação [R] (Roxo): %d encontrada(s)", E.num_filtered);
            else
                set_status_msg("Filtro removido: exibindo todas as %d notas", E.num_filtered);
            return;
        }
        if (c == '6' || c == 'O') {
            E.filter_urgency = (E.filter_urgency == 6) ? 0 : 6;
            apply_note_search(E.search_query);
            if (E.filter_urgency == 6)
                set_status_msg("🟢 Filtrando notas com Marcação [O] (Verde): %d encontrada(s)", E.num_filtered);
            else
                set_status_msg("Filtro removido: exibindo todas as %d notas", E.num_filtered);
            return;
        }
        if (c == '0') {
            E.filter_urgency = 0;
            apply_note_search(E.search_query);
            set_status_msg("Filtro removido: exibindo todas as %d notas", E.num_filtered);
            return;
        }
        if (c == 'R') {
            rescan_notes();
            set_status_msg("Notas atualizadas (%d encontradas)", E.num_notes);
            return;
        }
        if (c == '/') {
            E.mode = MODE_SEARCH_LEFT;
            E.search_query[0] = '\0';
            E.search_cursor = 0;
            apply_note_search("");
            if (E.num_filtered > 0) {
                int actual = E.filtered_indices[E.selected_note_idx];
                load_preview(E.notes[actual].filepath);
            } else {
                free_preview();
            }
            set_status_msg("Buscar notas: digite termos de pesquisa");
            return;
        }
        if (c == 'g') {
            int next_c = read_raw_byte(1000);
            if (next_c == 'g') {
                if (E.num_filtered > 0) {
                    E.selected_note_idx = 0;
                    E.sidebar_scroll_y = 0;
                    int actual = E.filtered_indices[0];
                    load_preview(E.notes[actual].filepath);
                    set_status_msg("Primeira nota (1 de %d)", E.num_filtered);
                }
                return;
            } else if (next_c != -1 && next_c != KEY_ESC) {
                g_unget_byte = next_c;
            }
            return;
        }
        if (c == 'G') {
            if (E.num_filtered > 0) {
                E.selected_note_idx = E.num_filtered - 1;
                int max_visible = E.screen_rows - 5;
                if (E.selected_note_idx >= max_visible) {
                    E.sidebar_scroll_y = E.selected_note_idx - max_visible + 1;
                    if (E.sidebar_scroll_y < 0) E.sidebar_scroll_y = 0;
                }
                int actual = E.filtered_indices[E.selected_note_idx];
                load_preview(E.notes[actual].filepath);
                set_status_msg("Última nota (%d de %d)", E.num_filtered, E.num_filtered);
            }
            return;
        }
        if (c == ':') {
            E.mode = MODE_COMMAND;
            E.cmd_buf[0] = '\0';
            E.cmd_cursor = 0;
            return;
        }
        return;
    }

    /* ---------------------------------------------------------------------
     * Insert Mode Processing
     * --------------------------------------------------------------------- */
    if (E.mode == MODE_INSERT) {
        if (c == KEY_ESC) {
            E.has_selection = false;
            E.mode = MODE_NORMAL;
            if (E.cursor_x > 0) E.cursor_x--;
            set_status_msg("-- MODO NORMAL --");
            return;
        }

        /* Ctrl+C (3): Copy selected text */
        if (c == 3) {
            if (E.has_selection) {
                yank_visual_selection(false);
                set_status_msg("Texto selecionado copiado para a área de transferência! (Ctrl+C)");
            } else {
                set_status_msg("Nenhum texto selecionado para copiar (use Shift+Setas ou Mouse)");
            }
            return;
        }

        /* Ctrl+V (22): Paste from system clipboard in Insert Mode */
        if (c == 22) {
            int clip_len = 0;
            char *clip_text = get_system_clipboard(&clip_len);
            if (clip_text && clip_len > 0) {
                if (E.yank_buf) free(E.yank_buf);
                E.yank_buf = clip_text;
                E.yank_is_linewise = (strchr(clip_text, '\n') != NULL);
            } else if (clip_text) {
                free(clip_text);
            }
            if (E.has_selection) {
                delete_visual_selection_ex(false);
                E.has_selection = false;
            }
            if (E.yank_buf && *E.yank_buf) {
                editor_paste_string(E.yank_buf, (int)strlen(E.yank_buf));
                set_status_msg("Colado da área de transferência (Ctrl+V)");
            } else {
                set_status_msg("Área de transferência vazia.");
            }
            return;
        }

        /* Ctrl+X (24): Cut selected text */
        if (c == 24) {
            if (E.has_selection) {
                delete_visual_selection();
                E.has_selection = false;
                set_status_msg("Texto recortado para a área de transferência!");
                return;
            }
        }

        /* Ctrl+A (1): Select all text */
        if (c == 1) {
            if (E.num_lines > 0) {
                E.visual_start_y = 0;
                E.visual_start_x = 0;
                E.cursor_y = E.num_lines - 1;
                E.cursor_x = E.lines[E.cursor_y].len;
                E.has_selection = true;
                set_status_msg("Todo o texto selecionado! (Ctrl+C para copiar)");
                editor_scroll();
            }
            return;
        }

        /* Shift + Arrows: Text Selection in Insert Mode */
        if (c == KEY_SHIFT_LEFT || c == KEY_CTRL_SHIFT_LEFT) {
            if (!E.has_selection) {
                E.visual_start_y = E.cursor_y;
                E.visual_start_x = E.cursor_x;
            }
            if (c == KEY_CTRL_SHIFT_LEFT) {
                Line *cur = &E.lines[E.cursor_y];
                while (E.cursor_x > 0 && cur->chars[E.cursor_x - 1] == ' ') E.cursor_x--;
                while (E.cursor_x > 0 && cur->chars[E.cursor_x - 1] != ' ') {
                    E.cursor_x--;
                    while (E.cursor_x > 0 && ((unsigned char)cur->chars[E.cursor_x] & 0xC0) == 0x80) E.cursor_x--;
                }
            } else {
                if (E.cursor_x > 0) {
                    E.cursor_x--;
                    while (E.cursor_x > 0 && ((unsigned char)E.lines[E.cursor_y].chars[E.cursor_x] & 0xC0) == 0x80) {
                        E.cursor_x--;
                    }
                } else if (E.cursor_y > 0) {
                    E.cursor_y--;
                    E.cursor_x = E.lines[E.cursor_y].len;
                }
            }
            E.has_selection = (E.visual_start_y != E.cursor_y || E.visual_start_x != E.cursor_x);
            set_status_msg("Texto selecionado | Ctrl+C para copiar | Shift+Setas para expandir");
            editor_scroll();
            return;
        }
        if (c == KEY_SHIFT_RIGHT || c == KEY_CTRL_SHIFT_RIGHT) {
            if (!E.has_selection) {
                E.visual_start_y = E.cursor_y;
                E.visual_start_x = E.cursor_x;
            }
            if (c == KEY_CTRL_SHIFT_RIGHT) {
                Line *cur = &E.lines[E.cursor_y];
                while (E.cursor_x < cur->len && cur->chars[E.cursor_x] != ' ') {
                    int clen = utf8_charlen((unsigned char)cur->chars[E.cursor_x]);
                    if (clen <= 0) clen = 1;
                    E.cursor_x += clen;
                }
                while (E.cursor_x < cur->len && cur->chars[E.cursor_x] == ' ') E.cursor_x++;
            } else {
                if (E.cursor_x < E.lines[E.cursor_y].len) {
                    int clen = utf8_charlen((unsigned char)E.lines[E.cursor_y].chars[E.cursor_x]);
                    if (clen <= 0) clen = 1;
                    E.cursor_x += clen;
                } else if (E.cursor_y + 1 < E.num_lines) {
                    E.cursor_y++;
                    E.cursor_x = 0;
                }
            }
            E.has_selection = (E.visual_start_y != E.cursor_y || E.visual_start_x != E.cursor_x);
            set_status_msg("Texto selecionado | Ctrl+C para copiar | Shift+Setas para expandir");
            editor_scroll();
            return;
        }
        if (c == KEY_SHIFT_UP) {
            if (!E.has_selection) {
                E.visual_start_y = E.cursor_y;
                E.visual_start_x = E.cursor_x;
            }
            move_cursor_visual_up();
            E.has_selection = (E.visual_start_y != E.cursor_y || E.visual_start_x != E.cursor_x);
            set_status_msg("Texto selecionado | Ctrl+C para copiar | Shift+Setas para expandir");
            editor_scroll();
            return;
        }
        if (c == KEY_SHIFT_DOWN) {
            if (!E.has_selection) {
                E.visual_start_y = E.cursor_y;
                E.visual_start_x = E.cursor_x;
            }
            move_cursor_visual_down();
            E.has_selection = (E.visual_start_y != E.cursor_y || E.visual_start_x != E.cursor_x);
            set_status_msg("Texto selecionado | Ctrl+C para copiar | Shift+Setas para expandir");
            editor_scroll();
            return;
        }

        g_last_typing_time = get_current_time_ms();

        /* Table auto-alignment and new row on Enter (Vimwiki style) */
        if (c == '\r' || c == '\n') {
            if (E.has_selection) {
                delete_visual_selection();
                E.has_selection = false;
            }
            if (c == '\r') {
                int next_ch = read_raw_byte(5);
                if (next_ch != -1 && next_ch != '\n') {
                    g_unget_byte = next_ch;
                }
            }
            if (is_table_line(&E.lines[E.cursor_y])) {
                if (table_insert_row_and_advance()) {
                    return;
                }
            }
            save_undo();
            Line *cur = &E.lines[E.cursor_y];
            buf_insert_line(E.cursor_y + 1, &cur->chars[E.cursor_x], cur->len - E.cursor_x);
            cur = &E.lines[E.cursor_y];
            cur->chars[E.cursor_x] = '\0';
            cur->len = E.cursor_x;
            detect_heading(cur);
            E.cursor_y++;
            E.cursor_x = 0;
            update_folding();
            return;
        }

        /* Tab in Insert Mode: Vimwiki table cell advance or 4-space indent */
        if (c == '\t') {
            if (E.has_selection) {
                delete_visual_selection();
                E.has_selection = false;
            }
            if (is_table_line(&E.lines[E.cursor_y])) {
                table_jump_next_cell(true);
                return;
            }
            save_undo();
            for (int sp = 0; sp < 4; sp++) {
                line_insert_char(&E.lines[E.cursor_y], E.cursor_x, ' ');
                E.cursor_x++;
            }
            mark_modified();
            return;
        }
        if (c == KEY_BACKSPACE || c == 8 || c == 127) {
            if (E.has_selection) {
                delete_visual_selection();
                E.has_selection = false;
                return;
            }
            if (E.cursor_x > 0) {
                save_undo();
                int prev_x = E.cursor_x - 1;
                while (prev_x > 0 && ((unsigned char)E.lines[E.cursor_y].chars[prev_x] & 0xC0) == 0x80) {
                    prev_x--;
                }
                int del_bytes = E.cursor_x - prev_x;
                line_replace_range(&E.lines[E.cursor_y], prev_x, del_bytes, NULL, 0);
                E.cursor_x = prev_x;
            } else if (E.cursor_y > 0) {
                save_undo();
                int prev_len = E.lines[E.cursor_y - 1].len;
                line_append_string(&E.lines[E.cursor_y - 1], E.lines[E.cursor_y].chars, E.lines[E.cursor_y].len);
                buf_delete_line(E.cursor_y);
                E.cursor_y--;
                E.cursor_x = prev_len;
            }
            return;
        }
        if (c == KEY_DELETE) {
            if (E.has_selection) {
                delete_visual_selection();
                E.has_selection = false;
                return;
            }
            Line *cur = &E.lines[E.cursor_y];
            if (E.cursor_x < cur->len) {
                save_undo();
                line_delete_char(cur, E.cursor_x);
            } else if (E.cursor_y + 1 < E.num_lines) {
                save_undo();
                line_append_string(cur, E.lines[E.cursor_y + 1].chars, E.lines[E.cursor_y + 1].len);
                buf_delete_line(E.cursor_y + 1);
            }
            return;
        }
        if (c == KEY_ARROW_UP) { E.has_selection = false; move_cursor_visual_up(); return; }
        if (c == KEY_ARROW_DOWN) { E.has_selection = false; move_cursor_visual_down(); return; }
        if (c == KEY_ARROW_LEFT) {
            E.has_selection = false;
            if (E.cursor_x > 0) {
                E.cursor_x--;
                while (E.cursor_x > 0 && ((unsigned char)E.lines[E.cursor_y].chars[E.cursor_x] & 0xC0) == 0x80) E.cursor_x--;
            } else if (E.cursor_y > 0) {
                E.cursor_y--;
                E.cursor_x = E.lines[E.cursor_y].len;
            }
            return;
        }
        if (c == KEY_ARROW_RIGHT) {
            E.has_selection = false;
            if (E.cursor_x < E.lines[E.cursor_y].len) {
                int clen = utf8_charlen((unsigned char)E.lines[E.cursor_y].chars[E.cursor_x]);
                if (clen <= 0) clen = 1;
                E.cursor_x += clen;
            } else if (E.cursor_y + 1 < E.num_lines) {
                E.cursor_y++;
                E.cursor_x = 0;
            }
            return;
        }
        if (c == KEY_HOME) { E.has_selection = false; E.cursor_x = 0; return; }
        if (c == KEY_END) { E.has_selection = false; E.cursor_x = E.lines[E.cursor_y].len; return; }
        if (c == KEY_CTRL_LEFT) {
            E.has_selection = false;
            Line *cur = &E.lines[E.cursor_y];
            while (E.cursor_x > 0 && cur->chars[E.cursor_x - 1] == ' ') E.cursor_x--;
            while (E.cursor_x > 0 && cur->chars[E.cursor_x - 1] != ' ') {
                E.cursor_x--;
                while (E.cursor_x > 0 && ((unsigned char)cur->chars[E.cursor_x] & 0xC0) == 0x80) E.cursor_x--;
            }
            return;
        }
        if (c == KEY_CTRL_RIGHT) {
            E.has_selection = false;
            Line *cur = &E.lines[E.cursor_y];
            while (E.cursor_x < cur->len && cur->chars[E.cursor_x] != ' ') {
                int clen = utf8_charlen((unsigned char)cur->chars[E.cursor_x]);
                if (clen <= 0) clen = 1;
                E.cursor_x += clen;
            }
            while (E.cursor_x < cur->len && cur->chars[E.cursor_x] == ' ') E.cursor_x++;
            return;
        }

        if ((unsigned char)c >= 32 && (unsigned char)c != 127) {
            if (E.has_selection) {
                delete_visual_selection();
                E.has_selection = false;
            }
            int ins_len = (g_last_utf8_len > 0) ? g_last_utf8_len : 1;
            save_undo_for_typing(E.cursor_y, E.cursor_x, ins_len);
            if (g_last_utf8_len > 0) {
                line_replace_range(&E.lines[E.cursor_y], E.cursor_x, 0, g_last_utf8_char, g_last_utf8_len);
                E.cursor_x += g_last_utf8_len;
            } else {
                char ch = (char)c;
                line_replace_range(&E.lines[E.cursor_y], E.cursor_x, 0, &ch, 1);
                E.cursor_x++;
            }
            return;
        }
        return;
    }
    /* ---------------------------------------------------------------------
     * Normal & Visual Modes (Vim / Vimwiki compatibility)
     * --------------------------------------------------------------------- */

    /* Shift + Arrows in Normal Mode: Start Visual Selection */
    if (E.mode == MODE_NORMAL && (c == KEY_SHIFT_LEFT || c == KEY_SHIFT_RIGHT || c == KEY_SHIFT_UP || c == KEY_SHIFT_DOWN)) {
        E.visual_start_y = E.cursor_y;
        E.visual_start_x = E.cursor_x;
        E.mode = MODE_VISUAL_CHAR;
        if (c == KEY_SHIFT_LEFT && E.cursor_x > 0) E.cursor_x--;
        else if (c == KEY_SHIFT_RIGHT && E.cursor_x < E.lines[E.cursor_y].len) E.cursor_x++;
        else if (c == KEY_SHIFT_UP) move_cursor_visual_up();
        else if (c == KEY_SHIFT_DOWN) move_cursor_visual_down();
        set_status_msg("-- VISUAL (Shift+Setas) --");
        return;
    }

    /* Ctrl+C (3) in Visual Mode: Copy selection */
    if (c == 3 && (E.mode == MODE_VISUAL_CHAR || E.mode == MODE_VISUAL_LINE)) {
        yank_visual_selection(E.mode == MODE_VISUAL_LINE);
        E.mode = MODE_NORMAL;
        set_status_msg("Seleção copiada para a área de transferência! (Ctrl+C)");
        return;
    }

    /* Escape in Normal mode resets visual mode and pending commands */
    if (c == KEY_ESC) {
        E.mode = MODE_NORMAL;
        E.pending_op = 0;
        set_status_msg("-- MODO NORMAL --");
        return;
    }

    /* Cancel Ctrl+Tab as requested by user */
    if (c == KEY_CTRL_TAB) {
        return;
    }

    /* Ctrl+N (14): Jump to next link in note */
    if (c == 14) {
        jump_to_next_link();
        return;
    }

    /* Vim commands: gt, gT, gg, gl, gq (format table) */
    if (c == 'g') {
        int next_c = read_raw_byte(1000);
        if (next_c == 'q') {
            int third_c = read_raw_byte(1000);
            if (third_c == 'q' || third_c == ' ' || third_c == '\r' || third_c == '\n') {
                if (table_align_at(E.cursor_y)) return;
            }
            return;
        }
        if (next_c == 't') {
            tab_next();
            return;
        } else if (next_c == 'T') {
            tab_prev();
            return;
        } else if (next_c == 'g') {
            /* gg: Top of note */
            E.cursor_y = 0;
            while (E.cursor_y + 1 < E.num_lines && E.lines[E.cursor_y].hidden) {
                E.cursor_y++;
            }
            E.cursor_x = 0;
            E.scroll_y = 0;
            editor_scroll();
            g_screen_clear_needed = 1;
            set_status_msg("Topo da nota (linha %d)", E.cursor_y + 1);
            return;
        } else if (next_c == 'l' || next_c == '\t') {
            /* gl or g-Tab: Jump to next link */
            jump_to_next_link();
            return;
        } else if (next_c != -1 && next_c != KEY_ESC) {
            g_unget_byte = next_c;
        }
        return;
    }

    /* G: End of note */
    if (c == 'G') {
        if (E.num_lines > 0) {
            E.cursor_y = E.num_lines - 1;
            while (E.cursor_y > 0 && E.lines[E.cursor_y].hidden) {
                E.cursor_y--;
            }
            E.cursor_x = 0;
            editor_scroll();
            g_screen_clear_needed = 1;
            set_status_msg("Fim da nota (linha %d)", E.cursor_y + 1);
        }
        return;
    }

    /* History Navigation: Backspace and Ctrl+Backspace (Req 19) */
    if (c == KEY_BACKSPACE || c == 8 || c == 127) {
        navigate_history(1);
        return;
    }
    if (c == KEY_CTRL_BACKSPACE) {
        navigate_history(3);
        return;
    }

    /* Normal mode cursor navigation (h, j, k, l) */
    if (c == 'h' || c == KEY_ARROW_LEFT) {
        if (E.cursor_x > 0) {
            E.cursor_x--;
            if (should_hide_markdown_styles(false) && !E.lines[E.cursor_y].is_code_fence) {
                MarkdownStyleSpan spans[16];
                int n = find_markdown_styles(&E.lines[E.cursor_y], spans, 16);
                for (int s = 0; s < n; s++) {
                    if (E.cursor_x >= spans[s].open_start && E.cursor_x < spans[s].content_start) {
                        E.cursor_x = spans[s].open_start > 0 ? utf8_align_start(E.lines[E.cursor_y].chars, E.lines[E.cursor_y].len, spans[s].open_start - 1) : 0;
                        break;
                    }
                    if (E.cursor_x >= spans[s].close_start && E.cursor_x < spans[s].close_start + spans[s].close_len) {
                        if (spans[s].content_end > spans[s].content_start) {
                            E.cursor_x = utf8_align_start(E.lines[E.cursor_y].chars, E.lines[E.cursor_y].len, spans[s].content_end - 1);
                        } else {
                            E.cursor_x = spans[s].open_start;
                        }
                        break;
                    }
                }
            }
        } else if (E.mode == MODE_NORMAL) {
            /* Column 0: jump focus to left notes panel ONLY in Normal mode */
            E.focus = FOCUS_LEFT;
            set_status_msg("Foco na lista de notas (use j/k para navegar)");
        }
        return;
    }
    if (c == 'l' || c == KEY_ARROW_RIGHT) {
        if (E.cursor_x < E.lines[E.cursor_y].len) {
            E.cursor_x++;
            if (should_hide_markdown_styles(false) && !E.lines[E.cursor_y].is_code_fence) {
                MarkdownStyleSpan spans[16];
                int n = find_markdown_styles(&E.lines[E.cursor_y], spans, 16);
                for (int s = 0; s < n; s++) {
                    if (E.cursor_x >= spans[s].open_start && E.cursor_x < spans[s].content_start) {
                        E.cursor_x = spans[s].content_start;
                        break;
                    }
                    if (E.cursor_x >= spans[s].close_start && E.cursor_x < spans[s].close_start + spans[s].close_len) {
                        E.cursor_x = spans[s].close_start + spans[s].close_len;
                        break;
                    }
                }
            }
        }
        return;
    }
    if (c == 'j' || c == KEY_ARROW_DOWN) {
        move_cursor_visual_down();
        return;
    }
    if (c == 'k' || c == KEY_ARROW_UP) {
        move_cursor_visual_up();
        return;
    }

    /* Shortcut 'c' in Normal mode: Close current tab */
    if (c == 'c' && E.mode == MODE_NORMAL) {
        if (E.num_tabs > 0) {
            tab_close(E.active_tab);
        } else {
            set_status_msg("Nenhuma aba aberta.");
        }
        return;
    }

    /* Shortcut 'C' (Shift+C) in Normal mode: Close all tabs */
    if (c == 'C' && E.mode == MODE_NORMAL) {
        close_all_tabs();
        return;
    }

    /* Spacebar in Normal Mode: Insert a space character at cursor position */
    if (c == ' ' && E.mode == MODE_NORMAL) {
        save_undo();
        if (E.num_lines == 0) {
            buf_insert_line(0, " ", 1);
            E.cursor_y = 0;
            E.cursor_x = 1;
        } else {
            line_insert_char(&E.lines[E.cursor_y], E.cursor_x, ' ');
            E.cursor_x++;
        }
        mark_modified();
        set_status_msg("Espaço inserido");
        return;
    }

    /* Words navigation (w, b) */
    if (c == 'w') {
        Line *l = &E.lines[E.cursor_y];
        while (E.cursor_x < l->len && !isspace((unsigned char)l->chars[E.cursor_x])) E.cursor_x++;
        while (E.cursor_x < l->len && isspace((unsigned char)l->chars[E.cursor_x])) E.cursor_x++;
        if (E.cursor_x >= l->len && E.cursor_y + 1 < E.num_lines) {
            E.cursor_y++;
            E.cursor_x = 0;
        }
        return;
    }
    if (c == 'b') {
        if (E.cursor_x > 0) {
            Line *l = &E.lines[E.cursor_y];
            while (E.cursor_x > 0 && isspace((unsigned char)l->chars[E.cursor_x - 1])) E.cursor_x--;
            while (E.cursor_x > 0 && !isspace((unsigned char)l->chars[E.cursor_x - 1])) E.cursor_x--;
        } else if (E.cursor_y > 0) {
            E.cursor_y--;
            E.cursor_x = E.lines[E.cursor_y].len;
        }
        return;
    }
    if (c == '0' || c == KEY_HOME) { E.cursor_x = 0; return; }
    if (c == '$' || c == KEY_END) { E.cursor_x = E.lines[E.cursor_y].len; return; }

    if (E.num_tabs == 0) {
        if (c == 'i' || c == 'I' || c == 'a' || c == 'A' || c == 'o' || c == 'O' || c == 'n') {
            E.prompt_buf[0] = '\0';
            E.prompt_cursor = 0;
            E.mode = MODE_PROMPT_NEW_NOTE;
            set_status_msg("Criar nova nota:");
            return;
        }
        /* A linha de comando PRECISA continuar acessivel sem nenhuma aba
         * aberta: sem isto o ':' era engolido pelo return abaixo e nao havia
         * como sair do programa com :q / :q!. Isso ficou visivel a partir da
         * v6.29.0, quando o estado sem abas passou a ser o inicial (banco
         * novo/vazio). Tambem reabilita :life, :theme, :help etc. nesse estado. */
        if (c == ':') {
            E.mode = MODE_COMMAND;
            E.cmd_buf[0] = '\0';
            E.cmd_cursor = 0;
            return;
        }
        return;
    }

    /* Enter insert mode (i, I, a, A, o, O) */
    if (c == 'i') {
        E.mode = MODE_INSERT;
        set_status_msg("-- MODO INSERÇÃO --");
        return;
    }
    if (c == 'I') {
        E.cursor_x = 0;
        Line *l = &E.lines[E.cursor_y];
        while (E.cursor_x < l->len && isspace((unsigned char)l->chars[E.cursor_x])) E.cursor_x++;
        E.mode = MODE_INSERT;
        set_status_msg("-- MODO INSERÇÃO --");
        return;
    }
    if (c == 'a') {
        if (E.cursor_x < E.lines[E.cursor_y].len) E.cursor_x++;
        E.mode = MODE_INSERT;
        set_status_msg("-- MODO INSERÇÃO --");
        return;
    }
    if (c == 'A') {
        E.cursor_x = E.lines[E.cursor_y].len;
        E.mode = MODE_INSERT;
        set_status_msg("-- MODO INSERÇÃO --");
        return;
    }
    if (c == 'o') {
        save_undo();
        buf_insert_line(E.cursor_y + 1, "", 0);
        E.cursor_y++;
        E.cursor_x = 0;
        E.mode = MODE_INSERT;
        set_status_msg("-- MODO INSERÇÃO --");
        return;
    }
    if (c == 'O') {
        save_undo();
        buf_insert_line(E.cursor_y, "", 0);
        E.cursor_x = 0;
        E.mode = MODE_INSERT;
        set_status_msg("-- MODO INSERÇÃO --");
        return;
    }

    /* Visual mode (v, V) */
    if (c == 'v') {
        if (E.mode == MODE_VISUAL_CHAR) {
            E.mode = MODE_NORMAL;
            set_status_msg("-- MODO NORMAL --");
        } else if (E.mode == MODE_VISUAL_LINE) {
            E.mode = MODE_VISUAL_CHAR;
            set_status_msg("-- VISUAL --");
        } else {
            E.mode = MODE_VISUAL_CHAR;
            E.visual_start_y = E.cursor_y;
            E.visual_start_x = E.cursor_x;
            set_status_msg("-- VISUAL --");
        }
        return;
    }
    if (c == 'V') {
        if (E.mode == MODE_VISUAL_LINE) {
            E.mode = MODE_NORMAL;
            set_status_msg("-- MODO NORMAL --");
        } else if (E.mode == MODE_VISUAL_CHAR) {
            E.mode = MODE_VISUAL_LINE;
            set_status_msg("-- VISUAL LINHA --");
        } else {
            E.mode = MODE_VISUAL_LINE;
            E.visual_start_y = E.cursor_y;
            E.visual_start_x = 0;
            set_status_msg("-- VISUAL LINHA --");
        }
        return;
    }

    /* Vim Replace (r): Replace character under cursor or in visual selection */
    if (c == 'r') {
        if (E.mode == MODE_VISUAL_CHAR || E.mode == MODE_VISUAL_LINE) {
            set_status_msg("-- REPLACE --");
            refresh_screen();
            int rep_c = -1;
            while (1) {
                rep_c = editor_read_key();
                if (rep_c == -2 || g_window_resized) {
                    g_window_resized = 0;
                    refresh_screen();
                    continue;
                }
                if (rep_c != -1) break;
            }
            if (rep_c == KEY_ESC || rep_c == 3 || rep_c == KEY_MOUSE_EVENT) {
                set_status_msg(E.mode == MODE_VISUAL_LINE ? "-- VISUAL LINHA --" : "-- VISUAL --");
                return;
            }
            if (rep_c >= 32 && rep_c <= 255 && rep_c != 127) {
                save_undo();
                char ch = (char)rep_c;
                const char *rep_str = (g_last_utf8_len > 0) ? g_last_utf8_char : &ch;
                int rep_len = (g_last_utf8_len > 0) ? g_last_utf8_len : 1;
                int start_y = E.visual_start_y, end_y = E.cursor_y;
                int start_x = E.visual_start_x, end_x = E.cursor_x;
                if (start_y > end_y || (start_y == end_y && start_x > end_x)) {
                    int ty = start_y; start_y = end_y; end_y = ty;
                    int tx = start_x; start_x = end_x; end_x = tx;
                }
                if (E.mode == MODE_VISUAL_LINE) {
                    for (int y = start_y; y <= end_y && y < E.num_lines; y++) {
                        Line *l = &E.lines[y];
                        int x = 0;
                        while (x < l->len) {
                            int b = 1;
                            utf8_decode_codepoint(&l->chars[x], &b);
                            if (b <= 0) b = 1;
                            line_replace_range(l, x, b, rep_str, rep_len);
                            x += rep_len;
                        }
                    }
                } else {
                    for (int y = start_y; y <= end_y && y < E.num_lines; y++) {
                        Line *l = &E.lines[y];
                        int sx = (y == start_y) ? start_x : 0;
                        int ex = (y == end_y) ? end_x : l->len - 1;
                        if (sx > l->len) sx = l->len;
                        if (ex >= l->len) ex = l->len - 1;
                        int x = sx;
                        while (x <= ex && x < l->len) {
                            int b = 1;
                            utf8_decode_codepoint(&l->chars[x], &b);
                            if (b <= 0) b = 1;
                            line_replace_range(l, x, b, rep_str, rep_len);
                            x += rep_len;
                            ex += (rep_len - b);
                        }
                    }
                }
                E.mode = MODE_NORMAL;
                set_status_msg("-- MODO NORMAL --");
                return;
            }
            return;
        }

        if (E.mode == MODE_NORMAL) {
            if (E.num_lines == 0) return;
            Line *cur = &E.lines[E.cursor_y];
            if (E.cursor_x >= cur->len) return;

            set_status_msg("-- REPLACE --");
            refresh_screen();

            int rep_c = -1;
            while (1) {
                rep_c = editor_read_key();
                if (rep_c == -2 || g_window_resized) {
                    g_window_resized = 0;
                    refresh_screen();
                    continue;
                }
                if (rep_c != -1) break;
            }

            if (rep_c == KEY_ESC || rep_c == 3 || rep_c == KEY_MOUSE_EVENT) {
                set_status_msg("-- MODO NORMAL --");
                return;
            }

            int bytes_under_cursor = 1;
            utf8_decode_codepoint(&cur->chars[E.cursor_x], &bytes_under_cursor);
            if (bytes_under_cursor <= 0) bytes_under_cursor = 1;
            if (E.cursor_x + bytes_under_cursor > cur->len) bytes_under_cursor = cur->len - E.cursor_x;

            save_undo();

            if (rep_c == '\r' || rep_c == '\n') {
                /* Vim behavior: r<Enter> replaces char with newline */
                char *remaining = cur->chars + E.cursor_x + bytes_under_cursor;
                int rem_len = cur->len - (E.cursor_x + bytes_under_cursor);
                char *new_line_str = (rem_len > 0) ? strdup(remaining) : strdup("");
                cur->len = E.cursor_x;
                cur->chars[cur->len] = '\0';
                detect_heading(cur);
                buf_insert_line(E.cursor_y + 1, new_line_str, rem_len);
                free(new_line_str);
                E.cursor_y++;
                E.cursor_x = 0;
            } else if (g_last_utf8_len > 0) {
                line_replace_range(cur, E.cursor_x, bytes_under_cursor, g_last_utf8_char, g_last_utf8_len);
            } else if (rep_c >= 32 && rep_c <= 255 && rep_c != 127) {
                char ch = (char)rep_c;
                line_replace_range(cur, E.cursor_x, bytes_under_cursor, &ch, 1);
            }
            set_status_msg("-- MODO NORMAL --");
            return;
        }
    }

    /* Toggle urgency / attention of current note (m or !) */
    if (c == 'm' || c == '!') {
        if (E.cur_filepath[0]) {
            for (int i = 0; i < E.num_notes; i++) {
                if (strcmp(E.notes[i].filepath, E.cur_filepath) == 0) {
                    toggle_note_attention_at(i);
                    break;
                }
            }
        }
        return;
    }

    /* Cut character under cursor (x) or visual selection (x) - like Neovim */
    if (c == 'x') {
        if (E.mode == MODE_VISUAL_LINE || E.mode == MODE_VISUAL_CHAR || E.has_selection) {
            delete_visual_selection();
            E.mode = MODE_NORMAL;
            E.has_selection = false;
            set_status_msg("Texto selecionado recortado para a área de transferência! (x)");
            return;
        }
        Line *cur = &E.lines[E.cursor_y];
        if (cur->len > 0 && E.cursor_x < cur->len) {
            save_undo();
            int bytes_under_cursor = 1;
            utf8_decode_codepoint(&cur->chars[E.cursor_x], &bytes_under_cursor);
            if (bytes_under_cursor <= 0) bytes_under_cursor = 1;
            if (E.cursor_x + bytes_under_cursor > cur->len) bytes_under_cursor = cur->len - E.cursor_x;

            /* Cut character to yank_buf and system clipboard */
            if (E.yank_buf) free(E.yank_buf);
            E.yank_buf = (char *)malloc(bytes_under_cursor + 1);
            if (E.yank_buf) {
                memcpy(E.yank_buf, &cur->chars[E.cursor_x], bytes_under_cursor);
                E.yank_buf[bytes_under_cursor] = '\0';
                E.yank_is_linewise = false;
                copy_to_system_clipboard(E.yank_buf, bytes_under_cursor);
            }

            line_replace_range(cur, E.cursor_x, bytes_under_cursor, NULL, 0);
            if (E.cursor_x >= cur->len && E.cursor_x > 0) {
                E.cursor_x--;
                while (E.cursor_x > 0 && ((unsigned char)cur->chars[E.cursor_x] & 0xC0) == 0x80) {
                    E.cursor_x--;
                }
            }
            set_status_msg("Caractere recortado para a área de transferência (x)");
        }
        return;
    }

    /* Delete line (dd) or visual selection (d) */
    if (c == 'd') {
        if (E.mode == MODE_VISUAL_LINE || E.mode == MODE_VISUAL_CHAR) {
            delete_visual_selection();
            return;
        }
        int next_key = editor_read_key();
        if (next_key == 'd') {
            save_undo();
            if (E.yank_buf) free(E.yank_buf);
            E.yank_buf = strdup(E.lines[E.cursor_y].chars ? E.lines[E.cursor_y].chars : "");
            E.yank_is_linewise = true;
            copy_to_system_clipboard(E.yank_buf, (int)strlen(E.yank_buf));

            buf_delete_line(E.cursor_y);
            if (E.cursor_y >= E.num_lines && E.cursor_y > 0) E.cursor_y--;
            set_status_msg("1 linha excluída");
        }
        return;
    }

    /* Yank (y, yy or Y) - System Clipboard Sharing (like Neovim) */
    if (c == 'y' || c == 'Y') {
        if (E.mode == MODE_VISUAL_LINE || E.mode == MODE_VISUAL_CHAR || E.has_selection) {
            yank_visual_selection(E.mode == MODE_VISUAL_LINE);
            E.mode = MODE_NORMAL;
            E.has_selection = false;
            set_status_msg("Texto selecionado copiado para a área de transferência! (y)");
            return;
        }
        /* In Normal mode without selection, pressing 'y' or 'Y' copies the current line immediately.
         * If the user types 'yy', the first 'y' copies the line,
         * and the second 'y' also confirms copying without waiting or blocking! */
        if (E.cursor_y >= 0 && E.cursor_y < E.num_lines) {
            if (E.yank_buf) free(E.yank_buf);
            E.yank_buf = strdup(E.lines[E.cursor_y].chars ? E.lines[E.cursor_y].chars : "");
            E.yank_is_linewise = true;
            copy_to_system_clipboard(E.yank_buf, (int)strlen(E.yank_buf));
            set_status_msg("1 linha copiada para a área de transferência (y)");
        }
        return;
    }

    /* Paste (p or P) - System Clipboard Synchronized (Requirement 23 & Windows compatibility) */
    if (c == 'p' || c == 'P') {
        if (E.mode == MODE_VISUAL_CHAR || E.mode == MODE_VISUAL_LINE || E.has_selection) {
            paste_replace_selection();
            return;
        }

        /* 1. Try to read fresh content from system clipboard */
        int clip_len = 0;
        char *clip_text = get_system_clipboard(&clip_len);
        if (clip_text && clip_len > 0) {
            if (E.yank_buf) free(E.yank_buf);
            E.yank_buf = clip_text;
            E.yank_is_linewise = (strchr(clip_text, '\n') != NULL);
        } else if (clip_text) {
            free(clip_text);
        }

        /* 2. Paste yank_buf if available */
        if (E.yank_buf && *E.yank_buf) {
            editor_paste_string(E.yank_buf, (int)strlen(E.yank_buf));
            set_status_msg("Colado da área de transferência");
        } else {
            set_status_msg("Área de transferência vazia.");
        }
        return;
    }

    /* Undo (u) */
    if (c == 'u') {
        apply_undo();
        return;
    }

    /* Redo (Ctrl+R) */
    if (c == 18) {
        apply_redo();
        return;
    }

    /* Folding commands (za, zM, zR, zm, zr, zc, zo) */
    if (c == 'z') {
        int next = read_raw_byte(1000);
        if (next == 'a') {
            toggle_fold_at_cursor();
        } else if (next == 'M' || next == 'm') {
            collapse_all_headings();
        } else if (next == 'R' || next == 'r') {
            uncollapse_all_headings();
        } else if (next == 'c') {
            if (E.cursor_y >= 0 && E.cursor_y < E.num_lines) {
                int target_idx = E.cursor_y;
                if (E.lines[target_idx].heading_lvl == 0) {
                    for (int i = target_idx; i >= 0; i--) {
                        if (E.lines[i].heading_lvl > 0) { target_idx = i; break; }
                    }
                }
                if (E.lines[target_idx].heading_lvl > 0) {
                    E.lines[target_idx].folded = true;
                    update_folding();
                    E.cursor_y = target_idx;
                    editor_scroll();
                    g_screen_clear_needed = 1;
                    set_status_msg("Seção recolhida (zc)");
                }
            }
        } else if (next == 'o') {
            if (E.cursor_y >= 0 && E.cursor_y < E.num_lines) {
                int target_idx = E.cursor_y;
                if (E.lines[target_idx].heading_lvl == 0) {
                    for (int i = target_idx; i >= 0; i--) {
                        if (E.lines[i].heading_lvl > 0) { target_idx = i; break; }
                    }
                }
                if (E.lines[target_idx].heading_lvl > 0) {
                    E.lines[target_idx].folded = false;
                    update_folding();
                    editor_scroll();
                    g_screen_clear_needed = 1;
                    set_status_msg("Seção expandida (zo)");
                }
            }
        } else if (next != -1 && next != KEY_ESC) {
            g_unget_byte = next;
        }
        return;
    }

    /* Next/Previous search match (n, N) */
    if (c == 'n') {
        if (E.last_search_pat[0]) {
            bool found = false;
            for (int i = E.cursor_y + 1; i < E.num_lines; i++) {
                if (E.lines[i].chars && str_contains_caseless(E.lines[i].chars, E.last_search_pat)) {
                    E.cursor_y = i;
                    E.cursor_x = 0;
                    set_status_msg("Ocorrência na linha %d", i + 1);
                    found = true;
                    break;
                }
            }
            if (!found) {
                for (int i = 0; i <= E.cursor_y; i++) {
                    if (E.lines[i].chars && str_contains_caseless(E.lines[i].chars, E.last_search_pat)) {
                        E.cursor_y = i;
                        E.cursor_x = 0;
                        set_status_msg("Ocorrência na linha %d (reinício)", i + 1);
                        found = true;
                        break;
                    }
                }
            }
            if (!found) set_status_msg("Padrão não encontrado: %s", E.last_search_pat);
        }
        return;
    }

    /* Enter activates link under cursor (Markdown link or [[wikilink]]) */
    if (c == '\r' || c == '\n') {
        activate_link_at_cursor();
        return;
    }

    /* Bottom Command Line activator (:) */
    if (c == ':') {
        E.mode = MODE_COMMAND;
        E.cmd_buf[0] = '\0';
        E.cmd_cursor = 0;
        return;
    }

    /* In-buffer search shortcut (/) */
    if (c == '/') {
        E.mode = MODE_COMMAND;
        E.cmd_buf[0] = '/';
        E.cmd_buf[1] = '\0';
        E.cmd_cursor = 1;
        return;
    }
}

/* =========================================================================
 * Clean Shutdown & Memory Freeing
 * ========================================================================= */

static void free_editor(void) {
    free_preview();
    tab_save_active();

    if (E.tabs) {
        for (int i = 0; i < E.num_tabs; i++) {
            Tab *t = &E.tabs[i];
            if (t->lines) {
                for (int j = 0; j < t->num_lines; j++) line_free(&t->lines[j]);
                free(t->lines);
                t->lines = NULL;
            }
            for (int u = 0; u < t->undo_count; u++) free_undo_state(&t->undo_stack[u]);
            for (int r = 0; r < t->redo_count; r++) free_undo_state(&t->redo_stack[r]);
        }
        free(E.tabs);
        E.tabs = NULL;
        E.num_tabs = 0;
        E.cap_tabs = 0;
    }

    if (E.notes) {
        for (int i = 0; i < E.num_notes; i++) {
            if (E.notes[i].search_norm) {
                free(E.notes[i].search_norm);
                E.notes[i].search_norm = NULL;
            }
        }
        free(E.notes);
        E.notes = NULL;
        E.num_notes = 0;
    }

    if (E.filtered_indices) {
        free(E.filtered_indices);
        E.filtered_indices = NULL;
    }

    if (E.yank_buf) {
        free(E.yank_buf);
        E.yank_buf = NULL;
    }

    /* Fecha o banco por ultimo: as rotinas acima podem consultar/apagar notas. */
    db_close();
}

/* =========================================================================
 * Main Initialization & Loop
 * ========================================================================= */

static void init_editor(void) {
    memset(&E, 0, sizeof(E));
    E.left_width = DEFAULT_LEFT_WIDTH;
    E.focus = FOCUS_RIGHT;
    E.mode = MODE_NORMAL;
    E.current_theme = 0; /* Tokyo Night default */
    g_last_activity_time = get_current_time_ms();
    E.wrap_mode = true;

    get_window_size(&E.screen_rows, &E.screen_cols);

    /* Abre (criando se necessario) o banco SQLite. Ele NASCE VAZIO: nenhuma
     * nota de exemplo e criada; o estado sem abas ja e tratado pela interface. */
    if (!db_open()) {
        disable_raw_mode();
        fprintf(stderr, "Erro fatal: nao foi possivel abrir/criar o banco %s\n", DB_PATH);
        exit(1);
    }

    /* Varredura das notas no banco */
    rescan_notes();

    /* If there are notes, open the first/most recent one in Tab 0 */
    if (E.num_notes > 0) {
        open_note_by_path(E.notes[0].filepath, false);
        E.selected_note_idx = 0;
        E.focus = FOCUS_LEFT;
        load_preview(E.notes[0].filepath);
    } else {
        /* Banco vazio: nao cria nota de exemplo. O editor ja exibe a aba
         * [Nova Nota] e as instrucoes no painel direito nesse estado. */
        set_status_msg("Banco de dados vazio. Use [Nova] para criar ou [Importar] para trazer seus .md.");
    }
}

/* =========================================================================
 * Dirty-Flag: decide se esta na hora de redesenhar a tela
 * -------------------------------------------------------------------------
 * Chamada uma vez por iteracao do loop principal, ANTES de bloquear na leitura
 * de teclado. Retorna true se ha motivo real para redesenhar:
 *   1. algo mudou o estado visual desde o ultimo desenho (g_needs_redraw);
 *   2. a janela foi redimensionada (SIGWINCH) ou a tela pede limpeza;
 *   3. ha marquee de titulo longo ativo E o passo da animacao mudou (300 ms);
 *   4. a mensagem de status acabou de expirar (5 s) e o rodape precisa voltar
 *      a mostrar a dica de atalhos -- exatamente uma vez;
 *   5. passou o batimento de manutencao (30 s), que mantem o relogio relativo
 *      da barra lateral ("now", "5m", "2h") atualizado mesmo ocioso.
 * Fora desses casos retorna false, e o redesenho completo da tela e evitado.
 * ========================================================================= */
static bool should_refresh_now(void) {
    if (g_needs_redraw || g_window_resized || g_screen_clear_needed) {
        return true;
    }

    int64_t now = get_current_time_ms();

    /* Marquee de titulo longo: mesma cadencia de 300 ms usada em render_tab_bar,
     * e somente quando ha de fato algo rolando. */
    if (g_marquee_active && (now / 300) != g_last_marquee_tick) {
        return true;
    }

    /* Mensagem de status: quando expira, o rodape volta a dica de atalhos.
     * Limpar a mensagem aqui garante que isso aconteca uma unica vez. */
    if (E.status_msg[0]) {
        if ((long)(time(NULL) - E.status_msg_time) >= 5) {
            E.status_msg[0] = '\0';
            return true;
        }
        return false; /* ainda visivel: nada a fazer ate expirar ou haver tecla */
    }

    /* Batimento de manutencao: mantem o relogio relativo da barra lateral vivo. */
    if (g_last_heartbeat_draw == 0 || now - g_last_heartbeat_draw >= IDLE_HEARTBEAT_MS) {
        return true;
    }

    return false;
}

int main(int argc, char **argv) {
    setlocale(LC_ALL, "");
#ifdef _WIN32
    /* Codepage UTF-8 e configurada em enable_raw_mode(), que guarda os valores
     * anteriores para restaura-los no fim. Usar a API Wide evita falhar quando
     * o executavel esta em uma pasta com acentos/Unicode. */
    WCHAR wexe_dir[32768];
    DWORD wlen = GetModuleFileNameW(NULL, wexe_dir, (DWORD)(sizeof(wexe_dir) / sizeof(wexe_dir[0])));
    if (wlen > 0 && wlen < (DWORD)(sizeof(wexe_dir) / sizeof(wexe_dir[0]))) {
        WCHAR *slash = wcsrchr(wexe_dir, L'\\');
        if (slash) {
            *slash = L'\0';
            SetCurrentDirectoryW(wexe_dir);
        }
    }
#else
    char exe_dir[4096];
    ssize_t elen = readlink("/proc/self/exe", exe_dir, sizeof(exe_dir) - 1);
    if (elen > 0) {
        exe_dir[elen] = '\0';
        char *slash = strrchr(exe_dir, '/');
        if (slash) {
            *slash = '\0';
            chdir(exe_dir);
        }
    }
#endif

    enable_raw_mode();
    init_editor();

    if (argc > 1) {
        open_note_by_path(argv[1], true);
        E.focus = FOCUS_RIGHT;
    }

    /* A mensagem de estado vazio definida por init_editor() nao pode ser
     * sobrescrita aqui: ela e o unico aviso de que o banco esta vazio. */
    int total_no_banco = db_count_notes();
    if (total_no_banco == 0) {
        set_status_msg("Banco SQLite vazio. Use [Nova] para criar uma nota ou [\xe2\xac\x87] para importar seus .md");
    } else {
        set_status_msg("Notas Definitivas | SQLite: %d nota(s) | Pressione :q para sair", total_no_banco);
    }

    while (!E.quit_requested) {
        /* Dirty-flag: so redesenha quando ha mudanca real de estado, animacao
         * de marquee ativa, expiracao da mensagem de status ou batimento de
         * manutencao (ver should_refresh_now). Antes, refresh_screen() era
         * chamada incondicionalmente a cada 150 ms, redesenhando a tela toda. */
        if (should_refresh_now()) {
            refresh_screen();
        }
        editor_process_keypress();
        while (!E.quit_requested && has_pending_input()) {
            editor_process_keypress();
        }
    }

    disable_raw_mode();
    return 0;
}
