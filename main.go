package main

import (
	"encoding/json"
	"errors"
	"fmt"
	"log"
	"os"
	"sync"

	"github.com/adrg/xdg"
	"github.com/google/go-dap"
)

var (
	adapters    map[string]*adapter
	breakpoints map[string][]dap.SourceBreakpoint
	config      configFile
	ui          *UI
	wg          sync.WaitGroup
)

func abort(message error) {
	if message != nil {
		fmt.Println(message)
	}
	os.Exit(1)
}

func main() {
	// Logging
	logPath, err := xdg.StateFile("pesticide.log")
	if err != nil {
		abort(err)
	}
	file, err := os.OpenFile(logPath, os.O_CREATE|os.O_TRUNC|os.O_WRONLY, 0666)
	if err != nil {
		abort(err)
	}
	log.SetOutput(file)

	adapters = make(map[string]*adapter)
	breakpoints = make(map[string][]dap.SourceBreakpoint)

	// TODO: Handle vscode-style launch.json?
	parseConfig(".pesticide")

	ui = initUi()

	wg.Wait()

	for _, a := range adapters {
		a.finish()
	}
}

func parseConfig(path string) {
	file, err := os.ReadFile(path)
	if err != nil {
		abort(err)
	}
	if err = json.Unmarshal(file, &config); err != nil {
		abort(err)
	}

	if len(config.Adapters) == 0 {
		abort(errors.New("No adapters were specified"))
	}
	for name, adapter := range config.Adapters {
		if adapter.Addr == nil && adapter.Cmd == nil {
			abort(errors.New("Adapters must have an address or command to run"))
		}
		if adapter.Cmd != nil {
			expanded := os.ExpandEnv(*adapter.Cmd)
			adapter.Cmd = &expanded
		}
		config.Adapters[name] = adapter
	}
}

type configFile struct {
	Adapters map[string]adapterConfig
}
