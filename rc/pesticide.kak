### Options

declare-option str pesticide_cmd "pesticide --session %val{session}"
declare-option str-list breakpoints
declare-option line-specs pesticide_flags
declare-option int stackframe_line 0

declare-option str breakpoint_symbol "●"
declare-option str step_symbol "▶"

### Faces

set-face global Breakpoint Error
set-face global StepIndicator yellow

### Highlighters

hook global BufCreate .* %{
    add-highlighter buffer/ flag-lines default pesticide_flags
}

### Commands

define-command pesticide-toggle-breakpoint \
-docstring "Toggle a breakpoint on the current line" \
%{
    evaluate-commands %sh{
        $kak_opt_pesticide_cmd --request '{"command": "toggle_breakpoint", "file": "'"$kak_bufname"'", "line": '"$kak_cursor_line"'}'
    }
}
