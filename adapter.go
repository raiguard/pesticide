package main

import (
	"bufio"
	"encoding/json"
	"log"
	"net"
	"os/exec"

	"github.com/google/go-dap"
)

type adapter struct {
	// Common I/O
	rw        bufio.ReadWriter
	recvQueue chan dap.Message
	sendQueue chan dap.Message
	// TCP
	conn *net.Conn
	// Stdio
	cmd        *exec.Cmd
	launchArgs json.RawMessage
	// Adapter settings
	adapterCapabilities dap.Capabilities
	// State
	phase sessionPhase
	seq   int
}

type sessionPhase uint8

const (
	phaseInitializing sessionPhase = iota
	phaseRunning
)

func newStdioAdapter(cmd string, args []string, launchArgs json.RawMessage) *adapter {
	child := exec.Command(cmd, args...)
	stdin, err := child.StdinPipe()
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

	s := &adapter{
		rw:         bufio.ReadWriter{Reader: bufio.NewReader(stdout), Writer: bufio.NewWriter(stdin)},
		cmd:        child,
		launchArgs: launchArgs,
		recvQueue:  make(chan dap.Message),
		sendQueue:  make(chan dap.Message),
	}

	go s.sendFromQueue()
	go s.recv()

	return s
}

func newTcpAdapter(addr string) *adapter {
	conn, err := net.Dial("tcp", addr)
	if err != nil {
		panic(err)
	}

	s := &adapter{
		rw:        bufio.ReadWriter{Reader: bufio.NewReader(conn), Writer: bufio.NewWriter(conn)},
		conn:      &conn,
		recvQueue: make(chan dap.Message),
		sendQueue: make(chan dap.Message),
	}
	go s.sendFromQueue()
	go s.recv()
	return s
}

func (a *adapter) finish() {
	conn := a.conn
	if conn != nil {
		(*conn).Close()
	}
	cmd := a.cmd
	if cmd != nil {
		cmd.Process.Kill()
	}
}

// Queue a message to be sent to the adapter.
func (a *adapter) send(message dap.Message) {
	a.sendQueue <- message
}

// Sequentially send messages to the adapter.
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
}

// Sequentially read messages from the adapter.
func (a *adapter) recv() {
	for {
		msg, err := dap.ReadProtocolMessage(a.rw.Reader)
		if err != nil {
			log.Println("Error parsing adapter message: ", err)
			// TODO: Proper error handling
			break
		}
		log.Printf("ADAPTER -> %#v", msg)
		a.recvQueue <- msg
	}
}

// Construct a new Request and increment the sequence number.
func (a *adapter) newRequest(command string) dap.Request {
	a.seq++
	return dap.Request{
		ProtocolMessage: dap.ProtocolMessage{Seq: a.seq, Type: "request"},
		Command:         command,
	}
}
