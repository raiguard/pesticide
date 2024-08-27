package router

import (
	"errors"
	"fmt"

	"github.com/google/go-dap"
	"github.com/raiguard/pesticide/adapter"
	"github.com/raiguard/pesticide/command"
)

func (r *Router) handleCommand(cmd command.Command) error {
	switch cmd := cmd.(type) {
	case command.Break:
		return r.handleBreakCommand(cmd)
	case command.Continue:
		return r.handleContinueCommand(cmd)
	case command.Evaluate:
		return r.handleEvaluateCommand(cmd)
	case command.Launch:
		return r.handleLaunchCommand(cmd)
	case command.Pause:
		return r.handlePauseCommand(cmd)
	case command.Quit:
		return r.handleQuitCommand(cmd)
	}
	return nil
}

func (r *Router) handleBreakCommand(cmd command.Break) error {
	a := r.focusedAdapter
	if a == nil {
		return errors.New("No adapter in focus")
	}
	if _, ok := a.Breakpoints[cmd.File]; !ok {
		a.Breakpoints[cmd.File] = []dap.SourceBreakpoint{}
	}
	// TODO: Deduplicate
	a.Breakpoints[cmd.File] = append(a.Breakpoints[cmd.File], dap.SourceBreakpoint{Line: cmd.Line})
	r.sendSetBreakpointsRequest(a)
	return nil
}

func (r *Router) handleContinueCommand(cmd command.Continue) error {
	a := r.focusedAdapter
	if a == nil {
		return errors.New("No adapter in focus")
	}
	a.Send(&dap.ContinueRequest{Request: a.NewRequest("continue")})
	return nil
}

func (r *Router) handleEvaluateCommand(cmd command.Evaluate) error {
	a := r.focusedAdapter
	if a == nil {
		return errors.New("No adapter in focus")
	}
	if a.State != adapter.Stopped {
		return errors.New("Cannot evaluate expressions while running")
	}
	a.Send(&dap.EvaluateRequest{
		Request: a.NewRequest("evaluate"),
		Arguments: dap.EvaluateArguments{
			Expression: cmd.Expr,
			FrameId:    a.FocusedStackFrame.Id,
			Context:    "repl",
		},
	})
	return nil
}

func (r *Router) handleLaunchCommand(cmd command.Launch) error {
	adapterConfig, ok := r.config.Adapters[cmd.Name]
	if !ok {
		return errors.New(fmt.Sprintf("Unknown debug adapter %s", cmd.Name))
	}
	a, err := adapter.New(adapterConfig, r.input)
	if err != nil {
		return err
	}
	r.adapters[a.ID] = a
	r.focusedAdapter = a
	a.Send(&dap.InitializeRequest{
		Request: a.NewRequest("initialize"),
		Arguments: dap.InitializeRequestArguments{
			ClientID:        "pesticide",
			ClientName:      "Pesticide",
			Locale:          "en-US",
			PathFormat:      "path",
			LinesStartAt1:   true,
			ColumnsStartAt1: true,
		},
	})
	r.printf("Sent initialization request")
	return nil
}

func (r *Router) handlePauseCommand(cmd command.Pause) error {
	a := r.focusedAdapter
	if a == nil {
		return errors.New("No adapter in focus")
	}
	a.Send(&dap.PauseRequest{
		Request: a.NewRequest("pause"),
	})
	return nil

}

func (r *Router) handleQuitCommand(cmd command.Quit) error {
	a := r.focusedAdapter
	if a == nil {
		// Quit everything
		close(r.input)
		return nil
	}
	// TODO: Store whether terminate has been sent and send disconnect in that case
	if a.Capabilities.SupportsTerminateRequest {
		a.Send(&dap.TerminateRequest{Request: a.NewRequest("terminate")})
	} else {
		a.Send(&dap.DisconnectRequest{
			Request: a.NewRequest("disconnect"),
			Arguments: dap.DisconnectArguments{
				TerminateDebuggee: true,
			},
		})
		// TODO: Force-remove the adapter from the adapters list
	}
	return nil

}
