package main

import (
	"github.com/charmbracelet/bubbles/textinput"
	tea "github.com/charmbracelet/bubbletea"

	"github.com/raiguard/pesticide/config"
)

type model struct {
	config         config.Config
	commandHistory []string
	historyIndex   int

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
			m.commandHistory = append(m.commandHistory, m.textinput.Value())
			m.textinput.SetValue("")
			m.historyIndex = -1
			cmds = append(cmds, tea.Println("(pesticide) ", m.commandHistory[len(m.commandHistory)-1]))
		case tea.KeyUp:
			if m.historyIndex == -1 {
				m.historyIndex = len(m.commandHistory) - 1
			} else {
				m.historyIndex--
				if m.historyIndex < 0 {
					m.historyIndex = 0
				}
			}
			if m.historyIndex > -1 {
				m.textinput.SetValue(m.commandHistory[m.historyIndex])
				m.textinput.SetCursor(999)
			}
		case tea.KeyDown:
			if m.historyIndex < len(m.commandHistory) {
				m.historyIndex++
			}
			if m.historyIndex == len(m.commandHistory) {
				m.textinput.SetValue("")
			} else {
				m.textinput.SetValue(m.commandHistory[m.historyIndex])
			}
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
		config:         config.New("pesticide.json"),
		commandHistory: []string{},
		historyIndex:   -1,
		textinput:      textinput.Model{},
	})
	if _, err := p.Run(); err != nil {
		panic(err)
	}
}
