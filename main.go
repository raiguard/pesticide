package main

import (
	"github.com/charmbracelet/bubbles/textinput"
	tea "github.com/charmbracelet/bubbletea"

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
	return textinput.Blink
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
			m.commandHistory.Append(m.textinput.Value())
			cmds = append(cmds, tea.Println("(pesticide) ", m.textinput.Value()))
			m.textinput.SetValue("")
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
	cmds = append(cmds, cmd)
	return m, tea.Batch(cmds...)
}

func (m *model) View() string {
	return m.textinput.View()
}

func main() {
	p := tea.NewProgram(&model{
		commandHistory: ui.NewCommandHistory(),
		config:         config.New("pesticide.json"),
		textinput:      textinput.Model{},
	})
	if _, err := p.Run(); err != nil {
		panic(err)
	}
}
