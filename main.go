package main

import (
	"log"

	"github.com/charmbracelet/bubbles/textinput"
	tea "github.com/charmbracelet/bubbletea"

	"github.com/raiguard/pesticide/command"
	"github.com/raiguard/pesticide/config"
	"github.com/raiguard/pesticide/ui"
)

type model struct {
	commandHistory ui.CommandHistory
	config         config.Config

	textinput textinput.Model
}

func (m *model) Init() tea.Cmd {
	m.textinput = textinput.New()
	m.textinput.Prompt = "(pesticide) "
	m.textinput.Focus()
	return tea.Batch(textinput.Blink, tea.Println("Type a command and press <ret> to submit, or press <ctrl-d> to exit"))
}

func (m *model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	var cmds []tea.Cmd
	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch msg.Type {
		case tea.KeyCtrlC:
			m.textinput.SetValue("")
		case tea.KeyCtrlD:
			return m, tea.Quit
		case tea.KeyEnter:
			input := m.textinput.Value()
			m.textinput.SetValue("")
			m.commandHistory.Append(input)
			cmds = append(cmds, tea.Println("(pesticide) ", input))
			cmd, err := command.Parse(input)
			if err != nil {
				cmds = append(cmds, tea.Println(err))
				break
			}
			cmds = append(cmds, tea.Printf("Command: %s", cmd.Type))
			// TODO: Parse command to do DAP stuffs!
		case tea.KeyUp:
			m.commandHistory.Up()
			m.textinput.SetValue(m.commandHistory.Get())
			m.textinput.SetCursor(999)
		case tea.KeyDown:
			m.commandHistory.Down()
			m.textinput.SetValue(m.commandHistory.Get())
			m.textinput.SetCursor(999)
		}
	}
	var cmd tea.Cmd
	m.textinput, cmd = m.textinput.Update(msg)
	if cmd != nil {
		cmds = append(cmds, cmd)
	}
	log.Printf("%+v", cmds)
	return m, tea.Sequence(cmds...)
}

func (m *model) View() string {
	return m.textinput.View()
}

func main() {
	f, err := tea.LogToFile("/tmp/pesticide.log", "tea")
	if err != nil {
		panic(err)
	}
	defer f.Close()
	p := tea.NewProgram(&model{
		commandHistory: ui.NewCommandHistory(),
		config:         config.New("pesticide.json"),
		textinput:      textinput.Model{},
	})
	if _, err := p.Run(); err != nil {
		panic(err)
	}
}
