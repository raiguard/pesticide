package ui

import (
	"log"

	"github.com/charmbracelet/bubbles/textinput"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/raiguard/pesticide/command"
	"github.com/raiguard/pesticide/config"
	"github.com/raiguard/pesticide/message"
)

type Model struct {
	config config.Config // TODO: Is this needed in the UI?
	output chan message.Message

	commandHistory CommandHistory
	textinput      textinput.Model
}

func New(config config.Config, output chan message.Message) *tea.Program {
	return tea.NewProgram(&Model{
		config:         config,
		output:         output,
		commandHistory: CommandHistory{},
		textinput:      textinput.Model{},
	})
}

func (m *Model) Init() tea.Cmd {
	m.textinput = textinput.New()
	m.textinput.Prompt = lipgloss.NewStyle().Foreground(lipgloss.Color("12")).Render("(pesticide) ")
	m.textinput.Focus()
	return tea.Batch(textinput.Blink, tea.Println("Type a command and press <ret> to submit, or press <ctrl-d> to exit"))
}

func (m *Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	var cmds []tea.Cmd
	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch msg.Type {
		case tea.KeyCtrlC:
			m.textinput.SetValue("")
		case tea.KeyCtrlD:
			// Closing the outgoing channel will cause the main thread to shut down the UI and clean up.
			// TODO: Send "quit" pesticide command to shut down adapters etc.
			close(m.output)
			return m, nil
		case tea.KeyEnter:
			cmds = append(cmds, tea.Println(m.textinput.View()))
			input := m.textinput.Value()
			m.textinput.SetValue("")
			m.commandHistory.Append(input)
			cmd, err := command.Parse(input)
			if err != nil {
				cmds = append(cmds, tea.Println(err))
				break
			}
			log.Printf("Command: %+v", cmd)
			m.output <- message.Command{Cmd: cmd}
		case tea.KeyUp:
			m.commandHistory.Up()
			m.textinput.SetValue(m.commandHistory.Get())
			m.textinput.SetCursor(999)
		case tea.KeyDown:
			m.commandHistory.Down()
			m.textinput.SetValue(m.commandHistory.Get())
			m.textinput.SetCursor(999)
		}
	case message.Print:
		cmds = append(cmds, tea.Println(msg.Obj...))
	}
	var cmd tea.Cmd
	m.textinput, cmd = m.textinput.Update(msg)
	if cmd != nil {
		cmds = append(cmds, cmd)
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
	return m.textinput.View()
}
