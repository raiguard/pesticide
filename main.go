package main

import (
	tea "github.com/charmbracelet/bubbletea"

	"github.com/raiguard/pesticide/config"
	"github.com/raiguard/pesticide/ui"
)

func main() {
	f, err := tea.LogToFile("/tmp/pesticide.log", "tea")
	if err != nil {
		panic(err)
	}
	defer f.Close()
	fromUI := make(chan tea.Msg)
	config := config.New("pesticide.json")
	p := ui.New(config, fromUI)
	go func() {
		if _, err := p.Run(); err != nil {
			panic(err)
		}
	}()
	for msg := range fromUI {
		p.Printf("Received message from UI: %+v", msg)
	}
	p.Quit()
	p.Wait()
}
