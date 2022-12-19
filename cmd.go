package main

import (
	"encoding/json"
	"errors"
	"fmt"
	"log"
	"os"
	"strings"

	"git.sr.ht/~emersion/go-scfg"
	"github.com/google/go-dap"
)

type adapterConfig struct {
	name string
	cmd  *string
	args json.RawMessage
	addr *string
}

// Cmd parses user input and configuration files using the scfg syntax, and
// executes UI or adapter commands.

func cmdReadFile(path string) {
	block, err := scfg.Load(path)
	if err != nil {
		log.Println("Failed to read file", path, ":", err)
		return
	}
	cmdParseBlock(block)
}

func cmdRead(input string) error {
	reader := strings.NewReader(input)
	block, err := scfg.Read(reader)
	if err != nil {
		return err
	}
	return cmdParseBlock(block)
}

func cmdParseBlock(block scfg.Block) error {
	for i := 0; i < len(block); i++ {
		err := cmdParseDirective(block[i])
		if err != nil {
			return err
		}
	}
	return nil
}

func cmdParseDirective(directive *scfg.Directive) error {
	switch directive.Name {
	case "adapter":
		return cmdParseAdapter(directive)
	case "launch", "l":
		return cmdParseLaunch(directive)
	case "quit", "q":
		return cmdParseQuit(directive)
	case "continue", "c":
		return cmdParseContinue(directive)
	default:
		return errors.New(fmt.Sprint("Unknown command: ", directive.Name, "\n"))
	}
}

func cmdParseAdapter(directive *scfg.Directive) error {
	if len(directive.Params) != 1 {
		return errors.New("adapter command must have only one argument")
	}

	cfg := adapterConfig{name: directive.Params[0]}

	for _, child := range directive.Children {
		switch child.Name {
		case "cmd":
			expanded := os.ExpandEnv(child.Params[0])
			cfg.cmd = &expanded
		case "addr":
			cfg.addr = &child.Params[0]
		case "args":
			// TODO: Make this more ergonomic
			value := child.Params[0]
			bytes := make(json.RawMessage, len(value))
			copy(bytes, value)
			cfg.args = bytes
		default:
		}
	}

	adapterConfigs[directive.Params[0]] = &cfg

	return nil
}

func cmdParseLaunch(directive *scfg.Directive) error {
	if len(directive.Params) == 0 {
		return errors.New("did not specify a configuration to launch\n")
	}
	if len(adapterConfigs) == 0 {
		return errors.New(fmt.Sprint("unknown adapter ", directive.Params[0], "\n"))
	}
	cfg := adapterConfigs[directive.Params[0]]
	if cfg == nil {
		return errors.New(fmt.Sprint("unknown adapter ", directive.Params[0], "\n"))
	}
	if cfg.cmd == nil {
		return errors.New("adapter configuration is missing 'cmd' field\n")
	}
	adapter, err := newAdapter(*cfg)
	if err != nil {
		return err
	}
	if ui != nil {
		ui.focusedAdapter = &adapter.id
	}

	return nil
}

func cmdParseQuit(directive *scfg.Directive) error {
	if len(directive.Params) == 0 {
		for _, adapter := range adapters {
			adapter.finish()
		}
		if ui != nil {
			ui.send(uiShutdown)
		}
		return nil
	}

	adapter := adapters[directive.Params[0]]
	if adapter == nil {
		return errors.New(fmt.Sprint("adapter", directive.Params[0], "is not active"))
	}
	adapter.finish()
	return nil
}

func cmdParseContinue(directive *scfg.Directive) error {
	if ui == nil {
		return nil
	}
	adapter := adapters[*ui.focusedAdapter]
	if adapter == nil {
		return nil
	}
	adapter.send(&dap.ContinueRequest{
		Request: adapter.newRequest("continue"),
		Arguments: dap.ContinueArguments{
			// TODO:
			ThreadId: 1,
		},
	})
	return nil
}
