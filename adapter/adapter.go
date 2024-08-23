package adapter

import (
	"bufio"
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"log"
	"net"
	"os/exec"
	"syscall"
	"time"

	"github.com/google/go-dap"
	"github.com/google/shlex"
	"github.com/raiguard/pesticide/config"
)

type Adapter struct {
	rw        bufio.ReadWriter
	sendQueue chan dap.Message

	cmd        *exec.Cmd
	launchArgs json.RawMessage
	conn       *net.Conn

	capabilities      dap.Capabilities
	id                string
	seq               int
	threads           []dap.Thread
	stackframes       map[int][]dap.StackFrame
	pendingRequests   map[int]dap.Message
	focusedStackFrame *dap.StackFrame
	focusedThread     int
}

func New(config config.AdapterConfig) (*Adapter, error) {
	var cmd *exec.Cmd
	var conn *net.Conn
	var rw *bufio.ReadWriter
	var id string
	if config.Cmd != nil {
		args, err := shlex.Split(*config.Cmd)
		if err != nil {
			return nil, err
		}
		child := exec.Command(args[0], args[1:]...)
		// Prevent propagation of signals
		child.SysProcAttr = &syscall.SysProcAttr{
			Setpgid: true,
			Pgid:    0,
		}
		stdin, err := child.StdinPipe()
		if err != nil {
			return nil, err
		}
		stdout, err := child.StdoutPipe()
		if err != nil {
			return nil, err
		}
		err = child.Start()
		if err != nil {
			return nil, err
		}
		cmd = child

		reader := bufio.NewReader(stdout)
		writer := bufio.NewWriter(stdin)
		rw = &bufio.ReadWriter{Reader: reader, Writer: writer}
		id = fmt.Sprint(cmd.Process.Pid)
	}
	if config.Addr != nil {
		if cmd != nil {
			time.Sleep(time.Millisecond * 500) // Give time for the cmd to init
		}
		conn, err := net.Dial("tcp", *config.Addr)
		// TODO: Handle errors gracefully
		if err != nil {
			return nil, err
		}

		reader := bufio.NewReader(conn)
		writer := bufio.NewWriter(conn)

		// TODO: Combine with STDIO input/output
		rw = &bufio.ReadWriter{Reader: reader, Writer: writer}
		id = *config.Addr
	}

	if rw == nil {
		return nil, errors.New("Adapter must either have a connection or a subprocess")
	}

	a := &Adapter{
		rw:                *rw,
		sendQueue:         make(chan dap.Message),
		cmd:               cmd,
		launchArgs:        config.Args,
		conn:              conn,
		capabilities:      dap.Capabilities{},
		id:                id,
		seq:               0,
		threads:           []dap.Thread{},
		stackframes:       map[int][]dap.StackFrame{},
		pendingRequests:   map[int]dap.Message{},
		focusedStackFrame: &dap.StackFrame{},
		focusedThread:     0,
	}

	go a.sendFromQueue()
	go a.receive()

	return a, nil
}

func (a *Adapter) Shutdown() {
	close(a.sendQueue)
	conn := a.conn
	if conn != nil {
		(*conn).Close()
	}
	cmd := a.cmd
	if cmd != nil {
		cmd.Process.Kill()
	}
	log.Printf("[%s] EXITED\n", a.id)
	// TODO: Remove from controller adapters list
}

func (a *Adapter) Send(msg dap.Message) {
	a.sendQueue <- msg
}

func (a *Adapter) sendFromQueue() {
	for msg := range a.sendQueue {
		err := dap.WriteProtocolMessage(a.rw.Writer, msg)
		if err != nil {
			log.Println("Unable to send message to adapter: ", err)
			continue
		}
		log.Printf("[%s] <- %#v", a.id, msg)
		a.rw.Writer.Flush()
	}
}

func (a *Adapter) receive() {
	for {
		msg, err := dap.ReadProtocolMessage(a.rw.Reader)
		if err != nil {
			break
		}
		switch msg.(type) {
		case *dap.OutputEvent:
		default:
			log.Printf("[%s] -> %#v", a.id, msg)
		}
		// Increment seq
		seq := msg.GetSeq()
		if seq > a.seq {
			a.seq = seq
		}
		a.handleMessage(msg)
	}
}

func (a *Adapter) NewRequest(command string) dap.Request {
	a.seq++
	return dap.Request{
		ProtocolMessage: dap.ProtocolMessage{Seq: a.seq, Type: "request"},
		Command:         command,
	}
}

func (a *Adapter) handleMessage(msg dap.Message) {
	switch msg := msg.(type) {
	case dap.ResponseMessage:
		ctx := a.pendingRequests[msg.GetResponse().RequestSeq]
		delete(a.pendingRequests, msg.GetResponse().RequestSeq)
		// TODO: Handle error responses
		switch msg := msg.(type) {
		case *dap.InitializeResponse:
			a.onInitializeResponse(msg)
		case *dap.StackTraceResponse:
			a.onStackTraceResponse(msg, ctx.(*dap.StackTraceRequest))
		case *dap.EvaluateResponse:
			a.onEvaluateResponse(msg)
		}
	case dap.EventMessage:
		switch msg := msg.(type) {
		case *dap.InitializedEvent:
			a.onInitializedEvent(msg)
		case *dap.TerminatedEvent:
			a.Shutdown()
		case *dap.OutputEvent:
			a.onOutputEvent(msg)
		case *dap.StoppedEvent:
			a.onStoppedEvent(msg)
		}
	}
}

func (a *Adapter) onInitializeResponse(res *dap.InitializeResponse) {
	a.capabilities = res.Body
	a.Send(&dap.LaunchRequest{
		Request:   a.NewRequest("launch"),
		Arguments: a.launchArgs,
	})
}

func (a *Adapter) onOutputEvent(ev *dap.OutputEvent) {
	// ui.print(strings.TrimSpace(ev.Body.Output))
}

func (a *Adapter) onStoppedEvent(ev *dap.StoppedEvent) {
	// ui.print(a.id, " stopped: ", ev.Body.Reason, ": ", ev.Body.Text)
	a.focusedThread = ev.Body.ThreadId
	a.Send(&dap.StackTraceRequest{
		Request:   a.NewRequest("stackTrace"),
		Arguments: dap.StackTraceArguments{ThreadId: ev.Body.ThreadId},
	})
}

func (a *Adapter) sendPauseRequest() {
	var threadId int
	if len(a.threads) == 0 {
		threadId = 1
	} else {
		threadId = a.focusedThread
	}
	a.Send(&dap.PauseRequest{
		Request: a.NewRequest("pause"),
		Arguments: dap.PauseArguments{
			ThreadId: threadId,
		},
	})
}

func (a *Adapter) onInitializedEvent(ev *dap.InitializedEvent) {
	a.sendSetBreakpointsRequest()
	if a.capabilities.SupportsConfigurationDoneRequest {
		a.Send(&dap.ConfigurationDoneRequest{
			Request:   a.NewRequest("configurationDone"),
			Arguments: dap.ConfigurationDoneArguments{},
		})
	}
}

func (a *Adapter) sendSetBreakpointsRequest() {
	// for filename, breakpoints := range breakpoints {
	// 	a.Send(&dap.SetBreakpointsRequest{
	// 		Request: a.newRequest("setBreakpoints"),
	// 		Arguments: dap.SetBreakpointsArguments{
	// 			Source: dap.Source{
	// 				Name: filename,
	// 				Path: filename,
	// 			},
	// 			Breakpoints: breakpoints,
	// 		},
	// 	})
	// }
}

func (a *Adapter) onStackTraceResponse(res *dap.StackTraceResponse, ctx *dap.StackTraceRequest) {
	a.stackframes[ctx.Arguments.ThreadId] = res.Body.StackFrames
	a.focusedStackFrame = &a.stackframes[ctx.Arguments.ThreadId][0]
	a.jumpInKak()
}

func (a *Adapter) onEvaluateResponse(res *dap.EvaluateResponse) {
	// ui.print(res.Body.Result)
}

func (a *Adapter) travelStackFrame(delta int) {
	if a.focusedStackFrame == nil {
		// ui.print("no stack frame is selected")
		return
	}

	stackFrames := a.stackframes[a.focusedThread]
	toFocus := -1
	for i, frame := range stackFrames {
		if frame.Id == a.focusedStackFrame.Id {
			toFocus = i + delta
			break
		}
	}
	if toFocus < 0 {
		toFocus = 0
	} else if len(stackFrames)-1 < toFocus {
		toFocus = len(stackFrames) - 1
	}
	a.focusedStackFrame = &stackFrames[toFocus]
	a.jumpInKak()
}

func (a *Adapter) jumpInKak() {
	cmd := exec.Command("kak", "-p", "Krastorio2")
	buffer := bytes.Buffer{}
	if a.focusedStackFrame.Source.Path == "" {
		return
	}
	buffer.WriteString(fmt.Sprintf("evaluate-commands -client %%opt{jumpclient} %%{edit %s %d}", a.focusedStackFrame.Source.Path, a.focusedStackFrame.Line))
	cmd.Stdin = &buffer
	cmd.Run()
}
