package ui

type CommandHistory struct {
	history []string
	pos     int
}

func NewCommandHistory() CommandHistory {
	return CommandHistory{
		history: []string{},
		pos:     0,
	}
}

func (c *CommandHistory) Append(item string) {
	c.history = append(c.history, item)
	c.pos = len(c.history)
}

func (c *CommandHistory) Up() {
	if c.pos == 0 {
		return
	}
	c.pos--
}

func (c *CommandHistory) Down() {
	if c.pos > len(c.history)-1 {
		return
	}
	c.pos++
}

func (c *CommandHistory) Get() string {
	if c.pos > len(c.history)-1 {
		return ""
	}
	return c.history[c.pos]
}
