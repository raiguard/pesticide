package command

import (
	"errors"
	"fmt"

	"github.com/google/shlex"
)

// Command represents a command given to Pesticide to perform an action.
// Commands are of the form <noun> <verb> [flags...] [arguments...]
type Command struct {
	Noun, Verb string
	Flags      map[string]string
	Args       []string
}

// Parse parses an input string and returns a Command.
func Parse(input string) (*Command, error) {
	args, err := shlex.Split(input)
	if err != nil {
		return nil, err
	}
	if len(args) == 0 {
		return nil, errors.New("Empty command")
	}
	switch args[0] {
	case "adapter":
		return &Command{Noun: "adapter"}, nil
	case "breakpoint":
		return &Command{Noun: "breakpoint"}, nil
	case "quit":
		return &Command{Noun: "quit"}, nil
	default:
		return nil, errors.New(fmt.Sprintf("'%s' is not a valid command", args[0]))
	}
}
