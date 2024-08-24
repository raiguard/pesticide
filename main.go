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
	f, err := tea.LogToFile("/tmp/pesticide.log", "")
	if err != nil {
		panic(err)
	}
	defer f.Close()
	fromUI := make(chan command.Command)
	config := config.New("pesticide.json")
	p := ui.New(config, fromUI)
	go func() {
		if _, err := p.Run(); err != nil {
			panic(err)
		}
	}()
	adapters := map[string]*adapter.Adapter{}
	var focusedAdapter *adapter.Adapter
cmdLoop:
	for cmd := range fromUI {
		switch cmd := cmd.(type) {
		case command.Launch:
			adapterConfig, ok := config.Adapters[cmd.Name]
			if !ok {
				p.Printf("Unknown debug adapter %s", cmd.Name)
				continue cmdLoop
			}
			a, err := adapter.New(adapterConfig)
			if err != nil {
				p.Println(err)
				continue cmdLoop
			}
			adapters[cmd.Name] = a
			focusedAdapter = a
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
		case command.Pause:
			focusedAdapter.Send(&dap.PauseRequest{
				Request: focusedAdapter.NewRequest("pause"),
			})
		case command.Continue:
			focusedAdapter.Send(&dap.ContinueRequest{
				Request: focusedAdapter.NewRequest("continue"),
			})
		}
	}
	for _, adapter := range adapters {
		adapter.Shutdown()
	}
	p.Quit()
	p.Wait()
}
