package main

import (
	"fmt"

	"github.com/ergochat/readline"
)

type UI struct {
	// Backend
	events chan uiEvent
	rl     *readline.Instance
	// State
	focusedAdapter *string
}

type uiEvent struct {
	kind uiEventKind
	data string
}

type uiEventKind uint8

const (
	uiCommand uiEventKind = iota
	uiDisplay
	uiShutdown
)

func initUi() *UI {
	rl, err := readline.New("(pest) ")
	if err != nil {
		panic(err)
	}
	ui := &UI{
		events: make(chan uiEvent, 5),
		rl:     rl,
	}
	go ui.eventWorker()
	go ui.inputWorker() // Will be closed at the end of eventWorker
	wg.Add(1)
	return ui
}

// FIXME: This should be the primary program loop
func (ui *UI) eventWorker() {
eventLoop:
	for event := range ui.events {
		switch event.kind {
		case uiCommand:
			err := cmdRead(event.data)
			if err != nil {
				ui.print(err)
			}
		case uiDisplay:
			fmt.Printf("\033[2K\r%s\r\n", event.data)
			ui.rl.Refresh()
		case uiShutdown:
			break eventLoop
		}
	}
	close(ui.events)
	ui.rl.Close()
	wg.Done()
}

func (ui *UI) inputWorker() {
	for {
		line, err := ui.rl.Readline()
		if err != nil { // io.EOF
			break
		}
		ui.send(uiEvent{uiCommand, line})
	}
}

func (ui *UI) print(in ...any) {
	ui.events <- uiEvent{uiDisplay, fmt.Sprint(in...)}
}

func (ui *UI) printf(format string, in ...any) {
	ui.print(fmt.Sprintf(format, in...))
}

func (ui *UI) send(ev uiEvent) {
	ui.events <- ev
}
