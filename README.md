# Pesticide

A TUI front-end for the Debug Adapter Protocol. Can be used as a standalone program, or integrated with terminal text editors.

## Status

This project is in early alpha. It currently shows variables and stack traces, and somewhat shows the output of the debugee, but does not do much else.

## Motivation

I am an avid [Kakoune](https://kakoune.org) user. Kak is a very niche text editor with few users. As such, it does not have a mature [Debug Adapter Protocol](https://microsoft.github.io/debug-adapter-protocol/) plugin.

Kakoune's philosophy is one of orthogonality. A text editor should be just that - a text editor. It should be easy to integrate with other tools in a POSIX environment. As such, Kakoune itself should not be used as a debugging interface, but only as the text editor portion of a debugging interface. Thus, everything except for the text editor itself should be handled by another program or programs. This is where Pesticide fits in.

## Goals

- Start, manage, and stop DAP sessions
- Show variables, watch, stack trace, breakpoints, and all other DAP views
- Simple built-in text view for setting breakpoints
- Easily integrate with terminal text editors for setting breakpoints and moving around the project
- Client-server architecture to allow multiple terminals to show different views

## Non-goals

- Integrate with GUI text editors
- Manage debug adapters
- Windows support

## Inspirations

- [gdb-dashboard](https://github.com/cyrus-and/gdb-dashboard)
- [kak-dap](https://codeberg.org/jdugan6240/kak-dap)
- [nvim-dap](https://github.com/mfussenegger/nvim-dap)

## Attributions

DAP types and framed codec were borrowed from [vscode-lldb](https://github.com/vadimcn/vscode-lldb) under the MIT license, with modifications to custom extensions.
