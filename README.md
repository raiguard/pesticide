# Pesticide

A terminal front-end for the [Debug Adapter Protocol](https://microsoft.github.io/debug-adapter-protocol/).

## Build

Dependencies:
- [go](https://golang.org)
- [scdoc](https://git.sr.ht/~sircmpwn/scdoc) (for man pages)

```
make
sudo make install
```

## Usage / configuration

Place a `.pesticide` file in your project directory and specify your adapter configurations. The file is JSON formatted:

```json
{
  "adapters": {
    "fmtk": {
      "cmd": "fmtk debug /home/rai/dev/factorio/1.1/bin/x64/factorio",
      "args": {
        "hookControl": [ "UltimateResearchQueue" ],
        "modsPath": "/home/rai/dev/factorio/1.1/mods"
      }
    },
    "mock": {
      "cmd": "mockserver",
      "addr": ":54321"
    }
  }
}
```

- `cmd`: A command-line command to execute.
- `addr`: An IP address to connect to. This can be used in combination with `cmd`.
- `args`: Any adapter-specific arguments.

Launch the `pest` executable in your project directory and it will source the configuration file. You can now run commands.

### Current commands

- `break <filename> <line>`
- `continue`
- `evaluate <expression>`
- `launch <adapter name>`
- `pause`
- `quit`
