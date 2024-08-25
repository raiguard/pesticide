package adapter

import (
	"bufio"
	"encoding/json"
	"errors"
	"fmt"
	"log"
	"net"
	"os/exec"
	"time"

	"github.com/google/go-dap"
	"github.com/google/shlex"
	"github.com/raiguard/pesticide/config"
	"github.com/raiguard/pesticide/message"
)

type Adapter struct {
	// Internally managed
	Capabilities    dap.Capabilities
	ID              string
	Seq             int
	PendingRequests map[int]dap.Message
	// Managed by router
	Threads           []dap.Thread
	FocusedThread     int
	StackFrames       map[int][]dap.StackFrame
	FocusedStackFrame *dap.StackFrame
	Breakpoints       map[string][]dap.SourceBreakpoint
	// Channels
	sendQueue chan dap.Message
	recvQueue chan message.Message
	// Internal I/O
	rw         bufio.ReadWriter
	cmd        *exec.Cmd
	launchArgs json.RawMessage
	conn       *net.Conn
}

func New(config config.AdapterConfig, recvQueue chan message.Message) (*Adapter, error) {
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
		Capabilities:      dap.Capabilities{},
		ID:                id,
		Seq:               0,
		PendingRequests:   map[int]dap.Message{},
		Threads:           []dap.Thread{},
		FocusedThread:     0,
		StackFrames:       map[int][]dap.StackFrame{},
		FocusedStackFrame: &dap.StackFrame{},
		Breakpoints:       map[string][]dap.SourceBreakpoint{},
		sendQueue:         make(chan dap.Message),
		recvQueue:         recvQueue,
		rw:                *rw,
		cmd:               cmd,
		launchArgs:        config.Args,
		conn:              conn,
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
	log.Printf("[%s] EXITED\n", a.ID)
	// TODO: Remove from controller adapters list
}

func (a *Adapter) Send(msg dap.Message) {
	a.PendingRequests[msg.GetSeq()] = msg
	a.sendQueue <- msg
}

func (a *Adapter) NewRequest(command string) dap.Request {
	a.Seq++
	return dap.Request{
		ProtocolMessage: dap.ProtocolMessage{Seq: a.Seq, Type: "request"},
		Command:         command,
	}
}

func (a *Adapter) Launch() {
	a.Send(&dap.LaunchRequest{
		Request:   a.NewRequest("launch"),
		Arguments: a.launchArgs,
	})
}

func (a *Adapter) sendFromQueue() {
	for msg := range a.sendQueue {
		err := dap.WriteProtocolMessage(a.rw.Writer, msg)
		if err != nil {
			log.Println("Unable to send message to adapter: ", err)
			continue
		}
		val, err := json.Marshal(msg)
		if err != nil {
			panic(err)
		}
		log.Printf("[%s] <- %s", a.ID, string(val))
		a.rw.Writer.Flush()
	}
}

func (a *Adapter) receive() {
	for {
		msg, err := dap.ReadProtocolMessage(a.rw.Reader)
		if err != nil {
			break
		}
		val, err := json.Marshal(msg)
		if err != nil {
			panic(err)
		}
		log.Printf("[%s] -> %s", a.ID, string(val))
		// Increment seq
		seq := msg.GetSeq()
		if seq > a.Seq {
			a.Seq = seq
		}
		a.recvQueue <- message.DapMsg{Adapter: a.ID, Msg: msg}
	}
}
