package ui

import (
	"github.com/charmbracelet/bubbles/textinput"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/raiguard/pesticide/adapter"
	"github.com/raiguard/pesticide/config"
)

type Model struct {
	config         config.Config
	adapters       map[string]*adapter.Adapter
	focusedAdapter *adapter.Adapter

	prompt prompt
}

func New(config config.Config) *tea.Program {
	return tea.NewProgram(&Model{
		config:   config,
		adapters: map[string]*adapter.Adapter{},
	})
}

func (m *Model) Init() tea.Cmd {
	m.prompt.Init()
	return tea.Batch(textinput.Blink, tea.Println("Type a command and press <ret> to submit"))
}

func (m *Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	var cmds []tea.Cmd
	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch msg.Type {
		case tea.KeyCtrlD:
			return m, tea.Quit // Escape hatch for development
		}
	case adapter.Msg:
		cmds = append(cmds, m.handleAdapterMessage(msg))
	}
	cmd := m.prompt.Update(msg, &cmds)
	if cmd != nil {
		cmds = append(cmds, m.handleCommand(cmd))
	}

	// Bubbletea does not validate sequence commands, so we must do it ourselves to avoid high CPU usage.
	var validCmds []tea.Cmd //nolint:prealloc
	for _, c := range cmds {
		if c == nil {
			continue
		}
		validCmds = append(validCmds, c)
	}
	switch len(validCmds) {
	case 0:
		return m, nil
	case 1:
		return m, validCmds[0]
	default:
		return m, tea.Sequence(cmds...)
	}
}

func (m *Model) View() string {
	return m.prompt.View()
}
