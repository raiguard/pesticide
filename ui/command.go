package ui

import (
	"fmt"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/google/go-dap"
	"github.com/raiguard/pesticide/adapter"
	"github.com/raiguard/pesticide/command"
)

func (m *Model) handleCommand(cmd command.Command) tea.Cmd {
	switch cmd := cmd.(type) {
	case command.Backtrace:
		return m.handleBacktraceCommand(cmd)
	case command.Break:
		return m.handleBreakCommand(cmd)
	case command.Continue:
		return m.handleContinueCommand(cmd)
	case command.Evaluate:
		return m.handleEvaluateCommand(cmd)
	case command.Launch:
		return m.handleLaunchCommand(cmd)
	case command.Pause:
		return m.handlePauseCommand(cmd)
	case command.Quit:
		return m.handleQuitCommand(cmd)
	case command.Down:
		return m.handleStackFrameCommand(-int(cmd))
	case command.Up:
		return m.handleStackFrameCommand(int(cmd))
	}
	return nil
}

func (m *Model) handleBacktraceCommand(cmd command.Backtrace) tea.Cmd {
	a := m.focusedAdapter
	if a == nil {
		return tea.Println("No adapter in focus")
	}
	var output string
	for _, frame := range a.StackFrames[a.FocusedThread] {
		output += fmt.Sprintf("%s:%d\n", frame.Source.Path, frame.Line)
	}
	return tea.Printf("%s", output)
}

func (m *Model) handleBreakCommand(cmd command.Break) tea.Cmd {
	a := m.focusedAdapter
	if a == nil {
		return tea.Println("No adapter in focus")
	}
	if _, ok := a.Breakpoints[cmd.File]; !ok {
		a.Breakpoints[cmd.File] = []dap.SourceBreakpoint{}
	}
	// TODO: Deduplicate
	a.Breakpoints[cmd.File] = append(a.Breakpoints[cmd.File], dap.SourceBreakpoint{Line: cmd.Line})
	m.sendSetBreakpointsRequest(a)
	return nil
}

func (m *Model) handleContinueCommand(cmd command.Continue) tea.Cmd {
	a := m.focusedAdapter
	if a == nil {
		return tea.Println("No adapter in focus")
	}
	a.Send(&dap.ContinueRequest{Request: a.NewRequest("continue")})
	return nil
}

func (m *Model) handleEvaluateCommand(cmd command.Evaluate) tea.Cmd {
	a := m.focusedAdapter
	if a == nil {
		return tea.Println("No adapter in focus")
	}
	if a.State != adapter.Stopped {
		return tea.Println("Cannot evaluate expressions while running")
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

func (m *Model) handleLaunchCommand(cmd command.Launch) tea.Cmd {
	adapterConfig, ok := m.config.Adapters[cmd.Name]
	if !ok {
		return tea.Printf("Unknown debug adapter %s", cmd.Name)
	}
	a, err := adapter.New(adapterConfig)
	if err != nil {
		return tea.Println(err)
	}
	m.adapters[a.ID] = a
	m.focusedAdapter = a
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
	return func() tea.Msg { return a.Receive() }
}

func (m *Model) handlePauseCommand(cmd command.Pause) tea.Cmd {
	a := m.focusedAdapter
	if a == nil {
		return tea.Println("No adapter in focus")
	}
	a.Send(&dap.PauseRequest{
		Request: a.NewRequest("pause"),
	})
	return nil

}

func (m *Model) handleQuitCommand(cmd command.Quit) tea.Cmd {
	a := m.focusedAdapter
	if a == nil {
		// TODO: Clean up unfocused adapters
		return tea.Quit
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

func (m *Model) handleStackFrameCommand(delta int) tea.Cmd {
	a := m.focusedAdapter
	if a == nil {
		return tea.Println("No adapter in focus")
	}
	return m.travelStackFrame(a, delta)
}
