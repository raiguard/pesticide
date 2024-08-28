package router

import (
	"errors"
	"os"
	"strings"

	"github.com/google/go-dap"
	"github.com/raiguard/pesticide/adapter"
	"github.com/raiguard/pesticide/message"
)

func (r *Router) handleDAPMessage(msg message.DapMsg) error {
	a, ok := r.adapters[msg.Adapter]
	if !ok {
		return errors.New("Received message for nonexistent adapter")
	}

	switch msg := msg.Msg.(type) {
	case dap.ResponseMessage:
		ctx, ok := a.PendingRequests[msg.GetResponse().RequestSeq]
		if !ok {
			return errors.New("Received a response to a non-existent request")
		}
		delete(a.PendingRequests, msg.GetResponse().RequestSeq)
		// TODO: Handle error responses
		switch msg := msg.(type) {
		case *dap.InitializeResponse:
			return r.onInitializeResponse(a, msg)
		case *dap.StackTraceResponse:
			return r.onStackTraceResponse(a, msg, ctx.(*dap.StackTraceRequest))
		case *dap.EvaluateResponse:
			return r.onEvaluateResponse(msg)
		}
	case dap.EventMessage:
		switch msg := msg.(type) {
		case *dap.ContinuedEvent:
			a.State = adapter.Running
		case *dap.InitializedEvent:
			return r.onInitializedEvent(a, msg)
		case *dap.TerminatedEvent:
			a.Shutdown()
			delete(r.adapters, a.ID)
			if r.focusedAdapter == a {
				r.focusedAdapter = nil
				// TODO: Focus a different adapter? Decide on UX for this.
			}
		case *dap.StoppedEvent:
			return r.onStoppedEvent(a, msg)
		case *dap.OutputEvent:
			return r.onOutputEvent(a, msg)
		}
	}
	return nil
}

func (r *Router) onInitializeResponse(a *adapter.Adapter, res *dap.InitializeResponse) error {
	a.Capabilities = res.Body
	a.Launch()
	return nil
}

func (r *Router) onOutputEvent(a *adapter.Adapter, ev *dap.OutputEvent) error {
	r.println(strings.TrimSpace(ev.Body.Output))
	return nil
}

func (r *Router) onStoppedEvent(a *adapter.Adapter, event *dap.StoppedEvent) error {
	a.State = adapter.Stopped
	r.println(a.ID, " stopped: ", event.Body.Reason, ": ", event.Body.Text)
	a.FocusedThread = event.Body.ThreadId
	a.Send(&dap.StackTraceRequest{
		Request:   a.NewRequest("stackTrace"),
		Arguments: dap.StackTraceArguments{ThreadId: event.Body.ThreadId},
	})
	return nil
}

// func (r *Router) sendPauseRequest() {
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

func (r *Router) onInitializedEvent(a *adapter.Adapter, ev *dap.InitializedEvent) error {
	a.State = adapter.Running
	r.sendSetBreakpointsRequest(a)
	if a.Capabilities.SupportsConfigurationDoneRequest {
		a.Send(&dap.ConfigurationDoneRequest{
			Request:   a.NewRequest("configurationDone"),
			Arguments: dap.ConfigurationDoneArguments{},
		})
	}
	return nil
}

func (r *Router) sendSetBreakpointsRequest(a *adapter.Adapter) {
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

func (r *Router) onStackTraceResponse(a *adapter.Adapter, res *dap.StackTraceResponse, ctx *dap.StackTraceRequest) error {
	a.StackFrames[ctx.Arguments.ThreadId] = res.Body.StackFrames
	a.FocusedStackFrame = &a.StackFrames[ctx.Arguments.ThreadId][0]
	return r.printFileLocation(a)
}

func (r *Router) onEvaluateResponse(res *dap.EvaluateResponse) error {
	r.println(res.Body.Result)
	return nil
}

func (r *Router) printFileLocation(a *adapter.Adapter) error {
	sf := a.FocusedStackFrame
	if sf == nil {
		return errors.New("No stack frame in context")
	}
	if sf.Source.SourceReference != 0 {
		return errors.New("sourceReference is unimplemented")
	}
	path := sf.Source.Path
	if path == "" {
		return errors.New("Path is empty")
	}
	contents, err := os.ReadFile(path)
	if err != nil {
		return err
	}
	lines := strings.Split(string(contents), "\n")
	if len(lines)-1 < sf.Line {
		return errors.New("Invalid line number")
	}
	for i := sf.Line - 3; i < sf.Line+4; i++ {
		if i < 0 || i > len(lines)-1 {
			continue
		}
		prefix := ""
		if i == sf.Line-1 {
			prefix = "->"
		}
		r.printf("%3s %3d: %s", prefix, i+1, lines[i])
	}
	return nil
}

// func (r *Router) travelStackFrame(delta int) {
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

// func (r *Router) jumpInKak() {
// 	cmd := exec.Command("kak", "-p", "Krastorio2")
// 	buffer := bytes.Buffer{}
// 	if r.focusedStackFrame.Source.Path == "" {
// 		return
// 	}
// 	buffer.WriteString(fmt.Sprintf("evaluate-commands -client %%opt{jumpclient} %%{edit %s %d}", r.focusedStackFrame.Source.Path, r.focusedStackFrame.Line))
// 	cmd.Stdin = &buffer
// 	cmd.Run()
// }
