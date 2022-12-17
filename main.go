package main

import (
	"fmt"
	"log"
	"os"

	"github.com/adrg/xdg"
	"github.com/google/go-dap"
)

type fmtkLaunchArgs struct {
	hookControl []string
	modsPath    string
}

func main() {
	// Logging
	logPath, err := xdg.StateFile("pesticide.log")
	if err != nil {
		panic(err)
	}
	file, err := os.OpenFile(logPath, os.O_CREATE|os.O_TRUNC|os.O_WRONLY, 0666)
	if err != nil {
		panic(err)
	}
	log.SetOutput(file)

	// s := newTcpSession(":54321")
	s := newStdioSession(
		"fmtk",
		[]string{"debug", "/home/rai/dev/factorio/1.1/bin/x64/factorio"},
		[]byte(`{"modsPath": "/home/rai/dev/factorio/1.1/mods"}`),
	)

	// Initialize
	s.send(&dap.InitializeRequest{
		Request: s.newRequest("initialize"),
		Arguments: dap.InitializeRequestArguments{
			AdapterID:                    "mock",
			ClientID:                     "pest",
			ClientName:                   "Pesticide",
			Locale:                       "en-US",
			PathFormat:                   "path",
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
			s.send(&dap.LaunchRequest{
				Request:   s.newRequest("launch"),
				Arguments: s.launchArgs,
			})
		}
	case *dap.ConfigurationDoneResponse:
		s.phase = phaseRunning
		s.send(&dap.LaunchRequest{
			Request:   s.newRequest("launch"),
			Arguments: s.launchArgs,
		})
	case *dap.TerminatedEvent:
		fmt.Println("Debug adapter sent terminated event, exiting...")
		os.Exit(1)
	case *dap.OutputEvent:
		fmt.Print(msg.Body.Output)
	}
}
