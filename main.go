package main

import (
	tea "github.com/charmbracelet/bubbletea"

	"github.com/raiguard/pesticide/config"
	"github.com/raiguard/pesticide/ui"
)

func main() {
	f, err := tea.LogToFile("/tmp/pesticide.log", "")
	if err != nil {
		panic(err)
	}
	defer f.Close()

	ui := ui.New(config.New("pesticide.json"))
	if _, err := ui.Run(); err != nil {
		panic(err)
	}
}
