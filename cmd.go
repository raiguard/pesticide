package main

import (
	"encoding/json"
	"errors"
	"fmt"
	"path/filepath"
	"strconv"
	"strings"

	"git.sr.ht/~emersion/go-scfg"
	"github.com/google/go-dap"
)

type adapterConfig struct {
	Cmd  *string
	Args json.RawMessage
	Addr *string
}

// Cmd parses user input and configuration files using the scfg syntax, and
// executes UI or adapter commands.

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
	case "launch", "l":
		return cmdParseLaunch(directive)
	case "quit", "q":
		return cmdParseQuit(directive)
	case "continue", "c":
		return cmdParseContinue(directive)
	case "break", "b":
		return cmdParseBreak(directive)
	default:
		return errors.New(fmt.Sprint("Unknown command: ", directive.Name, "\n"))
	}
}

func cmdParseLaunch(directive *scfg.Directive) error {
	if len(directive.Params) == 0 {
		return errors.New("did not specify a configuration to launch\n")
	}
	cfg, ok := config.Adapters[directive.Params[0]]
	if !ok {
		return errors.New(fmt.Sprint("unknown adapter ", directive.Params[0], "\n"))
	}
	if cfg.Cmd == nil {
		return errors.New("adapter configuration is missing 'cmd' field\n")
	}
	adapter, err := newAdapter(cfg)
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

func cmdParseBreak(directive *scfg.Directive) error {
	if len(directive.Params) != 2 {
		return errors.New("break command must have two arguments")
	}
	filename, err := filepath.Abs(directive.Params[0])
	if err != nil {
		return err
	}
	line, err := strconv.ParseInt(directive.Params[1], 0, 0)
	if err != nil {
		return err
	}

	if breakpoints[filename] == nil {
		breakpoints[filename] = []dap.SourceBreakpoint{}
	}
	breakpoints[filename] = append(breakpoints[filename], dap.SourceBreakpoint{Line: int(line)})

	if ui != nil && ui.focusedAdapter != nil {
		adapter := adapters[*ui.focusedAdapter]
		if adapter != nil {
			adapter.sendSetBreakpointsRequest()
		}
	}

	return errors.New(fmt.Sprint("Set breakpoint at ", filename, " line ", line, "\n"))
}
