package main

import (
	"log"
	"os"
	"sync"

	"github.com/adrg/xdg"
)

var (
	adapters map[string]*adapter
	ui       *UI
	wg       sync.WaitGroup
)

func main() {
	// Logging
	// TODO: Handle these errors
	logPath, err := xdg.StateFile("pesticide.log")
	if err != nil {
		panic(err)
	}
	file, err := os.OpenFile(logPath, os.O_CREATE|os.O_TRUNC|os.O_WRONLY, 0666)
	if err != nil {
		panic(err)
	}
	log.SetOutput(file)

	adapters = make(map[string]*adapter)
	// TODO: Headless mode?
	ui = initUi()

	wg.Wait()

	for _, a := range adapters {
		a.finish()
	}
}
