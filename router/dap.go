package router

import (
	"encoding/json"
	"errors"
	"strings"

	"github.com/google/go-dap"
	"github.com/raiguard/pesticide/adapter"
	"github.com/raiguard/pesticide/message"
)

func (r *Router) handleDAPMessage(msg message.DapMsg) error {
	if _, ok := msg.Msg.(*dap.OutputEvent); !ok {
		val, err := json.Marshal(msg)
		if err != nil {
			return err
		}
		r.printf("Received DAP message: %s", string(val))
	}

	a, ok := r.adapters[msg.Adapter]
	if !ok {
		return errors.New("Received message for nonexistent adapter")
	}

	switch msg := msg.Msg.(type) {
	case dap.ResponseMessage:
		_, ok := a.PendingRequests[msg.GetResponse().RequestSeq]
		if !ok {
			return errors.New("Received a response to a non-existent request")
		}
		delete(a.PendingRequests, msg.GetResponse().RequestSeq)
		// TODO: Handle error responses
		switch msg := msg.(type) {
		case *dap.InitializeResponse:
			return r.onInitializeResponse(a, msg)
			// case *dap.StackTraceResponse:
			// 	return r.onStackTraceResponse(msg, ctx.(*dap.StackTraceRequest))
			// case *dap.EvaluateResponse:
			// 	return r.onEvaluateResponse(msg)
		}
	case dap.EventMessage:
		switch msg := msg.(type) {
		// case *dap.InitializedEvent:
		// 	return r.onInitializedEvent(msg)
		// case *dap.TerminatedEvent:
		// 	return r.Shutdown()
		case *dap.StoppedEvent:
			return r.onStoppedEvent(a, msg)
		case *dap.OutputEvent:
			r.println(strings.TrimSpace(msg.Body.Output))
		}
	}
	return nil
}

func (r *Router) onInitializeResponse(a *adapter.Adapter, res *dap.InitializeResponse) error {
	a.Capabilities = res.Body
	a.Launch()
	return nil
}

// func (r *Router) onOutputEvent(ev *dap.OutputEvent) {
// 	// ui.print(strings.TrimSpace(ev.Body.Output))
// }

func (r *Router) onStoppedEvent(a *adapter.Adapter, event *dap.StoppedEvent) error {
	r.println(a.ID, " stopped: ", event.Body.Reason, ": ", event.Body.Text)
	a.FocusedThread = event.Body.ThreadId
	// a.Send(&dap.StackTraceRequest{
	// 	Request:   a.NewRequest("stackTrace"),
	// 	Arguments: dap.StackTraceArguments{ThreadId: event.Body.ThreadId},
	// })
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

// func (r *Router) onInitializedEvent(ev *dap.InitializedEvent) {
// 	r.sendSetBreakpointsRequest()
// 	if r.Capabilities.SupportsConfigurationDoneRequest {
// 		r.Send(&dap.ConfigurationDoneRequest{
// 			Request:   r.NewRequest("configurationDone"),
// 			Arguments: dap.ConfigurationDoneArguments{},
// 		})
// 	}
// }

// func (r *Router) sendSetBreakpointsRequest() {
// 	// for filename, breakpoints := range breakpoints {
// 	// 	a.Send(&dap.SetBreakpointsRequest{
// 	// 		Request: a.newRequest("setBreakpoints"),
// 	// 		Arguments: dap.SetBreakpointsArguments{
// 	// 			Source: dap.Source{
// 	// 				Name: filename,
// 	// 				Path: filename,
// 	// 			},
// 	// 			Breakpoints: breakpoints,
// 	// 		},
// 	// 	})
// 	// }
// }

// func (r *Router) onStackTraceResponse(res *dap.StackTraceResponse, ctx *dap.StackTraceRequest) {
// 	r.stackframes[ctx.Arguments.ThreadId] = res.Body.StackFrames
// 	r.focusedStackFrame = &r.stackframes[ctx.Arguments.ThreadId][0]
// 	r.jumpInKak()
// }

// func (r *Router) onEvaluateResponse(res *dap.EvaluateResponse) {
// 	// ui.print(res.Body.Result)
// }

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
