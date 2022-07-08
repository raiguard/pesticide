define-command pesticide-log %{
    edit ~/.local/share/pesticide/pesticide.log -readonly -scroll
}

hook global BufReload .*pesticide\.log %{
    execute-keys 'gjgh'
}
