# Pesticide

A terminal front-end for the [Debug Adapter
Protocol](https://microsoft.github.io/debug-adapter-protocol/). Currently
includes a CLI front-end, and will include a TUI front-end in the future.

## Status

The project is progressing alarmingly quickly. Go is agreeing with me much more
than Rust ever did, and the project is coming together at a frightening pace.
It currently loads, reads adapter configurations from the `.pesticide` folder
in your PWD, and allows you to launch one of them. It doesn't do much more than
that yet, but things are progressing smoothly.

## Build and Run

Dependencies:
- [go](https://golang.org)
- [scdoc](https://git.sr.ht/~sircmpwn/scdoc) (for man pages)

## Contributing

Please send questions, patches, or bug reports to the [mailing
list](https://lists.sr.ht/~raiguard/public-inbox).
