package main

// Adapter creates and manages communication with a debug adapter. A debugging
// session may have zero or more adapters.

import (
	"bufio"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"log"
	"net"
	"os/exec"
	"syscall"
	"time"

	"github.com/google/go-dap"
	"github.com/google/shlex"
)

type adapter struct {
	// Common I/O
	rw        bufio.ReadWriter
	sendQueue chan dap.Message
	// TCP
	conn *net.Conn
	// Stdio
	cmd        *exec.Cmd
	launchArgs json.RawMessage
	// State
	capabilities dap.Capabilities
	id           string
	phase        adapterPhase
	seq          int
	threads      []dap.Thread
}

type adapterPhase uint8

const (
	adapterInitializing adapterPhase = iota
	adapterRunning
)

func newAdapter(config adapterConfig) (*adapter, error) {
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

		rw = &bufio.ReadWriter{Reader: reader, Writer: writer}
		id = *config.Addr
	}

	if rw == nil {
		return nil, errors.New("Adapter must either have a connection or a subprocess")
	}

	a := &adapter{
		rw:         *rw,
		sendQueue:  make(chan dap.Message),
		conn:       conn,
		cmd:        cmd,
		launchArgs: config.Args,
		id:         id,
	}

	a.start()

	return a, nil
}

func (a *adapter) start() {
	adapters[a.id] = a

	go a.recv()
	go a.sendFromQueue()

	wg.Add(2)

	log.Printf("[%s] STARTED\n", a.id)

	// Initialize
	a.send(&dap.InitializeRequest{
		Request: a.newRequest("initialize"),
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

func (a *adapter) finish() {
	close(a.sendQueue)
	conn := a.conn
	if conn != nil {
		(*conn).Close()
	}
	cmd := a.cmd
	if cmd != nil {
		cmd.Process.Kill()
	}
	delete(adapters, a.id)
	log.Printf("[%s] EXITED\n", a.id)
}

func (a *adapter) send(message dap.Message) {
	a.sendQueue <- message
}

func (a *adapter) sendFromQueue() {
	for msg := range a.sendQueue {
		err := dap.WriteProtocolMessage(a.rw.Writer, msg)
		if err != nil {
			log.Println("Unable to send message to adapter: ", err)
			continue
		}
		log.Printf("[%s] <- %#v", a.id, msg)
		a.rw.Writer.Flush()
	}
	wg.Done()
}

func (a *adapter) recv() {
	for {
		msg, err := dap.ReadProtocolMessage(a.rw.Reader)
		if err != nil {
			if err != io.EOF {
				log.Println("Error parsing adapter message: ", err)
			}
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
	wg.Done()
}

func (a *adapter) newRequest(command string) dap.Request {
	a.seq++
	return dap.Request{
		ProtocolMessage: dap.ProtocolMessage{Seq: a.seq, Type: "request"},
		Command:         command,
	}
}

func (a *adapter) handleMessage(msg dap.Message) {
	// TODO: Handle error responses
	switch msg := msg.(type) {
	case *dap.InitializeResponse:
		a.onInitializeResponse(msg)
	case *dap.InitializedEvent:
		a.onInitializedEvent(msg)
	case *dap.TerminatedEvent:
		a.finish()
	case *dap.OutputEvent:
		a.onOutputEvent(msg)
	case *dap.StoppedEvent:
		a.onStoppedEvent(msg)
	}
}

func (a *adapter) onInitializeResponse(res *dap.InitializeResponse) {
	a.capabilities = res.Body
	// a.phase = adapterRunning
	a.send(&dap.LaunchRequest{
		Request:   a.newRequest("launch"),
		Arguments: a.launchArgs,
	})
}

func (a *adapter) onOutputEvent(ev *dap.OutputEvent) {
	ui.display(ev.Body.Output)
}

func (a *adapter) onStoppedEvent(ev *dap.StoppedEvent) {
	ui.display(a.id, " stopped: ", ev.Body.Reason, "\n")
}

func (a *adapter) sendPauseRequest() {
	var threadId int
	if len(a.threads) == 0 {
		threadId = 1
	} else {
		// TODO: Selected thread
		threadId = a.threads[0].Id
	}
	a.send(&dap.PauseRequest{
		Request: a.newRequest("pause"),
		Arguments: dap.PauseArguments{
			ThreadId: threadId,
		},
	})
}

func (a *adapter) onInitializedEvent(ev *dap.InitializedEvent) {
	a.sendSetBreakpointsRequest()
	if a.capabilities.SupportsConfigurationDoneRequest {
		a.send(&dap.ConfigurationDoneRequest{
			Request:   a.newRequest("configurationDone"),
			Arguments: dap.ConfigurationDoneArguments{},
		})
	}

}

func (a *adapter) sendSetBreakpointsRequest() {
	for filename, breakpoints := range breakpoints {
		a.send(&dap.SetBreakpointsRequest{
			Request: a.newRequest("setBreakpoints"),
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
