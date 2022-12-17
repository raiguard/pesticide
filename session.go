package main

import (
	"bufio"
	"log"
	"net"

	"github.com/google/go-dap"
)

type session struct {
	adapter             bufio.ReadWriter
	adapterCapabilities dap.Capabilities
	recvQueue           chan dap.Message
	sendQueue           chan dap.Message

	// State variables
	phase sessionPhase
	seq   int
}

type sessionPhase uint8

const (
	phaseInitializing sessionPhase = iota
	phaseRunning
)

func newTcpSession(conn net.Conn) *session {
	s := &session{
		adapter:   bufio.ReadWriter{Reader: bufio.NewReader(conn), Writer: bufio.NewWriter(conn)},
		recvQueue: make(chan dap.Message),
		sendQueue: make(chan dap.Message),
	}
	go s.sendFromQueue()
	go s.recv()
	return s
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
