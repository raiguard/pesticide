package main

// Adapter creates and manages communication with a debug adapter. A debugging
// session may have zero or more adapters.

import (
	"bufio"
	"encoding/json"
	"fmt"
	"log"
	"net"
	"os/exec"

	"github.com/google/go-dap"
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
	phase        adapterState
	seq          int
}

type adapterState uint8

const (
	adapterInitializing adapterState = iota
	adapterRunning
)

// Creates a new adapter communicating over STDIO. The adapter will be spawned
// as a child process.
func newStdioAdapter(cmd string, args []string, launchArgs json.RawMessage) *adapter {
	child := exec.Command(cmd, args...)
	// // Prevent propagation of signals
	// child.SysProcAttr = &syscall.SysProcAttr{
	// 	Setpgid: true,
	// 	Pgid:    0,
	// }
	stdin, err := child.StdinPipe()
	// TODO: Handle errors gracefully
	if err != nil {
		panic(err)
	}
	stdout, err := child.StdoutPipe()
	if err != nil {
		panic(err)
	}
	err = child.Start()
	if err != nil {
		panic(err)
	}

	reader := bufio.NewReader(stdout)
	writer := bufio.NewWriter(stdin)

	a := &adapter{
		rw:        bufio.ReadWriter{Reader: reader, Writer: writer},
		sendQueue: make(chan dap.Message),

		cmd:        child,
		launchArgs: launchArgs,

		id: fmt.Sprint(child.Process.Pid),
	}

	a.start()

	return a
}

// Creates a new adapter communicating over TCP. The adapter is unmanaged and
// must have been started separately.
func newTcpAdapter(addr string) *adapter {
	conn, err := net.Dial("tcp", addr)
	// TODO: Handle errors gracefully
	if err != nil {
		panic(err)
	}

	reader := bufio.NewReader(conn)
	writer := bufio.NewWriter(conn)

	a := &adapter{
		rw:        bufio.ReadWriter{Reader: reader, Writer: writer},
		conn:      &conn,
		sendQueue: make(chan dap.Message),
		id:        conn.LocalAddr().String(),
	}

	a.start()

	return a
}

func (a *adapter) start() {
	adapters[a.id] = a

	go a.recv()
	go a.sendFromQueue()

	wg.Add(2)

	// Initialize
	a.send(&dap.InitializeRequest{
		Request: a.newRequest("initialize"),
		Arguments: dap.InitializeRequestArguments{
			ClientID:   "pest",
			ClientName: "Pesticide",
			Locale:     "en-US",
			PathFormat: "path",
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
	fmt.Println("Adapter id", a.id, "exited")
	if len(adapters) == 0 {
		ui.send(uiNextCmd)
	}
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
		log.Printf("ADAPTER <- %#v", msg)
		a.rw.Writer.Flush()
	}
	wg.Done()
}

func (a *adapter) recv() {
	for {
		msg, err := dap.ReadProtocolMessage(a.rw.Reader)
		if err != nil {
			log.Println("Error parsing adapter message: ", err)
			// TODO: Proper error handling
			break
		}
		log.Printf("ADAPTER -> %#v", msg)
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
	case *dap.ConfigurationDoneResponse:
		a.onConfigurationDoneResponse(msg)
	case *dap.TerminatedEvent:
		a.finish()
	case *dap.OutputEvent:
		a.onOutputEvent(msg)
	}
}

func (a *adapter) onInitializeResponse(res *dap.InitializeResponse) {
	a.capabilities = res.Body
	if a.capabilities.SupportsConfigurationDoneRequest {
		a.send(&dap.ConfigurationDoneRequest{
			Request:   a.newRequest("configurationDone"),
			Arguments: dap.ConfigurationDoneArguments{},
		})
	} else {
		a.phase = adapterRunning
		a.send(&dap.LaunchRequest{
			Request:   a.newRequest("launch"),
			Arguments: a.launchArgs,
		})
	}
}

func (a *adapter) onConfigurationDoneResponse(res *dap.ConfigurationDoneResponse) {
	a.phase = adapterRunning
	a.send(&dap.LaunchRequest{
		Request:   a.newRequest("launch"),
		Arguments: a.launchArgs,
	})
}

func (a *adapter) onOutputEvent(ev *dap.OutputEvent) {
	ui.display(ev.Body.Output)
}
