package main

import (
	"fmt"
	"log"
	"os"

	"github.com/adrg/xdg"
	"github.com/google/go-dap"
)

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

	// a := newTcpAdapter(":54321")
	a := newStdioAdapter(
		"fmtk",
		[]string{"debug", os.ExpandEnv("$FACTORIO")},
		[]byte(`{"modsPath": "/home/rai/dev/factorio/1.1/mods"}`),
	)
	defer a.finish()

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

	// Main loop
	for {
		select {
		case msg := <-a.recvQueue:
			handleMessage(a, msg)
		}
	}
}

func handleMessage(s *adapter, msg dap.Message) {
	// TODO: Error handling
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
		os.Exit(1) // FIXME: This sucks
	case *dap.OutputEvent:
		fmt.Print(msg.Body.Output)
	}
}
