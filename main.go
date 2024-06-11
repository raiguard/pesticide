package main

import (
	"encoding/json"
	"errors"
	"log"
	"os"
	"sync"

	"github.com/google/go-dap"
)

var (
	adapters    map[string]*adapter
	breakpoints map[string][]dap.SourceBreakpoint
	config      configFile
	ui          *UI
	wg          sync.WaitGroup
)

func main() {
	file, err := os.OpenFile("/tmp/pesticide.log", os.O_CREATE|os.O_TRUNC|os.O_WRONLY, 0666)
	if err != nil {
		panic(err)
	}
	log.SetOutput(file)

	adapters = make(map[string]*adapter)
	breakpoints = make(map[string][]dap.SourceBreakpoint)

	// TODO: Handle vscode-style launch.json?
	parseConfig(".pesticide")

	ui = initUi()

	wg.Wait()

	for _, adapter := range adapters {
		adapter.finish()
	}
}

func parseConfig(path string) {
	file, err := os.ReadFile(path)
	if err != nil {
		panic(err)
	}
	if err = json.Unmarshal(file, &config); err != nil {
		panic(err)
	}

	if len(config.Adapters) == 0 {
		panic(errors.New("No adapters were specified"))
	}
	for name, adapter := range config.Adapters {
		if adapter.Addr == nil && adapter.Cmd == nil {
			panic(errors.New("Adapters must have an address or command to run"))
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
