package main

import (
	tea "github.com/charmbracelet/bubbletea"
	"github.com/google/go-dap"

	"github.com/raiguard/pesticide/adapter"
	"github.com/raiguard/pesticide/command"
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
	adapters := map[string]*adapter.Adapter{}
msgLoop:
	for msg := range fromUI {
		switch msg := msg.(type) {
		case command.Command:
			switch msg.Type {
			case command.CommandLaunch:
				adapterConfig, ok := config.Adapters[msg.Data]
				if !ok {
					p.Printf("Unknown debug adapter %s", msg.Data)
					continue msgLoop
				}
				a, err := adapter.New(adapterConfig)
				if err != nil {
					p.Println(err)
					continue msgLoop
				}
				adapters[msg.Data] = a
				a.Send(&dap.InitializeRequest{
					Request: a.NewRequest("initialize"),
					Arguments: dap.InitializeRequestArguments{
						ClientID:        "pesticide",
						ClientName:      "Pesticide",
						Locale:          "en-US",
						PathFormat:      "path",
						LinesStartAt1:   true,
						ColumnsStartAt1: true,
					},
				})
				p.Println("Sent initialization request")
			}
		}
	}
	p.Quit()
	p.Wait()
	for _, adapter := range adapters {
		adapter.Shutdown()
	}
}
