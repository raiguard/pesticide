### Options

declare-option str pesticide_cmd "pesticide --session %val{session}"
declare-option line-specs breakpoints
declare-option line-specs step_indicator

declare-option str breakpoint_symbol "●"
declare-option str step_symbol "▶"

### Faces

set-face global Breakpoint Error
set-face global StepIndicator yellow

### Highlighters

hook global BufCreate .* %{
    add-highlighter buffer/ flag-lines default breakpoints
    add-highlighter buffer/ flag-lines default step_indicator
}

### Commands

define-command pesticide-toggle-breakpoint \
-docstring "Toggle a breakpoint on the current line" \
%{
    nop %sh{
        $kak_opt_pesticide_cmd --request '{"cmd": "toggle_breakpoint", "file": "'"$kak_buffile"'", "line": '"$kak_cursor_line"', "column": '"$kak_cursor_column"'}'
    }
}

declare-user-mode pesticide
map global pesticide t ": pesticide-toggle-breakpoint<ret>" -docstring "toggle breakpoint"

# TEMPORARY:
map global user d ": enter-user-mode pesticide<ret>"
map global user D ": enter-user-mode -lock pesticide<ret>"
