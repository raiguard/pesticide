package ui

import (
	"errors"
	"fmt"
	"os"
	"strings"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/google/go-dap"
	"github.com/raiguard/pesticide/adapter"
)

func (m *Model) handleAdapterMessage(msg adapter.Msg) tea.Cmd {
	if msg.ID == "" || msg.Msg == nil {
		return nil // The adapter was quit
	}
	var cmd tea.Cmd
	a, ok := m.adapters[msg.ID]
	if !ok {
		cmd = tea.Println("Received message for nonexistent adapter")
	}
	switch msg := msg.Msg.(type) {
	case dap.Message:
		cmd = m.handleDAPMessage(a, msg)
	case tea.Cmd:
		cmd = msg
	}
	return tea.Batch(cmd, func() tea.Msg { return a.Receive() })
}

func (m *Model) handleDAPMessage(a *adapter.Adapter, msg dap.Message) tea.Cmd {
	switch msg := msg.(type) {
	case dap.ResponseMessage:
		ctx, ok := a.PendingRequests[msg.GetResponse().RequestSeq]
		if !ok {
			return tea.Println("Received a response to a non-existent request")
		}
		delete(a.PendingRequests, msg.GetResponse().RequestSeq)
		// TODO: Handle error responses
		switch msg := msg.(type) {
		case *dap.InitializeResponse:
			return m.onInitializeResponse(a, msg)
		case *dap.StackTraceResponse:
			return m.onStackTraceResponse(a, msg, ctx.(*dap.StackTraceRequest))
		case *dap.EvaluateResponse:
			return m.onEvaluateResponse(msg)
		}
	case dap.EventMessage:
		switch msg := msg.(type) {
		case *dap.ContinuedEvent:
			a.State = adapter.Running
		case *dap.InitializedEvent:
			return m.onInitializedEvent(a, msg)
		case *dap.TerminatedEvent:
			a.Shutdown()
			delete(m.adapters, a.ID)
			if m.focusedAdapter == a {
				m.focusedAdapter = nil
				// TODO: Focus a different adapter? Decide on UX for this.
			}
		case *dap.StoppedEvent:
			return m.onStoppedEvent(a, msg)
		case *dap.OutputEvent:
			return m.onOutputEvent(a, msg)
		}
	}
	return nil
}

func (m *Model) onInitializeResponse(a *adapter.Adapter, res *dap.InitializeResponse) tea.Cmd {
	a.Capabilities = res.Body
	a.Launch()
	return nil
}

func (m *Model) onOutputEvent(a *adapter.Adapter, ev *dap.OutputEvent) tea.Cmd {
	return tea.Println(strings.TrimSpace(ev.Body.Output))
}

func (m *Model) onStoppedEvent(a *adapter.Adapter, event *dap.StoppedEvent) tea.Cmd {
	a.State = adapter.Stopped
	a.FocusedThread = event.Body.ThreadId
	a.Send(&dap.StackTraceRequest{
		Request:   a.NewRequest("stackTrace"),
		Arguments: dap.StackTraceArguments{ThreadId: event.Body.ThreadId},
	})
	return tea.Println(a.ID, " stopped: ", event.Body.Reason, ": ", event.Body.Text)
}

// func (r *Model) sendPauseRequest() {
// 	var threadId int
// 	if len(r.threads) == 0 {
// 		threadId = 1
// 	} else {
// 		threadId = r.focusedThread
// 	}
// 	r.Send(&dap.PauseRequest{
// 		Request: r.NewRequest("pause"),
// 		Arguments: dap.PauseArguments{
// 			ThreadId: threadId,
// 		},
// 	})
// }

func (m *Model) onInitializedEvent(a *adapter.Adapter, ev *dap.InitializedEvent) tea.Cmd {
	a.State = adapter.Running
	m.sendSetBreakpointsRequest(a)
	if a.Capabilities.SupportsConfigurationDoneRequest {
		a.Send(&dap.ConfigurationDoneRequest{
			Request:   a.NewRequest("configurationDone"),
			Arguments: dap.ConfigurationDoneArguments{},
		})
	}
	return nil
}

func (m *Model) sendSetBreakpointsRequest(a *adapter.Adapter) {
	for filename, breakpoints := range a.Breakpoints {
		a.Send(&dap.SetBreakpointsRequest{
			Request: a.NewRequest("setBreakpoints"),
			Arguments: dap.SetBreakpointsArguments{
				Source: dap.Source{
					Name: filename,
					Path: filename,
				},
				Breakpoints: breakpoints,
			},
		})
	}
}

func (m *Model) onStackTraceResponse(a *adapter.Adapter, res *dap.StackTraceResponse, ctx *dap.StackTraceRequest) tea.Cmd {
	a.StackFrames[ctx.Arguments.ThreadId] = res.Body.StackFrames
	a.FocusedStackFrame = &a.StackFrames[ctx.Arguments.ThreadId][0]
	return m.printFileLocation(a)
}

func (m *Model) onEvaluateResponse(res *dap.EvaluateResponse) tea.Cmd {
	return tea.Println(res.Body.Result)
}

func (m *Model) printFileLocation(a *adapter.Adapter) tea.Cmd {
	sf := a.FocusedStackFrame
	if sf == nil {
		return tea.Println(errors.New("No stack frame in context"))
	}
	if sf.Source.SourceReference != 0 {
		return tea.Println(errors.New("sourceReference is unimplemented"))
	}
	path := sf.Source.Path
	if path == "" {
		return tea.Println(errors.New("Path is empty"))
	}
	contents, err := os.ReadFile(path)
	if err != nil {
		return tea.Println(err)
	}
	lines := strings.Split(string(contents), "\n")
	if len(lines)-1 < sf.Line {
		return tea.Println(errors.New("Invalid line number"))
	}
	var output string
	for i := sf.Line - 3; i < sf.Line+4; i++ {
		if i < 0 || i > len(lines)-1 {
			continue
		}
		prefix := ""
		if i == sf.Line-1 {
			prefix = "->"
		}
		output += fmt.Sprintf("%3s %3d: %s\n", prefix, i+1, lines[i])
	}
	return tea.Println(output)
}

// func (r *Model) travelStackFrame(delta int) {
// 	if r.focusedStackFrame == nil {
// 		// ui.print("no stack frame is selected")
// 		return
// 	}

// 	stackFrames := r.stackframes[r.focusedThread]
// 	toFocus := -1
// 	for i, frame := range stackFrames {
// 		if frame.Id == r.focusedStackFrame.Id {
// 			toFocus = i + delta
// 			break
// 		}
// 	}
// 	if toFocus < 0 {
// 		toFocus = 0
// 	} else if len(stackFrames)-1 < toFocus {
// 		toFocus = len(stackFrames) - 1
// 	}
// 	r.focusedStackFrame = &stackFrames[toFocus]
// 	r.jumpInKak()
// }

// func (r *Model) jumpInKak() {
// 	cmd := exec.Command("kak", "-p", "Krastorio2")
// 	buffer := bytes.Buffer{}
// 	if r.focusedStackFrame.Source.Path == "" {
// 		return
// 	}
// 	buffer.WriteString(fmt.Sprintf("evaluate-commands -client %%opt{jumpclient} %%{edit %s %d}", r.focusedStackFrame.Source.Path, r.focusedStackFrame.Line))
// 	cmd.Stdin = &buffer
// 	cmd.Run()
// }
