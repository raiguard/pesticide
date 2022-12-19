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
	events chan uiEvent
	in     *bufio.Reader

	focusedAdapter *string
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
	go ui.signalWorker()
	wg.Add(2)
	ui.send(uiNextCmd)
	return ui
}

func (ui *UI) eventWorker() {
eventLoop:
	for event := range ui.events {
		switch event.kind {
		// FIXME: This isn't working out very well
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

func (ui *UI) signalWorker() {
	sigs := make(chan os.Signal)
	signal.Notify(sigs, syscall.SIGINT)
	for range sigs {
		if ui.focusedAdapter == nil {
			continue
		}
		adapter := adapters[*ui.focusedAdapter]
		if adapter == nil {
			continue
		}
		adapter.sendPauseRequest()
	}
	// FIXME: This will never get closed
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
	cmdRead(cmdStr)
}

func (ui *UI) display(in ...any) {
	ui.events <- uiEvent{kind: uiDisplay, data: fmt.Sprint(in...)}
}

func (ui *UI) send(kind uiEventKind) {
	ui.events <- uiEvent{kind: kind}
}
