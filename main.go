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
	input := make(chan message.Message)
	output := make(chan message.Message)
	config := config.New("pesticide.json")
	p := ui.New(config, input)
	go func() {
		if _, err := p.Run(); err != nil {
			panic(err)
		}
	}()
	router := router.New(input, output, config)
	go router.Run()
	for msg := range output {
		p.Send(msg)
	}
	p.Quit()
	p.Wait()
}
