package main

import (
	"bufio"
	"encoding/json"
	"log"
	"net"
	"os/exec"

	"github.com/google/go-dap"
)

type session struct {
	// Common I/O
	adapter   bufio.ReadWriter
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

func newStdioSession(cmd string, args []string, launchArgs json.RawMessage) *session {
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

	s := &session{
		adapter:    bufio.ReadWriter{Reader: bufio.NewReader(stdout), Writer: bufio.NewWriter(stdin)},
		cmd:        child,
		launchArgs: launchArgs,
		recvQueue:  make(chan dap.Message),
		sendQueue:  make(chan dap.Message),
	}

	go s.sendFromQueue()
	go s.recv()

	return s
}

func newTcpSession(addr string) *session {
	conn, err := net.Dial("tcp", addr)
	if err != nil {
		panic(err)
	}

	s := &session{
		adapter:   bufio.ReadWriter{Reader: bufio.NewReader(conn), Writer: bufio.NewWriter(conn)},
		conn:      &conn,
		recvQueue: make(chan dap.Message),
		sendQueue: make(chan dap.Message),
	}
	go s.sendFromQueue()
	go s.recv()
	return s
}

func (s *session) finish() {
	conn := s.conn
	if conn != nil {
		(*conn).Close()
	}
	cmd := s.cmd
	if cmd != nil {
		cmd.Process.Kill()
	}
}

// Queue a message to be sent to the adapter.
func (s *session) send(message dap.Message) {
	s.sendQueue <- message
}

// Sequentially send messages to the adapter.
func (s *session) sendFromQueue() {
	for msg := range s.sendQueue {
		err := dap.WriteProtocolMessage(s.adapter.Writer, msg)
		if err != nil {
			log.Println("Unable to send message to adapter: ", err)
			continue
		}
		log.Printf("ADAPTER <- %#v", msg)
		s.adapter.Writer.Flush()
	}
}

// Sequentially read messages from the adapter.
func (s *session) recv() {
	for {
		msg, err := dap.ReadProtocolMessage(s.adapter.Reader)
		if err != nil {
			log.Println("Error parsing adapter message: ", err)
			// TODO: Proper error handling
			break
		}
		log.Printf("ADAPTER -> %#v", msg)
		s.recvQueue <- msg
	}
}

// Construct a new Request and increment the sequence number.
func (s *session) newRequest(command string) dap.Request {
	s.seq++
	return dap.Request{
		ProtocolMessage: dap.ProtocolMessage{Seq: s.seq, Type: "request"},
		Command:         command,
	}
}
