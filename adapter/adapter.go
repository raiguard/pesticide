package adapter

import (
	"bufio"
	"encoding/json"
	"errors"
	"io"
	"log"
	"net"
	"os/exec"
	"time"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/google/go-dap"
	"github.com/google/shlex"
	"github.com/raiguard/pesticide/config"
)

type AdapterState int

const (
	Initializing AdapterState = iota
	Running
	Stopped
)

type Adapter struct {
	// Internally managed
	Capabilities    dap.Capabilities
	ID              string
	Seq             int
	PendingRequests map[int]dap.Message
	// Managed by router
	State             AdapterState
	Threads           []dap.Thread
	FocusedThread     int
	StackFrames       map[int][]dap.StackFrame
	FocusedStackFrame *dap.StackFrame
	Breakpoints       map[string][]dap.SourceBreakpoint
	// Internal I/O
	rw         bufio.ReadWriter
	cmd        *exec.Cmd
	launchArgs json.RawMessage
	conn       *net.Conn
}

type Msg struct {
	ID  string
	Msg tea.Msg
}

func New(config config.AdapterConfig) (*Adapter, error) {
	var cmd *exec.Cmd
	var conn *net.Conn
	var rw *bufio.ReadWriter
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
	}

	if rw == nil {
		return nil, errors.New("Adapter must either have a connection or a subprocess")
	}

	a := &Adapter{
		Capabilities:      dap.Capabilities{},
		ID:                config.ID,
		Seq:               0,
		PendingRequests:   map[int]dap.Message{},
		Threads:           []dap.Thread{},
		FocusedThread:     0,
		StackFrames:       map[int][]dap.StackFrame{},
		FocusedStackFrame: &dap.StackFrame{},
		Breakpoints:       map[string][]dap.SourceBreakpoint{},
		rw:                *rw,
		cmd:               cmd,
		launchArgs:        config.Args,
		conn:              conn,
	}

	return a, nil
}

func (a *Adapter) Shutdown() {
	conn := a.conn
	if conn != nil {
		(*conn).Close()
	}
	cmd := a.cmd
	if cmd != nil {
		cmd.Process.Kill()
	}
	log.Printf("[%s] EXITED\n", a.ID)
}

func (a *Adapter) Send(msg dap.Message) {
	a.PendingRequests[msg.GetSeq()] = msg
	err := dap.WriteProtocolMessage(a.rw.Writer, msg)
	if err != nil {
		log.Println("Unable to send message to adapter: ", err)
		return
	}
	val, err := json.Marshal(msg)
	if err != nil {
		panic(err)
	}
	log.Printf("[%s] <- %s", a.ID, string(val))
	a.rw.Writer.Flush()
}

func (a *Adapter) Receive() Msg {
	if a == nil {
		return Msg{}
	}
	res := Msg{ID: a.ID}
	msg, err := dap.ReadProtocolMessage(a.rw.Reader)
	if err != nil {
		if !errors.Is(err, io.EOF) {
			res.Msg = tea.Println(err)
		}
		return res
	}
	val, err := json.Marshal(msg)
	if err != nil {
		res.Msg = tea.Println(err)
		return res
	}
	log.Printf("[%s] -> %s", a.ID, string(val))
	// Increment seq
	seq := msg.GetSeq()
	if seq > a.Seq {
		a.Seq = seq
	}
	res.Msg = msg
	return res
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
