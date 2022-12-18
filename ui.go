package main

import (
	"bufio"
	"fmt"
	"log"
	"os"
	"strings"

	"git.sr.ht/~emersion/go-scfg"
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
	cmdStr := string(in)
	log.Printf("User command: '%s'\n", cmdStr)

	block, err := scfg.Read(strings.NewReader(cmdStr))
	if err != nil {
		fmt.Println(err)
	}
	cmd := block[0]

	// TODO: Unify command dispatch logic
	switch cmd.Name {
	case "quit", "q":
		ui.send(uiShutdown)
	case "launch", "l":
		cfg := adapterConfigs[cmd.Params[0]]
		if cfg == nil {
			fmt.Printf("Unknown adapter '%s'\n", cmd.Params[0])
			ui.send(uiNextCmd)
			return
		}
		// TODO: One of these might not exist
		newStdioAdapter(*cfg.cmd, *cfg.args)
	// case "attach":
	// 	newTcpAdapter(":54321")
	case "adapter-configs", "ac":
		fmt.Print("Configured adapters: ")
		for name := range adapterConfigs {
			fmt.Printf("%s ", name)
		}
		fmt.Print("\n")
		ui.send(uiNextCmd)
	case "help", "h":
		fmt.Print("Available commands:\nadapter-configs -- List available adapter configurations\nlaunch <name> -- Launch the specified adapter\nquit -- Quit pesticide\nhelp -- Show help menu\n")
		ui.send(uiNextCmd)
	default:
		ui.display("Unknown command: ", cmdStr, "\n")
		ui.send(uiNextCmd)
	}
}

func (ui *UI) display(in ...any) {
	ui.events <- uiEvent{kind: uiDisplay, data: fmt.Sprint(in...)}
}

func (ui *UI) send(kind uiEventKind) {
	ui.events <- uiEvent{kind: kind}
}
