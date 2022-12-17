package main

import (
	"bufio"
	"fmt"
	"log"
	"os"
	"os/signal"
	"syscall"
)

type UI struct {
	input chan string
	sigs  chan os.Signal
}

func initUi() *UI {
	ui := &UI{
		input: make(chan string),
		sigs:  make(chan os.Signal),
	}
	go ui.inputWorker()
	go ui.signalWorker()
	wg.Add(2)
	return ui
}

func (ui *UI) inputWorker() {
	in := bufio.NewReader(os.Stdin)
	for {
		fmt.Print("(pest) ")
		in, _, err := in.ReadLine()
		if err != nil {
			fmt.Errorf("%s\n", err)
			continue
		}
		cmd := string(in)
		log.Println("User command: '", cmd, "'")
		if cmd == "q" {
			break
		} else if cmd == "launch" {
			newStdioAdapter(
				"fmtk",
				[]string{"debug", os.ExpandEnv("$FACTORIO")},
				[]byte(`{"modsPath": "/home/rai/dev/factorio/1.1/mods"}`),
			)
		} else if cmd == "attach" {
			newTcpAdapter(":54321")
		}
	}
	close(ui.input)
	close(ui.sigs)
	wg.Done()
}

func (ui *UI) signalWorker() {
	signal.Notify(ui.sigs, syscall.SIGINT)
	for sig := range ui.sigs {
		fmt.Println("\n", sig.String())
	}
	wg.Done()
}
