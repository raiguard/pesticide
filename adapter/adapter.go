package adapter

// Package adapter implements DAP adapters via STDIO and/or TCP.

import (
	"bufio"
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
)

type Adapter struct {
	// Common I/O
	rw        bufio.ReadWriter
	sendQueue chan dap.Message
	// TCP
	conn *net.Conn
	// Stdio
	child      *exec.Cmd
	launchArgs json.RawMessage
	// Lifecycle
	ID          string
	initialized bool
	// State
	capabilities      dap.Capabilities
	seq               int
	threads           []dap.Thread
	stackframes       map[int][]dap.StackFrame
	pendingRequests   map[int]dap.Message
	FocusedStackFrame int
	FocusedThread     int
}

func New(config Config) (*Adapter, error) {
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
			Chroot:                     "",
			Credential:                 &syscall.Credential{},
			Ptrace:                     false,
			Setsid:                     false,
			Setpgid:                    true,
			Setctty:                    false,
			Noctty:                     false,
			Ctty:                       0,
			Foreground:                 false,
			Pgid:                       0,
			Pdeathsig:                  0,
			Cloneflags:                 0,
			Unshareflags:               0,
			UidMappings:                []syscall.SysProcIDMap{},
			GidMappings:                []syscall.SysProcIDMap{},
			GidMappingsEnableSetgroups: false,
			AmbientCaps:                []uintptr{},
			UseCgroupFD:                false,
			CgroupFD:                   0,
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
		rw:              *rw,
		sendQueue:       make(chan dap.Message),
		conn:            conn,
		child:           cmd,
		launchArgs:      config.Args,
		ID:              id,
		pendingRequests: make(map[int]dap.Message),
		stackframes:     make(map[int][]dap.StackFrame),
	}

	return a, nil
}

func (a *Adapter) Start() {
	go a.recv()
	go a.sendFromQueue()

	// wg.Add(2)

	log.Printf("[%s] STARTED\n", a.ID)

	// Initialize
	a.Send(&dap.InitializeRequest{
		Request: a.NewRequest("initialize"),
		Arguments: dap.InitializeRequestArguments{
			ClientID:        "pest",
			ClientName:      "Pesticide",
			Locale:          "en-US",
			PathFormat:      "path",
			LinesStartAt1:   true,
			ColumnsStartAt1: true,
		},
	})
}

func (a *Adapter) Finish() {
	close(a.sendQueue)
	conn := a.conn
	if conn != nil {
		(*conn).Close()
	}
	cmd := a.child
	if cmd != nil {
		cmd.Process.Kill()
	}
	log.Printf("[%s] EXITED\n", a.ID)
}

func (a *Adapter) Send(message dap.Message) {
	a.pendingRequests[message.GetSeq()] = message
	a.sendQueue <- message
}

func (a *Adapter) sendFromQueue() {
	for msg := range a.sendQueue {
		err := dap.WriteProtocolMessage(a.rw.Writer, msg)
		if err != nil {
			log.Println("Unable to send message to adapter: ", err)
			continue
		}
		log.Printf("[%s] <- %#v", a.ID, msg)
		a.rw.Writer.Flush()
	}
}

func (a *Adapter) recv() {
	for {
		msg, err := dap.ReadProtocolMessage(a.rw.Reader)
		if err != nil {
			break
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
			// case *dap.EvaluateResponse:
			// 	a.onEvaluateResponse(msg)
		}
	case dap.EventMessage:
		switch msg := msg.(type) {
		case *dap.InitializedEvent:
			a.onInitializedEvent(msg)
		case *dap.TerminatedEvent:
			a.Finish()
		// case *dap.OutputEvent:
		// 	a.onOutputEvent(msg)
		case *dap.StoppedEvent:
			a.onStoppedEvent(msg)
		}
	}
}

func (a *Adapter) onInitializeResponse(res *dap.InitializeResponse) {
	a.capabilities = res.Body
	// a.phase = adapterRunning
	a.Send(&dap.LaunchRequest{
		Request:   a.NewRequest("launch"),
		Arguments: a.launchArgs,
	})
}

// func (a *Adapter) onOutputEvent(ev *dap.OutputEvent) {
// 	ui.print(strings.TrimSpace(ev.Body.Output))
// }

func (a *Adapter) onStoppedEvent(ev *dap.StoppedEvent) {
	// ui.print(a.id, " stopped: ", ev.Body.Reason)
	a.FocusedThread = ev.Body.ThreadId
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
		threadId = a.FocusedThread
	}
	a.Send(&dap.PauseRequest{
		Request: a.NewRequest("pause"),
		Arguments: dap.PauseArguments{
			ThreadId: threadId,
		},
	})
}

func (a *Adapter) onInitializedEvent(ev *dap.InitializedEvent) {
	// a.sendSetBreakpointsRequest()
	if a.capabilities.SupportsConfigurationDoneRequest {
		a.Send(&dap.ConfigurationDoneRequest{
			Request:   a.NewRequest("configurationDone"),
			Arguments: dap.ConfigurationDoneArguments{},
		})
	}
}

// func (a *Adapter) sendSetBreakpointsRequest() {
// 	for filename, breakpoints := range breakpoints {
// 		a.Send(&dap.SetBreakpointsRequest{
// 			Request: a.newRequest("setBreakpoints"),
// 			Arguments: dap.SetBreakpointsArguments{
// 				Source: dap.Source{
// 					Name: filename,
// 					Path: filename,
// 				},
// 				Breakpoints: breakpoints,
// 			},
// 		})
// 	}
// }

func (a *Adapter) onStackTraceResponse(res *dap.StackTraceResponse, ctx *dap.StackTraceRequest) {
	a.stackframes[ctx.Arguments.ThreadId] = res.Body.StackFrames
	a.FocusedStackFrame = res.Body.StackFrames[0].Id
}

// func (a *Adapter) onEvaluateResponse(res *dap.EvaluateResponse) {
// 	ui.print(res.Body.Result)
// }
