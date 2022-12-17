package main

import (
	"log"
	"os"
	"sync"

	"github.com/adrg/xdg"
)

var wg sync.WaitGroup

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

	// a := newTcpAdapter(":54321")
	a := newStdioAdapter(
		"fmtk",
		[]string{"debug", os.ExpandEnv("$FACTORIO")},
		[]byte(`{"modsPath": "/home/rai/dev/factorio/1.1/mods"}`),
	)
	defer a.finish()

	wg.Wait()
}
