define-command pesticide-log %{
    edit ~/.local/state/pesticide.log -readonly -scroll
}

hook global WinCreate .*pesticide\.log %{
    add-highlighter window/pest-datetime regex '\d*/\d*/\d* \d*:\d*:\d*' 0:comment
    add-highlighter window/pest-adapter regex ' (\[\d*\]) ' 1:builtin
    add-highlighter window/pest-adapter-send regex ' (<-) ' 1:function
    add-highlighter window/pest-adapter-recv regex ' (->) ' 1:string
}

hook global BufReload .*pesticide\.log %{
    execute-keys 'gjgh'
}

# vim: ft=kak
