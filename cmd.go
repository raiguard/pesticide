package main

import (
	"encoding/json"
	"errors"
	"fmt"
	"path/filepath"
	"strconv"
	"strings"

	"github.com/google/go-dap"
	"github.com/google/shlex"
)

type adapterConfig struct {
	Cmd  *string
	Args json.RawMessage
	Addr *string
}

// Cmd parses user input and configuration files using the scfg syntax, and
// executes UI or adapter commands.

func cmdRead(input string) error {
	args, err := shlex.Split(input)
	if err != nil {
		return err
	}
	if len(args) == 0 {
		return nil
	}
	var handler func([]string) error
	switch args[0] {
	case "break", "b":
		handler = cmdParseBreak
	case "continue", "c":
		handler = cmdParseContinue
	case "evaluate", "eval", "e":
		handler = cmdParseEvaluate
	case "launch", "l":
		handler = cmdParseLaunch
	case "pause", "p":
		handler = cmdParsePause
	case "quit", "q":
		handler = cmdParseQuit
	default:
		ui.printf("Unknown command: %s", args[0])
		return nil
	}
	return handler(args[1:])
}

func cmdParseBreak(args []string) error {
	if len(args) != 2 {
		return errors.New("break command must have two arguments")
	}
	filename, err := filepath.Abs(args[0])
	if err != nil {
		return err
	}
	line, err := strconv.ParseInt(args[1], 0, 0)
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

	return errors.New(fmt.Sprint("Set breakpoint at ", filename, " line ", line))
}

func cmdParseContinue(args []string) error {
	if ui == nil || ui.focusedAdapter == nil {
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

func cmdParseEvaluate(args []string) error {
	if ui == nil || ui.focusedAdapter == nil {
		return nil
	}
	adapter := adapters[*ui.focusedAdapter]
	adapter.send(&dap.EvaluateRequest{
		Request: adapter.newRequest("evaluate"),
		Arguments: dap.EvaluateArguments{
			Expression: strings.Join(args, " "),
			FrameId:    adapter.focusedStackFrame,
			Context:    "watch",
		},
	})
	return nil
}

func cmdParseLaunch(args []string) error {
	if len(args) == 0 {
		return errors.New("did not specify a configuration to launch\n")
	}
	adapterConfig, ok := config.Adapters[args[0]]
	if !ok {
		return errors.New(fmt.Sprint("unknown adapter ", args[0], "\n"))
	}
	adapter, err := newAdapter(adapterConfig)
	if err != nil {
		return err
	}
	if ui != nil {
		ui.focusedAdapter = &adapter.id
	}
	return nil
}

func cmdParsePause(args []string) error {
	focused := ui.focusedAdapter
	if focused == nil {
		return errors.New(fmt.Sprint("No adapter is currently running"))
	}
	adapter := adapters[*focused]
	if adapter == nil {
		return nil
	}
	adapter.sendPauseRequest()
	return nil
}

func cmdParseQuit(args []string) error {
	if focused := ui.focusedAdapter; focused != nil {
		adapter := adapters[*focused]
		ui.focusedAdapter = nil
		if adapter != nil {
			adapter.finish()
			return nil
		}
	}
	if ui != nil {
		ui.send(uiEvent{uiShutdown, ""})
	}
	return nil
}
