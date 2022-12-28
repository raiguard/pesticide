# Pesticide

A terminal front-end for the [Debug Adapter
Protocol](https://microsoft.github.io/debug-adapter-protocol/).

## Status

The project is progressing alarmingly quickly. Go is agreeing with me much more
than Rust ever did, and the project is coming together at a frightening pace.

## Build

Dependencies:
- [go](https://golang.org)
- [scdoc](https://git.sr.ht/~sircmpwn/scdoc) (for man pages)

```
make
sudo make install
```

## Usage

Place a `.pesticide` file in your project directory and specify your adapter
configurations:

```scfg
adapter fmtk {
	cmd "fmtk debug $FACTORIO"
	args '{"modsPath": "/home/rai/dev/factorio/1.1/mods", "hookControl": ["UltimateResearchQueue"]}'
}

adapter mock {
	cmd mockserver
	addr :54321
}
```

Configuration follows the [scfg](https://git.sr.ht/~emersion/scfg) file format.

Launch the `pest` executable, and it will read the configuration file. Use
`break filename line` to set a breakpoint, and `launch <adapter name>` to
launch the debug adapter. Use `control+c` to pause execution, `continue` to
resume execution, and `quit` to quit the active adapter or the program.

Currently, if the adapter fails to pause, then you cannot do anything. This
will be resolved once the proper TUI is implemented.

## Contributing

Please send questions, patches, or bug reports to the [mailing
list](https://lists.sr.ht/~raiguard/public-inbox).
