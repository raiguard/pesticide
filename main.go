package main

import (
	tea "github.com/charmbracelet/bubbletea"

	"github.com/raiguard/pesticide/config"
	"github.com/raiguard/pesticide/message"
	"github.com/raiguard/pesticide/router"
	"github.com/raiguard/pesticide/ui"
)

func main() {
	f, err := tea.LogToFile("/tmp/pesticide.log", "")
	if err != nil {
		panic(err)
	}
	defer f.Close()

	fromUI := make(chan message.Message)
	fromRouter := make(chan message.Message)

	ui := ui.New(fromUI)
	go ui.Run()

	router := router.New(fromUI, fromRouter, config.New("pesticide.json"))
	go router.Run()

	for msg := range fromRouter {
		ui.Send(msg)
	}

	ui.Quit()
	ui.Wait()
}
