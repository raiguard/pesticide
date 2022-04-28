terminal-tab kak -c %val{session} -e "edit ~/.local/share/pesticide/pesticide.log -readonly -scroll"

hook global BufCreate .*dap_types\.rs %{
    add-highlighter buffer/ column 81 ",rgb:%opt{subbg}"
}
