package main

import (
	"bufio"
	"fmt"
	"os"
	"os/signal"
	"strings"
	"syscall"

	"golang.org/x/term"
)

type UI struct {
	// Terminal internals
	currentCmd []byte
	in         *bufio.Reader
	oldState   *term.State
	// Channels
	events chan uiEvent
	sigs   chan os.Signal
	// State
	commandHistory []string
	focusedAdapter *string
}

type uiEvent struct {
	kind uiEventKind
	data string
}

type uiEventKind uint8

const (
	uiShutdown uiEventKind = iota
	uiDisplay
)

func initUi() *UI {
	oldState, err := term.MakeRaw(int(os.Stdin.Fd()))
	if err != nil {
		panic(err)
	}

	ui := &UI{
		// Terminal internals
		currentCmd: []byte{},
		in:         bufio.NewReader(os.Stdin),
		oldState:   oldState,
		// Channels
		events: make(chan uiEvent, 5),
		sigs:   make(chan os.Signal),
		// State
		commandHistory: []string{},
	}
	go ui.eventWorker()
	go ui.inputWorker()
	go ui.signalWorker()
	wg.Add(2)

	ui.drawInputLine()
	return ui
}

func (ui *UI) eventWorker() {
eventLoop:
	for event := range ui.events {
		switch event.kind {
		case uiDisplay:
			ui.clearLine()
			fmt.Printf("%s\r\n", strings.TrimSpace(event.data))
			ui.drawInputLine()
		case uiShutdown:
			ui.clearLine()
			break eventLoop
		}
	}
	// Shut down
	close(ui.events)
	close(ui.sigs)
	term.Restore(int(os.Stdin.Fd()), ui.oldState)
	wg.Done()
}

func (ui *UI) inputWorker() {
	var b []byte = make([]byte, 1)
	for {
		n, err := os.Stdin.Read(b)
		if err != nil {
			ui.display(err)
			continue
		} else if n == 0 {
			continue
		}
		switch b[0] {
		case '\r':
			ui.drawInputLine()
			fmt.Print("\r\n")
			err := cmdRead(string(ui.currentCmd))
			if err != nil {
				ui.display(err)
			} else {
				ui.currentCmd = []byte{}
			}
		default:
			ui.currentCmd = append(ui.currentCmd, b[0])
		}
		ui.drawInputLine()
	}
}

func (ui *UI) clearLine() {
	fmt.Print("\033[2K\r")
}

func (ui *UI) drawInputLine() {
	ui.clearLine()
	fmt.Printf("(pest) %s", ui.currentCmd)
}

func (ui *UI) signalWorker() {
	signal.Notify(ui.sigs, syscall.SIGINT)
	for range ui.sigs {
		if ui.focusedAdapter == nil {
			continue
		}
		adapter := adapters[*ui.focusedAdapter]
		if adapter == nil {
			continue
		}
		adapter.sendPauseRequest()
	}
	wg.Done()
}

func (ui *UI) display(in ...any) {
	ui.events <- uiEvent{kind: uiDisplay, data: fmt.Sprint(in...)}
}

func (ui *UI) send(kind uiEventKind) {
	ui.events <- uiEvent{kind: kind}
}
