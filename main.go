package main

import (
	"log"
	"os"
	"sync"

	"github.com/adrg/xdg"
	"github.com/google/go-dap"
)

var (
	adapters       map[string]*adapter
	adapterConfigs map[string]*adapterConfig
	breakpoints    map[string][]dap.SourceBreakpoint
	ui             *UI
	wg             sync.WaitGroup
)

func main() {
	// Logging
	// FIXME: Handle these errors
	logPath, _ := xdg.StateFile("pesticide.log")
	file, _ := os.OpenFile(logPath, os.O_CREATE|os.O_TRUNC|os.O_WRONLY, 0666)
	log.SetOutput(file)

	adapters = make(map[string]*adapter)
	adapterConfigs = make(map[string]*adapterConfig)
	breakpoints = make(map[string][]dap.SourceBreakpoint)
	// TODO: Headless mode?
	ui = initUi()

	// Read configuration
	cmdReadFile(".pesticide")

	wg.Wait()

	for _, a := range adapters {
		a.finish()
	}
}
