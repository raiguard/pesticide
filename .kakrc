define-command pesticide-log %{
    edit ~/.local/state/pesticide.log -readonly -scroll
}

hook global BufReload .*pesticide\.log %{
    execute-keys 'gjgh'
}

# vim: ft=kak
