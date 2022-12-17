package main

import (
	"log"
	"net"
	"os"

	"github.com/google/go-dap"
)

func main() {
	// Connect to mock server
	conn, err := net.Dial("tcp", ":54321")
	if err != nil {
		panic(err)
	}
	defer conn.Close()

	s := newTcpSession(conn)

	// Initialize
	s.send(&dap.InitializeRequest{
		Request: s.newRequest("initialize"),
		Arguments: dap.InitializeRequestArguments{
			AdapterID:                    "mock",
			ClientID:                     "pest",
			ClientName:                   "Pesticide",
			Locale:                       "en-US",
			PathFormat:                   "uri",
			SupportsRunInTerminalRequest: true,
		},
	})

	// Main loop
	for {
		select {
		case msg := <-s.recvQueue:
			handleMessage(s, msg)
		}
	}
}

func handleMessage(s *session, msg dap.Message) {
	switch msg := msg.(type) {
	case *dap.InitializeResponse:
		s.adapterCapabilities = msg.Body
		if s.adapterCapabilities.SupportsConfigurationDoneRequest {
			s.send(&dap.ConfigurationDoneRequest{
				Request:   s.newRequest("configurationDone"),
				Arguments: dap.ConfigurationDoneArguments{},
			})
		} else {
			s.phase = phaseRunning
			s.send(&dap.LaunchRequest{Request: s.newRequest("launch")})
		}
	case *dap.ConfigurationDoneResponse:
		s.phase = phaseRunning
		s.send(&dap.LaunchRequest{Request: s.newRequest("launch")})
	case *dap.TerminatedEvent:
		log.Println("Debug adapter sent terminated event, exiting...")
		os.Exit(1)
	}
}
