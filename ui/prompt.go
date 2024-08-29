package ui

import (
	"log"

	"github.com/charmbracelet/bubbles/textinput"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/raiguard/pesticide/command"
)

type prompt struct {
	commandHistory CommandHistory
	textinput      textinput.Model
}

func (p *prompt) Init() {
	p.textinput = textinput.New()
	p.textinput.Prompt = lipgloss.NewStyle().Foreground(lipgloss.Color("12")).Render("(pesticide) ")
	p.textinput.Focus()
}

func (p *prompt) Update(msg tea.Msg, cmds *[]tea.Cmd) command.Command {
	{
		var cmd tea.Cmd
		p.textinput, cmd = p.textinput.Update(msg)
		if cmd != nil {
			*cmds = append(*cmds, cmd)
		}
	}
	keyMsg, ok := msg.(tea.KeyMsg)
	if !ok {
		return nil
	}
	switch keyMsg.Type {
	case tea.KeyCtrlC:
		p.textinput.SetValue("")
	case tea.KeyEnter:
		input := p.textinput.Value()
		*cmds = append(*cmds, tea.Println(p.textinput.Prompt, input))
		p.textinput.SetValue("")
		p.commandHistory.Append(input)
		cmd, err := command.Parse(input)
		if err != nil {
			*cmds = append(*cmds, tea.Println(err))
			break
		}
		log.Printf("Command: %s", input)
		return cmd
	case tea.KeyUp:
		p.commandHistory.Up()
		p.textinput.SetValue(p.commandHistory.Get())
		p.textinput.SetCursor(999)
	case tea.KeyDown:
		p.commandHistory.Down()
		p.textinput.SetValue(p.commandHistory.Get())
		p.textinput.SetCursor(999)
	}
	return nil
}

func (p *prompt) View() string {
	return p.textinput.View()
}
