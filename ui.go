package main

import (
	"bufio"
	"fmt"
	"log"
	"os"
)

type UI struct {
	events chan uiEvent
	in     *bufio.Reader
}

type uiEvent struct {
	kind uiEventKind
	data string
}

type uiEventKind uint8

const (
	uiNextCmd uiEventKind = iota
	uiShutdown
	uiDisplay
)

func initUi() *UI {
	ui := &UI{
		events: make(chan uiEvent, 5),
		in:     bufio.NewReader(os.Stdin),
	}
	go ui.eventWorker()
	wg.Add(1)
	ui.send(uiNextCmd)
	return ui
}

func (ui *UI) eventWorker() {
eventLoop:
	for event := range ui.events {
		switch event.kind {
		case uiNextCmd:
			ui.handleNextCmd()
		case uiDisplay:
			fmt.Print(event.data)
		case uiShutdown:
			break eventLoop
		}
	}
	close(ui.events)
	wg.Done()
}

func (ui *UI) handleNextCmd() {
retry:
	fmt.Print("(pest) ")
	in, _, err := ui.in.ReadLine()
	if err != nil {
		log.Println("Failed to read from stdin:", err)
		goto retry
	}
	cmd := string(in)
	log.Printf("User command: '%s'\n", cmd)

	// TODO: Parse scfg command
	switch cmd {
	case "q":
		ui.send(uiShutdown)
	case "launch":
		newStdioAdapter(
			"fmtk",
			[]string{"debug", os.ExpandEnv("$FACTORIO")},
			[]byte(`{"modsPath": "/home/rai/dev/factorio/1.1/mods"}`),
		)
	case "attach":
		newTcpAdapter(":54321")
	default:
		ui.display("Unknown command: ", cmd, "\n")
		ui.send(uiNextCmd)
	}
}

func (ui *UI) display(in ...any) {
	ui.events <- uiEvent{kind: uiDisplay, data: fmt.Sprint(in...)}
}

func (ui *UI) send(kind uiEventKind) {
	ui.events <- uiEvent{kind: kind}
}
