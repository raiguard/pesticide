package adapter

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
	"github.com/raiguard/pesticide/config"
	"github.com/raiguard/pesticide/message"
)

type Adapter struct {
	Capabilities    dap.Capabilities
	ID              string
	PendingRequests map[int]dap.Message
	FocusedThread   int

	rw        bufio.ReadWriter
	sendQueue chan dap.Message
	recvQueue chan message.Message

	cmd        *exec.Cmd
	launchArgs json.RawMessage
	conn       *net.Conn

	seq               int
	threads           []dap.Thread
	stackframes       map[int][]dap.StackFrame
	focusedStackFrame *dap.StackFrame
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
		recvQueue:         recvQueue,
		cmd:               cmd,
		launchArgs:        config.Args,
		conn:              conn,
		Capabilities:      dap.Capabilities{},
		ID:                id,
		seq:               0,
		threads:           []dap.Thread{},
		stackframes:       map[int][]dap.StackFrame{},
		PendingRequests:   map[int]dap.Message{},
		focusedStackFrame: &dap.StackFrame{},
		FocusedThread:     0,
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
	a.seq++
	return dap.Request{
		ProtocolMessage: dap.ProtocolMessage{Seq: a.seq, Type: "request"},
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
		log.Printf("[%s] <- %#v", a.ID, msg)
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
			log.Printf("[%s] -> %#v", a.ID, msg)
		}
		// Increment seq
		seq := msg.GetSeq()
		if seq > a.seq {
			a.seq = seq
		}
		a.recvQueue <- message.DapMsg{Adapter: a.ID, Msg: msg}
	}
}
