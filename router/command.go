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
	case command.Continue:
		r.focusedAdapter.Send(&dap.ContinueRequest{
			Request: r.focusedAdapter.NewRequest("continue"),
		})
	case command.Launch:
		return r.handleLaunchCommand(cmd)
	case command.Pause:
		r.focusedAdapter.Send(&dap.PauseRequest{
			Request: r.focusedAdapter.NewRequest("pause"),
		})
	case command.Quit:
		if r.focusedAdapter == nil {
			// Quit everything
			close(r.input)
			return nil
		}
		// TODO: Store whether terminate has been sent and send disconnect in that case
		if r.focusedAdapter.Capabilities.SupportsTerminateRequest {
			r.focusedAdapter.Send(&dap.TerminateRequest{
				Request: r.focusedAdapter.NewRequest("terminate"),
			})
		} else {
			r.focusedAdapter.Send(&dap.DisconnectRequest{
				Request: r.focusedAdapter.NewRequest("disconnect"),
				Arguments: dap.DisconnectArguments{
					TerminateDebuggee: true,
				},
			})
		}
	}
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
